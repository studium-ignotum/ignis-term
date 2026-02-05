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
}

/// WebSocket client for connecting to the relay server.
/// Handles connection, registration, and auto-reconnect with exponential backoff.
pub struct RelayClient {
    relay_url: String,
    client_id: String,
    event_tx: Sender<RelayEvent>,
    reconnect_attempts: u32,
}

impl RelayClient {
    /// Create a new RelayClient.
    ///
    /// # Arguments
    /// * `relay_url` - WebSocket URL (e.g., "ws://localhost:3000/ws")
    /// * `event_tx` - Channel sender for emitting RelayEvents to the main thread
    pub fn new(relay_url: String, event_tx: Sender<RelayEvent>) -> Self {
        let client_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Created RelayClient with client_id: {}", client_id);

        Self {
            relay_url,
            client_id,
            event_tx,
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

        // Message handling loop
        while let Some(msg_result) = read.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    self.handle_text_message(&text)?;
                }
                Ok(Message::Binary(data)) => {
                    // Binary messages are terminal I/O - forward to shell sessions
                    // Placeholder for Phase 6 shell integration
                    tracing::debug!("Received binary message: {} bytes", data.len());
                }
                Ok(Message::Close(frame)) => {
                    tracing::info!("Received close frame: {:?}", frame);
                    break;
                }
                Ok(Message::Ping(data)) => {
                    tracing::trace!("Received ping, sending pong");
                    write.send(Message::Pong(data)).await?;
                }
                Ok(Message::Pong(_)) => {
                    tracing::trace!("Received pong");
                }
                Ok(Message::Frame(_)) => {
                    // Raw frame, typically not used directly
                }
                Err(e) => {
                    tracing::error!("WebSocket error: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(())
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
    }

    #[test]
    fn test_relay_client_creation() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let client = RelayClient::new("ws://localhost:3000/ws".into(), tx);

        // Verify client_id is a valid UUID
        assert!(uuid::Uuid::parse_str(&client.client_id).is_ok());
        assert_eq!(client.reconnect_attempts, 0);
    }
}
