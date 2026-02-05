//! Session types for IPC connections.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Registration message sent by shell integration when connecting.
///
/// This defines the contract for Phase 6 shell integration scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellRegistration {
    /// Human-readable name for the session (e.g., "zsh - ~/project")
    pub name: String,
    /// Shell type (e.g., "zsh", "bash")
    pub shell: String,
    /// Process ID of the shell
    pub pid: u32,
}

/// Represents an active shell session connected via IPC.
#[derive(Debug)]
pub struct Session {
    /// Unique session identifier (UUID)
    pub id: String,
    /// Registration info from the shell
    pub registration: ShellRegistration,
    /// When the session connected
    pub connected_at: Instant,
}

impl Session {
    /// Create a new session from registration info.
    pub fn new(id: String, registration: ShellRegistration) -> Self {
        Self {
            id,
            registration,
            connected_at: Instant::now(),
        }
    }

    /// Get the session's display name.
    pub fn name(&self) -> &str {
        &self.registration.name
    }

    /// Get how long this session has been connected, in seconds.
    pub fn duration_secs(&self) -> u64 {
        self.connected_at.elapsed().as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_session_creation() {
        let reg = ShellRegistration {
            name: "Test Session".into(),
            shell: "bash".into(),
            pid: 1234,
        };
        let session = Session::new("sess-1".into(), reg);
        assert_eq!(session.id, "sess-1");
        assert_eq!(session.name(), "Test Session");
        assert_eq!(session.registration.shell, "bash");
    }

    #[test]
    fn test_session_duration() {
        let reg = ShellRegistration {
            name: "Duration Test".into(),
            shell: "zsh".into(),
            pid: 5678,
        };
        let session = Session::new("sess-2".into(), reg);
        // Duration should be at least 0 seconds
        assert!(session.duration_secs() <= 1);
    }
}
