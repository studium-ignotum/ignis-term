use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::protocol::ControlMessage;
use crate::state::{AppState, BrowserMessage, MacMessage};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();

    // Wait for first message to determine client type
    let Some(Ok(first_msg)) = receiver.next().await else {
        tracing::debug!("Client disconnected before sending first message");
        return;
    };

    // Parse first message as JSON to determine client type
    let Message::Text(text) = first_msg else {
        tracing::warn!("First message must be JSON Text, got binary");
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&ControlMessage::Error {
                    message: "First message must be JSON".into(),
                })
                .unwrap()
                .into(),
            ))
            .await;
        return;
    };

    let Ok(control_msg) = serde_json::from_str::<ControlMessage>(&text) else {
        tracing::warn!("Invalid JSON in first message");
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&ControlMessage::Error {
                    message: "Invalid JSON".into(),
                })
                .unwrap()
                .into(),
            ))
            .await;
        return;
    };

    match control_msg {
        ControlMessage::Register { client_id } => {
            handle_mac_client(sender, receiver, state, client_id).await;
        }
        ControlMessage::Auth { session_code } => {
            handle_browser(sender, receiver, state, session_code).await;
        }
        _ => {
            tracing::warn!("Unexpected first message type");
            let _ = sender
                .send(Message::Text(
                    serde_json::to_string(&ControlMessage::Error {
                        message: "First message must be Register or Auth".into(),
                    })
                    .unwrap()
                    .into(),
                ))
                .await;
        }
    }
}

/// Handle a mac-client connection
async fn handle_mac_client(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    state: AppState,
    client_id: String,
) {
    // Create channel for receiving messages to send to mac-client
    let (mac_tx, mut mac_rx) = mpsc::channel::<MacMessage>(1000);

    // Register and get session code
    let code = state.register_mac_client(mac_tx);

    // Send registration confirmation
    let response = ControlMessage::Registered { code: code.clone() };
    if sender
        .send(Message::Text(
            serde_json::to_string(&response).unwrap().into(),
        ))
        .await
        .is_err()
    {
        state.remove_session(&code);
        return;
    }

    tracing::info!(code = %code, client_id = %client_id, "Mac-client connected");

    // Spawn task to forward messages from browsers to mac-client
    let code_clone = code.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = mac_rx.recv().await {
            let result = match msg {
                MacMessage::Binary(data) => sender.send(Message::Binary(data.into())).await,
                MacMessage::Text(text) => sender.send(Message::Text(text.into())).await,
            };
            if result.is_err() {
                break;
            }
        }
    });

    // Process incoming messages from mac-client (terminal output)
    while let Some(msg_result) = receiver.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                // Forward terminal output to all connected browsers
                state.broadcast_to_browsers(&code_clone, data.to_vec()).await;
            }
            Ok(Message::Text(text)) => {
                // Handle control messages from mac-client
                if let Ok(ctrl) = serde_json::from_str::<ControlMessage>(&text) {
                    tracing::info!(code = %code_clone, "Mac-client control message: {:?}", ctrl);
                    // Forward session messages to browsers
                    match &ctrl {
                        ControlMessage::SessionList { sessions } => {
                            tracing::info!(code = %code_clone, "Forwarding SessionList ({} sessions) to browsers", sessions.len());
                            state.broadcast_text_to_browsers(&code_clone, &text).await;
                        }
                        ControlMessage::SessionConnected { .. } => {
                            tracing::info!(code = %code_clone, "Forwarding SessionConnected to browsers");
                            state.broadcast_text_to_browsers(&code_clone, &text).await;
                        }
                        ControlMessage::SessionDisconnected { session_id } => {
                            tracing::info!(code = %code_clone, session_id = %session_id, "Forwarding SessionDisconnected to browsers, purging scrollback");
                            state.purge_session_scrollback(&code_clone, &session_id).await;
                            state.broadcast_text_to_browsers(&code_clone, &text).await;
                        }
                        ControlMessage::SessionResize { session_id, cols, rows } => {
                            tracing::debug!(code = %code_clone, session_id = %session_id, cols = cols, rows = rows, "Forwarding SessionResize to browsers");
                            state.broadcast_text_to_browsers(&code_clone, &text).await;
                        }
                        _ => {}
                    }
                } else {
                    tracing::warn!(code = %code_clone, "Failed to parse mac-client message: {}", text);
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::debug!(code = %code_clone, "Mac-client error: {}", e);
                break;
            }
            _ => {} // Ignore ping/pong
        }
    }

    // Notify all browsers that the session is gone, then clean up
    let error_msg = serde_json::to_string(&ControlMessage::Error {
        message: "Session disconnected".into(),
    }).unwrap();
    state.broadcast_text_to_browsers(&code_clone, &error_msg).await;

    send_task.abort();
    state.remove_session(&code_clone);
    tracing::info!(code = %code_clone, "Mac-client disconnected");
}

/// Handle a browser connection
async fn handle_browser(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    state: AppState,
    session_code: String,
) {
    let code = session_code.to_uppercase();

    // Validate session code
    if !state.validate_session_code(&code) {
        let response = ControlMessage::AuthFailed {
            reason: "Invalid session code".into(),
        };
        let _ = sender
            .send(Message::Text(
                serde_json::to_string(&response).unwrap().into(),
            ))
            .await;
        tracing::info!(code = %code, "Browser auth failed - invalid code");
        return;
    }

    // Create channel for receiving messages to send to browser
    let (browser_tx, mut browser_rx) = mpsc::channel::<BrowserMessage>(1000);
    let browser_id = nanoid::nanoid!(8);

    // Register browser with session
    state.add_browser(&code, browser_id.clone(), browser_tx);

    // Send auth success
    let response = ControlMessage::AuthSuccess;
    if sender
        .send(Message::Text(
            serde_json::to_string(&response).unwrap().into(),
        ))
        .await
        .is_err()
    {
        state.remove_browser(&code, &browser_id);
        return;
    }

    tracing::info!(code = %code, browser_id = %browser_id, "Browser connected");

    // Replay scrollback so browser gets terminal history immediately.
    let scrollback = state.get_scrollback(&code).await;
    if !scrollback.is_empty() {
        tracing::info!(code = %code, frames = scrollback.len(), "Replaying scrollback to browser");
        for frame in scrollback {
            if sender.send(Message::Binary(frame.into())).await.is_err() {
                state.remove_browser(&code, &browser_id);
                return;
            }
        }
    }

    // Notify mac-client that a browser connected (so it can send session list)
    let browser_connected_msg = ControlMessage::BrowserConnected {
        browser_id: browser_id.clone(),
    };
    let msg_json = serde_json::to_string(&browser_connected_msg).unwrap();
    tracing::info!(code = %code, "Sending BrowserConnected to mac-client: {}", msg_json);
    state.send_text_to_mac_client(&code, &msg_json).await;

    // Spawn task to forward messages to browser
    let code_clone = code.clone();
    let browser_id_clone = browser_id.clone();
    let send_task = tokio::spawn(async move {
        while let Some(msg) = browser_rx.recv().await {
            let result = match msg {
                BrowserMessage::Binary(data) => sender.send(Message::Binary(data.into())).await,
                BrowserMessage::Text(text) => sender.send(Message::Text(text.into())).await,
            };
            if result.is_err() {
                break;
            }
        }
    });

    // Process incoming messages from browser (keyboard input)
    while let Some(msg_result) = receiver.next().await {
        match msg_result {
            Ok(Message::Binary(data)) => {
                // Forward keyboard input to mac-client
                state.send_to_mac_client(&code_clone, data.to_vec()).await;
            }
            Ok(Message::Text(text)) => {
                // Handle control messages from browser
                if let Ok(ctrl) = serde_json::from_str::<ControlMessage>(&text) {
                    tracing::debug!(code = %code_clone, "Browser control: {:?}", ctrl);
                    match ctrl {
                        ControlMessage::CloseSession { session_id } => {
                            // Forward to mac-client as binary frame:
                            // [session_id_len][session_id][payload]
                            let payload = b"{\"type\":\"close_session\"}";
                            let mut frame = Vec::with_capacity(1 + session_id.len() + payload.len());
                            frame.push(session_id.len() as u8);
                            frame.extend_from_slice(session_id.as_bytes());
                            frame.extend_from_slice(payload);
                            state.send_to_mac_client(&code_clone, frame).await;
                        }
                        ControlMessage::CreateSession => {
                            state.send_text_to_mac_client(&code_clone, &text).await;
                        }
                        _ => {}
                    }
                }
            }
            Ok(Message::Close(_)) => break,
            Err(e) => {
                tracing::debug!(code = %code_clone, "Browser error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    send_task.abort();
    state.remove_browser(&code_clone, &browser_id_clone);
    tracing::info!(code = %code_clone, browser_id = %browser_id_clone, "Browser disconnected");
}
