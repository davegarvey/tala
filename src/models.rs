use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Wire protocol version. Bump this whenever the wire protocol changes
/// incompatibly (new required fields, changed shapes); the CLI refuses to
/// talk to a daemon with a mismatched version.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub port: u16,
    pub host: String,
    pub started_at: DateTime<Utc>,
    /// Stale daemon.json files (older daemons) deserialize as 0 → mismatch.
    #[serde(default)]
    pub protocol_version: u32,
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

/// The smallest unit of message content (A2A-inspired). A message's content is
/// an ordered list of parts; legacy string content is a single text part.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    Text {
        content: String,
    },
    File {
        path: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    Data {
        value: serde_json::Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

impl Part {
    /// Short placeholder used in summaries when a message has no text part.
    pub fn placeholder(&self) -> &'static str {
        match self {
            Part::Text { .. } => "[text]",
            Part::File { .. } => "[file]",
            Part::Data { .. } => "[data]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u64,
    pub session_id: String,
    pub sender: String,
    pub parts: Vec<Part>,
    pub timestamp: DateTime<Utc>,
    pub intent: Intent,
    pub reply_to: Option<u64>,
    pub expect_reply: bool,
    pub waiting_until: Option<DateTime<Utc>>,
    /// Client-generated idempotency key; None for legacy/created messages.
    pub idempotency_key: Option<String>,
}

impl Message {
    /// The message's text parts newline-joined (legacy `content` view).
    pub fn text_content(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Text { content } => Some(content.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Human rendering: text parts as-is, file/data parts as annotations, in
    /// part order.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for p in &self.parts {
            if !out.is_empty() {
                out.push('\n');
            }
            match p {
                Part::Text { content } => out.push_str(content),
                Part::File { path, label } => match label {
                    Some(l) => out.push_str(&format!("[file: {} ({})]", path, l)),
                    None => out.push_str(&format!("[file: {}]", path)),
                },
                Part::Data { value, label } => match label {
                    Some(l) => out.push_str(&format!("[data: {} ({})]", value, l)),
                    None => out.push_str(&format!("[data: {}]", value)),
                },
            }
        }
        out
    }

    /// Snippet for summaries: the first text part (truncated), or a
    /// placeholder derived from the first non-text part.
    pub fn snippet(&self) -> String {
        match self.parts.iter().find(|p| matches!(p, Part::Text { .. })) {
            Some(Part::Text { content }) => content.chars().take(120).collect(),
            _ => self
                .parts
                .first()
                .map(|p| p.placeholder().to_string())
                .unwrap_or_default(),
        }
    }

    /// Match filters (e.g. `listen --match`) apply to text parts only.
    pub fn matches(&self, needle: &str) -> bool {
        self.parts.iter().any(|p| match p {
            Part::Text { content } => content.contains(needle),
            _ => false,
        })
    }
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("Message", 11)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("session_id", &self.session_id)?;
        s.serialize_field("sender", &self.sender)?;
        s.serialize_field("content", &self.text_content())?;
        s.serialize_field("parts", &self.parts)?;
        s.serialize_field("timestamp", &self.timestamp)?;
        s.serialize_field("intent", &self.intent)?;
        if let Some(r) = &self.reply_to {
            s.serialize_field("reply_to", r)?;
        }
        s.serialize_field("expect_reply", &self.expect_reply)?;
        if let Some(w) = &self.waiting_until {
            s.serialize_field("waiting_until", w)?;
        }
        if let Some(k) = &self.idempotency_key {
            s.serialize_field("idempotency_key", k)?;
        }
        s.end()
    }
}

impl<'de> Deserialize<'de> for Message {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Inner {
            id: u64,
            session_id: String,
            sender: String,
            #[serde(default)]
            parts: Option<Vec<Part>>,
            #[serde(default)]
            content: Option<String>,
            timestamp: DateTime<Utc>,
            #[serde(default)]
            intent: Intent,
            #[serde(default)]
            reply_to: Option<u64>,
            #[serde(default)]
            expect_reply: bool,
            #[serde(default)]
            waiting_until: Option<DateTime<Utc>>,
            #[serde(default)]
            idempotency_key: Option<String>,
        }
        let inner = Inner::deserialize(deserializer)?;
        let parts = match inner.parts {
            Some(p) => p,
            None => inner
                .content
                .map(|c| vec![Part::Text { content: c }])
                .unwrap_or_default(),
        };
        Ok(Message {
            id: inner.id,
            session_id: inner.session_id,
            sender: inner.sender,
            parts,
            timestamp: inner.timestamp,
            intent: inner.intent,
            reply_to: inner.reply_to,
            expect_reply: inner.expect_reply,
            waiting_until: inner.waiting_until,
            idempotency_key: inner.idempotency_key,
        })
    }
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
    /// Legacy form: a single text part. Accepted for backward compatibility
    /// with older clients; `parts` takes precedence when both are present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<Part>>,
    /// Required: client-generated, reused across retries so the daemon can
    /// deduplicate (see send-idempotency spec).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
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
    /// Legacy `content` view (text parts newline-joined) for older clients.
    pub content: String,
    pub parts: Vec<Part>,
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
    /// True when the send was deduplicated against an earlier message with the
    /// same idempotency key; `id`/`session_id` then name the original message.
    #[serde(default)]
    pub duplicate: bool,
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
    /// Highest message id read per sender in this session (B021 read receipts).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub read_by: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub pid: u32,
    pub port: u16,
    pub uptime_seconds: i64,
    pub session_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_waits: Vec<ActiveWaitInfo>,
    /// Live protocol version for compatibility checks; older daemons (no
    /// field) report 0 via serde default.
    #[serde(default)]
    pub protocol_version: u32,
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
    /// Identity of the reader; used to record read receipts (B021).
    pub sender: Option<String>,
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
            parts: vec![Part::Text {
                content: "hello **world**".into(),
            }],
            timestamp: now,
            intent: Intent::Req,
            reply_to: Some(2),
            expect_reply: true,
            waiting_until: Some(now),
            idempotency_key: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, 1);
        assert_eq!(deserialized.sender, "test-agent");
        assert_eq!(deserialized.intent, Intent::Req);
        assert_eq!(deserialized.reply_to, Some(2));
        assert!(deserialized.expect_reply);
        assert_eq!(deserialized.parts, msg.parts);
    }

    #[test]
    fn test_message_serializes_parts_and_legacy_content() {
        let now = Utc::now();
        let msg = Message {
            id: 1,
            session_id: "sess_test".into(),
            sender: "a".into(),
            parts: vec![
                Part::Text {
                    content: "first".into(),
                },
                Part::File {
                    path: "src/lib.rs".into(),
                    label: None,
                },
                Part::Text {
                    content: "second".into(),
                },
            ],
            timestamp: now,
            intent: Intent::Fyi,
            reply_to: None,
            expect_reply: false,
            waiting_until: None,
            idempotency_key: Some("k1".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(val["content"], "first\nsecond");
        assert_eq!(val["parts"].as_array().map(|a| a.len()), Some(3));
        assert_eq!(val["parts"][1]["type"], "file");
        assert_eq!(val["idempotency_key"], "k1");
    }

    #[test]
    fn test_message_default_intent_is_fyi() {
        let json = r#"{"id":1,"session_id":"s","sender":"a","content":"c","timestamp":"2024-01-01T00:00:00Z"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(msg.intent, Intent::Fyi);
        assert_eq!(msg.reply_to, None);
        assert!(!msg.expect_reply);
        assert_eq!(msg.waiting_until, None);
        assert_eq!(msg.parts.len(), 1);
        assert_eq!(msg.render(), "c");
    }

    #[test]
    fn test_message_legacy_content_loads_as_text_part() {
        let json = r#"{"id":5,"session_id":"s","sender":"a","content":"legacy text","timestamp":"2024-01-01T00:00:00Z"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        assert_eq!(
            msg.parts,
            vec![Part::Text {
                content: "legacy text".into()
            }]
        );
        assert_eq!(msg.text_content(), "legacy text");
    }

    #[test]
    fn test_message_snippet_and_match() {
        let msg = Message {
            id: 1,
            session_id: "s".into(),
            sender: "a".into(),
            parts: vec![
                Part::Text {
                    content: "please check parse_row".into(),
                },
                Part::File {
                    path: "x".into(),
                    label: None,
                },
            ],
            timestamp: Utc::now(),
            intent: Intent::Req,
            reply_to: None,
            expect_reply: false,
            waiting_until: None,
            idempotency_key: None,
        };
        assert_eq!(msg.snippet(), "please check parse_row");
        assert!(msg.matches("parse_row"));
        assert!(!msg.matches("src/api.rs"));

        let file_only = Message {
            parts: vec![Part::File {
                path: "x".into(),
                label: None,
            }],
            ..msg
        };
        assert_eq!(file_only.snippet(), "[file]");
        assert!(!file_only.matches("anything"));
        assert_eq!(file_only.render(), "[file: x]");
    }

    #[test]
    fn test_daemon_info_serialization() {
        let info = DaemonInfo {
            pid: 12345,
            port: 54321,
            host: "127.0.0.1".into(),
            started_at: Utc::now(),
            protocol_version: PROTOCOL_VERSION,
        };
        let json = serde_json::to_string_pretty(&info).unwrap();
        assert!(json.contains("\"pid\": 12345"));
        assert!(json.contains("\"port\": 54321"));
        assert!(json.contains(&format!("\"protocol_version\": {}", PROTOCOL_VERSION)));

        let deserialized: DaemonInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pid, 12345);
        assert_eq!(deserialized.port, 54321);
        assert_eq!(deserialized.host, "127.0.0.1");
        assert_eq!(deserialized.protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn test_stale_daemon_info_reads_version_zero() {
        // A daemon.json written by an older tala has no protocol_version; it
        // must read as 0 so the CLI detects the mismatch.
        let json = r#"{"pid":1,"port":2,"host":"127.0.0.1","started_at":"2024-01-01T00:00:00Z"}"#;
        let info: DaemonInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.protocol_version, 0);
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
            content: Some("test".into()),
            parts: None,
            idempotency_key: Some("k1".into()),
            intent: Some(Intent::Req),
            reply_to: Some(1),
            expect_reply: true,
            wait_timeout: Some(30),
        };
        let json = serde_json::to_string(&msg_req).unwrap();
        let deserialized: SendMessageRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.content.as_deref(), Some("test"));
        assert_eq!(deserialized.idempotency_key.as_deref(), Some("k1"));
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
