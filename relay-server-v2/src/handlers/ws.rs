use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};

use crate::protocol::ControlMessage;
use crate::state::AppState;

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

/// Handle a mac-client connection (placeholder for Task 2)
async fn handle_mac_client(
    _sender: futures_util::stream::SplitSink<WebSocket, Message>,
    _receiver: futures_util::stream::SplitStream<WebSocket>,
    _state: AppState,
    _client_id: String,
) {
    // Will be implemented in Task 2
    tracing::info!("Mac-client handler called");
}

/// Handle a browser connection (placeholder for Task 2)
async fn handle_browser(
    _sender: futures_util::stream::SplitSink<WebSocket, Message>,
    _receiver: futures_util::stream::SplitStream<WebSocket>,
    _state: AppState,
    _session_code: String,
) {
    // Will be implemented in Task 2
    tracing::info!("Browser handler called");
}
