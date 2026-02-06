use serde::{Deserialize, Serialize};

/// Control messages sent as JSON over WebSocket Text frames.
/// Terminal I/O is sent as Binary frames (not wrapped in ControlMessage).
///
/// This MUST match the relay-server-v2/src/protocol.rs exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    // Mac-client -> Relay
    Register { client_id: String },

    // Relay -> Mac-client
    Registered { code: String },
    BrowserConnected { browser_id: String },
    BrowserDisconnected { browser_id: String },

    // Browser -> Relay (not used by mac-client but included for completeness)
    Auth { session_code: String },

    // Relay -> Browser (not used by mac-client)
    AuthSuccess,
    AuthFailed { reason: String },

    // Browser -> Relay -> Mac-client
    CloseSession { session_id: String },
    CreateSession,

    // Mac-client -> Relay -> Browser (session list on connect)
    SessionList { sessions: Vec<SessionInfo> },
    SessionConnected { session_id: String, name: String },
    SessionDisconnected { session_id: String },
    SessionResize { session_id: String, cols: u16, rows: u16 },

    // Bidirectional
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_serialization() {
        let msg = ControlMessage::Register {
            client_id: "test".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"register\""));
        assert!(json.contains("\"client_id\":\"test\""));
    }

    #[test]
    fn test_registered_deserialization() {
        let json = r#"{"type":"registered","code":"ABC123"}"#;
        let msg: ControlMessage = serde_json::from_str(json).unwrap();
        match msg {
            ControlMessage::Registered { code } => {
                assert_eq!(code, "ABC123");
            }
            _ => panic!("Expected Registered message"),
        }
    }

    #[test]
    fn test_browser_connected_deserialization() {
        let json = r#"{"type":"browser_connected","browser_id":"browser-uuid"}"#;
        let msg: ControlMessage = serde_json::from_str(json).unwrap();
        match msg {
            ControlMessage::BrowserConnected { browser_id } => {
                assert_eq!(browser_id, "browser-uuid");
            }
            _ => panic!("Expected BrowserConnected message"),
        }
    }

    #[test]
    fn test_error_deserialization() {
        let json = r#"{"type":"error","message":"Something went wrong"}"#;
        let msg: ControlMessage = serde_json::from_str(json).unwrap();
        match msg {
            ControlMessage::Error { message } => {
                assert_eq!(message, "Something went wrong");
            }
            _ => panic!("Expected Error message"),
        }
    }
}
