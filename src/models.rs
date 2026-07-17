use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub port: u16,
    pub host: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    #[serde(rename = "session_id")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameSessionRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub session_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub message: Option<String>,
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub sender: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub id: u64,
    pub session_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResponse {
    pub messages: Vec<Message>,
    pub timeout: bool,
    pub timeout_after: Option<u64>,
    pub closed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapResponse {
    pub session: Session,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    #[serde(rename = "session_id")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub closed: bool,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub pid: u32,
    pub port: u16,
    pub uptime_seconds: i64,
    pub session_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseSessionResponse {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DaemonEvent {
    NewMessage(Message),
    SessionClosed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveEvent {
    pub session_id: String,
    pub session_name: Option<String>,
    pub r#type: String,
    pub message: Option<Message>,
}

/// Query parameters for the recap endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapQuery {
    pub since: Option<u64>,
    pub limit: Option<usize>,
    pub from: Option<String>,
    pub cursor: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serialization() {
        let now = Utc::now();
        let session = Session {
            id: "sess_test".into(),
            name: None,
            created_at: now,
            last_activity: now,
            closed: false,
        };
        let json = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "sess_test");
        assert!(!deserialized.closed);
    }

    #[test]
    fn test_message_serialization() {
        let now = Utc::now();
        let msg = Message {
            id: 1,
            session_id: "sess_test".into(),
            sender: "test-agent".into(),
            content: "hello **world**".into(),
            timestamp: now,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.sender, "test-agent");
        assert_eq!(deserialized.content, "hello **world**");
    }

    #[test]
    fn test_daemon_info_serialization() {
        let info = DaemonInfo {
            pid: 12345,
            port: 54321,
            host: "127.0.0.1".into(),
            started_at: Utc::now(),
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        assert!(json.contains("\"pid\": 12345"));
        assert!(json.contains("\"port\": 54321"));

        let deserialized: DaemonInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pid, 12345);
        assert_eq!(deserialized.port, 54321);
        assert_eq!(deserialized.host, "127.0.0.1");
    }

    #[test]
    fn test_round_trip_all_request_types() {
        let req = CreateSessionRequest {
            message: Some("hello".into()),
            sender: Some("agent".into()),
            name: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CreateSessionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message.unwrap(), "hello");

        let msg_req = SendMessageRequest {
            sender: "agent".into(),
            content: "test".into(),
        };
        let json = serde_json::to_string(&msg_req).unwrap();
        let deserialized: SendMessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "test");
    }

    #[test]
    fn test_wait_response_timeout() {
        let resp = WaitResponse {
            messages: vec![],
            timeout: true,
            timeout_after: Some(30),
            closed: false,
            cursor: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: WaitResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.timeout);
        assert_eq!(deserialized.timeout_after, Some(30));
        assert!(!deserialized.closed);
        assert_eq!(deserialized.cursor, None);
    }
}
