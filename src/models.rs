use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 4;

/// Every error code with its retry class, meaning and default next command.
/// Retry classes: `safe` (repeat as is), `same_key` (repeat with the same
/// idempotency key), `after_fix` (change something first), `never`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Retry {
    Safe,
    SameKey,
    AfterFix,
    #[default]
    Never,
}
impl Retry {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::SameKey => "same_key",
            Self::AfterFix => "after_fix",
            Self::Never => "never",
        }
    }
}
pub const ERRORS: &[(&str, Retry, &str, Option<&str>)] = &[
    (
        "USAGE_ERROR",
        Retry::AfterFix,
        "Invalid arguments or input",
        Some("tala --help"),
    ),
    (
        "IDENTITY_REQUIRED",
        Retry::AfterFix,
        "No identity selected",
        Some("tala register --name=<name> --tool=<tool>"),
    ),
    (
        "IDENTITY_ERROR",
        Retry::AfterFix,
        "Handle or credential is not valid for acting as an agent",
        None,
    ),
    (
        "CONNECTION_ERROR",
        Retry::SameKey,
        "Request to the daemon failed in transit; nothing is known to be stored",
        None,
    ),
    (
        "DAEMON_UNREACHABLE",
        Retry::SameKey,
        "Daemon may be running but cannot be reached; sandbox or network rules may block loopback",
        Some("tala status"),
    ),
    (
        "DAEMON_START_FAILED",
        Retry::AfterFix,
        "A new daemon did not start",
        Some("tala status"),
    ),
    (
        "DAEMON_NOT_RUNNING",
        Retry::AfterFix,
        "Command needs a running daemon",
        Some("tala whoami"),
    ),
    (
        "DAEMON_ALREADY_RUNNING",
        Retry::Never,
        "Another daemon owns this Tala home",
        Some("tala status"),
    ),
    (
        "DAEMON_IDENTITY",
        Retry::AfterFix,
        "Request reached a different daemon instance",
        Some("tala status"),
    ),
    (
        "DAEMON_ERROR",
        Retry::Never,
        "Daemon failed internally",
        Some("tala status"),
    ),
    (
        "PROTOCOL_MISMATCH",
        Retry::AfterFix,
        "CLI and daemon protocol versions differ",
        Some("tala status"),
    ),
    (
        "STOP_TIMEOUT",
        Retry::Safe,
        "Daemon did not finish shutting down in time",
        Some("tala status"),
    ),
    (
        "INVALID_REQUEST",
        Retry::AfterFix,
        "Daemon rejected the request body",
        None,
    ),
    (
        "INVALID_OPERATION",
        Retry::AfterFix,
        "Operation is not allowed in this state",
        Some("tala --help"),
    ),
    (
        "NOT_PROJECT_OWNER",
        Retry::Never,
        "Only the owning agent can change project lifecycle",
        None,
    ),
    (
        "RECIPIENT_NOT_FOUND",
        Retry::AfterFix,
        "No agent has that ID or name",
        Some("tala agents"),
    ),
    (
        "AMBIGUOUS_RECIPIENT",
        Retry::AfterFix,
        "Several agents share that name",
        Some("tala agents"),
    ),
    (
        "MESSAGE_NOT_FOUND",
        Retry::AfterFix,
        "No message has that ID",
        Some("tala"),
    ),
    (
        "THREAD_NOT_VISIBLE",
        Retry::AfterFix,
        "Thread is missing or not visible to this agent",
        Some("tala"),
    ),
    (
        "NOT_RECIPIENT",
        Retry::Never,
        "Only the addressed recipient can reply",
        None,
    ),
    (
        "IDEMPOTENCY_CONFLICT",
        Retry::AfterFix,
        "Key already used for a different message; use a new key",
        None,
    ),
    (
        "WAIT_FAILED",
        Retry::Never,
        "Message was stored but waiting for its reply failed; do not resend",
        None,
    ),
    (
        "STORAGE_ERROR",
        Retry::Never,
        "Storage failed; preserve the database",
        Some("tala status"),
    ),
    (
        "DATABASE_VERSION",
        Retry::Never,
        "Database was written by an incompatible Tala version",
        Some("tala status"),
    ),
    (
        "SERIALIZATION_ERROR",
        Retry::Never,
        "Internal encoding failed",
        None,
    ),
    (
        "IO_ERROR",
        Retry::AfterFix,
        "A file could not be read or written",
        None,
    ),
    (
        "INTEGRATION_WRITE_FAILED",
        Retry::AfterFix,
        "Generated instructions could not be written",
        Some("tala init --check"),
    ),
];
type ErrorEntry = (&'static str, Retry, &'static str, Option<&'static str>);
fn lookup(code: &str) -> Option<&'static ErrorEntry> {
    ERRORS.iter().find(|e| e.0 == code)
}
pub fn help_text() -> String {
    let mut text = String::from(
        "Start: tala register --name=codex --tool=codex\n\
Pass the returned private handle with --agent=<handle> or TALA_AGENT on every command.\n\
Then: tala (what needs attention); tala agents; tala send --to=<id> --request 'Review this'; tala inbox; tala reply <id> 'Done'\n\
Projects: tala list-projects [--stale|--closed|--all]; owner lifecycle: tala close-project / tala reopen-project\n\
Output is one JSON envelope; --human renders text.\n\n\
Exit codes: 0 success, 1 operational error (nothing stored), 2 usage error, 3 wait timeout, 4 stored but wait failed (do not resend).\n\n\
Error codes (retry class: safe | same_key | after_fix | never):\n",
    );
    for (code, retry, meaning, _) in ERRORS {
        text.push_str(&format!("  {code:<25} {:<9} {meaning}\n", retry.as_str()));
    }
    text
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Failure {
    pub code: String,
    pub message: String,
    pub hint: String,
    #[serde(default)]
    pub retry: Retry,
    #[serde(default)]
    pub stored: Option<u64>,
    #[serde(default)]
    pub next: Option<String>,
}
impl Failure {
    pub fn new(code: &str, message: impl Into<String>, hint: &str) -> Self {
        let entry = lookup(code);
        debug_assert!(entry.is_some(), "error code {code} missing from ERRORS");
        Self {
            code: code.into(),
            message: message.into(),
            hint: hint.into(),
            retry: entry.map_or(Retry::Never, |e| e.1),
            stored: None,
            next: entry.and_then(|e| e.3).map(Into::into),
        }
    }
    pub fn with_next(mut self, next: impl Into<String>) -> Self {
        self.next = Some(next.into());
        self
    }
    pub fn is_transport(&self) -> bool {
        ["CONNECTION_ERROR", "DAEMON_UNREACHABLE"].contains(&self.code.as_str())
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
    /// Compact summary of work awaiting the selected agent; omitted when none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention: Option<Value>,
}
impl Envelope {
    pub fn success(data: Value) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            attention: None,
        }
    }
    pub fn failure(error: Failure) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(error),
            attention: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub project: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,
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
    pub age_seconds: i64,
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
    ListProjects {
        include_stale: bool,
        include_closed: bool,
        after: String,
        limit: usize,
    },
    CloseProject,
    ReopenProject,
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
        stale: bool,
        after: u64,
        limit: usize,
    },
    Attention,
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

#[cfg(test)]
mod tests {
    use super::*;
    /// Every code passed to Failure::new anywhere in the source is listed in
    /// ERRORS, so help output cannot omit a code the CLI can emit.
    #[test]
    fn every_emitted_error_code_is_documented() {
        let sources = [
            include_str!("api.rs"),
            include_str!("cli.rs"),
            include_str!("client.rs"),
            include_str!("daemon.rs"),
            include_str!("integration.rs"),
            include_str!("main.rs"),
            include_str!("models.rs"),
            include_str!("store.rs"),
        ];
        let mut found = 0;
        for source in sources {
            for (i, _) in source.match_indices("Failure::new(") {
                let rest = source[i + "Failure::new(".len()..].trim_start();
                let Some(rest) = rest.strip_prefix('"') else {
                    continue;
                };
                let code = &rest[..rest.find('"').unwrap()];
                if code.is_empty() || !code.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    continue;
                }
                assert!(
                    lookup(code).is_some(),
                    "{code} is emitted but not in ERRORS"
                );
                found += 1;
            }
        }
        assert!(found > 20);
        for (code, ..) in ERRORS {
            assert!(help_text().contains(code));
        }
    }
    #[test]
    fn failures_carry_retry_class_and_nullable_fields() {
        for (code, retry, _, next) in ERRORS {
            let f = Failure::new(code, "m", "h");
            assert_eq!(f.retry, *retry);
            assert_eq!(f.next.as_deref(), *next);
            let v = serde_json::to_value(&f).unwrap();
            assert_eq!(v["retry"], retry.as_str());
            assert!(v["stored"].is_null() && v.get("stored").is_some());
            assert!(v.get("next").is_some());
        }
        let f = Failure::new("RECIPIENT_NOT_FOUND", "m", "h");
        assert_eq!(
            (f.retry.as_str(), f.next.as_deref()),
            ("after_fix", Some("tala agents"))
        );
    }
}
