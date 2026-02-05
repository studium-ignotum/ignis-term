//! Session types for IPC connections.

use serde::{Deserialize, Serialize};

/// Registration message sent by shell integration when connecting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellRegistration {
    /// Human-readable name for the session
    pub name: String,
    /// Shell type (bash, zsh, etc.)
    pub shell: String,
    /// Process ID of the shell
    pub pid: u32,
}

/// Represents an active shell session connected via IPC.
#[derive(Debug, Clone)]
pub struct Session {
    /// Unique session identifier
    pub id: String,
    /// Registration info from the shell
    pub registration: ShellRegistration,
}

impl Session {
    /// Create a new session from registration info.
    pub fn new(id: String, registration: ShellRegistration) -> Self {
        Self { id, registration }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_registration_deserialize() {
        let json = r#"{"name":"Terminal","shell":"zsh","pid":12345}"#;
        let reg: ShellRegistration = serde_json::from_str(json).unwrap();
        assert_eq!(reg.name, "Terminal");
        assert_eq!(reg.shell, "zsh");
        assert_eq!(reg.pid, 12345);
    }

    #[test]
    fn test_session_creation() {
        let reg = ShellRegistration {
            name: "Test".into(),
            shell: "bash".into(),
            pid: 1234,
        };
        let session = Session::new("sess-1".into(), reg);
        assert_eq!(session.id, "sess-1");
        assert_eq!(session.registration.name, "Test");
    }
}
