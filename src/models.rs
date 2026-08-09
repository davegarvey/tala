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
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Intent {
    #[default]
    Fyi,
    Req,
    Reply,
    Out,
}

impl Intent {
    pub fn badge(self) -> &'static str {
        match self {
            Intent::Req => "REQ",
            Intent::Fyi => "FYI",
            Intent::Reply => "REPLY",
            Intent::Out => "OUT",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "req" => Some(Intent::Req),
            "fyi" => Some(Intent::Fyi),
            "reply" => Some(Intent::Reply),
            "out" => Some(Intent::Out),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub session_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub intent: Intent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<u64>,
    #[serde(default)]
    pub expect_reply: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_until: Option<DateTime<Utc>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_message_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub sender: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<Intent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<u64>,
    #[serde(default)]
    pub expect_reply: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_timeout: Option<u64>,
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
    #[serde(default)]
    pub intent: Intent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<u64>,
    #[serde(default)]
    pub expect_reply: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitResponse {
    pub messages: Vec<Message>,
    pub timeout: bool,
    pub timeout_after: Option<u64>,
    pub closed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlaps: Vec<WaitOverlap>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_waits: Vec<ActiveWaitInfo>,
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
    SessionCreated(String),
    SessionReopened(String),
    SessionRenamed {
        id: String,
        old_name: String,
        new_name: String,
    },
    WaitUpdate {
        identity: String,
        scope: WaitScope,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveEvent {
    pub session_id: String,
    pub session_name: Option<String>,
    pub r#type: String,
    pub message: Option<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSummary {
    pub sender: String,
    pub last_seen: DateTime<Utc>,
    pub message_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "session_id", rename_all = "snake_case")]
pub enum WaitScope {
    Session(String),
    AnyNewSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitOverlap {
    pub identity: String,
    pub scope: WaitScope,
    pub remaining_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveWaitInfo {
    pub identity: String,
    pub scope: WaitScope,
    pub since: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
    pub remaining_secs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingObligation {
    #[serde(rename = "session_id")]
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_name: Option<String>,
    pub message_id: u64,
    pub sender: String,
    pub content: String,
    pub elapsed_seconds: i64,
    #[serde(default)]
    pub intent: Intent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting_until: Option<DateTime<Utc>>,
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
            intent: Intent::Req,
            reply_to: Some(2),
            expect_reply: true,
            waiting_until: Some(now),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.sender, "test-agent");
        assert_eq!(deserialized.intent, Intent::Req);
        assert_eq!(deserialized.reply_to, Some(2));
        assert!(deserialized.expect_reply);
    }

    #[test]
    fn test_message_default_intent_is_fyi() {
        let json = r#"{"id":1,"session_id":"s","sender":"a","content":"c","timestamp":"2024-01-01T00:00:00Z"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.intent, Intent::Fyi);
        assert_eq!(msg.reply_to, None);
        assert!(!msg.expect_reply);
        assert_eq!(msg.waiting_until, None);
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
            intent: Some(Intent::Req),
            reply_to: Some(1),
            expect_reply: true,
            wait_timeout: Some(30),
        };
        let json = serde_json::to_string(&msg_req).unwrap();
        let deserialized: SendMessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content, "test");
        assert_eq!(deserialized.intent, Some(Intent::Req));
        assert_eq!(deserialized.reply_to, Some(1));
        assert!(deserialized.expect_reply);
        assert_eq!(deserialized.wait_timeout, Some(30));
    }

    #[test]
    fn test_wait_response_timeout() {
        let resp = WaitResponse {
            messages: vec![],
            timeout: true,
            timeout_after: Some(30),
            closed: false,
            cursor: None,
            overlaps: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: WaitResponse = serde_json::from_str(&json).unwrap();
        assert!(deserialized.timeout);
        assert_eq!(deserialized.timeout_after, Some(30));
        assert!(!deserialized.closed);
        assert_eq!(deserialized.cursor, None);
        assert!(deserialized.overlaps.is_empty());
    }
}
