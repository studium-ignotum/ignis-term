use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::session::generate_session_code;

/// Maximum scrollback buffer size (1 MB)
const MAX_SCROLLBACK: usize = 1024 * 1024;

/// Message types that can be sent to browsers
#[derive(Debug, Clone)]
pub enum BrowserMessage {
    Binary(Vec<u8>),
    Text(String),
}

/// Message types that can be sent to mac-client
#[derive(Debug, Clone)]
pub enum MacMessage {
    Binary(Vec<u8>),
    Text(String),
}

/// A connected mac-client session
pub struct Session {
    /// Channel to send messages to the mac-client
    pub mac_tx: mpsc::Sender<MacMessage>,
    /// Connected browsers: browser_id -> sender channel
    pub browsers: DashMap<String, mpsc::Sender<BrowserMessage>>,
    /// Accumulated terminal output frames for replay on browser reconnect.
    /// Each entry is a complete binary frame (with session ID prefix).
    scrollback_frames: Mutex<Vec<Vec<u8>>>,
    /// Total byte count of all frames in scrollback (for cap enforcement).
    scrollback_bytes: Mutex<usize>,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    /// Session code -> Session data
    sessions: DashMap<String, Session>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AppStateInner {
                sessions: DashMap::new(),
            }),
        }
    }

    /// Register a new mac-client, returns unique session code
    pub fn register_mac_client(&self, mac_tx: mpsc::Sender<MacMessage>) -> String {
        // Generate code with collision check
        let code = loop {
            let candidate = generate_session_code();
            if !self.inner.sessions.contains_key(&candidate) {
                break candidate;
            }
            tracing::debug!("Session code collision, regenerating");
        };

        self.inner.sessions.insert(
            code.clone(),
            Session {
                mac_tx,
                browsers: DashMap::new(),
                scrollback_frames: Mutex::new(Vec::new()),
                scrollback_bytes: Mutex::new(0),
            },
        );

        tracing::info!(code = %code, "Mac-client registered");
        code
    }

    /// Validate a session code, returns true if valid
    pub fn validate_session_code(&self, code: &str) -> bool {
        self.inner.sessions.contains_key(code)
    }

    /// Remove a session (when mac-client disconnects)
    pub fn remove_session(&self, code: &str) {
        if self.inner.sessions.remove(code).is_some() {
            tracing::info!(code = %code, "Session removed");
        }
    }

    /// Get count of active sessions (for debugging)
    pub fn session_count(&self) -> usize {
        self.inner.sessions.len()
    }

    /// Add a browser to a session
    pub fn add_browser(&self, code: &str, browser_id: String, tx: mpsc::Sender<BrowserMessage>) {
        if let Some(session) = self.inner.sessions.get(code) {
            session.browsers.insert(browser_id, tx);
        }
    }

    /// Remove a browser from a session
    pub fn remove_browser(&self, code: &str, browser_id: &str) {
        if let Some(session) = self.inner.sessions.get(code) {
            session.browsers.remove(browser_id);
        }
    }

    /// Broadcast terminal output (binary) to all browsers in a session
    pub async fn broadcast_to_browsers(&self, code: &str, data: Vec<u8>) {
        if let Some(session) = self.inner.sessions.get(code) {
            // Append frame to scrollback, dropping oldest frames if over cap
            {
                let frame_len = data.len();
                let mut frames = session.scrollback_frames.lock().await;
                let mut total = session.scrollback_bytes.lock().await;

                frames.push(data.clone());
                *total += frame_len;

                // Drop oldest frames until we're under the cap
                while *total > MAX_SCROLLBACK && !frames.is_empty() {
                    let removed = frames.remove(0);
                    *total -= removed.len();
                }
            }

            for entry in session.browsers.iter() {
                let _ = entry.value().send(BrowserMessage::Binary(data.clone())).await;
            }
        }
    }

    /// Purge scrollback frames belonging to a specific terminal session.
    /// Binary frame format: [1 byte session_id_len][session_id][payload]
    pub async fn purge_session_scrollback(&self, code: &str, terminal_session_id: &str) {
        if let Some(session) = self.inner.sessions.get(code) {
            let mut frames = session.scrollback_frames.lock().await;
            let mut total = session.scrollback_bytes.lock().await;

            let tid = terminal_session_id.as_bytes();
            let before = frames.len();
            frames.retain(|frame| {
                if frame.is_empty() {
                    return false;
                }
                let id_len = frame[0] as usize;
                if frame.len() < 1 + id_len {
                    return false;
                }
                let frame_sid = &frame[1..1 + id_len];
                frame_sid != tid
            });
            let after = frames.len();

            // Recalculate total bytes
            *total = frames.iter().map(|f| f.len()).sum();

            if before != after {
                tracing::info!(
                    code = %code,
                    terminal_session_id = %terminal_session_id,
                    purged = before - after,
                    remaining = after,
                    "Purged scrollback frames for dead session"
                );
            }
        }
    }

    /// Get scrollback frames for replay to a newly connected browser.
    pub async fn get_scrollback(&self, code: &str) -> Vec<Vec<u8>> {
        if let Some(session) = self.inner.sessions.get(code) {
            let frames = session.scrollback_frames.lock().await;
            frames.clone()
        } else {
            Vec::new()
        }
    }

    /// Broadcast text message (JSON) to all browsers in a session
    pub async fn broadcast_text_to_browsers(&self, code: &str, text: &str) {
        if let Some(session) = self.inner.sessions.get(code) {
            for entry in session.browsers.iter() {
                let _ = entry.value().send(BrowserMessage::Text(text.to_string())).await;
            }
        }
    }

    /// Send keyboard input (binary) to mac-client
    pub async fn send_to_mac_client(&self, code: &str, data: Vec<u8>) {
        if let Some(session) = self.inner.sessions.get(code) {
            let _ = session.mac_tx.send(MacMessage::Binary(data)).await;
        }
    }

    /// Send text message (JSON) to mac-client
    pub async fn send_text_to_mac_client(&self, code: &str, text: &str) {
        if let Some(session) = self.inner.sessions.get(code) {
            let _ = session.mac_tx.send(MacMessage::Text(text.to_string())).await;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
