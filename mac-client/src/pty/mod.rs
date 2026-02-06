//! PTY module for spawning and managing shell sessions.
//!
//! This module provides pseudo-terminal support for capturing terminal I/O.
//! Each PTY session runs a shell and forwards I/O to/from the relay.

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

/// Events emitted by PTY sessions.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// A new PTY session was created.
    SessionCreated { session_id: String, name: String },
    /// A PTY session exited.
    SessionExited { session_id: String },
    /// Terminal output from a PTY session.
    Output { session_id: String, data: Vec<u8> },
}

/// Commands that can be sent to PTY sessions.
#[derive(Debug)]
pub enum PtyCommand {
    /// Spawn a new PTY session with the user's default shell.
    SpawnShell,
    /// Write input to a PTY session.
    Write { session_id: String, data: Vec<u8> },
    /// Resize a PTY session.
    Resize { session_id: String, cols: u16, rows: u16 },
    /// Close a PTY session.
    Close { session_id: String },
    /// Shutdown all PTY sessions.
    Shutdown,
}

/// Manages multiple PTY sessions.
pub struct PtyManager {
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
    event_tx: mpsc::UnboundedSender<PtyEvent>,
    command_tx: mpsc::UnboundedSender<PtyCommand>,
}

struct PtySession {
    #[allow(dead_code)]
    pair: PtyPair,
    writer: Box<dyn Write + Send>,
    name: String,
}

impl PtyManager {
    /// Create a new PTY manager.
    /// Returns the manager and a receiver for PTY events.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<PtyEvent>, mpsc::UnboundedSender<PtyCommand>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let manager = Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            event_tx: event_tx.clone(),
            command_tx: command_tx.clone(),
        };

        // Start command processor
        let sessions = manager.sessions.clone();
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            process_commands(command_rx, sessions, event_tx_clone).await;
        });

        (manager, event_rx, command_tx)
    }

    /// Get a sender for PTY commands.
    pub fn command_sender(&self) -> mpsc::UnboundedSender<PtyCommand> {
        self.command_tx.clone()
    }
}

async fn process_commands(
    mut command_rx: mpsc::UnboundedReceiver<PtyCommand>,
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
    event_tx: mpsc::UnboundedSender<PtyEvent>,
) {
    while let Some(cmd) = command_rx.recv().await {
        match cmd {
            PtyCommand::SpawnShell => {
                spawn_shell(sessions.clone(), event_tx.clone()).await;
            }
            PtyCommand::Write { session_id, data } => {
                write_to_session(&sessions, &session_id, &data).await;
            }
            PtyCommand::Resize { session_id, cols, rows } => {
                resize_session(&sessions, &session_id, cols, rows).await;
            }
            PtyCommand::Close { session_id } => {
                close_session(&sessions, &session_id, &event_tx).await;
            }
            PtyCommand::Shutdown => {
                info!("PTY manager shutting down");
                let mut sessions_guard = sessions.lock().await;
                sessions_guard.clear();
                break;
            }
        }
    }
}

async fn spawn_shell(
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
    event_tx: mpsc::UnboundedSender<PtyEvent>,
) {
    // Run PTY creation in blocking task (PTY operations are blocking)
    let result = tokio::task::spawn_blocking(move || {
        let pty_system = native_pty_system();

        // Create PTY with reasonable default size
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Get user's default shell
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

        // Build command
        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");

        // Get current directory for session name
        let cwd = std::env::current_dir()
            .map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "~".to_string()))
            .unwrap_or_else(|_| "~".to_string());

        // Spawn the shell
        let _child = pair.slave.spawn_command(cmd)?;

        // Get writer for input
        let writer = pair.master.take_writer()?;

        // Get reader for output
        let reader = pair.master.try_clone_reader()?;

        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((pair, writer, reader, shell, cwd))
    }).await;

    match result {
        Ok(Ok((pair, writer, reader, shell, cwd))) => {
            let session_id = uuid::Uuid::new_v4().to_string();
            let name = format!("{} [{}]", cwd, std::process::id());

            info!(session_id = %session_id, shell = %shell, "PTY session spawned");

            // Send session created event
            let _ = event_tx.send(PtyEvent::SessionCreated {
                session_id: session_id.clone(),
                name: name.clone(),
            });

            // Store session
            {
                let mut sessions_guard = sessions.lock().await;
                sessions_guard.insert(session_id.clone(), PtySession {
                    pair,
                    writer,
                    name,
                });
            }

            // Spawn output reader task
            let session_id_clone = session_id.clone();
            let event_tx_clone = event_tx.clone();
            let sessions_clone = sessions.clone();

            tokio::task::spawn_blocking(move || {
                read_pty_output(reader, session_id_clone, event_tx_clone, sessions_clone);
            });
        }
        Ok(Err(e)) => {
            error!("Failed to spawn PTY: {}", e);
        }
        Err(e) => {
            error!("PTY spawn task panicked: {}", e);
        }
    }
}

fn read_pty_output(
    mut reader: Box<dyn Read + Send>,
    session_id: String,
    event_tx: mpsc::UnboundedSender<PtyEvent>,
    sessions: Arc<Mutex<HashMap<String, PtySession>>>,
) {
    let mut buf = [0u8; 4096];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                // EOF - PTY closed
                debug!(session_id = %session_id, "PTY EOF");
                break;
            }
            Ok(n) => {
                let data = buf[..n].to_vec();
                if event_tx.send(PtyEvent::Output {
                    session_id: session_id.clone(),
                    data,
                }).is_err() {
                    // Event channel closed
                    break;
                }
            }
            Err(e) => {
                debug!(session_id = %session_id, error = %e, "PTY read error");
                break;
            }
        }
    }

    // Send session exited event
    let _ = event_tx.send(PtyEvent::SessionExited {
        session_id: session_id.clone(),
    });

    // Remove session (use std::thread to avoid async in blocking context)
    let sessions_clone = sessions.clone();
    let session_id_clone = session_id.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Handle::try_current();
        if let Ok(rt) = rt {
            rt.block_on(async {
                let mut sessions_guard = sessions_clone.lock().await;
                sessions_guard.remove(&session_id_clone);
            });
        }
    });

    info!(session_id = %session_id, "PTY session exited");
}

async fn write_to_session(
    sessions: &Arc<Mutex<HashMap<String, PtySession>>>,
    session_id: &str,
    data: &[u8],
) {
    let mut sessions_guard = sessions.lock().await;
    if let Some(session) = sessions_guard.get_mut(session_id) {
        if let Err(e) = session.writer.write_all(data) {
            warn!(session_id = %session_id, error = %e, "Failed to write to PTY");
        }
    } else {
        debug!(session_id = %session_id, "Write to unknown session");
    }
}

async fn resize_session(
    sessions: &Arc<Mutex<HashMap<String, PtySession>>>,
    session_id: &str,
    cols: u16,
    rows: u16,
) {
    let sessions_guard = sessions.lock().await;
    if let Some(session) = sessions_guard.get(session_id) {
        if let Err(e) = session.pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            warn!(session_id = %session_id, error = %e, "Failed to resize PTY");
        } else {
            debug!(session_id = %session_id, cols = cols, rows = rows, "PTY resized");
        }
    }
}

async fn close_session(
    sessions: &Arc<Mutex<HashMap<String, PtySession>>>,
    session_id: &str,
    event_tx: &mpsc::UnboundedSender<PtyEvent>,
) {
    let mut sessions_guard = sessions.lock().await;
    if sessions_guard.remove(session_id).is_some() {
        info!(session_id = %session_id, "PTY session closed");
        let _ = event_tx.send(PtyEvent::SessionExited {
            session_id: session_id.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pty_command_variants() {
        let cmd = PtyCommand::SpawnShell;
        assert!(matches!(cmd, PtyCommand::SpawnShell));

        let cmd = PtyCommand::Write {
            session_id: "test".into(),
            data: vec![0x41],
        };
        assert!(matches!(cmd, PtyCommand::Write { .. }));
    }
}
