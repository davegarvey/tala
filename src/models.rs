use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub hint: String,
}
impl Failure {
    pub fn new(code: &str, message: impl Into<String>, hint: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for Failure {}
impl From<rusqlite::Error> for Failure {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(
            "STORAGE_ERROR",
            error.to_string(),
            "Inspect tala status and daemon.log; preserve the database before attempting recovery.",
        )
    }
}
impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::new(
            "IO_ERROR",
            error.to_string(),
            "Check the file path and permissions.",
        )
    }
}
pub type Result<T> = std::result::Result<T, Failure>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Failure>,
}
impl Envelope {
    pub fn success(data: Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }
    pub fn failure(error: Failure) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub project: String,
    pub branch: Option<String>,
    pub revision: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub tool: String,
    pub context: Context,
    pub last_seen: i64,
    pub active: bool,
    pub listening: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Part {
    Text { text: String },
    File { path: String },
    Data { value: Value },
}
impl Part {
    pub fn valid(&self) -> bool {
        match self {
            Self::Text { text } => !text.trim().is_empty(),
            Self::File { path } => !path.trim().is_empty(),
            Self::Data { value } => !value.is_null(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub thread_id: String,
    pub sender: String,
    pub recipient: Option<String>,
    pub project: Option<String>,
    pub context: Context,
    pub parts: Vec<Part>,
    pub kind: String,
    pub reply_to: Option<u64>,
    pub expects_reply: bool,
    pub created_at: i64,
    pub received_at: Option<i64>,
    pub answered_by: Option<u64>,
    pub cancelled: bool,
    pub delivery: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub messages: Vec<Message>,
    pub next_after: u64,
    pub has_more: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub agent: String,
    pub token: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rpc {
    pub protocol_version: u32,
    pub request_id: String,
    pub credential: Option<Credential>,
    pub operation: Operation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    Register {
        name: String,
        tool: String,
        context: Context,
    },
    Whoami,
    Unregister,
    Agents {
        project: Option<String>,
        after: String,
        limit: usize,
    },
    Send {
        to: Option<String>,
        project: Option<String>,
        thread: Option<String>,
        reply_to: Option<u64>,
        parts: Vec<Part>,
        request: bool,
        kind: String,
        key: String,
        context: Context,
    },
    Inbox {
        peek: bool,
        after: u64,
        limit: usize,
        reply_to: Option<u64>,
        listening: bool,
    },
    Board {
        project: String,
        after: u64,
        limit: usize,
    },
    History {
        thread: String,
        after: u64,
        limit: usize,
    },
    Search {
        query: String,
        project: Option<String>,
        after: u64,
        limit: usize,
    },
    Pending {
        direction: String,
        after: u64,
        limit: usize,
    },
    Resolve {
        message: u64,
    },
    MessageStatus {
        message: u64,
    },
    Status,
    Stop,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub port: u16,
    pub protocol_version: u32,
    #[serde(default)]
    pub instance: String,
}
