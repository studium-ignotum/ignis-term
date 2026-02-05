//! IPC module for shell integration connections.
//!
//! This module provides a Unix domain socket server that shell integration
//! scripts (Phase 6) connect to for terminal session management.

mod session;

pub use session::{Session, ShellRegistration};

use serde_json;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
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
    /// Total session count changed.
    SessionCountChanged(usize),
    /// An error occurred in the IPC server.
    Error(String),
}

/// IPC server that manages Unix socket connections from shell integrations.
pub struct IpcServer {
    listener: UnixListener,
    sessions: HashMap<String, Session>,
    event_tx: Sender<IpcEvent>,
}

impl IpcServer {
    /// Create a new IPC server.
    ///
    /// Removes any existing stale socket file and binds to SOCKET_PATH.
    pub async fn new(event_tx: Sender<IpcEvent>) -> std::io::Result<Self> {
        // Remove existing socket file if it exists (stale socket cleanup)
        if std::path::Path::new(SOCKET_PATH).exists() {
            warn!("Removing stale socket file at {}", SOCKET_PATH);
            std::fs::remove_file(SOCKET_PATH)?;
        }

        let listener = UnixListener::bind(SOCKET_PATH)?;
        info!("IPC server listening on {}", SOCKET_PATH);

        Ok(Self {
            listener,
            sessions: HashMap::new(),
            event_tx,
        })
    }

    /// Run the IPC server, accepting connections in a loop.
    ///
    /// This method spawns a new task for each incoming connection.
    pub async fn run(&mut self) {
        info!("IPC server starting accept loop");

        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    debug!("New connection accepted");
                    let event_tx = self.event_tx.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, event_tx).await {
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
    }

    /// Handle a single connection from a shell integration.
    ///
    /// Reads the registration message, tracks the session, and waits for disconnect.
    async fn handle_connection(
        stream: UnixStream,
        event_tx: Sender<IpcEvent>,
    ) -> std::io::Result<()> {
        let session_id = Uuid::new_v4().to_string();
        debug!("Handling connection with session_id: {}", session_id);

        // Read initial registration message (JSON on first line)
        let mut reader = BufReader::new(stream);
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

                        // Send connected event
                        let _ = event_tx.send(IpcEvent::SessionConnected {
                            session_id: session_id.clone(),
                            name: name.clone(),
                        });

                        // Note: We don't have accurate session count without shared state
                        // This will be handled properly in Plan 05-04 integration
                        let _ = event_tx.send(IpcEvent::SessionCountChanged(1));

                        // Wait for the connection to close (stream drop or read error)
                        // In a full implementation, we'd handle bidirectional communication here
                        let mut buf = String::new();
                        loop {
                            match reader.read_line(&mut buf).await {
                                Ok(0) => {
                                    // EOF - connection closed
                                    debug!("Session {} disconnected (EOF)", session_id);
                                    break;
                                }
                                Ok(_) => {
                                    // Got some data - in Phase 6 we'll handle terminal data
                                    buf.clear();
                                }
                                Err(e) => {
                                    debug!("Session {} read error: {}", session_id, e);
                                    break;
                                }
                            }
                        }

                        // Send disconnected event
                        let _ = event_tx.send(IpcEvent::SessionDisconnected {
                            session_id: session_id.clone(),
                        });
                        let _ = event_tx.send(IpcEvent::SessionCountChanged(0));

                        info!("Session {} disconnected", session_id);
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

    /// Get current session count.
    pub fn session_count(&self) -> usize {
        self.sessions.len()
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
}
