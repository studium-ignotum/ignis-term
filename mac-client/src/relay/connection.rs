use crate::protocol::ControlMessage;
use futures_util::{SinkExt, StreamExt};
use std::error::Error;
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Events emitted by the RelayClient to the main thread.
/// These are sent via std::sync::mpsc (not tokio::sync) for AppKit compatibility.
#[derive(Debug, Clone)]
pub enum RelayEvent {
    /// Successfully connected to relay server
    Connected,
    /// Disconnected from relay server (will auto-reconnect)
    Disconnected,
    /// Received session code from relay after registration
    SessionCode(String),
    /// A browser connected to this session
    BrowserConnected(String),
    /// A browser disconnected from this session
    BrowserDisconnected(String),
    /// Error message from relay
    Error(String),
    /// Terminal data received from relay (browser input -> shell)
    TerminalData { session_id: String, data: Vec<u8> },
    /// Close session request from browser
    CloseSession { session_id: String },
    /// Create new session request from browser
    CreateSession,
}

/// Commands sent to RelayClient for sending data to relay.
#[derive(Debug, Clone)]
pub enum RelayCommand {
    /// Send terminal data to relay (shell output -> browser)
    SendTerminalData { session_id: String, data: Vec<u8> },
    /// Send session list to relay (for browser)
    SendSessionList { sessions: Vec<(String, String)> }, // (id, name)
    /// Notify relay that a session connected
    SendSessionConnected { session_id: String, name: String },
    /// Notify relay that a session disconnected
    SendSessionDisconnected { session_id: String },
    /// Notify relay that a session resized (mac -> browser)
    SendSessionResize { session_id: String, cols: u16, rows: u16 },
    /// Disconnect and reconnect to get a new session code
    Reconnect,
}

/// WebSocket client for connecting to the relay server.
/// Handles connection, registration, and auto-reconnect with exponential backoff.
pub struct RelayClient {
    relay_url: String,
    client_id: String,
    event_tx: Sender<RelayEvent>,
    command_rx: tokio::sync::mpsc::UnboundedReceiver<RelayCommand>,
    reconnect_attempts: u32,
}

impl RelayClient {
    /// Create a new RelayClient.
    ///
    /// # Arguments
    /// * `relay_url` - WebSocket URL (e.g., "ws://localhost:3000/ws")
    /// * `event_tx` - Channel sender for emitting RelayEvents to the main thread
    /// * `command_rx` - Channel receiver for commands (e.g., send terminal data)
    pub fn new(
        relay_url: String,
        event_tx: Sender<RelayEvent>,
        command_rx: tokio::sync::mpsc::UnboundedReceiver<RelayCommand>,
    ) -> Self {
        let client_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Created RelayClient with client_id: {}", client_id);

        Self {
            relay_url,
            client_id,
            event_tx,
            command_rx,
            reconnect_attempts: 0,
        }
    }

    /// Main run loop. Connects to relay and auto-reconnects on disconnect.
    /// This method runs forever (until the task is cancelled).
    pub async fn run(&mut self) {
        loop {
            match self.connect_and_run().await {
                Ok(()) => {
                    // Clean disconnect, reconnect immediately
                    tracing::info!("Clean disconnect, reconnecting...");
                }
                Err(e) => {
                    tracing::error!("Connection error: {}", e);
                }
            }

            // Notify main thread of disconnection
            let _ = self.event_tx.send(RelayEvent::Disconnected);

            // Exponential backoff: 1s, 2s, 4s, 8s, 16s, 32s max
            let delay_secs = (2u64).pow(self.reconnect_attempts.min(5));
            tracing::info!("Reconnecting in {}s...", delay_secs);
            tokio::time::sleep(Duration::from_secs(delay_secs)).await;
            self.reconnect_attempts += 1;
        }
    }

    /// Connect to relay, register, and handle messages until disconnected.
    async fn connect_and_run(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        tracing::info!("Connecting to relay: {}", self.relay_url);

        // Connect to WebSocket
        let (ws_stream, _response) = connect_async(&self.relay_url).await?;
        tracing::info!("Connected to relay");

        // Notify main thread
        let _ = self.event_tx.send(RelayEvent::Connected);

        // Reset reconnect attempts on successful connection
        self.reconnect_attempts = 0;

        let (mut write, mut read) = ws_stream.split();

        // Send Register message
        let register_msg = ControlMessage::Register {
            client_id: self.client_id.clone(),
        };
        let json = serde_json::to_string(&register_msg)?;
        tracing::debug!("Sending Register: {}", json);
        write.send(Message::Text(json.into())).await?;

        // Message handling loop - select on both WebSocket and commands
        loop {
            tokio::select! {
                // Handle incoming WebSocket messages
                msg_result = read.next() => {
                    match msg_result {
                        Some(Ok(Message::Text(text))) => {
                            self.handle_text_message(&text)?;
                        }
                        Some(Ok(Message::Binary(data))) => {
                            // Binary messages are terminal I/O from browser
                            // Frame format: 1 byte session_id length + session_id + data
                            self.handle_binary_message(&data);
                        }
                        Some(Ok(Message::Close(frame))) => {
                            tracing::info!("Received close frame: {:?}", frame);
                            break;
                        }
                        Some(Ok(Message::Ping(data))) => {
                            tracing::trace!("Received ping, sending pong");
                            write.send(Message::Pong(data)).await?;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            tracing::trace!("Received pong");
                        }
                        Some(Ok(Message::Frame(_))) => {
                            // Raw frame, typically not used directly
                        }
                        Some(Err(e)) => {
                            tracing::error!("WebSocket error: {}", e);
                            return Err(e.into());
                        }
                        None => {
                            tracing::info!("WebSocket stream ended");
                            break;
                        }
                    }
                }

                // Handle commands from IPC (send terminal data to relay)
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(RelayCommand::SendTerminalData { session_id, data }) => {
                            if let Err(e) = Self::send_terminal_data(&mut write, &session_id, &data).await {
                                tracing::warn!("Failed to send terminal data: {}", e);
                            }
                        }
                        Some(RelayCommand::SendSessionList { sessions }) => {
                            let msg = ControlMessage::SessionList {
                                sessions: sessions.into_iter().map(|(id, name)| {
                                    crate::protocol::SessionInfo { id, name }
                                }).collect(),
                            };
                            let json = serde_json::to_string(&msg).unwrap();
                            tracing::debug!("Sending SessionList: {}", json);
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                tracing::warn!("Failed to send session list: {}", e);
                            }
                        }
                        Some(RelayCommand::SendSessionConnected { session_id, name }) => {
                            let msg = ControlMessage::SessionConnected { session_id, name };
                            let json = serde_json::to_string(&msg).unwrap();
                            tracing::debug!("Sending SessionConnected: {}", json);
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                tracing::warn!("Failed to send session connected: {}", e);
                            }
                        }
                        Some(RelayCommand::SendSessionDisconnected { session_id }) => {
                            let msg = ControlMessage::SessionDisconnected { session_id };
                            let json = serde_json::to_string(&msg).unwrap();
                            tracing::debug!("Sending SessionDisconnected: {}", json);
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                tracing::warn!("Failed to send session disconnected: {}", e);
                            }
                        }
                        Some(RelayCommand::SendSessionResize { session_id, cols, rows }) => {
                            let msg = ControlMessage::SessionResize { session_id, cols, rows };
                            let json = serde_json::to_string(&msg).unwrap();
                            tracing::debug!("Sending SessionResize: {}", json);
                            if let Err(e) = write.send(Message::Text(json.into())).await {
                                tracing::warn!("Failed to send session resize: {}", e);
                            }
                        }
                        Some(RelayCommand::Reconnect) => {
                            tracing::info!("Reconnect requested, closing connection");
                            let _ = write.send(Message::Close(None)).await;
                            break;
                        }
                        None => {
                            tracing::info!("Command channel closed");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Send terminal data to relay for a specific session.
    ///
    /// Frame format: 1 byte session_id length + session_id bytes + terminal data
    async fn send_terminal_data<S>(
        write: &mut S,
        session_id: &str,
        data: &[u8],
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        S: SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
    {
        // Frame format: 1 byte session_id length + session_id + data
        let mut frame = Vec::with_capacity(1 + session_id.len() + data.len());
        frame.push(session_id.len() as u8);
        frame.extend_from_slice(session_id.as_bytes());
        frame.extend_from_slice(data);

        tracing::trace!(
            "Sending terminal data: session={}, {} bytes",
            session_id,
            data.len()
        );
        write.send(Message::Binary(frame.into())).await?;
        Ok(())
    }

    /// Handle a binary message from the relay server (browser input -> shell).
    ///
    /// Frame format: 1 byte session_id length + session_id bytes + payload
    /// Payload can be either:
    /// - Raw terminal input (keystrokes)
    /// - JSON control message (e.g., {"type":"resize","cols":80,"rows":24})
    fn handle_binary_message(&self, data: &[u8]) {
        if data.len() < 2 {
            tracing::warn!("Binary message too short: {} bytes", data.len());
            return;
        }

        let id_len = data[0] as usize;
        if data.len() < 1 + id_len {
            tracing::warn!(
                "Binary message malformed: id_len={} but only {} bytes total",
                id_len,
                data.len()
            );
            return;
        }

        let session_id = String::from_utf8_lossy(&data[1..1 + id_len]).to_string();
        let payload = &data[1 + id_len..];

        // Check if payload is a JSON control message (starts with '{')
        if payload.first() == Some(&b'{') {
            if let Ok(text) = std::str::from_utf8(payload) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                    let msg_type = json.get("type").and_then(|t| t.as_str());

                    if msg_type == Some("close_session") {
                        tracing::info!("Received close_session: session={}", session_id);
                        let _ = self.event_tx.send(RelayEvent::CloseSession {
                            session_id,
                        });
                        return;
                    }
                }
            }
        }

        // Regular terminal input
        tracing::trace!(
            "Received terminal data: session={}, {} bytes",
            session_id,
            payload.len()
        );

        let _ = self.event_tx.send(RelayEvent::TerminalData {
            session_id,
            data: payload.to_vec(),
        });
    }

    /// Handle a text message from the relay server.
    fn handle_text_message(&self, text: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        tracing::debug!("Received text message: {}", text);

        let msg: ControlMessage = serde_json::from_str(text)?;

        match msg {
            ControlMessage::Registered { code } => {
                tracing::info!("Registered with session code: {}", code);
                let _ = self.event_tx.send(RelayEvent::SessionCode(code));
            }
            ControlMessage::BrowserConnected { browser_id } => {
                tracing::info!("Browser connected: {}", browser_id);
                let _ = self.event_tx.send(RelayEvent::BrowserConnected(browser_id));
            }
            ControlMessage::BrowserDisconnected { browser_id } => {
                tracing::info!("Browser disconnected: {}", browser_id);
                let _ = self.event_tx.send(RelayEvent::BrowserDisconnected(browser_id));
            }
            ControlMessage::Error { message } => {
                tracing::error!("Relay error: {}", message);
                let _ = self.event_tx.send(RelayEvent::Error(message));
            }
            ControlMessage::CreateSession => {
                tracing::info!("Received create_session request from browser");
                let _ = self.event_tx.send(RelayEvent::CreateSession);
            }
            // Other message types are for browser<->relay communication
            _ => {
                tracing::warn!("Received unexpected message type: {:?}", msg);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relay_event_variants() {
        // Compile check - events are constructible
        let _connected = RelayEvent::Connected;
        let _disconnected = RelayEvent::Disconnected;
        let _code = RelayEvent::SessionCode("ABC123".into());
        let _browser_conn = RelayEvent::BrowserConnected("browser-id".into());
        let _browser_disc = RelayEvent::BrowserDisconnected("browser-id".into());
        let _error = RelayEvent::Error("test error".into());
        let _terminal_data = RelayEvent::TerminalData {
            session_id: "sess-1".into(),
            data: vec![0x68, 0x65, 0x6c, 0x6c, 0x6f],
        };
    }

    #[test]
    fn test_relay_command_variants() {
        let _send = RelayCommand::SendTerminalData {
            session_id: "sess-1".into(),
            data: vec![0x01, 0x02, 0x03],
        };
    }

    #[test]
    fn test_relay_client_creation() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let (_cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let client = RelayClient::new("ws://localhost:3000/ws".into(), tx, cmd_rx);

        // Verify client_id is a valid UUID
        assert!(uuid::Uuid::parse_str(&client.client_id).is_ok());
        assert_eq!(client.reconnect_attempts, 0);
    }
}
