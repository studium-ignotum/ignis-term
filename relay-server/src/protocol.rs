use serde::{Deserialize, Serialize};

/// Control messages sent as JSON over WebSocket Text frames.
/// Terminal I/O is sent as Binary frames (not wrapped in ControlMessage).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    // Mac-client -> Relay
    Register { client_id: String },

    // Relay -> Mac-client
    Registered { code: String },
    BrowserConnected { browser_id: String },
    BrowserDisconnected { browser_id: String },

    // Browser -> Relay
    Auth { session_code: String },

    // Relay -> Browser
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
    fn test_serialize_register() {
        let msg = ControlMessage::Register { client_id: "test".into() };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"register\""));
        assert!(json.contains("\"client_id\":\"test\""));
    }

    #[test]
    fn test_serialize_registered() {
        let msg = ControlMessage::Registered { code: "ABC123".into() };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"registered\""));
        assert!(json.contains("\"code\":\"ABC123\""));
    }

    #[test]
    fn test_serialize_auth_success() {
        let msg = ControlMessage::AuthSuccess;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, "{\"type\":\"auth_success\"}");
    }

    #[test]
    fn test_deserialize_auth() {
        let json = r#"{"type":"auth","session_code":"XYZ789"}"#;
        let msg: ControlMessage = serde_json::from_str(json).unwrap();
        match msg {
            ControlMessage::Auth { session_code } => {
                assert_eq!(session_code, "XYZ789");
            }
            _ => panic!("Expected Auth message"),
        }
    }

    #[test]
    fn test_session_info() {
        let info = SessionInfo {
            id: "sess_1".into(),
            name: "My Session".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"sess_1\""));
        assert!(json.contains("\"name\":\"My Session\""));
    }
}
