use crate::models::*;
use rusqlite::{params, Connection, Transaction};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
pub fn home() -> PathBuf {
    std::env::var_os("TALA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".tala")
        })
}
pub fn private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
pub fn private_write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let tmp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut f = options.open(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
fn encode<T: serde::Serialize>(v: &T) -> Result<String> {
    serde_json::to_string(v)
        .map_err(|e| Failure::new("SERIALIZATION_ERROR", e.to_string(), "Report this failure."))
}
fn decode<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    serde_json::from_str(s).map_err(|e| {
        Failure::new(
            "STORAGE_ERROR",
            e.to_string(),
            "Preserve the database and inspect daemon.log.",
        )
    })
}
fn invalid(message: impl Into<String>) -> Failure {
    Failure::new(
        "INVALID_OPERATION",
        message,
        "Check tala --help and use explicit agent/message IDs.",
    )
}

pub struct Store {
    db: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 0 && version != 1 {
            return Err(Failure::new(
                "DATABASE_VERSION",
                format!("Unsupported database schema {version}"),
                "Use a matching Tala binary; do not delete the database.",
            ));
        }
        db.execute_batch("BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS agents(id TEXT PRIMARY KEY, name TEXT NOT NULL, tool TEXT NOT NULL, context TEXT NOT NULL, token TEXT NOT NULL, last_seen INTEGER NOT NULL, active INTEGER NOT NULL DEFAULT 1, listening_until INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY AUTOINCREMENT, thread TEXT NOT NULL, sender TEXT NOT NULL REFERENCES agents(id), recipient TEXT REFERENCES agents(id), project TEXT, context TEXT NOT NULL, parts TEXT NOT NULL, kind TEXT NOT NULL, reply_to INTEGER REFERENCES messages(id), expects_reply INTEGER NOT NULL, created_at INTEGER NOT NULL, received_at INTEGER, answered_by INTEGER REFERENCES messages(id), cancelled INTEGER NOT NULL DEFAULT 0, key TEXT NOT NULL, canonical TEXT NOT NULL, UNIQUE(sender,key));
CREATE INDEX IF NOT EXISTS inbox ON messages(recipient,received_at,id);
CREATE INDEX IF NOT EXISTS thread_messages ON messages(thread,id);
CREATE INDEX IF NOT EXISTS board_messages ON messages(project,id);
CREATE INDEX IF NOT EXISTS incoming_requests ON messages(recipient,expects_reply,answered_by);
CREATE INDEX IF NOT EXISTS outgoing_requests ON messages(sender,expects_reply,answered_by);
CREATE TABLE IF NOT EXISTS rpc_results(request_id TEXT PRIMARY KEY, canonical TEXT NOT NULL, response TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS participants(thread TEXT NOT NULL, agent TEXT NOT NULL REFERENCES agents(id), PRIMARY KEY(thread,agent));
CREATE TABLE IF NOT EXISTS board_deliveries(message INTEGER NOT NULL REFERENCES messages(id), agent TEXT NOT NULL REFERENCES agents(id), received_at INTEGER, PRIMARY KEY(message,agent));
CREATE INDEX IF NOT EXISTS unread_deliveries ON board_deliveries(agent,received_at);
PRAGMA user_version=1; COMMIT;")?;
        let check: String = db.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(Failure::new(
                "STORAGE_ERROR",
                check,
                "Preserve the database before recovery.",
            ));
        }
        Ok(Self { db })
    }
    #[cfg(test)]
    pub fn execute(&mut self, rpc: &Rpc) -> Result<Value> {
        Ok(self.execute_with_attention(rpc)?.0)
    }
    /// Executes one operation and, for an authenticated agent, computes its
    /// compact attention summary in the same transaction, after the
    /// operation's own effects.
    pub fn execute_with_attention(&mut self, rpc: &Rpc) -> Result<(Value, Option<Value>)> {
        if rpc.protocol_version != PROTOCOL_VERSION {
            return Err(Failure::new(
                "PROTOCOL_MISMATCH",
                "CLI and daemon protocol versions differ",
                "Stop the old daemon using its matching binary, then retry.",
            ));
        }
        let tx = self
            .db
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let cache = matches!(
            rpc.operation,
            Operation::Register { .. }
                | Operation::Send { .. }
                | Operation::Inbox { peek: false, .. }
                | Operation::Resolve { .. }
                | Operation::Unregister
        );
        let canonical = encode(rpc)?;
        if cache {
            match tx.query_row(
                "SELECT canonical,response FROM rpc_results WHERE request_id=?",
                [&rpc.request_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            ) {
                Ok((old, response)) if old == canonical => return Ok((decode(&response)?, None)),
                Ok(_) => return Err(invalid("Request ID reused for another operation")),
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let identity =
            if let Some(c) = &rpc.credential {
                let found = tx.query_row("SELECT token FROM agents WHERE id=?", [&c.agent], |r| {
                    r.get::<_, String>(0)
                });
                match found {
                    Ok(token) if token == c.token => {}
                    _ => return Err(Failure::new(
                        "IDENTITY_ERROR",
                        "Agent credential is unknown or invalid",
                        "Register an agent and pass its private handle with --agent or TALA_AGENT.",
                    )),
                }
                tx.execute(
                    "UPDATE agents SET last_seen=?, active=1 WHERE id=?",
                    params![now(), c.agent],
                )?;
                Some(c.agent.as_str())
            } else {
                None
            };
        let require = || {
            identity.ok_or_else(|| {
                Failure::new(
                    "IDENTITY_REQUIRED",
                    "Select your agent identity",
                    "Run tala register, then pass its private handle with --agent or TALA_AGENT.",
                )
            })
        };
        let result = match &rpc.operation {
            Operation::Register {
                name,
                tool,
                context,
            } => {
                if name.trim().is_empty() || tool.trim().is_empty() || context.project.is_empty() {
                    return Err(invalid("Name, tool and project are required"));
                }
                let id = format!("agt_{}", Uuid::new_v4().simple());
                let token = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES(?,?,?,?,?,?)",
                    params![id, name, tool, encode(context)?, token, now()],
                )?;
                json!({"agent":agent(&tx,&id)?,"credential":Credential{agent:id,token}})
            }
            Operation::Whoami => json!({"agent":agent(&tx,require()?)?}),
            Operation::Unregister => {
                let id = require()?;
                tx.execute(
                    "UPDATE agents SET active=0,listening_until=0 WHERE id=?",
                    [id],
                )?;
                json!({"agent_id":id,"registered":false})
            }
            Operation::Agents {
                project,
                after,
                limit,
            } => {
                bounds(*limit)?;
                let mut stmt = tx.prepare("SELECT id FROM agents WHERE id>? ORDER BY id")?;
                let ids = stmt
                    .query_map([after], |r| r.get::<_, String>(0))?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let mut list = Vec::new();
                for id in ids {
                    let a = agent(&tx, &id)?;
                    if project.as_ref().is_none_or(|p| &a.context.project == p) {
                        list.push(a);
                        if list.len() > *limit {
                            break;
                        }
                    }
                }
                let more = list.len() > *limit;
                list.truncate(*limit);
                let next = list.last().map(|a| a.id.as_str()).unwrap_or(after);
                json!({"next_after":next,"has_more":more,"agents":list})
            }
            Operation::Send { .. } => send(&tx, require()?, &rpc.operation)?,
            Operation::Inbox {
                peek,
                after,
                limit,
                reply_to,
                listening,
            } => {
                let id = require()?;
                tx.execute(
                    "UPDATE agents SET listening_until=? WHERE id=?",
                    params![if *listening { now() + 3 } else { 0 }, id],
                )?;
                let mut page = page(
                    &tx,
                    &format!("({UNREAD}) AND (?2 IS NULL OR messages.reply_to=?2)"),
                    params![id, reply_to],
                    *after,
                    *limit,
                )?;
                if !peek {
                    for msg in &mut page.messages {
                        // Direct mail is acknowledged on the message; a board
                        // reply only on this agent's delivery.
                        if msg.recipient.as_deref() == Some(id) {
                            tx.execute(
                                "UPDATE messages SET received_at=? WHERE id=?",
                                params![now(), msg.id],
                            )?;
                        } else {
                            tx.execute(
                                "UPDATE board_deliveries SET received_at=? WHERE message=? AND agent=?",
                                params![now(), msg.id, id],
                            )?;
                        }
                        msg.received_at = Some(now());
                        msg.delivery = delivery(msg);
                    }
                }
                let mut data = json!(page);
                if let Some(request_id) = reply_to {
                    let original = message(&tx, *request_id)?;
                    if original.sender != id {
                        return Err(invalid("Only the sender can wait for a request's reply"));
                    }
                    data["request"] = json!(original);
                }
                data
            }
            Operation::Board {
                project,
                after,
                limit,
            } => {
                require()?;
                json!(page(&tx, "project=?", [project], *after, *limit)?)
            }
            Operation::History {
                thread,
                after,
                limit,
            } => {
                let id = require()?;
                if !visible_thread(&tx, id, thread)? {
                    return Err(Failure::new(
                        "THREAD_NOT_VISIBLE",
                        "Thread is missing or not visible to this agent",
                        "Use tala inbox or board to find an accessible thread ID.",
                    ));
                }
                json!(page(&tx, "thread=?", [thread], *after, *limit)?)
            }
            Operation::Search {
                query,
                project,
                after,
                limit,
            } => {
                let id = require()?;
                let context = agent(&tx, id)?.context;
                let project = project.as_ref().unwrap_or(&context.project);
                if query.trim().is_empty() {
                    return Err(invalid("Search text must not be empty"));
                }
                json!(page(&tx,"(project=? OR thread IN (SELECT thread FROM participants WHERE agent=?)) AND EXISTS(SELECT 1 FROM json_each(messages.parts) WHERE instr(lower(COALESCE(json_extract(value,'$.text'),json_extract(value,'$.path'),CAST(json_extract(value,'$.value') AS TEXT))),lower(?))>0)",params![project,id,query],*after,*limit)?)
            }
            Operation::Pending {
                direction,
                stale,
                after,
                limit,
            } => {
                let id = require()?;
                let side = match direction.as_str() {
                    "incoming" => "messages.recipient=?1",
                    "outgoing" => "messages.sender=?1",
                    "all" => "(messages.recipient=?1 OR messages.sender=?1)",
                    _ => return Err(invalid("Direction must be incoming, outgoing or all")),
                };
                let cutoff = cutoff(STALE_AFTER);
                let open = format!("{OPEN} AND {side}");
                let selected = if *stale {
                    format!("{open} AND {STALE}")
                } else {
                    format!("{open} AND NOT {STALE}")
                };
                let mut data = json!(page(&tx, &selected, params![id, cutoff], *after, *limit)?);
                if !stale {
                    data["stale_count"] = json!(count(
                        &tx,
                        &format!("{open} AND {STALE}"),
                        params![id, cutoff]
                    )?);
                    data["stale_next"] = json!(if direction == "all" {
                        "tala pending --stale".to_string()
                    } else {
                        format!("tala pending --stale --direction={direction}")
                    });
                }
                data
            }
            Operation::Attention => summary(&tx, require()?, cutoff(STALE_AFTER))?,
            Operation::Resolve { message: id } => {
                let msg = message(&tx, *id)?;
                if msg.sender != require()? || !msg.expects_reply {
                    return Err(invalid("Only the sender can cancel their own request"));
                }
                if msg.answered_by.is_some() {
                    return Err(invalid("Request is already answered"));
                }
                tx.execute("UPDATE messages SET cancelled=1 WHERE id=?", [id])?;
                json!({"message":message(&tx,*id)?})
            }
            Operation::MessageStatus { message: id } => {
                let m = message(&tx, *id)?;
                if !visible_thread(&tx, require()?, &m.thread_id)? {
                    return Err(invalid("Message is not visible to this agent"));
                }
                json!({"message":m})
            }
            Operation::Status => {
                json!({"running":true,"protocol_version":PROTOCOL_VERSION,"agents":tx.query_row("SELECT count(*) FROM agents",[],|r|r.get::<_,u64>(0))?,"messages":tx.query_row("SELECT count(*) FROM messages",[],|r|r.get::<_,u64>(0))?})
            }
            Operation::Stop => json!({"stopped":true}),
        };
        let attention = match identity {
            Some(id) if !matches!(rpc.operation, Operation::Attention) => ambient(&tx, id)?,
            _ => None,
        };
        if cache {
            tx.execute(
                "INSERT INTO rpc_results(request_id,canonical,response) VALUES(?,?,?)",
                params![rpc.request_id, canonical, encode(&result)?],
            )?;
        }
        tx.commit()?;
        Ok((result, attention))
    }
}
/// Default age after which an unanswered request counts as stale.
pub const STALE_AFTER: u64 = 24 * 60 * 60;
/// Window within which an identical send without an explicit key is a duplicate.
const AUTO_KEY_WINDOW: i64 = 10 * 60;
/// Unread for agent ?1: direct messages and board replies delivered to it.
const UNREAD: &str = "(messages.recipient=?1 AND messages.received_at IS NULL) OR messages.id IN (SELECT message FROM board_deliveries WHERE agent=?1 AND received_at IS NULL)";
const OPEN: &str =
    "messages.expects_reply=1 AND messages.answered_by IS NULL AND messages.cancelled=0";
/// Stale for agent ?1 with cutoff ?2: too old, or the counterpart unregistered.
const STALE: &str = "(messages.created_at < ?2 OR EXISTS(SELECT 1 FROM agents x WHERE x.id = CASE WHEN messages.recipient=?1 THEN messages.sender ELSE messages.recipient END AND x.active=0))";
fn cutoff(stale_after: u64) -> i64 {
    now() - stale_after as i64
}
fn count<P: rusqlite::Params>(tx: &Transaction<'_>, predicate: &str, p: P) -> Result<u64> {
    Ok(tx.query_row(
        &format!("SELECT count(*) FROM messages WHERE {predicate}"),
        p,
        |r| r.get(0),
    )?)
}
fn reply_next(id: u64) -> String {
    format!("tala reply {id} '<answer>'")
}
fn with_next(page: Page, next: impl Fn(&Message) -> String) -> Vec<Value> {
    page.messages
        .iter()
        .map(|m| {
            let mut v = json!(m);
            v["next"] = json!(next(m));
            v
        })
        .collect()
}
/// Full attention summary for bare `tala`; acknowledges nothing.
fn summary(tx: &Transaction<'_>, id: &str, cutoff: i64) -> Result<Value> {
    let incoming = format!("{OPEN} AND messages.recipient=?1");
    let outgoing = format!("{OPEN} AND messages.sender=?1");
    let unread = UNREAD;
    let requests = page(
        tx,
        &format!("{incoming} AND NOT {STALE}"),
        params![id, cutoff],
        0,
        50,
    )?;
    let more = requests.has_more;
    let requests = with_next(requests, |m| reply_next(m.id));
    let waiting = with_next(
        page(
            tx,
            &format!("{outgoing} AND NOT {STALE}"),
            params![id, cutoff],
            0,
            50,
        )?,
        |m| format!("tala inbox --wait --reply-to={}", m.id),
    );
    let unread_count = count(tx, unread, params![id])?;
    let unread_page = with_next(page(tx, unread, params![id], 0, 50)?, |_| {
        "tala inbox".into()
    });
    let stale_in = count(tx, &format!("{incoming} AND {STALE}"), params![id, cutoff])?;
    let stale_out = count(tx, &format!("{outgoing} AND {STALE}"), params![id, cutoff])?;
    let next = if unread_count > 0 {
        Some("tala inbox".to_string())
    } else if let Some(first) = requests.first() {
        first["next"].as_str().map(Into::into)
    } else if stale_in + stale_out > 0 {
        Some("tala pending --stale".to_string())
    } else {
        None
    };
    Ok(json!({
        "incoming_requests": requests,
        "incoming_has_more": more,
        "unread": {"count": unread_count, "messages": unread_page},
        "outgoing_requests": waiting,
        "stale": {"incoming": stale_in, "outgoing": stale_out, "next": "tala pending --stale"},
        "next": next,
    }))
}
/// Compact attention attached to every authenticated response; None when
/// nothing awaits the agent. Contains no message bodies.
fn ambient(tx: &Transaction<'_>, id: &str) -> Result<Option<Value>> {
    let cutoff = cutoff(STALE_AFTER);
    let fresh = format!("{OPEN} AND messages.recipient=?1 AND NOT {STALE}");
    let unread = count(tx, UNREAD, params![id])?;
    let open = count(tx, &fresh, params![id, cutoff])?;
    let stale = count(
        tx,
        &format!("{OPEN} AND (messages.recipient=?1 OR messages.sender=?1) AND {STALE}"),
        params![id, cutoff],
    )?;
    if unread + open + stale == 0 {
        return Ok(None);
    }
    let mut stmt = tx.prepare(&format!(
        "SELECT id,sender,created_at FROM messages WHERE {fresh} ORDER BY id LIMIT 3"
    ))?;
    let requests = stmt
        .query_map(params![id, cutoff], |r| {
            Ok((
                r.get::<_, u64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let next = if unread > 0 {
        "tala inbox".to_string()
    } else if let Some((first, _, _)) = requests.first() {
        reply_next(*first)
    } else {
        "tala pending --stale".to_string()
    };
    let requests: Vec<Value> = requests
        .into_iter()
        .map(|(id, from, created)| json!({"id": id, "from": from, "age_seconds": (now() - created).max(0)}))
        .collect();
    Ok(Some(json!({
        "unread": unread,
        "requests": requests,
        "open_requests": open,
        "stale": stale,
        "next": next,
    })))
}
fn bounds(limit: usize) -> Result<()> {
    if !(1..=500).contains(&limit) {
        return Err(invalid("Limit must be between 1 and 500"));
    }
    Ok(())
}
fn agent(tx: &Transaction<'_>, id: &str) -> Result<Agent> {
    let row = tx.query_row(
        "SELECT name,tool,context,last_seen,active,listening_until FROM agents WHERE id=?",
        [id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, bool>(4)?,
                r.get::<_, i64>(5)?,
            ))
        },
    )?;
    Ok(Agent {
        id: id.into(),
        name: row.0,
        tool: row.1,
        context: decode(&row.2)?,
        last_seen: row.3,
        active: row.4 && now() - row.3 < 300,
        listening: row.5 > now(),
    })
}
fn resolve(tx: &Transaction<'_>, name: &str) -> Result<String> {
    let mut stmt = tx.prepare("SELECT id FROM agents WHERE id=? OR name=? ORDER BY id")?;
    let ids = stmt
        .query_map(params![name, name], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if ids.iter().any(|id| id == name) {
        return Ok(name.into());
    }
    match ids.as_slice() {
        [id] => Ok(id.clone()),
        [] => Err(Failure::new(
            "RECIPIENT_NOT_FOUND",
            format!("No registered agent '{name}'"),
            "Run tala agents and use an exact agent ID.",
        )),
        _ => Err(Failure::new(
            "AMBIGUOUS_RECIPIENT",
            format!("Multiple agents named '{name}': {}", ids.join(", ")),
            "Use an exact agent ID from tala agents.",
        )),
    }
}
fn delivery(m: &Message) -> String {
    if m.cancelled {
        "cancelled"
    } else if m.answered_by.is_some() {
        "answered"
    } else if m.received_at.is_some() {
        "received"
    } else {
        "stored"
    }
    .into()
}
fn row_message(r: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    let parse = |index| {
        let s: String = r.get(index)?;
        serde_json::from_str(&s).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })
    };
    let mut msg = Message {
        id: r.get(0)?,
        thread_id: r.get(1)?,
        sender: r.get(2)?,
        recipient: r.get(3)?,
        project: r.get(4)?,
        context: parse(5)?,
        parts: serde_json::from_str(&r.get::<_, String>(6)?).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(6, rusqlite::types::Type::Text, Box::new(e))
        })?,
        kind: r.get(7)?,
        reply_to: r.get(8)?,
        expects_reply: r.get(9)?,
        created_at: r.get(10)?,
        received_at: r.get(11)?,
        answered_by: r.get(12)?,
        cancelled: r.get(13)?,
        delivery: String::new(),
        age_seconds: (now() - r.get::<_, i64>(10)?).max(0),
    };
    msg.delivery = delivery(&msg);
    Ok(msg)
}
const COLUMNS:&str="id,thread,sender,recipient,project,context,parts,kind,reply_to,expects_reply,created_at,received_at,answered_by,cancelled";
fn message(tx: &Transaction<'_>, id: u64) -> Result<Message> {
    tx.query_row(
        &format!("SELECT {COLUMNS} FROM messages WHERE id=?"),
        [id],
        row_message,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Failure::new(
            "MESSAGE_NOT_FOUND",
            format!("Message {id} does not exist"),
            "Use inbox, board, history or search to find a message ID.",
        ),
        other => other.into(),
    })
}
fn page<P: rusqlite::Params>(
    tx: &Transaction<'_>,
    predicate: &str,
    p: P,
    after: u64,
    limit: usize,
) -> Result<Page> {
    bounds(limit)?;
    let mut stmt = tx.prepare(&format!(
        "SELECT {COLUMNS} FROM messages WHERE id>{after} AND ({predicate}) ORDER BY id LIMIT {}",
        limit + 1
    ))?;
    let mut messages = stmt
        .query_map(p, row_message)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let has_more = messages.len() > limit;
    messages.truncate(limit);
    Ok(Page {
        next_after: messages.last().map(|m| m.id).unwrap_or(after),
        messages,
        has_more,
    })
}
fn visible_thread(tx: &Transaction<'_>, id: &str, thread: &str) -> Result<bool> {
    let p = agent(tx, id)?.context.project;
    Ok(tx.query_row("SELECT EXISTS(SELECT 1 FROM participants WHERE thread=? AND agent=?) OR EXISTS(SELECT 1 FROM messages WHERE thread=? AND project=?)",params![thread,id,thread,p],|r|r.get(0))?)
}
fn send(tx: &Transaction<'_>, sender: &str, operation: &Operation) -> Result<Value> {
    let Operation::Send {
        to,
        project,
        thread,
        reply_to,
        parts,
        request,
        kind,
        key,
        context,
    } = operation
    else {
        unreachable!()
    };
    if parts.is_empty() || parts.iter().any(|p| !p.valid()) || key.trim().is_empty() {
        return Err(invalid("Nonempty parts and idempotency key are required"));
    }
    if !["message", "reply", "post", "handoff"].contains(&kind.as_str()) {
        return Err(invalid("Unknown message kind"));
    }
    if encode(parts)?.len() > 1024 * 1024 {
        return Err(invalid("Message exceeds the 1 MiB limit"));
    }
    let canonical = encode(operation)?;
    let mut stored_key = key.clone();
    if key.starts_with("auto:") {
        // Content-derived key: an identical send inside the window is a
        // duplicate; outside it, store a new message under a suffixed key.
        let prior = tx.query_row(
            "SELECT id,created_at FROM messages WHERE sender=?1 AND key>=?2 AND key<?3 ORDER BY id DESC LIMIT 1",
            params![sender, key, format!("{key};")],
            |r| Ok((r.get::<_, u64>(0)?, r.get::<_, i64>(1)?)),
        );
        match prior {
            Ok((id, created)) if now() - created <= AUTO_KEY_WINDOW => {
                return Ok(json!({"message":message(tx,id)?,"duplicate":true}));
            }
            Ok(_) => stored_key = format!("{key}:{}", Uuid::new_v4().simple()),
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(e) => return Err(e.into()),
        }
    } else if let Some(id) = explicit_key(tx, sender, key, &canonical)? {
        return Ok(json!({"message":message(tx,id)?,"duplicate":true}));
    }
    // `notify` is the author of an answered board message, who receives the
    // reply in their inbox while it stays public on the board.
    let (recipient, board, thread_id, notify) = if let Some(id) = reply_to {
        if kind != "reply" || to.is_some() || project.is_some() || thread.is_some() {
            return Err(invalid("Replies derive routing from message ID"));
        }
        let parent = message(tx, *id)?;
        if let Some(recipient) = &parent.recipient {
            if recipient != sender {
                return Err(Failure::new(
                    "NOT_RECIPIENT",
                    "Only the addressed recipient can reply",
                    "Select the recipient identity with --agent.",
                ));
            }
            (Some(parent.sender), None, parent.thread_id, None)
        } else {
            if *request {
                return Err(invalid(
                    "Board replies are informational; send an addressed request instead",
                ));
            }
            if !visible_thread(tx, sender, &parent.thread_id)? {
                return Err(invalid("Board thread is not visible to this agent"));
            }
            let notify = (parent.sender != sender).then_some(parent.sender);
            (None, parent.project, parent.thread_id, notify)
        }
    } else if let Some(project) = project {
        if to.is_some() || kind != "post" || *request {
            return Err(invalid(
                "Board posts cannot have a direct recipient or require a reply",
            ));
        }
        (
            None,
            Some(project.clone()),
            format!("thr_{}", Uuid::new_v4().simple()),
            None,
        )
    } else {
        if kind == "reply" || kind == "post" {
            return Err(invalid("Reply needs a message ID; post needs a project"));
        }
        let recipient = resolve(
            tx,
            to.as_deref()
                .ok_or_else(|| invalid("Direct send requires --to"))?,
        )?;
        let thread_id = match thread {
            Some(t) => {
                if !visible_thread(tx, sender, t)? {
                    return Err(invalid("Thread is not visible to the sender"));
                }
                let board: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM messages WHERE thread=? AND project IS NOT NULL)",
                    [t],
                    |r| r.get(0),
                )?;
                if board {
                    return Err(invalid("Direct messages and handoffs cannot continue a board thread; use reply for public discussion or start a new direct thread"));
                }
                t.clone()
            }
            None => format!("thr_{}", Uuid::new_v4().simple()),
        };
        (Some(recipient), None, thread_id, None)
    };
    if kind == "handoff" && thread.is_none() {
        return Err(invalid("Handoff requires a thread"));
    }
    if recipient.as_deref() == Some(sender) {
        return Err(invalid("Choose a different recipient"));
    }
    tx.execute("INSERT INTO messages(thread,sender,recipient,project,context,parts,kind,reply_to,expects_reply,created_at,key,canonical) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![thread_id,sender,recipient,board,encode(context)?,encode(parts)?,kind,reply_to,request,now(),stored_key,canonical])?;
    let id = tx.last_insert_rowid() as u64;
    tx.execute(
        "INSERT OR IGNORE INTO participants(thread,agent) VALUES(?,?)",
        params![thread_id, sender],
    )?;
    if let Some(recipient) = recipient {
        tx.execute(
            "INSERT OR IGNORE INTO participants(thread,agent) VALUES(?,?)",
            params![thread_id, recipient],
        )?;
    }
    if let Some(author) = notify {
        tx.execute(
            "INSERT INTO board_deliveries(message,agent) VALUES(?,?)",
            params![id, author],
        )?;
    }
    if let Some(parent) = reply_to {
        // Replying counts as receipt, for direct mail and board deliveries.
        tx.execute(
            "UPDATE messages SET received_at=COALESCE(received_at,?) WHERE id=? AND recipient=?",
            params![now(), parent, sender],
        )?;
        tx.execute(
            "UPDATE board_deliveries SET received_at=COALESCE(received_at,?) WHERE message=? AND agent=?",
            params![now(), parent, sender],
        )?;
        tx.execute("UPDATE messages SET answered_by=? WHERE id=? AND expects_reply=1 AND cancelled=0 AND answered_by IS NULL",params![id,parent])?;
    }
    Ok(json!({"message":message(tx,id)?,"duplicate":false}))
}

/// Explicit keys deduplicate without a window and reject changed content.
fn explicit_key(
    tx: &Transaction<'_>,
    sender: &str,
    key: &str,
    canonical: &str,
) -> Result<Option<u64>> {
    let prior = tx.query_row(
        "SELECT id,canonical FROM messages WHERE sender=? AND key=?",
        params![sender, key],
        |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)),
    );
    match prior {
        Ok((_, old)) if old != canonical => Err(Failure::new(
            "IDEMPOTENCY_CONFLICT",
            "Key already used for a different operation",
            "Use a fresh key for a new message; preserve keys only for identical retries.",
        )),
        Ok((id, _)) => Ok(Some(id)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rpc(operation: Operation, c: Option<Credential>) -> Rpc {
        Rpc {
            protocol_version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4().to_string(),
            credential: c,
            operation,
        }
    }
    fn context() -> Context {
        Context {
            project: "/test/project".into(),
            worktree: None,
            working_directory: None,
            branch: None,
            revision: None,
        }
    }
    fn register(s: &mut Store, name: &str) -> Credential {
        let v = s
            .execute(&rpc(
                Operation::Register {
                    name: name.into(),
                    tool: "test".into(),
                    context: context(),
                },
                None,
            ))
            .unwrap();
        serde_json::from_value(v["credential"].clone()).unwrap()
    }
    fn send(to: &str, key: &str) -> Operation {
        Operation::Send {
            to: Some(to.into()),
            project: None,
            thread: None,
            reply_to: None,
            parts: vec![Part::Text {
                text: "hello".into(),
            }],
            request: true,
            kind: "message".into(),
            key: key.into(),
            context: context(),
        }
    }
    fn age(store: &Store, id: u64, seconds: i64) {
        store
            .db
            .execute(
                "UPDATE messages SET created_at=created_at-? WHERE id=?",
                params![seconds, id],
            )
            .unwrap();
    }
    fn pending(stale: bool) -> Operation {
        Operation::Pending {
            direction: "incoming".into(),
            stale,
            after: 0,
            limit: 50,
        }
    }
    fn auto(to: &str, text: &str) -> Operation {
        let mut op = send(to, &format!("auto:{text}"));
        if let Operation::Send { parts, .. } = &mut op {
            *parts = vec![Part::Text { text: text.into() }];
        }
        op
    }
    #[test]
    fn ages_and_staleness_are_computed_at_response_time() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let fresh = store
            .execute(&rpc(send(&b.agent, "1"), Some(a.clone())))
            .unwrap();
        let old = store
            .execute(&rpc(send(&b.agent, "2"), Some(a.clone())))
            .unwrap();
        let fresh_id = fresh["message"]["id"].as_u64().unwrap();
        let old_id = old["message"]["id"].as_u64().unwrap();
        age(&store, fresh_id, 5400);
        age(&store, old_id, 30 * 86400);
        let listed = store
            .execute(&rpc(pending(false), Some(b.clone())))
            .unwrap();
        assert_eq!(listed["messages"].as_array().unwrap().len(), 1);
        let seconds = listed["messages"][0]["age_seconds"].as_i64().unwrap();
        assert!((5400..5410).contains(&seconds));
        assert_eq!(listed["stale_count"], 1);
        let stale = store.execute(&rpc(pending(true), Some(b.clone()))).unwrap();
        assert_eq!(stale["messages"][0]["id"], old_id);
        // Staleness does not change state: the stale request is still answerable.
        let mut reply = send("unused", "r");
        if let Operation::Send {
            to,
            reply_to,
            kind,
            request,
            ..
        } = &mut reply
        {
            *to = None;
            *reply_to = Some(old_id);
            *kind = "reply".into();
            *request = false;
        }
        store.execute(&rpc(reply, Some(b.clone()))).unwrap();
        let stale = store.execute(&rpc(pending(true), Some(b))).unwrap();
        assert_eq!(stale["messages"], json!([]));
    }
    #[test]
    fn unregistered_counterpart_makes_request_stale() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        store
            .execute(&rpc(send(&b.agent, "1"), Some(a.clone())))
            .unwrap();
        store.execute(&rpc(Operation::Unregister, Some(a))).unwrap();
        let listed = store
            .execute(&rpc(pending(false), Some(b.clone())))
            .unwrap();
        assert_eq!(
            (listed["messages"].clone(), listed["stale_count"].clone()),
            (json!([]), json!(1))
        );
        let summary = store.execute(&rpc(Operation::Attention, Some(b))).unwrap();
        assert_eq!(summary["incoming_requests"], json!([]));
        assert_eq!(summary["stale"]["incoming"], 1);
        assert_eq!(summary["next"], "tala inbox");
    }
    #[test]
    fn automatic_keys_deduplicate_only_inside_the_window() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let c = register(&mut store, "c");
        let first = store
            .execute(&rpc(auto(&b.agent, "same"), Some(a.clone())))
            .unwrap();
        let again = store
            .execute(&rpc(auto(&b.agent, "same"), Some(a.clone())))
            .unwrap();
        assert_eq!(again["duplicate"], true);
        assert_eq!(again["message"]["id"], first["message"]["id"]);
        // The same key from another sender is a different message.
        let other = store
            .execute(&rpc(auto(&b.agent, "same"), Some(c)))
            .unwrap();
        assert_eq!(other["duplicate"], false);
        age(
            &store,
            first["message"]["id"].as_u64().unwrap(),
            AUTO_KEY_WINDOW + 1,
        );
        let later = store
            .execute(&rpc(auto(&b.agent, "same"), Some(a.clone())))
            .unwrap();
        assert_eq!(later["duplicate"], false);
        assert_ne!(later["message"]["id"], first["message"]["id"]);
        let repeat = store
            .execute(&rpc(auto(&b.agent, "same"), Some(a)))
            .unwrap();
        assert_eq!(repeat["message"]["id"], later["message"]["id"]);
    }
    #[test]
    fn ambient_attention_reflects_the_operations_own_effects() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let (_, none) = store
            .execute_with_attention(&rpc(Operation::Whoami, Some(b.clone())))
            .unwrap();
        assert!(none.is_none());
        let sent = store.execute(&rpc(send(&b.agent, "k"), Some(a))).unwrap();
        let id = sent["message"]["id"].as_u64().unwrap();
        let (_, attention) = store
            .execute_with_attention(&rpc(Operation::Whoami, Some(b.clone())))
            .unwrap();
        let attention = attention.unwrap();
        assert_eq!(attention["unread"], 1);
        assert_eq!(attention["requests"][0]["id"], id);
        assert!(attention.to_string().find("hello").is_none());
        let consume = Operation::Inbox {
            peek: false,
            after: 0,
            limit: 50,
            reply_to: None,
            listening: false,
        };
        let (_, attention) = store
            .execute_with_attention(&rpc(consume, Some(b)))
            .unwrap();
        let attention = attention.unwrap();
        assert_eq!(attention["unread"], 0);
        assert_eq!(attention["open_requests"], 1);
        assert_eq!(attention["next"], format!("tala reply {id} '<answer>'"));
    }
    /// Measures ambient attention cost on 10,000 messages:
    /// `cargo test --release --bin tala attention_cost -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn attention_cost() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let c = register(&mut store, "c");
        for n in 0..10_000 {
            let (from, to) = if n % 2 == 0 { (&a, &b) } else { (&c, &a) };
            store
                .execute(&rpc(send(&to.agent, &n.to_string()), Some(from.clone())))
                .unwrap();
        }
        let runs = 500;
        let time = |store: &mut Store, f: &dyn Fn(&mut Store)| {
            let start = std::time::Instant::now();
            for _ in 0..runs {
                f(store);
            }
            start.elapsed() / runs
        };
        let whoami = time(&mut store, &|s| {
            s.execute_with_attention(&rpc(Operation::Whoami, Some(b.clone())))
                .unwrap();
        });
        let ambient_only = time(&mut store, &|s| {
            let tx = s.db.transaction().unwrap();
            ambient(&tx, &b.agent).unwrap();
        });
        let d = register(&mut store, "d");
        for n in 0..3 {
            store
                .execute(&rpc(send(&d.agent, &format!("d{n}")), Some(a.clone())))
                .unwrap();
        }
        let typical = time(&mut store, &|s| {
            s.execute_with_attention(&rpc(Operation::Whoami, Some(d.clone())))
                .unwrap();
        });
        let bare = time(&mut store, &|s| {
            let tx = s.db.transaction().unwrap();
            agent(&tx, &d.agent).unwrap();
        });
        println!("10,000 messages in the home:");
        println!(
            "  agent with 5,000 open requests: whoami {whoami:?}, attention alone {ambient_only:?}"
        );
        println!("  agent with 3 open requests:     whoami {typical:?}, whoami work without attention {bare:?}");
    }
    fn post(text: &str) -> Operation {
        let mut op = send("unused", &format!("post-{text}"));
        if let Operation::Send {
            to,
            project,
            kind,
            request,
            parts,
            ..
        } = &mut op
        {
            *to = None;
            *project = Some("/test/project".into());
            *kind = "post".into();
            *request = false;
            *parts = vec![Part::Text { text: text.into() }];
        }
        op
    }
    fn reply(parent: u64, text: &str) -> Operation {
        let mut op = send("unused", &format!("reply-{parent}-{text}"));
        if let Operation::Send {
            to,
            reply_to,
            kind,
            request,
            parts,
            ..
        } = &mut op
        {
            *to = None;
            *reply_to = Some(parent);
            *kind = "reply".into();
            *request = false;
            *parts = vec![Part::Text { text: text.into() }];
        }
        op
    }
    fn inbox(peek: bool) -> Operation {
        Operation::Inbox {
            peek,
            after: 0,
            limit: 50,
            reply_to: None,
            listening: false,
        }
    }
    fn texts(v: &Value) -> Vec<String> {
        v["messages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| m["parts"][0]["text"].as_str().unwrap().to_string())
            .collect()
    }
    fn id_of(v: &Value) -> u64 {
        v["message"]["id"].as_u64().unwrap()
    }
    #[test]
    fn board_replies_are_delivered_to_the_answered_author_only() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let c = register(&mut store, "c");
        let plan = id_of(&store.execute(&rpc(post("plan"), Some(a.clone()))).unwrap());
        // A self-reply delivers nothing.
        store
            .execute(&rpc(reply(plan, "self"), Some(a.clone())))
            .unwrap();
        assert_eq!(
            texts(&store.execute(&rpc(inbox(true), Some(a.clone()))).unwrap()),
            Vec::<String>::new()
        );
        let answer = id_of(
            &store
                .execute(&rpc(reply(plan, "no overlap"), Some(b.clone())))
                .unwrap(),
        );
        // A reply to B's reply goes to B, not to A.
        store
            .execute(&rpc(reply(answer, "agreed"), Some(c.clone())))
            .unwrap();
        assert_eq!(
            texts(&store.execute(&rpc(inbox(true), Some(a.clone()))).unwrap()),
            ["no overlap"]
        );
        assert_eq!(
            texts(&store.execute(&rpc(inbox(true), Some(b.clone()))).unwrap()),
            ["agreed"]
        );
        assert_eq!(
            texts(&store.execute(&rpc(inbox(true), Some(c))).unwrap()),
            Vec::<String>::new()
        );
        // Everything stays public on the board.
        let board = Operation::Board {
            project: "/test/project".into(),
            after: 0,
            limit: 50,
        };
        assert_eq!(
            texts(&store.execute(&rpc(board, Some(b.clone()))).unwrap()),
            ["plan", "self", "no overlap", "agreed"]
        );
    }
    #[test]
    fn board_deliveries_are_acknowledged_per_author() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let plan = id_of(&store.execute(&rpc(post("plan"), Some(a.clone()))).unwrap());
        store
            .execute(&rpc(reply(plan, "noted"), Some(b.clone())))
            .unwrap();
        let (_, attention) = store
            .execute_with_attention(&rpc(Operation::Whoami, Some(a.clone())))
            .unwrap();
        let attention = attention.unwrap();
        assert_eq!(
            (attention["unread"].clone(), attention["next"].clone()),
            (json!(1), json!("tala inbox"))
        );
        let summary = store
            .execute(&rpc(Operation::Attention, Some(a.clone())))
            .unwrap();
        assert_eq!(summary["unread"]["count"], 1);
        // Peeking leaves it unread; consuming acknowledges it once.
        store.execute(&rpc(inbox(true), Some(a.clone()))).unwrap();
        let consumed = store.execute(&rpc(inbox(false), Some(a.clone()))).unwrap();
        assert_eq!(texts(&consumed), ["noted"]);
        assert_eq!(consumed["messages"][0]["delivery"], "received");
        assert_eq!(
            texts(&store.execute(&rpc(inbox(false), Some(a.clone()))).unwrap()),
            Vec::<String>::new()
        );
        assert_eq!(
            texts(&store.execute(&rpc(inbox(true), Some(b))).unwrap()),
            Vec::<String>::new()
        );
        let (_, attention) = store
            .execute_with_attention(&rpc(Operation::Whoami, Some(a)))
            .unwrap();
        assert!(attention.is_none());
    }
    #[test]
    fn lost_inbox_response_replays_same_batch_after_restart() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let mut store = Store::open(&path).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        store.execute(&rpc(send(&b.agent, "k"), Some(a))).unwrap();
        let request = rpc(
            Operation::Inbox {
                peek: false,
                after: 0,
                limit: 50,
                reply_to: None,
                listening: false,
            },
            Some(b.clone()),
        );
        let first = store.execute(&request).unwrap();
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(store.execute(&request).unwrap(), first);
        let fresh = store.execute(&rpc(request.operation, Some(b))).unwrap();
        assert_eq!(fresh["messages"], json!([]));
    }
    #[test]
    fn wire_mismatch_has_no_effect() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let mut req = rpc(
            Operation::Register {
                name: "a".into(),
                tool: "test".into(),
                context: context(),
            },
            None,
        );
        for old in [1, 2] {
            req.protocol_version = old;
            assert_eq!(store.execute(&req).unwrap_err().code, "PROTOCOL_MISMATCH");
        }
        assert_eq!(
            store.execute(&rpc(Operation::Status, None)).unwrap()["agents"],
            0
        );
    }
    #[test]
    fn idempotency_compares_routing_and_request_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        let c = register(&mut store, "c");
        store
            .execute(&rpc(send(&b.agent, "k"), Some(a.clone())))
            .unwrap();
        assert_eq!(
            store
                .execute(&rpc(send(&c.agent, "k"), Some(a.clone())))
                .unwrap_err()
                .code,
            "IDEMPOTENCY_CONFLICT"
        );
        let mut changed = send(&b.agent, "k");
        if let Operation::Send { request, .. } = &mut changed {
            *request = false;
        }
        assert_eq!(
            store.execute(&rpc(changed, Some(a))).unwrap_err().code,
            "IDEMPOTENCY_CONFLICT"
        );
    }
    #[test]
    fn failed_transaction_rolls_back() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        assert!(store.execute(&rpc(send("unknown", "k"), Some(a))).is_err());
        assert_eq!(
            store.execute(&rpc(Operation::Status, None)).unwrap()["messages"],
            0
        );
    }
    #[test]
    fn storage_write_failure_never_reports_success() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        store.db.execute_batch("PRAGMA query_only=ON").unwrap();
        let error = store
            .execute(&rpc(send(&b.agent, "k"), Some(a)))
            .unwrap_err();
        assert_eq!(error.code, "STORAGE_ERROR");
        assert_eq!(
            store
                .db
                .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, u64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn credentials_cannot_impersonate_another_agent() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let a = register(&mut store, "a");
        let b = register(&mut store, "b");
        assert_eq!(
            store
                .execute(&rpc(
                    Operation::Whoami,
                    Some(Credential {
                        agent: b.agent,
                        token: a.token
                    })
                ))
                .unwrap_err()
                .code,
            "IDENTITY_ERROR"
        );
    }
}
