//! Read-only queries for the local web viewer.
//!
//! Every query runs on a connection opened with `SQLITE_OPEN_READ_ONLY` and
//! `PRAGMA query_only=ON`, inside one short read transaction per request.
//! No query here may read the agents' private handle column.

use crate::models::{Failure, Result};
use crate::store::SCHEMA_VERSION;
use rusqlite::{params_from_iter, types::Value as Sql, Connection, OpenFlags, Row};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

/// An agent counts as active when it was seen within this many seconds.
pub const ACTIVE_WINDOW: i64 = 86_400;
/// Agents seen within this many seconds are shown as recently active.
const RECENT_WINDOW: i64 = 900;
pub const DEFAULT_LIMIT: usize = 200;
pub const MAX_LIMIT: usize = 1000;
const THREAD_LIMIT: usize = 100;
const TRANSCRIPT_LIMIT: usize = 2000;
const PAIR_LIMIT: usize = 20;
const ATTENTION_LIMIT: usize = 200;

/// Columns every message query selects, in the order `message_row` reads.
const MESSAGE_COLUMNS: &str = "m.id,m.thread,m.sender,m.recipient,\
    COALESCE(m.project,json_extract(m.context,'$.project')),m.kind,m.reply_to,m.expects_reply,\
    m.created_at,m.received_at,m.answered_by,m.cancelled,m.parts,json_extract(m.context,'$.branch')";

fn db_error(error: rusqlite::Error) -> Failure {
    Failure::new(
        "STORAGE_ERROR",
        error.to_string(),
        "Retry; if it persists, check TALA_HOME/daemon.log and preserve the database.",
    )
}

/// Opens the store read-only and checks that its schema matches this binary.
pub fn open(path: &Path) -> Result<Connection> {
    if !path.exists() {
        return Err(Failure::new(
            "VIEWER_NO_STORE",
            format!("No Tala store at {}", path.display()),
            "Use Tala in this home first (for example tala register), or set TALA_HOME to a home that has one.",
        ));
    }
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(db_error)?;
    db.busy_timeout(std::time::Duration::from_secs(2))
        .map_err(db_error)?;
    db.execute_batch("PRAGMA query_only=ON").map_err(db_error)?;
    let version: i64 = db
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(db_error)?;
    if version != SCHEMA_VERSION {
        let hint = if version < SCHEMA_VERSION {
            "Run any messaging command, such as tala agents, so this binary's daemon migrates the store, then retry."
        } else {
            "Use the Tala binary that last wrote this store, or upgrade this one."
        };
        return Err(Failure::new(
            "DATABASE_VERSION",
            format!("Store schema {version} does not match this binary's schema {SCHEMA_VERSION}"),
            hint,
        ));
    }
    Ok(db)
}

/// Filters shared by every view. `project` is a full project path.
#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub project: Option<String>,
    pub agent: Option<String>,
    pub text: Option<String>,
}
impl Filter {
    /// SQL conditions over the message alias `m`, with their parameters.
    fn clause(&self) -> (String, Vec<Sql>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        if let Some(project) = &self.project {
            sql.push_str(" AND COALESCE(m.project,json_extract(m.context,'$.project'))=?");
            params.push(Sql::Text(project.clone()));
        }
        if let Some(agent) = &self.agent {
            sql.push_str(" AND (m.sender=? OR m.recipient=?)");
            params.push(Sql::Text(agent.clone()));
            params.push(Sql::Text(agent.clone()));
        }
        if let Some(text) = &self.text {
            let escaped = text
                .to_lowercase()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            sql.push_str(" AND lower(m.parts) LIKE ? ESCAPE '\\'");
            params.push(Sql::Text(format!("%{escaped}%")));
        }
        (sql, params)
    }
    fn is_empty(&self) -> bool {
        self.project.is_none() && self.agent.is_none() && self.text.is_none()
    }
}

/// A message row with its derived request and delivery state.
#[derive(Debug, Clone)]
struct MessageRow {
    id: i64,
    thread: String,
    sender: String,
    recipient: Option<String>,
    project: Option<String>,
    kind: String,
    reply_to: Option<i64>,
    expects_reply: bool,
    created_at: i64,
    received_at: Option<i64>,
    answered_by: Option<i64>,
    cancelled: bool,
    parts: String,
    branch: Option<String>,
}
impl MessageRow {
    fn read(row: &Row) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            thread: row.get(1)?,
            sender: row.get(2)?,
            recipient: row.get(3)?,
            project: row.get(4)?,
            kind: row.get(5)?,
            reply_to: row.get(6)?,
            expects_reply: row.get(7)?,
            created_at: row.get(8)?,
            received_at: row.get(9)?,
            answered_by: row.get(10)?,
            cancelled: row.get(11)?,
            parts: row.get(12)?,
            branch: row.get(13)?,
        })
    }
    fn unread(&self) -> bool {
        self.recipient.is_some() && self.received_at.is_none() && !self.cancelled
    }
}

/// Event types recorded against messages, keyed by message ID.
type Events = HashMap<i64, HashSet<String>>;

fn events_for(db: &Connection, rows: &[MessageRow]) -> Result<Events> {
    let mut events = Events::new();
    let (Some(lo), Some(hi)) = (
        rows.iter().map(|m| m.id).min(),
        rows.iter().map(|m| m.id).max(),
    ) else {
        return Ok(events);
    };
    let wanted: HashSet<i64> = rows.iter().map(|m| m.id).collect();
    let mut stmt = db
        .prepare("SELECT message,type FROM agent_events WHERE message BETWEEN ? AND ?")
        .map_err(db_error)?;
    let found = stmt
        .query_map([lo, hi], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(db_error)?;
    for item in found {
        let (message, kind) = item.map_err(db_error)?;
        if wanted.contains(&message) {
            events.entry(message).or_default().insert(kind);
        }
    }
    Ok(events)
}

/// Request state: answered, cancelled, declined, accepted or open.
fn request_state(m: &MessageRow, events: &Events) -> Option<&'static str> {
    if !m.expects_reply {
        return None;
    }
    let has = |kind: &str| events.get(&m.id).is_some_and(|e| e.contains(kind));
    Some(if m.cancelled {
        "cancelled"
    } else if m.answered_by.is_some() {
        "answered"
    } else if has("request_declined") {
        "declined"
    } else if has("request_accepted") {
        "accepted"
    } else {
        "open"
    })
}

/// Text of the stored parts. Non-text parts are shown as labelled values.
fn parts_text(parts: &str) -> String {
    let Ok(Value::Array(parts)) = serde_json::from_str::<Value>(parts) else {
        return parts.to_string();
    };
    parts
        .iter()
        .map(|p| match p["type"].as_str() {
            Some("text") => p["text"].as_str().unwrap_or_default().to_string(),
            Some("file") => format!("[file] {}", p["path"].as_str().unwrap_or_default()),
            Some("data") => format!("[data] {}", p["value"]),
            _ => p.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn message_json(m: &MessageRow, events: &Events) -> Value {
    let unread = m.unread();
    let overdue = unread
        && events
            .get(&m.id)
            .is_some_and(|e| e.contains("delivery_overdue"));
    json!({
        "id": m.id,
        "thread": m.thread,
        "sender": m.sender,
        "recipient": m.recipient,
        "project": m.project,
        "branch": m.branch,
        "kind": m.kind,
        "reply_to": m.reply_to,
        "expects_reply": m.expects_reply,
        "created_at": m.created_at,
        "received_at": m.received_at,
        "answered_by": m.answered_by,
        "state": request_state(m, events),
        "unread": unread,
        "overdue": overdue,
        "text": parts_text(&m.parts),
    })
}

fn messages_where(
    db: &Connection,
    filter: &Filter,
    extra: &str,
    extra_params: Vec<Sql>,
    tail: &str,
) -> Result<Vec<MessageRow>> {
    let (clause, mut params) = filter.clause();
    params.extend(extra_params);
    let sql = format!("SELECT {MESSAGE_COLUMNS} FROM messages m WHERE 1=1{clause}{extra} {tail}");
    let mut stmt = db.prepare(&sql).map_err(db_error)?;
    let rows = stmt
        .query_map(params_from_iter(params), MessageRow::read)
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    Ok(rows)
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

/// A change marker that differs whenever anything a view shows may have changed.
pub fn head(db: &Connection) -> Result<String> {
    db.query_row(
        "SELECT (SELECT IFNULL(MAX(id),0) FROM messages),\
                (SELECT IFNULL(MAX(id),0) FROM agent_events),\
                (SELECT COUNT(*) FROM messages WHERE received_at IS NOT NULL),\
                (SELECT COUNT(*) FROM messages WHERE answered_by IS NOT NULL OR cancelled=1),\
                (SELECT IFNULL(MAX(last_seen),0) FROM agents),\
                (SELECT IFNULL(MAX(listening_until),0) FROM agents),\
                (SELECT COUNT(*) FROM agents)",
        [],
        |r| {
            let parts: Vec<i64> = (0..7).map(|i| r.get(i)).collect::<rusqlite::Result<_>>()?;
            Ok(parts
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join("."))
        },
    )
    .map_err(db_error)
}

/// Agents (public fields only), projects with messages, and the global
/// attention count.
pub fn meta(db: &Connection) -> Result<Value> {
    let mut stmt = db
        .prepare(
            "SELECT id,name,tool,json_extract(context,'$.project'),json_extract(context,'$.branch'),\
                    last_seen,listening_until,active FROM agents ORDER BY last_seen DESC",
        )
        .map_err(db_error)?;
    let now = now();
    let agents = stmt
        .query_map([], |r| {
            let last_seen: i64 = r.get(5)?;
            let listening_until: i64 = r.get(6)?;
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "tool": r.get::<_, String>(2)?,
                "project": r.get::<_, Option<String>>(3)?,
                "branch": r.get::<_, Option<String>>(4)?,
                "last_seen": last_seen,
                "active": r.get::<_, bool>(7)?,
                "listening": listening_until > now,
                "recent": now - last_seen < RECENT_WINDOW,
            }))
        })
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    let mut stmt = db
        .prepare(
            "SELECT DISTINCT COALESCE(project,json_extract(context,'$.project')) AS p \
             FROM messages WHERE p IS NOT NULL ORDER BY p",
        )
        .map_err(db_error)?;
    let projects = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    let attention = attention(db, &Filter::default())?["count"].clone();
    Ok(json!({
        "now": now,
        "agents": agents,
        "projects": projects,
        "attention_count": attention,
    }))
}

/// A page of messages, newest first, older than `before` when given.
pub fn messages(
    db: &Connection,
    filter: &Filter,
    before: Option<i64>,
    limit: usize,
) -> Result<Value> {
    let limit = limit.clamp(1, MAX_LIMIT);
    let (extra, params) = match before {
        Some(id) => (" AND m.id<?", vec![Sql::Integer(id)]),
        None => ("", vec![]),
    };
    let mut rows = messages_where(
        db,
        filter,
        extra,
        params,
        &format!("ORDER BY m.id DESC LIMIT {}", limit + 1),
    )?;
    let more = rows.len() > limit;
    rows.truncate(limit);
    let events = events_for(db, &rows)?;
    Ok(json!({
        "messages": rows.iter().map(|m| message_json(m, &events)).collect::<Vec<_>>(),
        "more": more,
    }))
}

/// Threads with a matching message, by latest activity.
pub fn threads(db: &Connection, filter: &Filter, before: Option<i64>) -> Result<Value> {
    let (clause, mut params) = filter.clause();
    let before_sql = if let Some(id) = before {
        params.push(Sql::Integer(id));
        " HAVING MAX(t.id)<?"
    } else {
        ""
    };
    let sql = format!(
        "WITH chosen AS (SELECT DISTINCT m.thread FROM messages m WHERE 1=1{clause}) \
         SELECT t.thread,COUNT(*),MAX(t.id),MAX(t.created_at),MAX(t.kind='post'),\
                MAX(COALESCE(t.project,json_extract(t.context,'$.project'))),\
                SUM(t.expects_reply=1 AND t.answered_by IS NULL AND t.cancelled=0 AND NOT EXISTS(\
                    SELECT 1 FROM agent_events e WHERE e.message=t.id AND e.type='request_declined'))\
         FROM messages t JOIN chosen USING(thread) GROUP BY t.thread{before_sql} \
         ORDER BY MAX(t.id) DESC LIMIT {}",
        THREAD_LIMIT + 1
    );
    let mut stmt = db.prepare(&sql).map_err(db_error)?;
    let mut rows = stmt
        .query_map(params_from_iter(params), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    let more = rows.len() > THREAD_LIMIT;
    rows.truncate(THREAD_LIMIT);
    let mut latest = db
        .prepare("SELECT parts FROM messages WHERE id=?")
        .map_err(db_error)?;
    let mut people = db
        .prepare(
            "SELECT sender,MIN(id) FROM messages WHERE thread=?1 GROUP BY sender \
             UNION SELECT recipient,MIN(id) FROM messages WHERE thread=?1 AND recipient IS NOT NULL \
             GROUP BY recipient ORDER BY 2",
        )
        .map_err(db_error)?;
    let mut out = Vec::with_capacity(rows.len());
    for (thread, count, last_id, last_at, board, project, waiting) in rows {
        let parts: String = latest
            .query_row([last_id], |r| r.get(0))
            .map_err(db_error)?;
        let mut participants: Vec<String> = Vec::new();
        for who in people
            .query_map([&thread], |r| r.get::<_, String>(0))
            .map_err(db_error)?
        {
            let who = who.map_err(db_error)?;
            if !participants.contains(&who) {
                participants.push(who);
            }
        }
        let mut excerpt = parts_text(&parts);
        if let Some((cut, _)) = excerpt.char_indices().nth(200) {
            excerpt.truncate(cut);
        }
        out.push(json!({
            "thread": thread,
            "count": count,
            "latest_id": last_id,
            "latest_at": last_at,
            "board": board,
            "project": project,
            "waiting": waiting > 0,
            "participants": participants,
            "excerpt": excerpt,
        }));
    }
    Ok(json!({"threads": out, "more": more}))
}

/// The full transcript of one thread, oldest first.
pub fn transcript(db: &Connection, thread: &str) -> Result<Value> {
    let rows = messages_where(
        db,
        &Filter::default(),
        " AND m.thread=?",
        vec![Sql::Text(thread.to_string())],
        &format!("ORDER BY m.id LIMIT {TRANSCRIPT_LIMIT}"),
    )?;
    let events = events_for(db, &rows)?;
    Ok(json!({
        "thread": thread,
        "messages": rows.iter().map(|m| message_json(m, &events)).collect::<Vec<_>>(),
        "truncated": rows.len() == TRANSCRIPT_LIMIT,
    }))
}

/// Per-agent counts over matching messages, plus the busiest pairs.
pub fn agents(db: &Connection, filter: &Filter) -> Result<Value> {
    let (clause, params) = filter.clause();
    let sql = format!("SELECT m.sender,m.recipient FROM messages m WHERE 1=1{clause}");
    let mut stmt = db.prepare(&sql).map_err(db_error)?;
    let pairs_rows = stmt
        .query_map(params_from_iter(params), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
        })
        .map_err(db_error)?;
    let mut sent: HashMap<String, i64> = HashMap::new();
    let mut received: HashMap<String, i64> = HashMap::new();
    let mut pairs: BTreeMap<(String, String), i64> = BTreeMap::new();
    for row in pairs_rows {
        let (sender, recipient) = row.map_err(db_error)?;
        *sent.entry(sender.clone()).or_default() += 1;
        if let Some(recipient) = recipient {
            *received.entry(recipient.clone()).or_default() += 1;
            let key = if sender <= recipient {
                (sender, recipient)
            } else {
                (recipient, sender)
            };
            *pairs.entry(key).or_default() += 1;
        }
    }
    let unread = count_by(
        db,
        "SELECT recipient,COUNT(*) FROM messages WHERE recipient IS NOT NULL \
         AND received_at IS NULL AND cancelled=0 GROUP BY recipient",
    )?;
    let owes = count_by(
        db,
        "SELECT recipient,COUNT(*) FROM messages m WHERE recipient IS NOT NULL AND expects_reply=1 \
         AND answered_by IS NULL AND cancelled=0 AND NOT EXISTS(SELECT 1 FROM agent_events e \
         WHERE e.message=m.id AND e.type='request_declined') GROUP BY recipient",
    )?;
    let mut ids: Vec<String> = db
        .prepare("SELECT id FROM agents ORDER BY last_seen DESC")
        .map_err(db_error)?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    if !filter.is_empty() {
        ids.retain(|id| sent.contains_key(id) || received.contains_key(id));
    }
    let stats: Vec<Value> = ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "sent": sent.get(id).copied().unwrap_or(0),
                "received": received.get(id).copied().unwrap_or(0),
                "unread": unread.get(id).copied().unwrap_or(0),
                "owes": owes.get(id).copied().unwrap_or(0),
            })
        })
        .collect();
    let mut ranked: Vec<_> = pairs.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(PAIR_LIMIT);
    let pairs: Vec<Value> = ranked
        .into_iter()
        .map(|((a, b), n)| json!({"agents": [a, b], "count": n}))
        .collect();
    Ok(json!({"agents": stats, "pairs": pairs}))
}

fn count_by(db: &Connection, sql: &str) -> Result<HashMap<String, i64>> {
    let mut stmt = db.prepare(sql).map_err(db_error)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(db_error)?
        .collect::<rusqlite::Result<HashMap<_, _>>>()
        .map_err(db_error)?;
    Ok(rows)
}

/// Requests awaiting a reply and unread messages for active recipients, and
/// requests to recipients that have gone quiet.
pub fn attention(db: &Connection, filter: &Filter) -> Result<Value> {
    let cutoff = now() - ACTIVE_WINDOW;
    let active: HashSet<String> = db
        .prepare("SELECT id FROM agents WHERE last_seen>=?")
        .map_err(db_error)?
        .query_map([cutoff], |r| r.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<rusqlite::Result<_>>()
        .map_err(db_error)?;
    let rows = messages_where(
        db,
        filter,
        " AND m.recipient IS NOT NULL AND m.cancelled=0 \
          AND ((m.expects_reply=1 AND m.answered_by IS NULL) OR m.received_at IS NULL)",
        vec![],
        "ORDER BY m.id DESC",
    )?;
    let events = events_for(db, &rows)?;
    let mut awaiting = Vec::new();
    let mut unread = Vec::new();
    let mut abandoned = Vec::new();
    let mut quiet_unread = 0;
    let mut counted = HashSet::new();
    for m in &rows {
        let live = m.recipient.as_ref().is_some_and(|r| active.contains(r));
        let state = request_state(m, &events);
        let waiting = matches!(state, Some("open" | "accepted"));
        if waiting {
            if live {
                awaiting.push(m);
                counted.insert(m.id);
            } else {
                abandoned.push(m);
            }
        }
        if m.unread() {
            if live {
                unread.push(m);
                counted.insert(m.id);
            } else {
                quiet_unread += 1;
            }
        }
    }
    let listening: HashSet<String> = db
        .prepare("SELECT id FROM agents WHERE listening_until>?")
        .map_err(db_error)?
        .query_map([now()], |r| r.get::<_, String>(0))
        .map_err(db_error)?
        .collect::<rusqlite::Result<_>>()
        .map_err(db_error)?;
    let list = |items: Vec<&MessageRow>| -> Vec<Value> {
        items
            .into_iter()
            .take(ATTENTION_LIMIT)
            .map(|m| {
                let mut v = message_json(m, &events);
                v["recipient_listening"] =
                    json!(m.recipient.as_ref().is_some_and(|r| listening.contains(r)));
                v
            })
            .collect()
    };
    Ok(json!({
        "count": counted.len(),
        "awaiting": list(awaiting),
        "unread": list(unread),
        "abandoned": list(abandoned),
        "quiet_unread": quiet_unread,
        "active_window_seconds": ACTIVE_WINDOW,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::params;
    use tempfile::TempDir;

    /// A freshly migrated store with direct SQL access for seeding.
    struct Fixture {
        _dir: TempDir,
        path: std::path::PathBuf,
        db: Connection,
        next_key: i64,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("tala.sqlite3");
            drop(crate::store::Store::open(&path).unwrap());
            let db = Connection::open(&path).unwrap();
            Self {
                _dir: dir,
                path,
                db,
                next_key: 0,
            }
        }
        fn agent(&self, id: &str, last_seen_ago: i64, listening: bool) {
            let now = now();
            self.db
                .execute(
                    "INSERT INTO agents(id,name,tool,context,token,last_seen,listening_until) \
                     VALUES(?1,?1,'fixture','{\"project\":\"/p/alpha\",\"branch\":\"main\"}',?2,?3,?4)",
                    params![
                        id,
                        format!("SECRET-HANDLE-{id}"),
                        now - last_seen_ago,
                        if listening { now + 600 } else { 0 }
                    ],
                )
                .unwrap();
        }
        #[allow(clippy::too_many_arguments)]
        fn message(
            &mut self,
            thread: &str,
            sender: &str,
            recipient: Option<&str>,
            text: &str,
            request: bool,
            read: bool,
        ) -> i64 {
            self.next_key += 1;
            let parts = json!([{"type":"text","text":text}]).to_string();
            let kind = if recipient.is_some() {
                "message"
            } else {
                "post"
            };
            let project: Option<&str> = recipient.is_none().then_some("/p/alpha");
            self.db
                .execute(
                    "INSERT INTO messages(thread,sender,recipient,project,context,parts,kind,expects_reply,\
                     created_at,received_at,key,canonical) \
                     VALUES(?,?,?,?,'{\"project\":\"/p/alpha\",\"branch\":\"main\"}',?,?,?,?,?,?,'c')",
                    params![
                        thread,
                        sender,
                        recipient,
                        project,
                        parts,
                        kind,
                        request,
                        now() - 100 + self.next_key,
                        read.then(now),
                        format!("k{}", self.next_key)
                    ],
                )
                .unwrap();
            self.db.last_insert_rowid()
        }
        fn event(&self, agent: &str, kind: &str, message: i64) {
            self.db
                .execute(
                    "INSERT INTO agent_events(agent,type,message,created_at) VALUES(?,?,?,?)",
                    params![agent, kind, message, now()],
                )
                .unwrap();
        }
        fn reader(&self) -> Connection {
            open(&self.path).unwrap()
        }
    }

    fn seeded() -> Fixture {
        let f = Fixture::new();
        f.agent("agt_pm", 10, true);
        f.agent("agt_w1", 60, false);
        f.agent("agt_w2", 120, false);
        f.agent("agt_gone", 2 * ACTIVE_WINDOW, false);
        f
    }

    #[test]
    fn fresh_store_matches_schema_version() {
        let f = Fixture::new();
        let v: i64 =
            f.db.query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
        assert_eq!(v, SCHEMA_VERSION);
        open(&f.path).unwrap();
    }

    #[test]
    fn open_refuses_missing_and_mismatched_stores() {
        let dir = TempDir::new().unwrap();
        let missing = open(&dir.path().join("tala.sqlite3")).unwrap_err();
        assert_eq!(missing.code, "VIEWER_NO_STORE");
        let f = Fixture::new();
        f.db.execute_batch("PRAGMA user_version=5").unwrap();
        let older = open(&f.path).unwrap_err();
        assert_eq!(older.code, "DATABASE_VERSION");
        assert!(older.hint.contains("migrates"));
        f.db.execute_batch("PRAGMA user_version=99").unwrap();
        let newer = open(&f.path).unwrap_err();
        assert_eq!(newer.code, "DATABASE_VERSION");
    }

    #[test]
    fn reader_cannot_write() {
        let f = seeded();
        let db = f.reader();
        assert!(db.execute("DELETE FROM agents", []).is_err());
    }

    #[test]
    fn head_changes_on_send_receipt_and_event() {
        let mut f = seeded();
        let db = f.reader();
        let h0 = head(&db).unwrap();
        let id = f.message("t1", "agt_pm", Some("agt_w1"), "hi", true, false);
        let h1 = head(&db).unwrap();
        assert_ne!(h0, h1);
        f.db.execute("UPDATE messages SET received_at=1 WHERE id=?", [id])
            .unwrap();
        let h2 = head(&db).unwrap();
        assert_ne!(h1, h2);
        f.event("agt_w1", "request_accepted", id);
        assert_ne!(h2, head(&db).unwrap());
    }

    #[test]
    fn derived_states() {
        let mut f = seeded();
        let open_req = f.message("t1", "agt_pm", Some("agt_w1"), "open", true, true);
        let accepted = f.message("t2", "agt_pm", Some("agt_w1"), "accepted", true, true);
        f.event("agt_w1", "request_accepted", accepted);
        let declined = f.message("t3", "agt_pm", Some("agt_w1"), "declined", true, true);
        f.event("agt_w1", "request_declined", declined);
        let answered = f.message("t4", "agt_pm", Some("agt_w1"), "answered", true, true);
        let reply = f.message("t4", "agt_w1", Some("agt_pm"), "reply", false, true);
        f.db.execute(
            "UPDATE messages SET answered_by=? WHERE id=?",
            [reply, answered],
        )
        .unwrap();
        let cancelled = f.message("t5", "agt_pm", Some("agt_w1"), "cancelled", true, true);
        f.db.execute("UPDATE messages SET cancelled=1 WHERE id=?", [cancelled])
            .unwrap();
        let overdue = f.message("t6", "agt_pm", Some("agt_w2"), "overdue", false, false);
        f.event("agt_pm", "delivery_overdue", overdue);
        let db = f.reader();
        let page = messages(&db, &Filter::default(), None, 50).unwrap();
        let by_id: HashMap<i64, &Value> = page["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| (m["id"].as_i64().unwrap(), m))
            .collect();
        assert_eq!(by_id[&open_req]["state"], "open");
        assert_eq!(by_id[&accepted]["state"], "accepted");
        assert_eq!(by_id[&declined]["state"], "declined");
        assert_eq!(by_id[&answered]["state"], "answered");
        assert_eq!(by_id[&cancelled]["state"], "cancelled");
        assert!(by_id[&reply]["state"].is_null());
        assert_eq!(by_id[&overdue]["unread"], true);
        assert_eq!(by_id[&overdue]["overdue"], true);
        assert_eq!(by_id[&open_req]["unread"], false);
        assert_eq!(by_id[&open_req]["text"], "open");
    }

    #[test]
    fn filters_and_pages() {
        let mut f = seeded();
        for i in 0..25 {
            let to = if i % 2 == 0 { "agt_w1" } else { "agt_w2" };
            f.message(
                &format!("t{i}"),
                "agt_pm",
                Some(to),
                &format!("note {i}"),
                false,
                true,
            );
        }
        f.message("board", "agt_w1", None, "Board 100% done_ok", false, true);
        let db = f.reader();
        let all = Filter::default();
        let first = messages(&db, &all, None, 10).unwrap();
        assert_eq!(first["more"], true);
        let mut seen: Vec<i64> = first["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_i64().unwrap())
            .collect();
        loop {
            let before = *seen.last().unwrap();
            let page = messages(&db, &all, Some(before), 10).unwrap();
            seen.extend(
                page["messages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|m| m["id"].as_i64().unwrap()),
            );
            if page["more"] == false {
                break;
            }
        }
        let mut expected = seen.clone();
        expected.sort_unstable_by(|a, b| b.cmp(a));
        expected.dedup();
        assert_eq!(seen, expected, "pages overlap or are out of order");
        assert_eq!(seen.len(), 26);

        let agent = Filter {
            agent: Some("agt_w2".into()),
            ..Filter::default()
        };
        let page = messages(&db, &agent, None, 100).unwrap();
        assert_eq!(page["messages"].as_array().unwrap().len(), 12);

        let project = Filter {
            project: Some("/p/alpha".into()),
            ..Filter::default()
        };
        assert_eq!(
            messages(&db, &project, None, 100).unwrap()["messages"]
                .as_array()
                .unwrap()
                .len(),
            26
        );
        let other = Filter {
            project: Some("/p/beta".into()),
            ..Filter::default()
        };
        assert!(messages(&db, &other, None, 100).unwrap()["messages"]
            .as_array()
            .unwrap()
            .is_empty());

        let text = |q: &str| Filter {
            text: Some(q.into()),
            ..Filter::default()
        };
        let hits = messages(&db, &text("100%"), None, 100).unwrap();
        assert_eq!(hits["messages"].as_array().unwrap().len(), 1);
        assert_eq!(hits["messages"][0]["kind"], "post");
        assert_eq!(hits["messages"][0]["project"], "/p/alpha");
        let wildcard = messages(&db, &text("1_0"), None, 100).unwrap();
        assert!(wildcard["messages"].as_array().unwrap().is_empty());
        let case = messages(&db, &text("NOTE 2"), None, 100).unwrap();
        assert!(!case["messages"].as_array().unwrap().is_empty());
    }

    #[test]
    fn threads_and_transcripts() {
        let mut f = seeded();
        let q = f.message("t1", "agt_pm", Some("agt_w1"), "question", true, true);
        f.message("t1", "agt_w1", Some("agt_pm"), "still working", false, true);
        f.message("t2", "agt_pm", Some("agt_w2"), "fyi", false, true);
        let db = f.reader();
        let list = threads(&db, &Filter::default(), None).unwrap();
        let rows = list["threads"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["thread"], "t2");
        let t1 = &rows[1];
        assert_eq!(t1["count"], 2);
        assert_eq!(t1["waiting"], true);
        assert_eq!(t1["participants"], json!(["agt_pm", "agt_w1"]));
        assert_eq!(t1["excerpt"], "still working");
        let filtered = threads(
            &db,
            &Filter {
                agent: Some("agt_w2".into()),
                ..Filter::default()
            },
            None,
        )
        .unwrap();
        assert_eq!(filtered["threads"].as_array().unwrap().len(), 1);
        let tx = transcript(&db, "t1").unwrap();
        let ids: Vec<i64> = tx["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["id"].as_i64().unwrap())
            .collect();
        assert_eq!(ids[0], q);
        assert!(ids.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn agents_and_pairs() {
        let mut f = seeded();
        for _ in 0..3 {
            f.message("a", "agt_pm", Some("agt_w1"), "x", false, true);
        }
        f.message("b", "agt_w2", Some("agt_pm"), "y", true, false);
        let db = f.reader();
        let v = agents(&db, &Filter::default()).unwrap();
        let pairs = v["pairs"].as_array().unwrap();
        assert_eq!(pairs[0]["agents"], json!(["agt_pm", "agt_w1"]));
        assert_eq!(pairs[0]["count"], 3);
        assert_eq!(pairs[1]["count"], 1);
        let pm = v["agents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "agt_pm")
            .unwrap();
        assert_eq!(
            (pm["sent"].as_i64(), pm["received"].as_i64()),
            (Some(3), Some(1))
        );
        assert_eq!(pm["unread"], 1);
        assert_eq!(pm["owes"], 1);
        assert_eq!(v["agents"].as_array().unwrap().len(), 4);
        let filtered = agents(
            &db,
            &Filter {
                agent: Some("agt_w1".into()),
                ..Filter::default()
            },
        )
        .unwrap();
        assert_eq!(filtered["agents"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn attention_splits_active_and_abandoned() {
        let mut f = seeded();
        let live_req = f.message("a", "agt_pm", Some("agt_w1"), "do it", true, true);
        let gone_req = f.message(
            "b",
            "agt_pm",
            Some("agt_gone"),
            "are you there",
            true,
            false,
        );
        let live_unread = f.message("c", "agt_pm", Some("agt_w2"), "fyi", false, false);
        let db = f.reader();
        let v = attention(&db, &Filter::default()).unwrap();
        let ids = |k: &str| -> Vec<i64> {
            v[k].as_array()
                .unwrap()
                .iter()
                .map(|m| m["id"].as_i64().unwrap())
                .collect()
        };
        assert_eq!(ids("awaiting"), vec![live_req]);
        assert_eq!(ids("unread"), vec![live_unread]);
        assert_eq!(ids("abandoned"), vec![gone_req]);
        assert_eq!(v["quiet_unread"], 1);
        assert_eq!(v["count"], 2);
        assert_eq!(v["unread"][0]["recipient_listening"], false);
        assert_eq!(meta(&db).unwrap()["attention_count"], 2);
    }

    #[test]
    fn no_query_reads_private_handles() {
        let source = include_str!("query.rs");
        let queries = source.split("#[cfg(test)]").next().unwrap();
        for forbidden in [
            "token,",
            ",token",
            " token ",
            "a.token",
            "agents.token",
            "SELECT *",
        ] {
            assert!(
                !queries.contains(forbidden),
                "viewer query source mentions {forbidden:?}"
            );
        }
        let mut f = seeded();
        f.message("t", "agt_pm", Some("agt_w1"), "hello", true, false);
        let db = f.reader();
        let everything = json!([
            meta(&db).unwrap(),
            messages(&db, &Filter::default(), None, 100).unwrap(),
            threads(&db, &Filter::default(), None).unwrap(),
            transcript(&db, "t").unwrap(),
            agents(&db, &Filter::default()).unwrap(),
            attention(&db, &Filter::default()).unwrap(),
        ])
        .to_string();
        assert!(!everything.contains("SECRET-HANDLE"));
    }
}
