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
CREATE TABLE IF NOT EXISTS rpc_results(request_id TEXT PRIMARY KEY, canonical TEXT NOT NULL, response TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS participants(thread TEXT NOT NULL, agent TEXT NOT NULL REFERENCES agents(id), PRIMARY KEY(thread,agent));
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
    pub fn execute(&mut self, rpc: &Rpc) -> Result<Value> {
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
                Ok((old, response)) if old == canonical => return decode(&response),
                Ok(_) => return Err(invalid("Request ID reused for another operation")),
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(e) => return Err(e.into()),
            }
        }
        let identity = if let Some(c) = &rpc.credential {
            let found = tx.query_row("SELECT token FROM agents WHERE id=?", [&c.agent], |r| {
                r.get::<_, String>(0)
            });
            match found {
                Ok(token) if token == c.token => {}
                _ => {
                    return Err(Failure::new(
                        "IDENTITY_ERROR",
                        "Agent credential is unknown or invalid",
                        "Register an agent and select its ID with TALA_AGENT or --agent.",
                    ))
                }
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
                    "Run tala register, then set TALA_AGENT to its returned agent ID.",
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
                    "recipient=? AND received_at IS NULL AND (? IS NULL OR reply_to=?)",
                    params![id, reply_to, reply_to],
                    *after,
                    *limit,
                )?;
                if !peek {
                    for msg in &mut page.messages {
                        tx.execute(
                            "UPDATE messages SET received_at=? WHERE id=?",
                            params![now(), msg.id],
                        )?;
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
                after,
                limit,
            } => {
                let id = require()?;
                if !["all", "incoming", "outgoing"].contains(&direction.as_str()) {
                    return Err(invalid("Direction must be incoming, outgoing or all"));
                }
                json!(page(&tx,"expects_reply=1 AND answered_by IS NULL AND cancelled=0 AND ((recipient=? AND ?!='outgoing') OR (sender=? AND ?!='incoming'))",params![id,direction,id,direction],*after,*limit)?)
            }
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
        if cache {
            tx.execute(
                "INSERT INTO rpc_results(request_id,canonical,response) VALUES(?,?,?)",
                params![rpc.request_id, canonical, encode(&result)?],
            )?;
        }
        tx.commit()?;
        Ok(result)
    }
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
    let prior = tx.query_row(
        "SELECT id,canonical FROM messages WHERE sender=? AND key=?",
        params![sender, key],
        |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)),
    );
    match prior {
        Ok((id, old)) => {
            if old != canonical {
                return Err(Failure::new(
                    "IDEMPOTENCY_CONFLICT",
                    "Key already used for a different operation",
                    "Use a fresh key for a new message; preserve keys only for identical retries.",
                ));
            }
            return Ok(json!({"message":message(tx,id)?,"duplicate":true}));
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {}
        Err(e) => return Err(e.into()),
    }
    let (recipient, board, thread_id) = if let Some(id) = reply_to {
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
            (Some(parent.sender), None, parent.thread_id)
        } else {
            if *request {
                return Err(invalid(
                    "Board replies are informational; send an addressed request instead",
                ));
            }
            if !visible_thread(tx, sender, &parent.thread_id)? {
                return Err(invalid("Board thread is not visible to this agent"));
            }
            (None, parent.project, parent.thread_id)
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
        (Some(recipient), None, thread_id)
    };
    if kind == "handoff" && thread.is_none() {
        return Err(invalid("Handoff requires a thread"));
    }
    if recipient.as_deref() == Some(sender) {
        return Err(invalid("Choose a different recipient"));
    }
    tx.execute("INSERT INTO messages(thread,sender,recipient,project,context,parts,kind,reply_to,expects_reply,created_at,key,canonical) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",params![thread_id,sender,recipient,board,encode(context)?,encode(parts)?,kind,reply_to,request,now(),key,canonical])?;
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
    if let Some(parent) = reply_to {
        tx.execute(
            "UPDATE messages SET received_at=COALESCE(received_at,?) WHERE id=? AND recipient=?",
            params![now(), parent, sender],
        )?;
        tx.execute("UPDATE messages SET answered_by=? WHERE id=? AND expects_reply=1 AND cancelled=0 AND answered_by IS NULL",params![id,parent])?;
    }
    Ok(json!({"message":message(tx,id)?,"duplicate":false}))
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
        req.protocol_version = 1;
        assert_eq!(store.execute(&req).unwrap_err().code, "PROTOCOL_MISMATCH");
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
