//! PTY proxy integration module for managing terminal sessions.
//!
//! Replaces the tmux module. Instead of tmux, terminal sessions are captured
//! by pty-proxy instances that connect to us via Unix socket.
//!
//! Each pty-proxy sends:
//!   - Registration (JSON): shell info, pid, tty
//!   - Framed I/O: length-prefixed messages tagged 'I' (input) or 'O' (output)
//!   - Resize notifications
//!
//! We forward output to relay (-> browser) and inject browser input back.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

/// Socket path for pty-proxy connections.
pub const SOCKET_PATH: &str = "/tmp/terminal-remote.sock";

/// Information about a connected pty-proxy session.
#[derive(Debug, Clone)]
pub struct PtySessionInfo {
    pub name: String,
    pub shell: String,
    pub pid: u32,
    pub tty: String,
}

/// Events emitted by the PTY manager.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// A new pty-proxy session connected.
    Attached {
        session_id: String,
        session_name: String,
    },
    /// A pty-proxy session disconnected.
    Detached {
        session_id: String,
    },
    /// Terminal output from a session (shell -> browser).
    Output {
        session_id: String,
        data: Vec<u8>,
    },
    /// Terminal resized on mac (pty-proxy SIGWINCH → browser).
    SessionResize {
        session_id: String,
        cols: u16,
        rows: u16,
    },
    /// Error occurred.
    Error(String),
}

/// Commands that can be sent to the PTY manager.
#[derive(Debug)]
pub enum PtyCommand {
    /// Write input to a session (browser -> shell).
    Write {
        session_id: String,
        data: Vec<u8>,
    },
    /// Kill/close a session.
    KillSession {
        session_id: String,
    },
    /// Shutdown the PTY manager.
    Shutdown,
}

/// Registration message from pty-proxy.
#[derive(Debug, Deserialize)]
struct Registration {
    name: String,
    shell: String,
    pid: u32,
    tty: String,
}

/// Manages pty-proxy connections.
/// Exists to own the Drop impl that cleans up the socket file.
pub struct PtyManager;

/// Handle for writing to a connected pty-proxy.
struct SessionHandle {
    info: PtySessionInfo,
    writer: tokio::net::unix::OwnedWriteHalf,
}

/// Shared TTY map: session_id -> tty path.
/// Persists after session disconnect so late close_session commands can still
/// find the TTY to close the Terminal.app window.
type TtyMap = Arc<Mutex<HashMap<String, String>>>;

impl PtyManager {
    /// Create a new PtyManager.
    /// Returns the manager, event receiver, and command sender.
    ///
    /// This has the same signature pattern as TmuxManager::new() for easy swap.
    pub fn new() -> (
        Self,
        mpsc::UnboundedReceiver<PtyEvent>,
        mpsc::UnboundedSender<PtyCommand>,
    ) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        let sessions: Arc<Mutex<HashMap<String, SessionHandle>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // TTY map persists across session lifecycle for late close handling
        let tty_map: TtyMap = Arc::new(Mutex::new(HashMap::new()));

        // Start command processor
        let sessions_cmd = sessions.clone();
        let tty_map_cmd = tty_map.clone();
        tokio::spawn(async move {
            process_commands(command_rx, sessions_cmd, tty_map_cmd).await;
        });

        // Start Unix socket listener
        let event_tx_listen = event_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = run_listener(sessions, event_tx_listen, tty_map).await {
                error!("PTY listener failed: {}", e);
            }
        });

        (Self, event_rx, command_tx)
    }
}

/// Listen for pty-proxy connections on Unix socket.
async fn run_listener(
    sessions: Arc<Mutex<HashMap<String, SessionHandle>>>,
    event_tx: mpsc::UnboundedSender<PtyEvent>,
    tty_map: TtyMap,
) -> std::io::Result<()> {
    // Remove stale socket
    if std::path::Path::new(SOCKET_PATH).exists() {
        warn!("Removing stale socket at {}", SOCKET_PATH);
        std::fs::remove_file(SOCKET_PATH)?;
    }

    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("PTY manager listening on {}", SOCKET_PATH);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let sessions = sessions.clone();
                let event_tx = event_tx.clone();
                let tty_map = tty_map.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_proxy_connection(stream, sessions, event_tx, tty_map).await {
                        debug!("Proxy connection ended: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Accept failed: {}", e);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Handle a single pty-proxy connection.
async fn handle_proxy_connection(
    stream: UnixStream,
    sessions: Arc<Mutex<HashMap<String, SessionHandle>>>,
    event_tx: mpsc::UnboundedSender<PtyEvent>,
    tty_map: TtyMap,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (mut reader, writer) = stream.into_split();

    // Read registration frame: 4 bytes length + JSON
    let reg: Registration = {
        let len = reader.read_u32().await?;
        if len > 65536 {
            return Err("Registration too large".into());
        }
        let mut buf = vec![0u8; len as usize];
        reader.read_exact(&mut buf).await?;
        serde_json::from_slice(&buf)?
    };

    let session_name = reg.name.clone();
    let tty = reg.tty.clone();
    info!(
        session_id = %session_id,
        name = %reg.name,
        shell = %reg.shell,
        pid = reg.pid,
        tty = %reg.tty,
        "pty-proxy connected"
    );

    let info = PtySessionInfo {
        name: reg.name,
        shell: reg.shell,
        pid: reg.pid,
        tty: reg.tty,
    };

    // Store session and TTY mapping
    {
        let mut sessions_guard = sessions.lock().await;
        sessions_guard.insert(
            session_id.clone(),
            SessionHandle { info, writer },
        );
    }
    {
        let mut tty_guard = tty_map.lock().await;
        tty_guard.insert(session_id.clone(), tty.clone());
    }

    // Notify: session attached
    let _ = event_tx.send(PtyEvent::Attached {
        session_id: session_id.clone(),
        session_name,
    });

    // Read frames from pty-proxy
    let result = read_proxy_frames(&mut reader, &session_id, &event_tx).await;

    // Cleanup on disconnect
    {
        let mut sessions_guard = sessions.lock().await;
        sessions_guard.remove(&session_id);
    }
    let _ = event_tx.send(PtyEvent::Detached {
        session_id: session_id.clone(),
    });
    info!(session_id = %session_id, "pty-proxy disconnected");

    // Don't auto-close the Terminal.app window here. When the user types `exit`,
    // Terminal.app handles the window according to its own preferences. We only
    // force-close when the user explicitly clicks Close in the browser UI
    // (handled by KillSession).

    result
}

/// Read length-prefixed frames from pty-proxy.
/// Frame format: 4 bytes big-endian length + payload
/// Payload: first byte is tag ('I' = input echo, 'O' = output, '{' = JSON control)
async fn read_proxy_frames(
    reader: &mut tokio::net::unix::OwnedReadHalf,
    session_id: &str,
    event_tx: &mpsc::UnboundedSender<PtyEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        // Read frame length
        let len = match reader.read_u32().await {
            Ok(l) => l as usize,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e.into()),
        };

        if len == 0 {
            continue;
        }
        if len > 1_048_576 {
            // 1MB max frame
            return Err("Frame too large".into());
        }

        // Read payload
        let mut payload = vec![0u8; len];
        reader.read_exact(&mut payload).await?;

        // Dispatch based on tag
        match payload[0] {
            b'O' => {
                // Output from shell -> forward to browser
                let _ = event_tx.send(PtyEvent::Output {
                    session_id: session_id.to_string(),
                    data: payload[1..].to_vec(),
                });
            }
            b'I' => {
                // Input echo from terminal — we don't need this for browser,
                // the shell output already includes echo.
            }
            b'{' => {
                // JSON control message (e.g., resize from terminal)
                let text = String::from_utf8_lossy(&payload);
                debug!(session_id = %session_id, "Control message from proxy: {}", text);

                // Parse resize and forward to browser
                if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&payload) {
                    if json.get("type").and_then(|t| t.as_str()) == Some("resize") {
                        if let (Some(cols), Some(rows)) = (
                            json.get("cols").and_then(|c| c.as_u64()),
                            json.get("rows").and_then(|r| r.as_u64()),
                        ) {
                            let _ = event_tx.send(PtyEvent::SessionResize {
                                session_id: session_id.to_string(),
                                cols: cols as u16,
                                rows: rows as u16,
                            });
                        }
                    }
                }
            }
            tag => {
                debug!(session_id = %session_id, tag = tag, "Unknown frame tag");
            }
        }
    }
}

/// Send a length-prefixed frame atomically to a pty-proxy session.
async fn send_frame(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    data: &[u8],
) -> std::io::Result<()> {
    let len = (data.len() as u32).to_be_bytes();
    // Write length prefix and payload together
    writer.write_all(&len).await?;
    writer.write_all(data).await?;
    writer.flush().await?;
    Ok(())
}

/// Process commands sent to the PTY manager.
async fn process_commands(
    mut command_rx: mpsc::UnboundedReceiver<PtyCommand>,
    sessions: Arc<Mutex<HashMap<String, SessionHandle>>>,
    tty_map: TtyMap,
) {
    while let Some(cmd) = command_rx.recv().await {
        match cmd {
            PtyCommand::Write { session_id, data } => {
                let mut sessions_guard = sessions.lock().await;
                if let Some(session) = sessions_guard.get_mut(&session_id) {
                    // Send as JSON input message, length-prefixed
                    let msg = serde_json::json!({
                        "type": "input",
                        "data": data,
                    });
                    let json = serde_json::to_vec(&msg).unwrap();
                    if let Err(e) = send_frame(&mut session.writer, &json).await {
                        warn!(session_id = %session_id, error = %e, "Write failed");
                    }
                }
            }
            PtyCommand::KillSession { session_id } => {
                // Close the Terminal.app window FIRST — this kills the shell
                // naturally and prevents Terminal.app from reopening a new shell
                // (which happens when pty-proxy exits with code 0).
                let tty = {
                    let tty_guard = tty_map.lock().await;
                    tty_guard.get(&session_id).cloned()
                };

                if let Some(tty) = tty {
                    info!(session_id = %session_id, tty = %tty, "Closing terminal window first");
                    tokio::task::spawn_blocking(move || {
                        close_terminal_window_force(&tty);
                    }).await.ok();
                } else {
                    // Fallback: send close message to pty-proxy directly
                    let mut sessions_guard = sessions.lock().await;
                    if let Some(session) = sessions_guard.get_mut(&session_id) {
                        let pid = session.info.pid;
                        info!(session_id = %session_id, pid = pid, "No TTY found, sending close to pty-proxy");
                        let msg = serde_json::json!({ "type": "close" });
                        let json = serde_json::to_vec(&msg).unwrap();
                        if let Err(e) = send_frame(&mut session.writer, &json).await {
                            warn!(session_id = %session_id, error = %e, "Close message failed, killing by PID");
                            unsafe { libc::kill(pid as i32, libc::SIGTERM); }
                        }
                    } else {
                        info!(session_id = %session_id, "Session already disconnected, nothing to kill");
                    }
                }
            }
            PtyCommand::Shutdown => {
                info!("PTY manager shutting down");
                let mut sessions_guard = sessions.lock().await;
                for (id, session) in sessions_guard.drain() {
                    info!(session_id = %id, pid = session.info.pid, "Killing session on shutdown");
                    unsafe {
                        libc::kill(session.info.pid as i32, libc::SIGTERM);
                    }
                }
                break;
            }
        }
    }
}

/// Force-close a Terminal.app window by TTY — no `busy` check.
/// Used when the browser explicitly requests closing a session.
/// Closes the window first so Terminal.app kills the shell naturally,
/// preventing the "reopen shell on exit" cycle.
fn close_terminal_window_force(tty: &str) {
    if tty == "unknown" || tty.is_empty() {
        return;
    }
    let script = format!(
        r#"tell application "Terminal"
    repeat with w in windows
        try
            if tty of first tab of w is "{tty}" then
                close w saving no
            end if
        end try
    end repeat
end tell"#,
        tty = tty
    );
    match std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
    {
        Ok(output) if !output.status.success() => {
            warn!(
                "osascript force-close failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Err(e) => warn!("Failed to run osascript for force-close: {}", e),
        _ => info!(tty = %tty, "Terminal window force-closed"),
    }
}

impl Drop for PtyManager {
    fn drop(&mut self) {
        info!("PTY manager dropped, cleaning up socket");
        if let Err(e) = std::fs::remove_file(SOCKET_PATH) {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("Failed to remove socket: {}", e);
            }
        }
    }
}
