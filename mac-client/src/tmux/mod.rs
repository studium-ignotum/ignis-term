//! Tmux integration module for managing terminal sessions.
//!
//! Uses tmux as the backend for session management. This module provides:
//! - List existing tmux sessions
//! - Create new tmux sessions
//! - Attach to sessions and forward I/O to browser
//! - Kill sessions
//!
//! tmux handles all PTY management, session persistence, etc.
//! We just provide a remote interface to it.

use portable_pty::{native_pty_system, CommandBuilder, PtyPair, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

/// Find tmux binary, checking Homebrew paths first.
fn tmux_bin() -> &'static str {
    static BIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        for path in &["/opt/homebrew/bin/tmux", "/usr/local/bin/tmux"] {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }
        "tmux".to_string()
    })
}

/// Information about a tmux session.
#[derive(Debug, Clone)]
pub struct TmuxSessionInfo {
    pub name: String,
    pub windows: u32,
    pub created: String,
    pub attached: bool,
}

/// Events emitted by the tmux manager.
#[derive(Debug, Clone)]
pub enum TmuxEvent {
    /// List of available tmux sessions.
    SessionList(Vec<TmuxSessionInfo>),
    /// Attached to a tmux session, now streaming I/O.
    Attached { session_id: String, session_name: String },
    /// Detached from a tmux session.
    Detached { session_id: String },
    /// Terminal output from an attached session.
    Output { session_id: String, data: Vec<u8> },
    /// Error occurred.
    Error(String),
}

/// Commands that can be sent to the tmux manager.
#[derive(Debug)]
pub enum TmuxCommand {
    /// List all tmux sessions.
    ListSessions,
    /// Create a new tmux session and attach to it.
    NewSession { name: Option<String> },
    /// Attach to an existing tmux session.
    Attach { session_name: String },
    /// Attach to ALL existing tmux sessions (auto-attach on startup).
    AttachAll,
    /// Write input to an attached session.
    Write { session_id: String, data: Vec<u8> },
    /// Resize an attached session.
    Resize { session_id: String, cols: u16, rows: u16 },
    /// Force refresh/redraw a session (sends clear + redraw).
    Refresh { session_id: String },
    /// Kill a tmux session by name.
    KillSession { session_name: String },
    /// Kill a tmux session by our internal session_id.
    KillSessionById { session_id: String },
    /// Detach from a session (stop streaming).
    Detach { session_id: String },
    /// Shutdown the tmux manager.
    Shutdown,
}

/// Manages tmux sessions and provides remote access.
pub struct TmuxManager {
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
    #[allow(dead_code)]
    command_tx: mpsc::UnboundedSender<TmuxCommand>,
}

struct AttachedSession {
    #[allow(dead_code)]
    pair: PtyPair,
    writer: Box<dyn Write + Send>,
    session_name: String,
}

impl TmuxManager {
    /// Create a new TmuxManager.
    /// Returns the manager, event receiver, and command sender.
    pub fn new() -> (Self, mpsc::UnboundedReceiver<TmuxEvent>, mpsc::UnboundedSender<TmuxCommand>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let manager = Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            command_tx: command_tx.clone(),
        };

        // Start command processor
        let sessions = manager.sessions.clone();
        let event_tx_clone = event_tx.clone();
        tokio::spawn(async move {
            process_commands(command_rx, sessions, event_tx_clone).await;
        });

        // Start session watcher (auto-detect new tmux sessions)
        let sessions_for_watcher = manager.sessions.clone();
        let event_tx_for_watcher = event_tx.clone();
        tokio::spawn(async move {
            watch_for_new_sessions(sessions_for_watcher, event_tx_for_watcher).await;
        });

        (manager, event_rx, command_tx)
    }

    /// Get a sender for tmux commands.
    pub fn command_sender(&self) -> mpsc::UnboundedSender<TmuxCommand> {
        self.command_tx.clone()
    }
}

async fn process_commands(
    mut command_rx: mpsc::UnboundedReceiver<TmuxCommand>,
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
    event_tx: mpsc::UnboundedSender<TmuxEvent>,
) {
    while let Some(cmd) = command_rx.recv().await {
        match cmd {
            TmuxCommand::ListSessions => {
                let list = list_tmux_sessions();
                let _ = event_tx.send(TmuxEvent::SessionList(list));
            }
            TmuxCommand::NewSession { name } => {
                create_and_attach(sessions.clone(), event_tx.clone(), name).await;
            }
            TmuxCommand::Attach { session_name } => {
                attach_to_session(sessions.clone(), event_tx.clone(), session_name).await;
            }
            TmuxCommand::AttachAll => {
                attach_all_sessions(sessions.clone(), event_tx.clone()).await;
            }
            TmuxCommand::Write { session_id, data } => {
                write_to_session(&sessions, &session_id, &data).await;
            }
            TmuxCommand::Resize { session_id, cols, rows } => {
                resize_session(&sessions, &session_id, cols, rows).await;
            }
            TmuxCommand::Refresh { session_id } => {
                refresh_session(&sessions, &session_id).await;
            }
            TmuxCommand::KillSession { session_name } => {
                kill_tmux_session(&session_name, &event_tx);
            }
            TmuxCommand::KillSessionById { session_id } => {
                kill_session_by_id(&sessions, &session_id, &event_tx).await;
            }
            TmuxCommand::Detach { session_id } => {
                detach_session(&sessions, &session_id, &event_tx).await;
            }
            TmuxCommand::Shutdown => {
                info!("Tmux manager shutting down");
                let mut sessions_guard = sessions.lock().await;
                sessions_guard.clear();
                break;
            }
        }
    }
}

/// Watch for new tmux sessions and auto-attach.
/// Polls every 2 seconds for new sessions.
async fn watch_for_new_sessions(
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
    event_tx: mpsc::UnboundedSender<TmuxEvent>,
) {
    let mut known_sessions: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Initial population of known sessions
    for session in list_tmux_sessions() {
        known_sessions.insert(session.name);
    }

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let current_sessions = list_tmux_sessions();

        for session in &current_sessions {
            // Check if this is a new session we haven't seen
            if !known_sessions.contains(&session.name) {
                info!("Detected new tmux session: {}", session.name);
                known_sessions.insert(session.name.clone());

                // Check if already attached
                let already_attached = {
                    let sessions_guard = sessions.lock().await;
                    sessions_guard.values().any(|s| s.session_name == session.name)
                };

                if !already_attached {
                    info!("Auto-attaching to new tmux session: {}", session.name);
                    attach_to_session(sessions.clone(), event_tx.clone(), session.name.clone()).await;
                }
            }
        }

        // Remove sessions that no longer exist
        let current_names: std::collections::HashSet<String> =
            current_sessions.iter().map(|s| s.name.clone()).collect();
        known_sessions.retain(|name| current_names.contains(name));
    }
}

/// List all tmux sessions.
fn list_tmux_sessions() -> Vec<TmuxSessionInfo> {
    let output = Command::new(tmux_bin())
        .args(["list-sessions", "-F", "#{session_name}|#{session_windows}|#{session_created}|#{session_attached}"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split('|').collect();
                    if parts.len() >= 4 {
                        Some(TmuxSessionInfo {
                            name: parts[0].to_string(),
                            windows: parts[1].parse().unwrap_or(0),
                            created: parts[2].to_string(),
                            attached: parts[3] != "0",
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        Ok(out) => {
            debug!("tmux list-sessions failed: {}", String::from_utf8_lossy(&out.stderr));
            vec![]
        }
        Err(e) => {
            debug!("Failed to run tmux: {}", e);
            vec![]
        }
    }
}

/// Create a new tmux session and attach to it.
async fn create_and_attach(
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
    event_tx: mpsc::UnboundedSender<TmuxEvent>,
    name: Option<String>,
) {
    let session_name = name.unwrap_or_else(|| format!("remote-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()));

    // Set history-limit before creating, so new session inherits it
    let _ = Command::new(tmux_bin())
        .args(["set-option", "-g", "history-limit", "50000"])
        .output();

    // Create the tmux session in detached mode first
    let create_result = Command::new(tmux_bin())
        .args(["new-session", "-d", "-s", &session_name])
        .output();

    match create_result {
        Ok(out) if out.status.success() => {
            info!("Created tmux session: {}", session_name);
            // Now attach to it
            attach_to_session(sessions, event_tx, session_name).await;
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            error!("Failed to create tmux session: {}", err);
            let _ = event_tx.send(TmuxEvent::Error(format!("Failed to create session: {}", err)));
        }
        Err(e) => {
            error!("Failed to run tmux: {}", e);
            let _ = event_tx.send(TmuxEvent::Error(format!("tmux not available: {}", e)));
        }
    }
}

/// Attach to ALL existing tmux sessions.
async fn attach_all_sessions(
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
    event_tx: mpsc::UnboundedSender<TmuxEvent>,
) {
    let existing = list_tmux_sessions();

    if existing.is_empty() {
        info!("No existing tmux sessions to attach");
        return;
    }

    info!("Auto-attaching to {} existing tmux sessions", existing.len());

    // Get list of already attached session names
    let attached_names: Vec<String> = {
        let sessions_guard = sessions.lock().await;
        sessions_guard.values().map(|s| s.session_name.clone()).collect()
    };

    for session in existing {
        // Skip if already attached
        if attached_names.contains(&session.name) {
            debug!("Session {} already attached, skipping", session.name);
            continue;
        }

        info!("Auto-attaching to tmux session: {}", session.name);
        attach_to_session(sessions.clone(), event_tx.clone(), session.name).await;
    }
}

/// Attach to an existing tmux session and start streaming I/O.
async fn attach_to_session(
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
    event_tx: mpsc::UnboundedSender<TmuxEvent>,
    session_name: String,
) {
    let session_name_clone = session_name.clone();

    // Run PTY creation in blocking task
    let result = tokio::task::spawn_blocking(move || {
        let pty_system = native_pty_system();

        // Create PTY with reasonable default size
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        // Build tmux attach command
        let mut cmd = CommandBuilder::new(tmux_bin());
        cmd.arg("attach-session");
        cmd.arg("-t");
        cmd.arg(&session_name_clone);
        cmd.env("TERM", "xterm-256color");

        // Spawn tmux attach
        let _child = pair.slave.spawn_command(cmd)?;

        // Get writer for input
        let writer = pair.master.take_writer()?;

        // Get reader for output
        let reader = pair.master.try_clone_reader()?;

        Ok::<_, Box<dyn std::error::Error + Send + Sync>>((pair, writer, reader, session_name_clone))
    }).await;

    match result {
        Ok(Ok((pair, writer, reader, name))) => {
            let session_id = uuid::Uuid::new_v4().to_string();

            info!(session_id = %session_id, session_name = %name, "Attached to tmux session");

            // Send attached event first so relay/browser know about this session
            let _ = event_tx.send(TmuxEvent::Attached {
                session_id: session_id.clone(),
                session_name: name.clone(),
            });

            // Capture existing scrollback buffer so browser gets history
            if let Ok(out) = Command::new(tmux_bin())
                .args(["capture-pane", "-t", &name, "-p", "-S", "-"])
                .output()
            {
                if out.status.success() && !out.stdout.is_empty() {
                    debug!(session_id = %session_id, bytes = out.stdout.len(), "Sending scrollback history");
                    let _ = event_tx.send(TmuxEvent::Output {
                        session_id: session_id.clone(),
                        data: out.stdout,
                    });
                }
            }

            // Store session
            {
                let mut sessions_guard = sessions.lock().await;
                sessions_guard.insert(session_id.clone(), AttachedSession {
                    pair,
                    writer,
                    session_name: name,
                });
            }

            // Spawn output reader task
            let session_id_clone = session_id.clone();
            let event_tx_clone = event_tx.clone();
            let sessions_clone = sessions.clone();

            tokio::task::spawn_blocking(move || {
                read_tmux_output(reader, session_id_clone, event_tx_clone, sessions_clone);
            });
        }
        Ok(Err(e)) => {
            error!("Failed to attach to tmux session: {}", e);
            let _ = event_tx.send(TmuxEvent::Error(format!("Failed to attach: {}", e)));
        }
        Err(e) => {
            error!("Attach task panicked: {}", e);
            let _ = event_tx.send(TmuxEvent::Error(format!("Internal error: {}", e)));
        }
    }
}

fn read_tmux_output(
    mut reader: Box<dyn Read + Send>,
    session_id: String,
    event_tx: mpsc::UnboundedSender<TmuxEvent>,
    sessions: Arc<Mutex<HashMap<String, AttachedSession>>>,
) {
    let mut buf = [0u8; 4096];

    loop {
        match reader.read(&mut buf) {
            Ok(0) => {
                // EOF - tmux detached or session ended
                debug!(session_id = %session_id, "Tmux session EOF");
                break;
            }
            Ok(n) => {
                let data = buf[..n].to_vec();
                if event_tx.send(TmuxEvent::Output {
                    session_id: session_id.clone(),
                    data,
                }).is_err() {
                    // Event channel closed
                    break;
                }
            }
            Err(e) => {
                debug!(session_id = %session_id, error = %e, "Tmux read error");
                break;
            }
        }
    }

    // Send detached event
    let _ = event_tx.send(TmuxEvent::Detached {
        session_id: session_id.clone(),
    });

    // Remove session
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

    info!(session_id = %session_id, "Tmux session detached");
}

async fn write_to_session(
    sessions: &Arc<Mutex<HashMap<String, AttachedSession>>>,
    session_id: &str,
    data: &[u8],
) {
    let mut sessions_guard = sessions.lock().await;
    if let Some(session) = sessions_guard.get_mut(session_id) {
        if let Err(e) = session.writer.write_all(data) {
            warn!(session_id = %session_id, error = %e, "Failed to write to tmux");
        }
    } else {
        debug!(session_id = %session_id, "Write to unknown session");
    }
}

async fn resize_session(
    sessions: &Arc<Mutex<HashMap<String, AttachedSession>>>,
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
            warn!(session_id = %session_id, error = %e, "Failed to resize tmux");
        } else {
            debug!(session_id = %session_id, cols = cols, rows = rows, "Tmux resized");
        }
    }
}

/// Force a refresh/redraw of a tmux session by sending Ctrl+L.
/// This forces tmux to redraw the screen, sending content to the browser.
async fn refresh_session(
    sessions: &Arc<Mutex<HashMap<String, AttachedSession>>>,
    session_id: &str,
) {
    let mut sessions_guard = sessions.lock().await;
    if let Some(session) = sessions_guard.get_mut(session_id) {
        // Send Ctrl+L (ASCII 12, form feed) which clears and redraws the screen
        if let Err(e) = session.writer.write_all(&[12]) {
            warn!(session_id = %session_id, error = %e, "Failed to refresh tmux");
        } else {
            debug!(session_id = %session_id, "Tmux refresh sent (Ctrl+L)");
        }
    }
}

fn kill_tmux_session(session_name: &str, event_tx: &mpsc::UnboundedSender<TmuxEvent>) {
    let result = Command::new(tmux_bin())
        .args(["kill-session", "-t", session_name])
        .output();

    match result {
        Ok(out) if out.status.success() => {
            info!("Killed tmux session: {}", session_name);
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr).to_string();
            warn!("Failed to kill tmux session {}: {}", session_name, err);
            let _ = event_tx.send(TmuxEvent::Error(format!("Failed to kill session: {}", err)));
        }
        Err(e) => {
            error!("Failed to run tmux: {}", e);
            let _ = event_tx.send(TmuxEvent::Error(format!("tmux error: {}", e)));
        }
    }
}

async fn detach_session(
    sessions: &Arc<Mutex<HashMap<String, AttachedSession>>>,
    session_id: &str,
    event_tx: &mpsc::UnboundedSender<TmuxEvent>,
) {
    let mut sessions_guard = sessions.lock().await;
    if sessions_guard.remove(session_id).is_some() {
        info!(session_id = %session_id, "Detached from tmux session");
        let _ = event_tx.send(TmuxEvent::Detached {
            session_id: session_id.to_string(),
        });
    }
}

/// Kill a tmux session by our internal session_id (UUID).
/// This looks up the tmux session name and kills it.
async fn kill_session_by_id(
    sessions: &Arc<Mutex<HashMap<String, AttachedSession>>>,
    session_id: &str,
    event_tx: &mpsc::UnboundedSender<TmuxEvent>,
) {
    let session_name = {
        let sessions_guard = sessions.lock().await;
        sessions_guard.get(session_id).map(|s| s.session_name.clone())
    };

    if let Some(name) = session_name {
        info!(session_id = %session_id, session_name = %name, "Killing tmux session");

        // Kill the tmux session
        let result = Command::new(tmux_bin())
            .args(["kill-session", "-t", &name])
            .output();

        match result {
            Ok(out) if out.status.success() => {
                info!("Killed tmux session: {}", name);
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).to_string();
                warn!("Failed to kill tmux session {}: {}", name, err);
                let _ = event_tx.send(TmuxEvent::Error(format!("Failed to kill session: {}", err)));
            }
            Err(e) => {
                error!("Failed to run tmux: {}", e);
                let _ = event_tx.send(TmuxEvent::Error(format!("tmux error: {}", e)));
            }
        }

        // Remove from our tracking and emit detached event
        let mut sessions_guard = sessions.lock().await;
        if sessions_guard.remove(session_id).is_some() {
            // Emit Detached event so menu bar updates immediately
            let _ = event_tx.send(TmuxEvent::Detached {
                session_id: session_id.to_string(),
            });
        }
    } else {
        warn!(session_id = %session_id, "Cannot kill: session not found");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmux_command_variants() {
        let cmd = TmuxCommand::ListSessions;
        assert!(matches!(cmd, TmuxCommand::ListSessions));

        let cmd = TmuxCommand::NewSession { name: Some("test".into()) };
        assert!(matches!(cmd, TmuxCommand::NewSession { .. }));
    }
}
