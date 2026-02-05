//! Session tracking for connected shell integrations.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// A connected shell session.
pub struct Session {
    /// Unique session identifier (UUID).
    pub id: String,
    /// Display name for the session (e.g., "zsh - ~/project").
    pub name: String,
    /// When the session connected.
    pub connected_at: Instant,
}

impl Session {
    /// Create a new session with the given id and name.
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            connected_at: Instant::now(),
        }
    }

    /// Get how long this session has been connected, in seconds.
    pub fn duration_secs(&self) -> u64 {
        self.connected_at.elapsed().as_secs()
    }
}

/// Registration message sent by shell integration when connecting.
///
/// This defines the contract for Phase 6 shell integration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShellRegistration {
    /// Display name (e.g., "zsh - ~/project").
    pub name: String,
    /// Shell type (e.g., "zsh", "bash").
    pub shell: String,
    /// Process ID of the shell.
    pub pid: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new("test-id".to_string(), "test-name".to_string());
        assert_eq!(session.id, "test-id");
        assert_eq!(session.name, "test-name");
    }

    #[test]
    fn test_session_duration() {
        let session = Session::new("id".to_string(), "name".to_string());
        // Duration should be at least 0
        assert!(session.duration_secs() >= 0);
    }

    #[test]
    fn test_shell_registration_serialization() {
        let reg = ShellRegistration {
            name: "zsh - ~/project".to_string(),
            shell: "zsh".to_string(),
            pid: 12345,
        };
        let json = serde_json::to_string(&reg).unwrap();
        assert!(json.contains("\"name\":\"zsh - ~/project\""));
        assert!(json.contains("\"shell\":\"zsh\""));
        assert!(json.contains("\"pid\":12345"));
    }

    #[test]
    fn test_shell_registration_deserialization() {
        let json = r#"{"name":"bash - ~/code","shell":"bash","pid":54321}"#;
        let reg: ShellRegistration = serde_json::from_str(json).unwrap();
        assert_eq!(reg.name, "bash - ~/code");
        assert_eq!(reg.shell, "bash");
        assert_eq!(reg.pid, 54321);
    }
}
