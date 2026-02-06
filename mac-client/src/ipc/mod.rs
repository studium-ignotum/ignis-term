//! IPC module for shell integration connections.
//!
//! This module provides a Unix domain socket server that shell integration
//! scripts (Phase 6) connect to for terminal session management.

mod session;

pub use session::{Session, ShellMessage, ShellRegistration};

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Socket path for shell integration connections.
pub const SOCKET_PATH: &str = "/tmp/terminal-remote.sock";

/// Events sent from IPC server to main thread.
#[derive(Debug, Clone)]
pub enum IpcEvent {
    /// A new shell session connected.
    SessionConnected { session_id: String, name: String },
    /// A shell session disconnected.
    SessionDisconnected { session_id: String },
    /// A shell session was renamed (directory change).
    SessionRenamed { session_id: String, name: String },
    /// Total session count changed.
    SessionCountChanged(usize),
    /// Terminal data received from a shell session.
    TerminalData { session_id: String, data: Vec<u8> },
    /// An error occurred in the IPC server.
    Error(String),
}

/// Commands sent to IPC server for writing to sessions.
#[derive(Debug, Clone)]
pub enum IpcCommand {
    /// Write terminal data to a specific session.
    WriteToSession { session_id: String, data: Vec<u8> },
}

/// IPC server that manages Unix socket connections from shell integrations.
pub struct IpcServer {
    listener: UnixListener,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    event_tx: Sender<IpcEvent>,
    command_rx: tokio::sync::mpsc::UnboundedReceiver<IpcCommand>,
}

impl IpcServer {
    /// Create a new IPC server.
    ///
    /// Removes any existing stale socket file and binds to SOCKET_PATH.
    ///
    /// # Arguments
    /// * `event_tx` - Channel sender for emitting IpcEvents to the main thread
    /// * `command_rx` - Channel receiver for commands (e.g., write to session)
    pub async fn new(
        event_tx: Sender<IpcEvent>,
        command_rx: tokio::sync::mpsc::UnboundedReceiver<IpcCommand>,
    ) -> std::io::Result<Self> {
        // Remove existing socket file if it exists (stale socket cleanup)
        if std::path::Path::new(SOCKET_PATH).exists() {
            warn!("Removing stale socket file at {}", SOCKET_PATH);
            std::fs::remove_file(SOCKET_PATH)?;
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;
        info!("IPC server listening on {}", SOCKET_PATH);

        Ok(Self {
            listener,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            command_rx,
        })
    }

    /// Run the IPC server, accepting connections and handling commands.
    ///
    /// This method spawns a new task for each incoming connection and
    /// processes commands for writing to sessions.
    pub async fn run(&mut self) {
        info!("IPC server starting accept loop");

        loop {
            tokio::select! {
                // Accept new connections
                result = self.listener.accept() => {
                    match result {
                        Ok((stream, _addr)) => {
                            debug!("New connection accepted");
                            let event_tx = self.event_tx.clone();
                            let sessions = Arc::clone(&self.sessions);
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(stream, event_tx, sessions).await {
                                    error!("Connection handler error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Failed to accept connection: {}", e);
                            let _ = self.event_tx.send(IpcEvent::Error(format!(
                                "Accept error: {}",
                                e
                            )));
                        }
                    }
                }

                // Handle commands for writing to sessions
                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(IpcCommand::WriteToSession { session_id, data }) => {
                            if let Err(e) = self.write_to_session(&session_id, &data).await {
                                warn!("Failed to write to session {}: {}", session_id, e);
                            }
                        }
                        None => {
                            info!("IPC command channel closed");
                            break;
                        }
                    }
                }
            }
        }
    }

    /// Write terminal data to a specific session.
    async fn write_to_session(&self, session_id: &str, data: &[u8]) -> std::io::Result<()> {
        let mut sessions = self.sessions.lock().await;
        if let Some(session) = sessions.get_mut(session_id) {
            debug!("Writing {} bytes to session {}", data.len(), session_id);
            session.write(data).await
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Session not found: {}", session_id),
            ))
        }
    }

    /// Handle a single connection from a shell integration.
    ///
    /// Splits the stream, reads registration, stores session, and spawns
    /// a read task for terminal data.
    async fn handle_connection(
        stream: UnixStream,
        event_tx: Sender<IpcEvent>,
        sessions: Arc<Mutex<HashMap<String, Session>>>,
    ) -> std::io::Result<()> {
        let session_id = Uuid::new_v4().to_string();
        debug!("Handling connection with session_id: {}", session_id);

        // Split the stream for bidirectional communication
        let (read_half, write_half) = stream.into_split();

        // Read initial registration message (JSON on first line)
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        match reader.read_line(&mut line).await {
            Ok(0) => {
                // Connection closed before sending registration
                debug!("Connection closed before registration");
                return Ok(());
            }
            Ok(_) => {
                // Parse registration message
                match serde_json::from_str::<ShellRegistration>(&line) {
                    Ok(registration) => {
                        let name = registration.name.clone();
                        info!(
                            "Shell registered: {} (shell={}, pid={})",
                            name, registration.shell, registration.pid
                        );

                        // Create session and store it
                        let session = Session::new(
                            session_id.clone(),
                            registration,
                            write_half,
                        );

                        {
                            let mut sessions_guard = sessions.lock().await;
                            sessions_guard.insert(session_id.clone(), session);
                            let count = sessions_guard.len();

                            // Send connected event with accurate count
                            let _ = event_tx.send(IpcEvent::SessionConnected {
                                session_id: session_id.clone(),
                                name: name.clone(),
                            });
                            let _ = event_tx.send(IpcEvent::SessionCountChanged(count));
                        }

                        // Spawn task to read terminal data from shell
                        let event_tx_read = event_tx.clone();
                        let session_id_read = session_id.clone();
                        let sessions_read = Arc::clone(&sessions);

                        Self::read_terminal_data(
                            reader,
                            session_id_read,
                            event_tx_read,
                            sessions_read,
                        )
                        .await;
                    }
                    Err(e) => {
                        warn!("Invalid registration message: {} (error: {})", line.trim(), e);
                    }
                }
            }
            Err(e) => {
                error!("Failed to read registration: {}", e);
                return Err(e);
            }
        }

        Ok(())
    }

    /// Read messages from a shell session continuously.
    ///
    /// Handles JSON protocol messages (like rename) and raw terminal data.
    /// Handles session cleanup on disconnect.
    async fn read_terminal_data(
        mut reader: BufReader<tokio::net::unix::OwnedReadHalf>,
        session_id: String,
        event_tx: Sender<IpcEvent>,
        sessions: Arc<Mutex<HashMap<String, Session>>>,
    ) {
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF - connection closed
                    debug!("Session {} disconnected (EOF)", session_id);
                    break;
                }
                Ok(_) => {
                    // Try to parse as JSON message
                    if let Ok(msg) = serde_json::from_str::<ShellMessage>(&line) {
                        match msg {
                            ShellMessage::Rename { name } => {
                                info!("Session {} renamed to: {}", session_id, name);
                                // Update session name
                                {
                                    let mut sessions_guard = sessions.lock().await;
                                    if let Some(session) = sessions_guard.get_mut(&session_id) {
                                        session.set_name(name.clone());
                                    }
                                }
                                // Notify UI
                                let _ = event_tx.send(IpcEvent::SessionRenamed {
                                    session_id: session_id.clone(),
                                    name,
                                });
                            }
                        }
                    } else {
                        // Not a JSON message, treat as terminal data
                        debug!("Read {} bytes from session {}", line.len(), session_id);
                        let _ = event_tx.send(IpcEvent::TerminalData {
                            session_id: session_id.clone(),
                            data: line.as_bytes().to_vec(),
                        });
                    }
                }
                Err(e) => {
                    debug!("Session {} read error: {}", session_id, e);
                    break;
                }
            }
        }

        // Clean up session and send disconnected event
        {
            let mut sessions_guard = sessions.lock().await;
            sessions_guard.remove(&session_id);
            let count = sessions_guard.len();

            let _ = event_tx.send(IpcEvent::SessionDisconnected {
                session_id: session_id.clone(),
            });
            let _ = event_tx.send(IpcEvent::SessionCountChanged(count));
        }

        info!("Session {} disconnected", session_id);
    }

    /// Get current session count.
    pub async fn session_count(&self) -> usize {
        self.sessions.lock().await.len()
    }
}

impl Drop for IpcServer {
    fn drop(&mut self) {
        info!("IPC server shutting down, cleaning up socket file");
        if let Err(e) = std::fs::remove_file(SOCKET_PATH) {
            // Only warn if the file actually existed
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("Failed to remove socket file: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_constant() {
        assert_eq!(SOCKET_PATH, "/tmp/terminal-remote.sock");
    }

    #[test]
    fn test_ipc_event_debug() {
        let event = IpcEvent::SessionCountChanged(5);
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("SessionCountChanged"));
        assert!(debug_str.contains("5"));
    }

    #[test]
    fn test_ipc_event_terminal_data() {
        let event = IpcEvent::TerminalData {
            session_id: "sess-1".into(),
            data: vec![0x1b, 0x5b, 0x41],
        };
        let debug_str = format!("{:?}", event);
        assert!(debug_str.contains("TerminalData"));
        assert!(debug_str.contains("sess-1"));
    }

    #[test]
    fn test_ipc_command_debug() {
        let cmd = IpcCommand::WriteToSession {
            session_id: "sess-1".into(),
            data: vec![0x68, 0x69],
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("WriteToSession"));
        assert!(debug_str.contains("sess-1"));
    }
}
