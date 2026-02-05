//! Application state and channel types for UI <-> background communication.
//!
//! This module defines the unified event types and app state for integrating
//! the tray icon, relay client, and IPC server.

use muda::MenuItem;

/// Events sent from background tasks to the main UI thread.
///
/// These unify events from the relay client and IPC server into a single
/// channel that the main event loop processes.
#[derive(Debug, Clone)]
pub enum UiEvent {
    // From relay
    /// Successfully connected to relay server
    RelayConnected,
    /// Disconnected from relay server
    RelayDisconnected,
    /// Received session code from relay
    SessionCode(String),
    /// A browser connected to this session
    BrowserConnected(String),
    /// A browser disconnected from this session
    BrowserDisconnected(String),
    /// Error from relay
    RelayError(String),

    // From IPC
    /// A shell session connected via IPC
    ShellConnected { session_id: String, name: String },
    /// A shell session disconnected
    ShellDisconnected { session_id: String },
    /// Shell session count changed
    ShellCountChanged(usize),
    /// Error from IPC server
    IpcError(String),
}

/// Commands sent from the main UI thread to background tasks.
#[derive(Debug, Clone)]
pub enum BackgroundCommand {
    /// Request to shutdown background tasks
    Shutdown,
    // Future: SendToRelay(message), etc.
}

/// Application state holding current values and menu item references.
///
/// This struct tracks the current connection state and provides methods
/// to update the menu display accordingly.
pub struct AppState {
    /// Current session code (None if not yet received)
    pub session_code: Option<String>,
    /// Whether we're connected to the relay server
    pub relay_connected: bool,
    /// Number of active shell sessions via IPC
    pub shell_count: usize,
    /// Number of connected browsers
    pub browser_count: usize,

    // Menu items that need dynamic updates
    /// Display item showing session code
    pub code_item: MenuItem,
    /// Display item showing connection status
    pub status_item: MenuItem,
    /// Display item showing session count
    pub count_item: MenuItem,
    /// Action item for copying code (text changes for confirmation)
    pub copy_item: MenuItem,
}

impl AppState {
    /// Create a new AppState with the given menu items.
    pub fn new(
        code_item: MenuItem,
        status_item: MenuItem,
        count_item: MenuItem,
        copy_item: MenuItem,
    ) -> Self {
        Self {
            session_code: None,
            relay_connected: false,
            shell_count: 0,
            browser_count: 0,
            code_item,
            status_item,
            count_item,
            copy_item,
        }
    }

    /// Update the code display menu item.
    pub fn update_code_display(&self) {
        let display = match &self.session_code {
            Some(code) => format!("Code: {}", code),
            None => "Code: ------".to_string(),
        };
        self.code_item.set_text(display);
    }

    /// Update the status display menu item.
    pub fn update_status_display(&self) {
        let status = if self.relay_connected {
            "Connected"
        } else {
            "Disconnected"
        };
        self.status_item.set_text(format!("Status: {}", status));
    }

    /// Update the session count display menu item.
    pub fn update_count_display(&self) {
        self.count_item
            .set_text(format!("Sessions: {}", self.shell_count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_event_variants() {
        // Compile check - events are constructible
        let _connected = UiEvent::RelayConnected;
        let _disconnected = UiEvent::RelayDisconnected;
        let _code = UiEvent::SessionCode("ABC123".into());
        let _browser_conn = UiEvent::BrowserConnected("browser-id".into());
        let _browser_disc = UiEvent::BrowserDisconnected("browser-id".into());
        let _relay_error = UiEvent::RelayError("test error".into());
        let _shell_conn = UiEvent::ShellConnected {
            session_id: "sess-1".into(),
            name: "zsh".into(),
        };
        let _shell_disc = UiEvent::ShellDisconnected {
            session_id: "sess-1".into(),
        };
        let _shell_count = UiEvent::ShellCountChanged(5);
        let _ipc_error = UiEvent::IpcError("ipc error".into());
    }

    #[test]
    fn test_background_command_variants() {
        let _shutdown = BackgroundCommand::Shutdown;
    }
}
