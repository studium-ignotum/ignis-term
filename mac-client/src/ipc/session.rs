//! Session types for IPC connections.

use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::net::unix::OwnedWriteHalf;

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

/// Messages sent by shell integration after initial registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShellMessage {
    /// Rename the session (sent on directory change)
    Rename {
        /// New session name
        name: String,
    },
}

/// Represents an active shell session connected via IPC.
///
/// Holds the write half of the Unix stream for sending data back to the shell.
pub struct Session {
    /// Unique session identifier (UUID)
    pub id: String,
    /// Registration info from the shell
    pub registration: ShellRegistration,
    /// When the session connected
    pub connected_at: Instant,
    /// Write half of the Unix stream for sending data to the shell
    write_half: OwnedWriteHalf,
}

impl Session {
    /// Create a new session from registration info and stream write half.
    pub fn new(id: String, registration: ShellRegistration, write_half: OwnedWriteHalf) -> Self {
        Self {
            id,
            registration,
            connected_at: Instant::now(),
            write_half,
        }
    }

    /// Get the session's display name.
    pub fn name(&self) -> &str {
        &self.registration.name
    }

    /// Update the session's display name (called on directory change).
    pub fn set_name(&mut self, name: String) {
        self.registration.name = name;
    }

    /// Get how long this session has been connected, in seconds.
    pub fn duration_secs(&self) -> u64 {
        self.connected_at.elapsed().as_secs()
    }

    /// Write terminal data to the shell.
    ///
    /// This sends data from the relay (browser input) to the local shell session.
    pub async fn write(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.write_half.write_all(data).await
    }
}

// Manual Debug implementation since OwnedWriteHalf doesn't implement Debug
impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("id", &self.id)
            .field("registration", &self.registration)
            .field("connected_at", &self.connected_at)
            .field("write_half", &"<OwnedWriteHalf>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::UnixStream;

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

    #[tokio::test]
    async fn test_session_creation() {
        let (stream1, _stream2) = UnixStream::pair().unwrap();
        let (_read_half, write_half) = stream1.into_split();

        let reg = ShellRegistration {
            name: "Test Session".into(),
            shell: "bash".into(),
            pid: 1234,
        };
        let session = Session::new("sess-1".into(), reg, write_half);
        assert_eq!(session.id, "sess-1");
        assert_eq!(session.name(), "Test Session");
        assert_eq!(session.registration.shell, "bash");
    }

    #[tokio::test]
    async fn test_session_duration() {
        let (stream1, _stream2) = UnixStream::pair().unwrap();
        let (_read_half, write_half) = stream1.into_split();

        let reg = ShellRegistration {
            name: "Duration Test".into(),
            shell: "zsh".into(),
            pid: 5678,
        };
        let session = Session::new("sess-2".into(), reg, write_half);
        // Duration should be at least 0 seconds
        assert!(session.duration_secs() <= 1);
    }

    #[tokio::test]
    async fn test_session_write() {
        let (stream1, stream2) = UnixStream::pair().unwrap();
        let (_read_half, write_half) = stream1.into_split();
        let (mut read_half2, _write_half2) = stream2.into_split();

        let reg = ShellRegistration {
            name: "Write Test".into(),
            shell: "bash".into(),
            pid: 9999,
        };
        let mut session = Session::new("sess-3".into(), reg, write_half);

        // Write data through session
        session.write(b"hello").await.unwrap();

        // Read it from the other end
        use tokio::io::AsyncReadExt;
        let mut buf = [0u8; 5];
        read_half2.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"hello");
    }
}
