use crate::models::*;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const PROJECT_STALE_AFTER: i64 = 30 * 24 * 60 * 60;
const THREAD_LIST_SNAPSHOT_TTL: i64 = 15 * 60;
const THREAD_LIST_SNAPSHOT_LIMIT: usize = 8;
const THREAD_LIST_SUMMARY_QUERY: &str =
    "WITH attention AS (
       SELECT thread,unread_count FROM thread_unread WHERE agent=?1
     ), pending AS (
       SELECT r.thread,
         SUM(CASE WHEN r.direction='incoming' AND r.created_at>=?2 AND a.active=1 THEN 1 ELSE 0 END) AS fresh_incoming,
         SUM(CASE WHEN r.direction='outgoing' AND r.created_at>=?2 AND a.active=1 THEN 1 ELSE 0 END) AS fresh_outgoing,
         SUM(CASE WHEN r.direction='incoming' AND (r.created_at<?2 OR a.active=0) THEN 1 ELSE 0 END) AS stale_incoming,
         SUM(CASE WHEN r.direction='outgoing' AND (r.created_at<?2 OR a.active=0) THEN 1 ELSE 0 END) AS stale_outgoing
       FROM thread_open_requests r
       JOIN agents a ON a.id=r.counterpart
       WHERE r.agent=?1
       GROUP BY r.thread
     )
     SELECT s.thread,s.kind,s.board_project,s.latest_message_id,s.latest_message_at,
            s.message_count,s.excerpt,COALESCE(attention.unread_count,0),
            COALESCE(pending.fresh_incoming,0),COALESCE(pending.fresh_outgoing,0),
            COALESCE(pending.stale_incoming,0),COALESCE(pending.stale_outgoing,0)
     FROM thread_list_access AS v INDEXED BY thread_list_access_order
     JOIN thread_summaries s ON s.thread=v.thread
     LEFT JOIN attention ON attention.thread=s.thread
     LEFT JOIN pending ON pending.thread=s.thread
     WHERE v.agent=?1
     ORDER BY v.latest_message_id DESC,v.thread ASC";

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

/// Identity and board summary read from one read-only SQLite snapshot.
pub struct StatusContextSnapshot {
    pub name: String,
    pub project: String,
    pub board_latest_id: u64,
}

/// Reads status context without opening the daemon store for writes or
/// refreshing agent presence. The identity lookup requires both public ID and
/// private token; only non-secret display context is returned.
pub fn status_context_snapshot(
    path: &Path,
    credential: &Credential,
) -> std::result::Result<Option<StatusContextSnapshot>, rusqlite::Error> {
    let db = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    db.busy_timeout(std::time::Duration::from_secs(1))?;
    let tx = db.unchecked_transaction()?;
    let identity = tx.query_row(
        "SELECT name,context FROM agents WHERE id=? AND token=?",
        params![credential.agent, credential.token],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );
    let (name, context) = match identity {
        Ok(identity) => identity,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(error) => return Err(error),
    };
    let context: Context = serde_json::from_str(&context).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let board_latest_id = tx.query_row(
        "SELECT COALESCE(MAX(id),0) FROM messages WHERE project=?",
        [&context.project],
        |row| row.get(0),
    )?;
    tx.commit()?;
    Ok(Some(StatusContextSnapshot {
        name,
        project: context.project,
        board_latest_id,
    }))
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
        replace_file(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}
/// Atomically moves a staged file into place, replacing an existing file.
/// `std::fs::rename` has replacement semantics on Unix but fails when the
/// destination exists on Windows.
pub fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        use std::{iter, os::windows::ffi::OsStrExt};

        #[link(name = "kernel32")]
        extern "system" {
            #[link_name = "MoveFileExW"]
            fn move_file_ex_w(existing: *const u16, new: *const u16, flags: u32) -> i32;
        }

        const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
        const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
        let from: Vec<u16> = from
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();
        let to: Vec<u16> = to.as_os_str().encode_wide().chain(iter::once(0)).collect();
        let result = unsafe {
            move_file_ex_w(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if result == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
    #[cfg(not(windows))]
    {
        std::fs::rename(from, to)
    }
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
fn is_bidi_formatting_control(c: char) -> bool {
    matches!(
        c as u32,
        0x061c | 0x200e..=0x200f | 0x202a..=0x202e | 0x2066..=0x2069
    )
}
fn normalize_purpose(purpose: Option<&str>) -> Result<Option<String>> {
    let Some(purpose) = purpose else {
        return Ok(None);
    };
    if purpose
        .chars()
        .any(|c| c.is_control() || is_bidi_formatting_control(c))
    {
        return Err(invalid(
            "Purpose cannot contain control or bidi formatting characters",
        ));
    }
    let purpose = purpose.trim();
    if purpose.chars().count() > 160 {
        return Err(invalid("Purpose must be at most 160 Unicode scalar values"));
    }
    Ok((!purpose.is_empty()).then(|| purpose.to_string()))
}
fn thread_not_visible() -> Failure {
    Failure::new(
        "THREAD_NOT_VISIBLE",
        "Thread is missing or not visible to this agent",
        "The thread may be outside this identity or project context. Select the original private identity, or ask a participant who can see it to run `tala handoff --to=<your-agt-id> --thread=<thread-id> \"Summary\"`. See `tala guide identity` for selector help.",
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
        let mut version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if !(0..=5).contains(&version) {
            return Err(Failure::new(
                "DATABASE_VERSION",
                format!("Unsupported database schema {version}"),
                "Use a matching Tala binary; do not delete the database.",
            ));
        }
        if version == 1 {
            db.execute_batch(
                "BEGIN IMMEDIATE;
                 ALTER TABLE messages ADD COLUMN recipient_listening INTEGER;
                 PRAGMA user_version=2;
                 COMMIT;",
            )?;
            version = 2;
        }
        db.execute_batch("BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS agents(id TEXT PRIMARY KEY, name TEXT NOT NULL, tool TEXT NOT NULL, context TEXT NOT NULL, token TEXT NOT NULL, last_seen INTEGER NOT NULL, active INTEGER NOT NULL DEFAULT 1, listening_until INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS messages(id INTEGER PRIMARY KEY AUTOINCREMENT, thread TEXT NOT NULL, sender TEXT NOT NULL REFERENCES agents(id), recipient TEXT REFERENCES agents(id), project TEXT, context TEXT NOT NULL, parts TEXT NOT NULL, kind TEXT NOT NULL, reply_to INTEGER REFERENCES messages(id), expects_reply INTEGER NOT NULL, created_at INTEGER NOT NULL, received_at INTEGER, answered_by INTEGER REFERENCES messages(id), cancelled INTEGER NOT NULL DEFAULT 0, key TEXT NOT NULL, canonical TEXT NOT NULL, recipient_listening INTEGER, UNIQUE(sender,key));
CREATE TABLE IF NOT EXISTS projects(project TEXT PRIMARY KEY, owner_agent TEXT NOT NULL REFERENCES agents(id), closed INTEGER NOT NULL DEFAULT 0, last_activity INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS inbox ON messages(recipient,received_at,id);
CREATE INDEX IF NOT EXISTS thread_messages ON messages(thread,id);
CREATE INDEX IF NOT EXISTS board_messages ON messages(project,id);
CREATE INDEX IF NOT EXISTS incoming_requests ON messages(recipient,expects_reply,answered_by);
CREATE INDEX IF NOT EXISTS outgoing_requests ON messages(sender,expects_reply,answered_by);
CREATE TABLE IF NOT EXISTS rpc_results(request_id TEXT PRIMARY KEY, canonical TEXT NOT NULL, response TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS participants(thread TEXT NOT NULL, agent TEXT NOT NULL REFERENCES agents(id), PRIMARY KEY(thread,agent));
CREATE TABLE IF NOT EXISTS board_deliveries(message INTEGER NOT NULL REFERENCES messages(id), agent TEXT NOT NULL REFERENCES agents(id), received_at INTEGER, PRIMARY KEY(message,agent));
CREATE INDEX IF NOT EXISTS unread_deliveries ON board_deliveries(agent,received_at);
INSERT OR IGNORE INTO projects(project,owner_agent,closed,last_activity)
SELECT json_extract(first.context,'$.project'), first.id, 0, activity.last_activity
FROM agents AS first
JOIN (
  SELECT json_extract(context,'$.project') AS project, MIN(rowid) AS first_rowid,
         MAX(last_seen) AS last_seen
  FROM agents
  WHERE json_extract(context,'$.project') IS NOT NULL
  GROUP BY json_extract(context,'$.project')
) AS owners ON owners.first_rowid=first.rowid
JOIN (
  SELECT json_extract(context,'$.project') AS project, MAX(last_seen) AS last_activity
  FROM agents
  WHERE json_extract(context,'$.project') IS NOT NULL
  GROUP BY json_extract(context,'$.project')
) AS activity ON activity.project=json_extract(first.context,'$.project');
UPDATE projects
SET last_activity=MAX(last_activity, COALESCE((
  SELECT MAX(created_at) FROM messages
  WHERE json_extract(messages.context,'$.project')=projects.project
     OR messages.project=projects.project
),last_activity));
COMMIT;")?;
        if version < 3 {
            db.execute_batch(
                "BEGIN IMMEDIATE;
                 ALTER TABLE messages ADD COLUMN supersedes INTEGER REFERENCES messages(id);
                 CREATE UNIQUE INDEX messages_supersedes_unique ON messages(supersedes);
                 PRAGMA user_version=3;
                 COMMIT;",
            )?;
        }
        if version < 4 {
            db.execute_batch(
                "BEGIN IMMEDIATE;
                 ALTER TABLE agents ADD COLUMN purpose TEXT;
                 ALTER TABLE agents ADD COLUMN purpose_updated_at INTEGER;
                 PRAGMA user_version=4;
                 COMMIT;",
            )?;
        }
        if version < 5 {
            db.execute_batch(
                "BEGIN IMMEDIATE;
                 CREATE TABLE thread_summaries(
                   thread TEXT PRIMARY KEY,
                   kind TEXT NOT NULL CHECK(kind IN ('direct','board')),
                   board_project TEXT,
                   latest_message_id INTEGER NOT NULL,
                   latest_message_at INTEGER NOT NULL,
                   message_count INTEGER NOT NULL,
                   excerpt TEXT NOT NULL
                 );
                 CREATE TABLE thread_conversation_participants(
                   thread TEXT NOT NULL REFERENCES thread_summaries(thread) ON DELETE CASCADE,
                   agent TEXT NOT NULL REFERENCES agents(id),
                   PRIMARY KEY(thread,agent)
                 );
                 CREATE TABLE thread_handoff_grants(
                   thread TEXT NOT NULL REFERENCES thread_summaries(thread) ON DELETE CASCADE,
                   agent TEXT NOT NULL REFERENCES agents(id),
                   grant_message INTEGER NOT NULL UNIQUE REFERENCES messages(id),
                   granted_by TEXT NOT NULL REFERENCES agents(id),
                   granted_at INTEGER NOT NULL,
                   PRIMARY KEY(thread,agent,grant_message)
                 );
                 CREATE TABLE thread_list_access(
                   agent TEXT NOT NULL REFERENCES agents(id),
                   thread TEXT NOT NULL REFERENCES thread_summaries(thread) ON DELETE CASCADE,
                   latest_message_id INTEGER NOT NULL,
                   PRIMARY KEY(agent,thread)
                 );
                 CREATE INDEX thread_list_access_order ON thread_list_access(agent,latest_message_id DESC,thread ASC);
                 CREATE TABLE thread_unread(
                   thread TEXT NOT NULL REFERENCES thread_summaries(thread) ON DELETE CASCADE,
                   agent TEXT NOT NULL REFERENCES agents(id),
                   unread_count INTEGER NOT NULL CHECK(unread_count>0),
                   PRIMARY KEY(thread,agent)
                 );
                 CREATE TABLE thread_open_requests(
                   message INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                   thread TEXT NOT NULL REFERENCES thread_summaries(thread) ON DELETE CASCADE,
                   agent TEXT NOT NULL REFERENCES agents(id),
                   counterpart TEXT NOT NULL REFERENCES agents(id),
                   direction TEXT NOT NULL CHECK(direction IN ('incoming','outgoing')),
                   created_at INTEGER NOT NULL,
                   PRIMARY KEY(message,agent)
                 );
                 CREATE INDEX thread_open_by_agent ON thread_open_requests(agent,thread,direction,created_at);
                 CREATE TABLE thread_list_snapshots(
                   id TEXT PRIMARY KEY,
                   agent TEXT NOT NULL REFERENCES agents(id),
                   high_water_id INTEGER NOT NULL,
                   captured_at INTEGER NOT NULL,
                   expires_at INTEGER NOT NULL
                 );
                 CREATE INDEX thread_list_snapshots_expiry ON thread_list_snapshots(expires_at);
                 CREATE INDEX thread_list_snapshots_agent ON thread_list_snapshots(agent,captured_at);
                 CREATE TABLE thread_list_snapshot_rows(
                   snapshot_id TEXT NOT NULL REFERENCES thread_list_snapshots(id) ON DELETE CASCADE,
                   latest_message_id INTEGER NOT NULL,
                   thread TEXT NOT NULL,
                   row_json TEXT NOT NULL,
                   PRIMARY KEY(snapshot_id,thread)
                 );
                 CREATE INDEX thread_list_rows_order ON thread_list_snapshot_rows(snapshot_id,latest_message_id DESC,thread ASC);
                 CREATE TABLE thread_list_cursors(
                   token TEXT PRIMARY KEY,
                   snapshot_id TEXT NOT NULL REFERENCES thread_list_snapshots(id) ON DELETE CASCADE,
                   agent TEXT NOT NULL REFERENCES agents(id),
                   last_message_id INTEGER NOT NULL,
                   last_thread TEXT NOT NULL,
                   UNIQUE(snapshot_id,last_message_id,last_thread)
                 );
                 CREATE INDEX thread_list_cursors_snapshot ON thread_list_cursors(snapshot_id);

                 WITH bounds AS (
                   SELECT thread,MIN(id) AS first_id,MAX(id) AS latest_id,COUNT(*) AS message_count
                   FROM messages GROUP BY thread
                 )
                 INSERT INTO thread_summaries(thread,kind,board_project,latest_message_id,latest_message_at,message_count,excerpt)
                 SELECT b.thread,
                        CASE WHEN first.project IS NULL THEN 'direct' ELSE 'board' END,
                        first.project,
                        b.latest_id,
                        latest.created_at,
                        b.message_count,
                        COALESCE((
                          SELECT substr(
                            CASE WHEN instr(text,char(10))>0 THEN substr(text,1,instr(text,char(10))-1) ELSE text END,
                            1,160
                          )
                          FROM (
                            SELECT json_extract(part.value,'$.text') AS text
                            FROM json_each(latest.parts) AS part
                            WHERE json_extract(part.value,'$.type')='text'
                            LIMIT 1
                          )
                        ),'')
                 FROM bounds b
                 JOIN messages first ON first.id=b.first_id
                 JOIN messages latest ON latest.id=b.latest_id;

                 INSERT OR IGNORE INTO thread_conversation_participants(thread,agent)
                   SELECT thread,sender FROM messages WHERE kind!='handoff';
                 INSERT OR IGNORE INTO thread_conversation_participants(thread,agent)
                   SELECT thread,recipient FROM messages WHERE kind!='handoff' AND recipient IS NOT NULL;
                 INSERT OR IGNORE INTO thread_handoff_grants(thread,agent,grant_message,granted_by,granted_at)
                   SELECT thread,recipient,id,sender,created_at FROM messages
                   WHERE kind='handoff' AND recipient IS NOT NULL;
                 INSERT INTO thread_list_access(agent,thread,latest_message_id)
                   SELECT roles.agent,roles.thread,s.latest_message_id
                   FROM (
                     SELECT agent,thread FROM thread_conversation_participants
                     UNION
                     SELECT agent,thread FROM thread_handoff_grants
                   ) roles
                   JOIN thread_summaries s ON s.thread=roles.thread;
                 INSERT INTO thread_unread(thread,agent,unread_count)
                   SELECT thread,agent,COUNT(*) FROM (
                     SELECT thread,recipient AS agent FROM messages
                     WHERE recipient IS NOT NULL AND received_at IS NULL
                     UNION ALL
                     SELECT m.thread,d.agent FROM board_deliveries d
                     JOIN messages m ON m.id=d.message
                     WHERE d.received_at IS NULL
                   ) unread GROUP BY thread,agent;
                 INSERT INTO thread_open_requests(message,thread,agent,counterpart,direction,created_at)
                   SELECT m.id,m.thread,m.sender,m.recipient,'outgoing',m.created_at
                   FROM messages m
                   WHERE m.expects_reply=1 AND m.recipient IS NOT NULL
                     AND m.answered_by IS NULL AND m.cancelled=0
                     AND NOT EXISTS(SELECT 1 FROM messages s WHERE s.supersedes=m.id);
                 INSERT INTO thread_open_requests(message,thread,agent,counterpart,direction,created_at)
                   SELECT m.id,m.thread,m.recipient,m.sender,'incoming',m.created_at
                   FROM messages m
                   WHERE m.expects_reply=1 AND m.recipient IS NOT NULL
                     AND m.answered_by IS NULL AND m.cancelled=0
                     AND NOT EXISTS(SELECT 1 FROM messages s WHERE s.supersedes=m.id);
                 PRAGMA user_version=5;
                 COMMIT;",
            )?;
        }
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
                | Operation::CloseProject
                | Operation::ReopenProject
                | Operation::Unregister
                | Operation::Rename { .. }
                | Operation::UpdatePurpose { .. }
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
                if !matches!(&rpc.operation, Operation::UpdatePurpose { .. }) {
                    tx.execute(
                        "UPDATE agents SET last_seen=?, active=1 WHERE id=?",
                        params![now(), c.agent],
                    )?;
                }
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
                purpose,
            } => {
                if name.trim().is_empty() || tool.trim().is_empty() || context.project.is_empty() {
                    return Err(invalid("Name, tool and project are required"));
                }
                let purpose = normalize_purpose(purpose.as_deref())?;
                let id = format!("agt_{}", Uuid::new_v4().simple());
                let token = Uuid::new_v4().to_string();
                let created_at = now();
                let purpose_updated_at = purpose.as_ref().map(|_| created_at);
                tx.execute(
                    "INSERT INTO agents(id,name,tool,context,token,last_seen,purpose,purpose_updated_at) VALUES(?,?,?,?,?,?,?,?)",
                    params![id, name, tool, encode(context)?, token, created_at, purpose, purpose_updated_at],
                )?;
                tx.execute(
                    "INSERT OR IGNORE INTO projects(project,owner_agent,closed,last_activity) VALUES(?,?,0,?)",
                    params![context.project, id, created_at],
                )?;
                tx.execute(
                    "UPDATE projects SET last_activity=? WHERE project=?",
                    params![created_at, context.project],
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
            Operation::Rename { name } => {
                if name.trim().is_empty() {
                    return Err(invalid("Name is required"));
                }
                let id = require()?;
                let previous_name: String =
                    tx.query_row("SELECT name FROM agents WHERE id=?", [id], |r| r.get(0))?;
                let changed = previous_name != *name;
                if changed {
                    tx.execute("UPDATE agents SET name=? WHERE id=?", params![name, id])?;
                }
                json!({
                    "agent": agent(&tx, id)?,
                    "previous_name": previous_name,
                    "changed": changed,
                })
            }
            Operation::UpdatePurpose { purpose } => {
                let id = require()?;
                let purpose = normalize_purpose(purpose.as_deref())?;
                tx.execute(
                    "UPDATE agents SET purpose=?,purpose_updated_at=? WHERE id=?",
                    params![purpose, now(), id],
                )?;
                json!({"agent": agent(&tx, id)?})
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
            Operation::ListProjects {
                include_stale,
                include_closed,
                after,
                limit,
            } => {
                bounds(*limit)?;
                let at = now();
                let stale_before = at - PROJECT_STALE_AFTER;
                let mut stmt = tx.prepare(
                    "SELECT project,owner_agent,closed,last_activity FROM projects ORDER BY project",
                )?;
                let rows = stmt
                    .query_map([], |r| {
                        Ok((
                            r.get::<_, String>(0)?,
                            r.get::<_, String>(1)?,
                            r.get::<_, bool>(2)?,
                            r.get::<_, i64>(3)?,
                        ))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let mut projects = Vec::new();
                let (mut hidden_stale, mut hidden_closed) = (0usize, 0usize);
                for (project, owner_id, closed, last_activity) in rows {
                    let stale = last_activity <= stale_before;
                    if closed && !include_closed {
                        hidden_closed += 1;
                    }
                    if stale && !closed && !include_stale {
                        hidden_stale += 1;
                    }
                    if (closed && !include_closed) || (!closed && stale && !include_stale) {
                        continue;
                    }
                    if project.as_str() <= after.as_str() {
                        continue;
                    }
                    let owner = agent(&tx, &owner_id)?;
                    projects.push(json!({
                        "project": project,
                        "owner_id": owner.id,
                        "owner_name": owner.name,
                        "closed": closed,
                        "stale": stale,
                        "last_activity": last_activity,
                    }));
                }
                let has_more = projects.len() > *limit;
                projects.truncate(*limit);
                let next = projects
                    .last()
                    .and_then(|p| p["project"].as_str())
                    .unwrap_or(after);
                json!({
                    "projects": projects,
                    "next_after": next,
                    "has_more": has_more,
                    "hidden": {"stale": hidden_stale, "closed": hidden_closed},
                    "include_stale": include_stale,
                    "include_closed": include_closed,
                })
            }
            Operation::CloseProject | Operation::ReopenProject => {
                let id = require()?;
                let project = agent(&tx, id)?.context.project;
                let (owner, closed): (String, bool) = tx.query_row(
                    "SELECT owner_agent,closed FROM projects WHERE project=?",
                    [&project],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                if owner != id {
                    return Err(Failure::new(
                        "NOT_PROJECT_OWNER",
                        "Only the owning agent can change project lifecycle",
                        "Select the owner identity or ask the owner agent to change the project state.",
                    ));
                }
                let should_close = matches!(&rpc.operation, Operation::CloseProject);
                if closed != should_close && should_close {
                    tx.execute("UPDATE projects SET closed=1 WHERE project=?", [&project])?;
                } else if closed != should_close {
                    tx.execute(
                        "UPDATE projects SET closed=0,last_activity=? WHERE project=?",
                        params![now(), project],
                    )?;
                }
                json!({"project":project,"closed":should_close,"changed":closed != should_close})
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
                        acknowledge_delivery(&tx, msg.id, &msg.thread_id, id)?;
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
                let page = page(&tx, "project=?", [project], *after, *limit)?;
                let latest_id: u64 = tx.query_row(
                    "SELECT COALESCE(MAX(id),0) FROM messages WHERE project=?",
                    [project],
                    |row| row.get(0),
                )?;
                let mut data = json!(page);
                data["latest_id"] = json!(latest_id);
                data
            }
            Operation::History {
                thread,
                after,
                limit,
            } => {
                let id = require()?;
                if !visible_thread(&tx, id, thread)? {
                    return Err(thread_not_visible());
                }
                json!(page(&tx, "thread=?", [thread], *after, *limit)?)
            }
            Operation::ListThreads { cursor, limit } => {
                list_threads(&tx, require()?, cursor.as_deref(), *limit)?
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
                tx.execute("DELETE FROM thread_open_requests WHERE message=?", [id])?;
                json!({"message":message(&tx,*id)?})
            }
            Operation::MessageStatus { message: id } => {
                let m = message(&tx, *id)?;
                if !visible_thread(&tx, require()?, &m.thread_id)? {
                    return Err(thread_not_visible());
                }
                json!({"message":m})
            }
            Operation::Status => {
                json!({"running":true,"protocol_version":PROTOCOL_VERSION,"agents":tx.query_row("SELECT count(*) FROM agents",[],|r|r.get::<_,u64>(0))?,"messages":tx.query_row("SELECT count(*) FROM messages",[],|r|r.get::<_,u64>(0))?})
            }
            Operation::Stop => json!({"stopped":true}),
            Operation::NetworkOpen { .. }
            | Operation::NetworkExtend { .. }
            | Operation::NetworkClose
            | Operation::NetworkRotateToken
            | Operation::NetworkStatus => {
                return Err(invalid("Network operations are handled by the daemon"))
            }
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
const OPEN: &str = "messages.expects_reply=1 AND messages.answered_by IS NULL AND messages.cancelled=0 AND NOT EXISTS(SELECT 1 FROM messages superseder WHERE superseder.supersedes=messages.id)";
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

fn list_threads(
    tx: &Transaction<'_>,
    agent_id: &str,
    cursor: Option<&str>,
    limit: usize,
) -> Result<Value> {
    bounds(limit)?;
    let at = now();
    if let Some(cursor) = cursor {
        let snapshot = tx
            .query_row(
                "SELECT c.snapshot_id,c.last_message_id,c.last_thread,s.high_water_id
                 FROM thread_list_cursors c
                 JOIN thread_list_snapshots s ON s.id=c.snapshot_id
                 WHERE c.token=?1 AND c.agent=?2 AND s.agent=?2 AND s.expires_at>?3",
                params![cursor, agent_id, at],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(invalid_cursor)?;
        let mut rows = tx
            .prepare(
                "SELECT latest_message_id,thread,row_json
                 FROM thread_list_snapshot_rows
                 WHERE snapshot_id=?1
                   AND (latest_message_id < ?2 OR (latest_message_id=?2 AND thread>?3))
                 ORDER BY latest_message_id DESC,thread ASC
                 LIMIT ?4",
            )?
            .query_map(
                params![snapshot.0, snapshot.1, snapshot.2, limit + 1],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let has_more = rows.len() > limit;
        rows.truncate(limit);
        let next_cursor = if has_more {
            rows.last()
                .map(|row| thread_list_cursor(tx, &snapshot.0, agent_id, row.0, &row.1))
                .transpose()?
        } else {
            None
        };
        let threads = rows
            .into_iter()
            .map(|(_, _, value)| decode::<Value>(&value))
            .collect::<Result<Vec<_>>>()?;
        return Ok(json!({
            "threads": threads,
            "snapshot_high_water_id": snapshot.3,
            "has_more": has_more,
            "next_cursor": next_cursor,
        }));
    }

    let high_water_id: i64 =
        tx.query_row("SELECT COALESCE(MAX(id),0) FROM messages", [], |row| {
            row.get(0)
        })?;
    let rows = thread_list_rows(tx, agent_id, at)?;
    let has_more = rows.len() > limit;
    let page = &rows[..rows.len().min(limit)];
    let next_cursor = if has_more {
        let snapshot_id = format!("tls_{}", Uuid::new_v4().simple());
        prune_thread_list_snapshots(tx, agent_id, at)?;
        tx.execute(
            "INSERT INTO thread_list_snapshots(id,agent,high_water_id,captured_at,expires_at)
             VALUES(?1,?2,?3,?4,?5)",
            params![
                snapshot_id,
                agent_id,
                high_water_id,
                at,
                at + THREAD_LIST_SNAPSHOT_TTL
            ],
        )?;
        for (latest_message_id, thread, row) in &rows {
            tx.execute(
                "INSERT INTO thread_list_snapshot_rows(snapshot_id,latest_message_id,thread,row_json)
                 VALUES(?1,?2,?3,?4)",
                params![snapshot_id, latest_message_id, thread, encode(row)?],
            )?;
        }
        let last = page.last().expect("has_more requires a nonempty page");
        Some(thread_list_cursor(
            tx,
            &snapshot_id,
            agent_id,
            last.0,
            &last.1,
        )?)
    } else {
        None
    };
    Ok(json!({
        "threads": page.iter().map(|row| &row.2).collect::<Vec<_>>(),
        "snapshot_high_water_id": high_water_id,
        "has_more": has_more,
        "next_cursor": next_cursor,
    }))
}

fn thread_list_rows(
    tx: &Transaction<'_>,
    agent_id: &str,
    captured_at: i64,
) -> Result<Vec<(i64, String, Value)>> {
    let summaries = tx
        .prepare(THREAD_LIST_SUMMARY_QUERY)?
        .query_map(params![agent_id, captured_at - STALE_AFTER as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
                row.get::<_, i64>(10)?,
                row.get::<_, i64>(11)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut rows = Vec::with_capacity(summaries.len());
    let (mut participants_by_thread, mut handoff_only_by_thread) =
        thread_list_people(tx, agent_id)?;
    for (
        thread,
        kind,
        project,
        latest_message_id,
        latest_message_at,
        message_count,
        excerpt,
        unread,
        pending_incoming,
        pending_outgoing,
        stale_pending_incoming,
        stale_pending_outgoing,
    ) in summaries
    {
        let participants = participants_by_thread.remove(&thread).unwrap_or_default();
        let handoff_only_recipients = handoff_only_by_thread.remove(&thread).unwrap_or_default();
        let row = json!({
            "thread": thread,
            "kind": kind,
            "project": project,
            "participants": participants,
            "handoff_only_recipients": handoff_only_recipients,
            "last_message_id": latest_message_id,
            "last_message_at": latest_message_at,
            "message_count": message_count,
            "unread": unread,
            "pending_incoming": pending_incoming,
            "pending_outgoing": pending_outgoing,
            "stale_pending_incoming": stale_pending_incoming,
            "stale_pending_outgoing": stale_pending_outgoing,
            "excerpt": excerpt,
        });
        rows.push((latest_message_id, thread, row));
    }
    Ok(rows)
}

type ThreadPeopleByThread = HashMap<String, Vec<Value>>;

fn thread_list_people(
    tx: &Transaction<'_>,
    agent_id: &str,
) -> Result<(ThreadPeopleByThread, ThreadPeopleByThread)> {
    let mut participants = ThreadPeopleByThread::new();
    let mut statement = tx.prepare(
        "SELECT p.thread,p.agent,a.name
         FROM thread_list_access AS v INDEXED BY thread_list_access_order
         JOIN thread_conversation_participants p ON p.thread=v.thread
         JOIN agents a ON a.id=p.agent
         WHERE v.agent=?1
         ORDER BY p.thread,p.agent",
    )?;
    let rows = statement
        .query_map([agent_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                json!({"id": row.get::<_, String>(1)?, "name": row.get::<_, String>(2)?}),
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (thread, person) in rows {
        participants.entry(thread).or_default().push(person);
    }

    let mut handoff_only = ThreadPeopleByThread::new();
    let mut statement = tx.prepare(
        "SELECT g.thread,g.agent,a.name
         FROM thread_list_access AS v INDEXED BY thread_list_access_order
         JOIN thread_handoff_grants g ON g.thread=v.thread
         JOIN agents a ON a.id=g.agent
         WHERE v.agent=?1
           AND NOT EXISTS(
           SELECT 1 FROM thread_conversation_participants p
           WHERE p.thread=g.thread AND p.agent=g.agent
         )
         GROUP BY g.thread,g.agent
         ORDER BY g.thread,g.agent",
    )?;
    let rows = statement
        .query_map([agent_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                json!({"id": row.get::<_, String>(1)?, "name": row.get::<_, String>(2)?}),
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for (thread, person) in rows {
        handoff_only.entry(thread).or_default().push(person);
    }
    Ok((participants, handoff_only))
}

fn prune_thread_list_snapshots(tx: &Transaction<'_>, agent_id: &str, at: i64) -> Result<()> {
    tx.execute(
        "DELETE FROM thread_list_snapshots WHERE expires_at<=?",
        [at],
    )?;
    let count: usize = tx.query_row(
        "SELECT COUNT(*) FROM thread_list_snapshots WHERE agent=?",
        [agent_id],
        |row| row.get(0),
    )?;
    if count >= THREAD_LIST_SNAPSHOT_LIMIT {
        let remove = count - THREAD_LIST_SNAPSHOT_LIMIT + 1;
        let oldest = tx
            .prepare(
                "SELECT id FROM thread_list_snapshots WHERE agent=?
                 ORDER BY rowid ASC LIMIT ?",
            )?
            .query_map(params![agent_id, remove], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for id in oldest {
            tx.execute("DELETE FROM thread_list_snapshots WHERE id=?", [id])?;
        }
    }
    Ok(())
}

fn thread_list_cursor(
    tx: &Transaction<'_>,
    snapshot_id: &str,
    agent_id: &str,
    last_message_id: i64,
    last_thread: &str,
) -> Result<String> {
    let existing = tx
        .query_row(
            "SELECT token FROM thread_list_cursors
             WHERE snapshot_id=?1 AND last_message_id=?2 AND last_thread=?3",
            params![snapshot_id, last_message_id, last_thread],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        return Ok(existing);
    }
    let token = format!("v1_{}", Uuid::new_v4().simple());
    tx.execute(
        "INSERT INTO thread_list_cursors(token,snapshot_id,agent,last_message_id,last_thread)
         VALUES(?1,?2,?3,?4,?5)",
        params![token, snapshot_id, agent_id, last_message_id, last_thread],
    )?;
    Ok(token)
}

fn invalid_cursor() -> Failure {
    Failure::new(
        "INVALID_CURSOR",
        "History list cursor is invalid or expired",
        "Start a new thread listing with tala history.",
    )
}

fn agent(tx: &Transaction<'_>, id: &str) -> Result<Agent> {
    let row = tx.query_row(
        "SELECT name,tool,purpose,purpose_updated_at,context,last_seen,active,listening_until FROM agents WHERE id=?",
        [id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, bool>(6)?,
                r.get::<_, i64>(7)?,
            ))
        },
    )?;
    Ok(Agent {
        id: id.into(),
        name: row.0,
        tool: row.1,
        purpose: row.2,
        purpose_updated_at: row.3,
        context: decode(&row.4)?,
        last_seen: row.5,
        active: row.6 && now() - row.5 < 300,
        listening: row.7 > now(),
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
        supersedes: r.get(9)?,
        superseded_by: r.get(10)?,
        expects_reply: r.get(11)?,
        created_at: r.get(12)?,
        received_at: r.get(13)?,
        answered_by: r.get(14)?,
        cancelled: r.get(15)?,
        delivery: String::new(),
        age_seconds: (now() - r.get::<_, i64>(12)?).max(0),
    };
    msg.delivery = delivery(&msg);
    Ok(msg)
}
const COLUMNS:&str="id,thread,sender,recipient,project,context,parts,kind,reply_to,supersedes,(SELECT id FROM messages successor WHERE successor.supersedes=messages.id),expects_reply,created_at,received_at,answered_by,cancelled";
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
fn recipient_is_listening(tx: &Transaction<'_>, recipient: &str) -> Result<bool> {
    let (active, listening_until): (bool, i64) = tx.query_row(
        "SELECT active,listening_until FROM agents WHERE id=?",
        [recipient],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(active && listening_until > now())
}
/// Return the point-in-time listener observation persisted with a direct
/// message. Rows created before this field existed are frozen on first replay.
fn recipient_presence(tx: &Transaction<'_>, message_id: u64) -> Result<Option<Value>> {
    let (recipient, observed): (Option<String>, Option<bool>) = tx.query_row(
        "SELECT recipient,recipient_listening FROM messages WHERE id=?",
        [message_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let Some(recipient) = recipient else {
        return Ok(None);
    };
    let listening = match observed {
        Some(listening) => listening,
        None => {
            let listening = recipient_is_listening(tx, &recipient)?;
            tx.execute(
                "UPDATE messages SET recipient_listening=? WHERE id=? AND recipient_listening IS NULL",
                params![listening, message_id],
            )?;
            listening
        }
    };
    Ok(Some(json!({"listening": listening})))
}
fn send_result(tx: &Transaction<'_>, message_id: u64, duplicate: bool) -> Result<Value> {
    let mut result = json!({
        "message": message(tx, message_id)?,
        "duplicate": duplicate,
    });
    if let Some(presence) = recipient_presence(tx, message_id)? {
        result["recipient_presence"] = presence;
    }
    Ok(result)
}

fn thread_excerpt(parts: &[Part]) -> String {
    let Some(text) = parts.iter().find_map(|part| match part {
        Part::Text { text } => Some(text.as_str()),
        Part::File { .. } | Part::Data { .. } => None,
    }) else {
        return String::new();
    };
    text.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(160)
        .collect()
}

fn update_thread_summary(
    tx: &Transaction<'_>,
    thread: &str,
    message_id: u64,
    created_at: i64,
    project: Option<&str>,
    parts: &[Part],
) -> Result<()> {
    let excerpt = thread_excerpt(parts);
    tx.execute(
        "INSERT INTO thread_summaries(thread,kind,board_project,latest_message_id,latest_message_at,message_count,excerpt)
         VALUES(?1,?2,?3,?4,?5,1,?6)
         ON CONFLICT(thread) DO UPDATE SET
           latest_message_id=excluded.latest_message_id,
           latest_message_at=excluded.latest_message_at,
           message_count=thread_summaries.message_count+1,
           excerpt=excluded.excerpt",
        params![
            thread,
            if project.is_some() { "board" } else { "direct" },
            project,
            message_id,
            created_at,
            excerpt
        ],
    )?;
    tx.execute(
        "UPDATE thread_list_access SET latest_message_id=? WHERE thread=?",
        params![message_id, thread],
    )?;
    Ok(())
}

fn add_thread_list_access(
    tx: &Transaction<'_>,
    thread: &str,
    agent: &str,
    latest_message_id: u64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO thread_list_access(agent,thread,latest_message_id) VALUES(?,?,?)
         ON CONFLICT(agent,thread) DO UPDATE SET latest_message_id=excluded.latest_message_id",
        params![agent, thread, latest_message_id],
    )?;
    Ok(())
}

fn add_thread_unread(tx: &Transaction<'_>, thread: &str, agent: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO thread_unread(thread,agent,unread_count) VALUES(?,?,1)
         ON CONFLICT(thread,agent) DO UPDATE SET unread_count=thread_unread.unread_count+1",
        params![thread, agent],
    )?;
    Ok(())
}

fn remove_thread_unread(tx: &Transaction<'_>, thread: &str, agent: &str) -> Result<()> {
    let count: Option<u64> = tx
        .query_row(
            "SELECT unread_count FROM thread_unread WHERE thread=? AND agent=?",
            params![thread, agent],
            |row| row.get(0),
        )
        .optional()?;
    match count {
        Some(1) => {
            tx.execute(
                "DELETE FROM thread_unread WHERE thread=? AND agent=?",
                params![thread, agent],
            )?;
        }
        Some(count) if count > 1 => {
            tx.execute(
                "UPDATE thread_unread SET unread_count=unread_count-1 WHERE thread=? AND agent=?",
                params![thread, agent],
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn acknowledge_delivery(
    tx: &Transaction<'_>,
    message_id: u64,
    thread: &str,
    agent: &str,
) -> Result<()> {
    let direct_received = tx
        .query_row(
            "SELECT received_at FROM messages WHERE id=? AND recipient=?",
            params![message_id, agent],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    if matches!(direct_received, Some(None)) {
        let changed = tx.execute(
            "UPDATE messages SET received_at=? WHERE id=? AND recipient=? AND received_at IS NULL",
            params![now(), message_id, agent],
        )?;
        if changed > 0 {
            remove_thread_unread(tx, thread, agent)?;
        }
        return Ok(());
    }
    if direct_received.is_some() {
        return Ok(());
    }
    let board_received = tx
        .query_row(
            "SELECT received_at FROM board_deliveries WHERE message=? AND agent=?",
            params![message_id, agent],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    if matches!(board_received, Some(None)) {
        let changed = tx.execute(
            "UPDATE board_deliveries SET received_at=? WHERE message=? AND agent=? AND received_at IS NULL",
            params![now(), message_id, agent],
        )?;
        if changed > 0 {
            remove_thread_unread(tx, thread, agent)?;
        }
    }
    Ok(())
}

fn add_open_request(
    tx: &Transaction<'_>,
    message_id: u64,
    thread: &str,
    sender: &str,
    recipient: &str,
    created_at: i64,
) -> Result<()> {
    tx.execute(
        "INSERT INTO thread_open_requests(message,thread,agent,counterpart,direction,created_at)
         VALUES(?1,?2,?3,?4,'outgoing',?5),(?1,?2,?4,?3,'incoming',?5)",
        params![message_id, thread, sender, recipient, created_at],
    )?;
    Ok(())
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
        supersedes,
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
                return send_result(tx, id, true);
            }
            Ok(_) => stored_key = format!("{key}:{}", Uuid::new_v4().simple()),
            Err(rusqlite::Error::QueryReturnedNoRows) => {}
            Err(e) => return Err(e.into()),
        }
    } else if let Some(id) = explicit_key(tx, sender, key, &canonical)? {
        return send_result(tx, id, true);
    }
    let supersedes_parent = if let Some(id) = supersedes {
        if kind != "message" || reply_to.is_some() || project.is_some() {
            return Err(invalid(
                "Only direct messages can supersede an earlier message",
            ));
        }
        let parent = message(tx, *id)?;
        if parent.sender != sender
            || parent.recipient.is_none()
            || parent.project.is_some()
            || parent.kind == "handoff"
        {
            return Err(invalid(
                "Supersession target must be your earlier direct message in the same thread",
            ));
        }
        if let Some(successor) = parent.superseded_by {
            return Err(invalid(format!(
                "Message {id} is already superseded by {successor}; supersede the latest message in the chain"
            )));
        }
        Some(parent)
    } else {
        None
    };
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
            if parent.expects_reply {
                if let Some(successor) = parent.superseded_by {
                    return Err(Failure::new(
                        "REQUEST_SUPERSEDED",
                        format!("Request {id} was superseded by message {successor}"),
                        "Read the updated thread before replying.",
                    )
                    .with_next(format!("tala history {}", parent.thread_id)));
                }
            }
            (Some(parent.sender), None, parent.thread_id, None)
        } else {
            if *request {
                return Err(invalid(
                    "Board replies are informational; send an addressed request instead",
                ));
            }
            if !visible_thread(tx, sender, &parent.thread_id)? {
                return Err(thread_not_visible());
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
        let thread_id = if let Some(parent) = &supersedes_parent {
            if parent.recipient.as_deref() != Some(recipient.as_str()) {
                return Err(invalid(
                    "Supersession target must have the same resolved recipient",
                ));
            }
            if let Some(explicit_thread) = thread {
                if explicit_thread != &parent.thread_id {
                    return Err(invalid(
                        "Supersession target must be in the same direct thread",
                    ));
                }
            }
            parent.thread_id.clone()
        } else {
            match thread {
                Some(t) => {
                    if !visible_thread(tx, sender, t)? {
                        return Err(thread_not_visible());
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
            }
        };
        (Some(recipient), None, thread_id, None)
    };
    if kind == "handoff" && thread.is_none() {
        return Err(invalid("Handoff requires a thread"));
    }
    if recipient.as_deref() == Some(sender) {
        return Err(invalid("Choose a different recipient"));
    }
    let created_at = now();
    let recipient_listening = recipient
        .as_deref()
        .map(|recipient| recipient_is_listening(tx, recipient))
        .transpose()?;
    tx.execute("INSERT INTO messages(thread,sender,recipient,project,context,parts,kind,reply_to,expects_reply,created_at,key,canonical,recipient_listening,supersedes) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)",params![thread_id,sender,recipient,board,encode(context)?,encode(parts)?,kind,reply_to,request,created_at,stored_key,canonical,recipient_listening,supersedes])?;
    let id = tx.last_insert_rowid() as u64;
    update_thread_summary(tx, &thread_id, id, created_at, board.as_deref(), parts)?;
    tx.execute(
        "UPDATE projects SET last_activity=? WHERE project=?",
        params![created_at, context.project],
    )?;
    if let Some(recipient_id) = recipient.as_deref() {
        let recipient_project = agent(tx, recipient_id)?.context.project;
        if recipient_project != context.project {
            tx.execute(
                "UPDATE projects SET last_activity=? WHERE project=?",
                params![created_at, recipient_project],
            )?;
        }
    }
    if let Some(board_project) = &board {
        if board_project != &context.project {
            tx.execute(
                "UPDATE projects SET last_activity=? WHERE project=?",
                params![created_at, board_project],
            )?;
        }
    }
    tx.execute(
        "INSERT OR IGNORE INTO participants(thread,agent) VALUES(?,?)",
        params![thread_id, sender],
    )?;
    if kind == "handoff" {
        if let Some(recipient_id) = recipient.as_deref() {
            tx.execute(
                "INSERT INTO thread_handoff_grants(thread,agent,grant_message,granted_by,granted_at)
                 VALUES(?,?,?,?,?)",
                params![thread_id, recipient_id, id, sender, created_at],
            )?;
            add_thread_list_access(tx, &thread_id, recipient_id, id)?;
        }
    } else {
        tx.execute(
            "INSERT OR IGNORE INTO thread_conversation_participants(thread,agent) VALUES(?,?)",
            params![thread_id, sender],
        )?;
        add_thread_list_access(tx, &thread_id, sender, id)?;
    }
    if let Some(recipient) = recipient {
        tx.execute(
            "INSERT OR IGNORE INTO participants(thread,agent) VALUES(?,?)",
            params![thread_id, recipient],
        )?;
        if kind != "handoff" {
            tx.execute(
                "INSERT OR IGNORE INTO thread_conversation_participants(thread,agent) VALUES(?,?)",
                params![thread_id, recipient],
            )?;
            add_thread_list_access(tx, &thread_id, &recipient, id)?;
        }
        add_thread_unread(tx, &thread_id, &recipient)?;
        if *request {
            add_open_request(tx, id, &thread_id, sender, &recipient, created_at)?;
        }
    }
    if let Some(superseded_message) = supersedes {
        tx.execute(
            "DELETE FROM thread_open_requests WHERE message=?",
            [superseded_message],
        )?;
    }
    if let Some(author) = notify {
        tx.execute(
            "INSERT INTO board_deliveries(message,agent) VALUES(?,?)",
            params![id, author],
        )?;
        add_thread_unread(tx, &thread_id, &author)?;
    }
    if let Some(parent) = reply_to {
        // Replying counts as receipt, for direct mail and board deliveries.
        let parent_thread: String =
            tx.query_row("SELECT thread FROM messages WHERE id=?", [parent], |row| {
                row.get(0)
            })?;
        acknowledge_delivery(tx, *parent, &parent_thread, sender)?;
        let answered = tx.execute(
            "UPDATE messages SET answered_by=? WHERE id=? AND expects_reply=1 AND cancelled=0 AND answered_by IS NULL",
            params![id,parent],
        )?;
        if answered > 0 {
            tx.execute("DELETE FROM thread_open_requests WHERE message=?", [parent])?;
        }
    }
    send_result(tx, id, false)
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
                    purpose: None,
                },
                None,
            ))
            .unwrap();
        serde_json::from_value(v["credential"].clone()).unwrap()
    }
    #[test]
    fn purpose_validation_trims_and_counts_unicode_scalars() {
        assert_eq!(normalize_purpose(None).unwrap(), None);
        assert_eq!(normalize_purpose(Some("  ")).unwrap(), None);
        assert_eq!(
            normalize_purpose(Some("  reviews agent handoffs  ")).unwrap(),
            Some("reviews agent handoffs".into())
        );
        assert_eq!(
            normalize_purpose(Some(&"é".repeat(160)))
                .unwrap()
                .unwrap()
                .chars()
                .count(),
            160
        );
        assert_eq!(
            normalize_purpose(Some(&"🧭".repeat(160)))
                .unwrap()
                .unwrap()
                .len(),
            160 * 4
        );
        assert_eq!(
            normalize_purpose(Some(&"é".repeat(161))).unwrap_err().code,
            "INVALID_OPERATION"
        );
        for invalid in ["line\nbreak", "\n", "tab\tcontrol", "c1\u{0085}"] {
            assert_eq!(
                normalize_purpose(Some(invalid)).unwrap_err().code,
                "INVALID_OPERATION",
                "accepted {invalid:?}"
            );
        }
        for bidi in [
            '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}',
            '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}', '\u{2069}',
        ] {
            assert_eq!(
                normalize_purpose(Some(&format!("safe{bidi}text")))
                    .unwrap_err()
                    .code,
                "INVALID_OPERATION",
                "accepted bidi control U+{:04X}",
                bidi as u32
            );
        }
    }
    #[test]
    fn purpose_updates_are_cached_and_preserve_identity_presence_and_history() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let author = register(&mut store, "author");
        let recipient = register(&mut store, "recipient");
        store
            .execute(&rpc(
                send(&recipient.agent, "before-update"),
                Some(author.clone()),
            ))
            .unwrap();
        let thread: String = store
            .db
            .query_row("SELECT thread FROM messages LIMIT 1", [], |row| row.get(0))
            .unwrap();
        let history_before = store
            .execute(&rpc(
                Operation::History {
                    thread: thread.clone(),
                    after: 0,
                    limit: 50,
                },
                Some(author.clone()),
            ))
            .unwrap();
        store
            .db
            .execute(
                "UPDATE agents SET last_seen=12345 WHERE id=?",
                [&author.agent],
            )
            .unwrap();
        let identity_before: (String, String, String, String, i64) = store
            .db
            .query_row(
                "SELECT name,tool,context,token,last_seen FROM agents WHERE id=?",
                [&author.agent],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        let messages_before: Vec<(i64, String, String, Option<String>, String)> = store
            .db
            .prepare("SELECT id,thread,sender,recipient,kind FROM messages ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();

        let request = rpc(
            Operation::UpdatePurpose {
                purpose: Some("  reviews incoming requests  ".into()),
            },
            Some(author.clone()),
        );
        let set = store.execute(&request).unwrap();
        assert_eq!(set["agent"]["purpose"], "reviews incoming requests");
        assert!(set["agent"]["purpose_updated_at"].as_i64().is_some());
        assert_eq!(store.execute(&request).unwrap(), set);
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT purpose FROM agents WHERE id=?",
                    [&author.agent],
                    |row| row.get::<_, Option<String>>(0)
                )
                .unwrap()
                .as_deref(),
            Some("reviews incoming requests")
        );

        store
            .db
            .execute(
                "UPDATE agents SET purpose_updated_at=1 WHERE id=?",
                [&author.agent],
            )
            .unwrap();
        let cleared = store
            .execute(&rpc(
                Operation::UpdatePurpose { purpose: None },
                Some(author.clone()),
            ))
            .unwrap();
        assert!(cleared["agent"]["purpose"].is_null());
        assert!(cleared["agent"]["purpose_updated_at"].as_i64().unwrap() > 1);
        let identity_after: (String, String, String, String, i64) = store
            .db
            .query_row(
                "SELECT name,tool,context,token,last_seen FROM agents WHERE id=?",
                [&author.agent],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(identity_after, identity_before);
        let messages_after: Vec<(i64, String, String, Option<String>, String)> = store
            .db
            .prepare("SELECT id,thread,sender,recipient,kind FROM messages ORDER BY id")
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .unwrap()
            .collect::<std::result::Result<_, _>>()
            .unwrap();
        assert_eq!(messages_after, messages_before);
        let history_after = store
            .execute(&rpc(
                Operation::History {
                    thread,
                    after: 0,
                    limit: 50,
                },
                Some(author),
            ))
            .unwrap();
        assert_eq!(
            history_after["messages"][0]["id"],
            history_before["messages"][0]["id"]
        );
        assert_eq!(
            history_after["messages"][0]["parts"],
            history_before["messages"][0]["parts"]
        );
    }
    #[test]
    fn rename_preserves_identity_and_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let credential = register(&mut store, "before");
        let request = rpc(
            Operation::Rename {
                name: "after".into(),
            },
            Some(credential.clone()),
        );
        let renamed = store.execute(&request).unwrap();
        assert_eq!(renamed["agent"]["id"], credential.agent);
        assert_eq!(renamed["agent"]["name"], "after");
        assert_eq!(renamed["previous_name"], "before");
        assert_eq!(renamed["changed"], true);

        // A replay with the same request ID returns the original response.
        assert_eq!(store.execute(&request).unwrap(), renamed);

        let no_op = store
            .execute(&rpc(
                Operation::Rename {
                    name: "after".into(),
                },
                Some(credential.clone()),
            ))
            .unwrap();
        assert_eq!(no_op["agent"]["id"], credential.agent);
        assert_eq!(no_op["changed"], false);
        assert_eq!(no_op["previous_name"], "after");

        let mut conflicting_retry = request.clone();
        conflicting_retry.operation = Operation::Rename {
            name: "different".into(),
        };
        assert_eq!(
            store.execute(&conflicting_retry).unwrap_err().code,
            "INVALID_OPERATION"
        );
        assert_eq!(
            store
                .execute(&rpc(Operation::Whoami, Some(credential)))
                .unwrap()["agent"]["name"],
            "after"
        );
    }

    #[test]
    fn rename_requires_valid_identity_and_nonempty_name() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let credential = register(&mut store, "before");
        let no_identity = store
            .execute(&rpc(
                Operation::Rename {
                    name: "after".into(),
                },
                None,
            ))
            .unwrap_err();
        assert_eq!(no_identity.code, "IDENTITY_REQUIRED");

        let invalid_identity = store
            .execute(&rpc(
                Operation::Rename {
                    name: "after".into(),
                },
                Some(Credential {
                    agent: credential.agent.clone(),
                    token: "wrong".into(),
                }),
            ))
            .unwrap_err();
        assert_eq!(invalid_identity.code, "IDENTITY_ERROR");

        let empty_name = store
            .execute(&rpc(
                Operation::Rename { name: "  ".into() },
                Some(credential.clone()),
            ))
            .unwrap_err();
        assert_eq!(empty_name.code, "INVALID_OPERATION");
        assert_eq!(
            store
                .execute(&rpc(Operation::Whoami, Some(credential)))
                .unwrap()["agent"]["name"],
            "before"
        );
    }
    fn list_projects(include_stale: bool, include_closed: bool) -> Operation {
        Operation::ListProjects {
            include_stale,
            include_closed,
            after: String::new(),
            limit: 50,
        }
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
            supersedes: None,
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
    fn amendment(to: &str, key: &str, target: u64, request: bool) -> Operation {
        let mut op = send(to, key);
        if let Operation::Send {
            thread,
            parts,
            request: wants_reply,
            supersedes,
            ..
        } = &mut op
        {
            *thread = None;
            *parts = vec![Part::Text {
                text: format!("correction to {target}"),
            }];
            *wants_reply = request;
            *supersedes = Some(target);
        }
        op
    }
    #[test]
    fn supersession_preserves_history_and_linear_request_obligations() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let mut store = Store::open(&path).unwrap();
        let sender = register(&mut store, "sender");
        let recipient = register(&mut store, "recipient");
        let original = store
            .execute(&rpc(
                send(&recipient.agent, "original"),
                Some(sender.clone()),
            ))
            .unwrap();
        let original_id = id_of(&original);
        let original_thread = original["message"]["thread_id"]
            .as_str()
            .unwrap()
            .to_string();

        let correction = store
            .execute(&rpc(
                amendment(&recipient.agent, "correction", original_id, true),
                Some(sender.clone()),
            ))
            .unwrap();
        let correction_id = id_of(&correction);
        assert_eq!(correction["message"]["thread_id"], original_thread);
        assert_eq!(correction["message"]["supersedes"], original_id);
        let old = store
            .execute(&rpc(
                Operation::MessageStatus {
                    message: original_id,
                },
                Some(sender.clone()),
            ))
            .unwrap()["message"]
            .clone();
        assert_eq!(old["superseded_by"], correction_id);
        assert_eq!(old["received_at"], Value::Null);
        assert_eq!(old["answered_by"], Value::Null);

        let active = store
            .execute(&rpc(pending(false), Some(recipient.clone())))
            .unwrap();
        assert_eq!(active["messages"].as_array().unwrap().len(), 1);
        assert_eq!(active["messages"][0]["id"], correction_id);
        let outgoing = store
            .execute(&rpc(
                Operation::Pending {
                    direction: "outgoing".into(),
                    stale: false,
                    after: 0,
                    limit: 50,
                },
                Some(sender.clone()),
            ))
            .unwrap();
        assert_eq!(outgoing["messages"].as_array().unwrap().len(), 1);
        assert_eq!(outgoing["messages"][0]["id"], correction_id);

        let second = store
            .execute(&rpc(
                amendment(&recipient.agent, "second-correction", correction_id, true),
                Some(sender.clone()),
            ))
            .unwrap();
        let second_id = id_of(&second);
        assert_eq!(second["message"]["thread_id"], original_thread);
        assert_eq!(second["message"]["supersedes"], correction_id);
        assert_eq!(second["message"]["expects_reply"], true);
        let old = store
            .execute(&rpc(
                Operation::MessageStatus {
                    message: original_id,
                },
                Some(sender.clone()),
            ))
            .unwrap()["message"]
            .clone();
        assert_eq!(old["superseded_by"], correction_id);
        let middle = store
            .execute(&rpc(
                Operation::MessageStatus {
                    message: correction_id,
                },
                Some(sender.clone()),
            ))
            .unwrap()["message"]
            .clone();
        assert_eq!(middle["supersedes"], original_id);
        assert_eq!(middle["superseded_by"], second_id);

        let active = store
            .execute(&rpc(pending(false), Some(recipient.clone())))
            .unwrap();
        assert_eq!(active["messages"].as_array().unwrap().len(), 1);
        assert_eq!(active["messages"][0]["id"], second_id);
        let unread = store
            .execute(&rpc(inbox(true), Some(recipient.clone())))
            .unwrap();
        assert_eq!(unread["messages"].as_array().unwrap().len(), 3);
        assert_eq!(unread["messages"][0]["superseded_by"], correction_id);
        assert_eq!(unread["messages"][1]["supersedes"], original_id);
        assert_eq!(unread["messages"][1]["superseded_by"], second_id);
        assert_eq!(unread["messages"][2]["supersedes"], correction_id);
        assert!(unread["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|message| message["received_at"].is_null()));
        let attention = store
            .execute(&rpc(Operation::Attention, Some(recipient.clone())))
            .unwrap();
        assert_eq!(attention["incoming_requests"].as_array().unwrap().len(), 1);
        assert_eq!(attention["incoming_requests"][0]["id"], second_id);
        assert_eq!(
            attention["unread"]["messages"][0]["superseded_by"],
            correction_id
        );

        drop(store);
        let mut store = Store::open(&path).unwrap();
        let persisted = store
            .execute(&rpc(
                Operation::MessageStatus {
                    message: original_id,
                },
                Some(sender),
            ))
            .unwrap();
        assert_eq!(persisted["message"]["superseded_by"], correction_id);
    }

    #[test]
    fn supersession_after_fetch_preserves_receipt_and_exposes_successor() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let sender = register(&mut store, "sender");
        let recipient = register(&mut store, "recipient");
        let original = store
            .execute(&rpc(
                send(&recipient.agent, "original"),
                Some(sender.clone()),
            ))
            .unwrap();
        let original_id = id_of(&original);

        let fetched = store
            .execute(&rpc(inbox(false), Some(recipient.clone())))
            .unwrap();
        assert_eq!(fetched["messages"][0]["id"], original_id);
        let receipt = fetched["messages"][0]["received_at"].clone();
        assert!(!receipt.is_null());

        let correction = store
            .execute(&rpc(
                amendment(&recipient.agent, "correction", original_id, true),
                Some(sender),
            ))
            .unwrap();
        let correction_id = id_of(&correction);
        let old = store
            .execute(&rpc(
                Operation::MessageStatus {
                    message: original_id,
                },
                Some(recipient.clone()),
            ))
            .unwrap()["message"]
            .clone();
        assert_eq!(old["received_at"], receipt);
        assert_eq!(old["superseded_by"], correction_id);

        let unread = store
            .execute(&rpc(inbox(true), Some(recipient.clone())))
            .unwrap();
        assert_eq!(unread["messages"].as_array().unwrap().len(), 1);
        assert_eq!(unread["messages"][0]["id"], correction_id);
        assert_eq!(unread["messages"][0]["supersedes"], original_id);
    }
    #[test]
    fn supersession_rejects_invalid_targets_and_branches() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let sender = register(&mut store, "sender");
        let other = register(&mut store, "other");
        let recipient = register(&mut store, "recipient");
        let other_recipient = register(&mut store, "other-recipient");
        let original = store
            .execute(&rpc(
                send(&recipient.agent, "original"),
                Some(sender.clone()),
            ))
            .unwrap();
        let original_id = id_of(&original);
        let thread = original["message"]["thread_id"]
            .as_str()
            .unwrap()
            .to_string();
        store
            .execute(&rpc(
                amendment(&recipient.agent, "first", original_id, false),
                Some(sender.clone()),
            ))
            .unwrap();

        let branch = store
            .execute(&rpc(
                amendment(&recipient.agent, "branch", original_id, false),
                Some(sender.clone()),
            ))
            .unwrap_err();
        assert_eq!(branch.code, "INVALID_OPERATION");
        assert!(branch.message.contains("already superseded"));

        for (credential, to, key, explicit_thread, target) in [
            (
                other.clone(),
                recipient.agent.as_str(),
                "wrong-sender",
                None,
                original_id,
            ),
            (
                sender.clone(),
                other_recipient.agent.as_str(),
                "wrong-recipient",
                None,
                original_id,
            ),
            (
                sender.clone(),
                recipient.agent.as_str(),
                "wrong-thread",
                Some("thr_other".to_string()),
                original_id,
            ),
        ] {
            let mut op = amendment(to, key, target, false);
            if let Operation::Send { thread, .. } = &mut op {
                *thread = explicit_thread;
            }
            assert_eq!(
                store.execute(&rpc(op, Some(credential))).unwrap_err().code,
                "INVALID_OPERATION"
            );
        }
        assert_eq!(
            store
                .execute(&rpc(
                    amendment(&recipient.agent, "missing", 9_999_999, false),
                    Some(sender.clone()),
                ))
                .unwrap_err()
                .code,
            "MESSAGE_NOT_FOUND"
        );

        let mut handoff = send(&recipient.agent, "handoff");
        if let Operation::Send {
            kind, thread: t, ..
        } = &mut handoff
        {
            *kind = "handoff".into();
            *t = Some(thread.clone());
        }
        let handoff_id = id_of(&store.execute(&rpc(handoff, Some(sender.clone()))).unwrap());
        assert_eq!(
            store
                .execute(&rpc(
                    amendment(&recipient.agent, "supersede-handoff", handoff_id, false),
                    Some(sender.clone()),
                ))
                .unwrap_err()
                .code,
            "INVALID_OPERATION"
        );

        let board_id = id_of(
            &store
                .execute(&rpc(post("board"), Some(sender.clone())))
                .unwrap(),
        );
        assert_eq!(
            store
                .execute(&rpc(
                    amendment(&recipient.agent, "supersede-board", board_id, false),
                    Some(sender.clone()),
                ))
                .unwrap_err()
                .code,
            "INVALID_OPERATION"
        );
        let board_reply_id = id_of(
            &store
                .execute(&rpc(reply(board_id, "board reply"), Some(other.clone())))
                .unwrap(),
        );
        assert_eq!(
            store
                .execute(&rpc(
                    amendment(
                        &sender.agent,
                        "supersede-board-reply",
                        board_reply_id,
                        false
                    ),
                    Some(other),
                ))
                .unwrap_err()
                .code,
            "INVALID_OPERATION"
        );
    }
    #[test]
    fn replies_to_superseded_requests_fail_but_informational_messages_remain_replyable() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let sender = register(&mut store, "sender");
        let recipient = register(&mut store, "recipient");
        let request = store
            .execute(&rpc(
                send(&recipient.agent, "request"),
                Some(sender.clone()),
            ))
            .unwrap();
        let request_id = id_of(&request);
        let thread = request["message"]["thread_id"]
            .as_str()
            .unwrap()
            .to_string();
        let replacement = store
            .execute(&rpc(
                amendment(&recipient.agent, "replacement", request_id, false),
                Some(sender.clone()),
            ))
            .unwrap();
        let replacement_id = id_of(&replacement);
        let before = store
            .db
            .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, u64>(0))
            .unwrap();

        let mut reply = send("unused", "stale-reply");
        if let Operation::Send {
            to,
            reply_to,
            kind,
            request,
            ..
        } = &mut reply
        {
            *to = None;
            *reply_to = Some(request_id);
            *kind = "reply".into();
            *request = false;
        }
        let error = store
            .execute(&rpc(reply, Some(recipient.clone())))
            .unwrap_err();
        assert_eq!(error.code, "REQUEST_SUPERSEDED");
        assert!(error.message.contains(&replacement_id.to_string()));
        let expected_next = format!("tala history {thread}");
        assert_eq!(error.next.as_deref(), Some(expected_next.as_str()));
        assert_eq!(
            store
                .db
                .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, u64>(0))
                .unwrap(),
            before
        );

        let mut info_op = send(&recipient.agent, "info");
        if let Operation::Send { request, .. } = &mut info_op {
            *request = false;
        }
        let informational = store.execute(&rpc(info_op, Some(sender.clone()))).unwrap();
        let informational_id = id_of(&informational);
        store
            .execute(&rpc(
                amendment(
                    &recipient.agent,
                    "info-replacement",
                    informational_id,
                    false,
                ),
                Some(sender),
            ))
            .unwrap();
        let mut informational_reply = send("unused", "info-reply");
        if let Operation::Send {
            to,
            reply_to,
            kind,
            request,
            ..
        } = &mut informational_reply
        {
            *to = None;
            *reply_to = Some(informational_id);
            *kind = "reply".into();
            *request = false;
        }
        assert!(store
            .execute(&rpc(informational_reply, Some(recipient)))
            .is_ok());
    }
    #[test]
    fn supersession_preserves_explicit_and_derived_retry_identity() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let sender = register(&mut store, "sender");
        let recipient = register(&mut store, "recipient");
        let first = id_of(
            &store
                .execute(&rpc(send(&recipient.agent, "first"), Some(sender.clone())))
                .unwrap(),
        );
        let second = id_of(
            &store
                .execute(&rpc(send(&recipient.agent, "second"), Some(sender.clone())))
                .unwrap(),
        );

        let first_operation = amendment(&recipient.agent, "stable-amendment", first, false);
        let first_result = store
            .execute(&rpc(first_operation.clone(), Some(sender.clone())))
            .unwrap();
        let first_successor = id_of(&first_result);
        let duplicate = store
            .execute(&rpc(first_operation.clone(), Some(sender.clone())))
            .unwrap();
        assert_eq!(duplicate["duplicate"], true);
        assert_eq!(id_of(&duplicate), first_successor);

        let changed_target = amendment(&recipient.agent, "stable-amendment", second, false);
        assert_eq!(
            store
                .execute(&rpc(changed_target, Some(sender.clone())))
                .unwrap_err()
                .code,
            "IDEMPOTENCY_CONFLICT"
        );

        let third = id_of(
            &store
                .execute(&rpc(send(&recipient.agent, "third"), Some(sender.clone())))
                .unwrap(),
        );
        let mut derived = amendment(&recipient.agent, "auto:amend-derived", third, false);
        if let Operation::Send { key, .. } = &mut derived {
            *key = "auto:amend-derived".into();
        }
        let first_derived = store
            .execute(&rpc(derived.clone(), Some(sender.clone())))
            .unwrap();
        let duplicate_derived = store.execute(&rpc(derived, Some(sender))).unwrap();
        assert_eq!(duplicate_derived["duplicate"], true);
        assert_eq!(id_of(&duplicate_derived), id_of(&first_derived));
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
    fn direct_message_presence_is_saved_and_reused_for_retries() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let sender = register(&mut store, "sender");
        let recipient = register(&mut store, "recipient");

        store
            .db
            .execute(
                "UPDATE agents SET listening_until=? WHERE id=?",
                params![now() + 30, recipient.agent],
            )
            .unwrap();
        let request = send(&recipient.agent, "listening-key");
        let first = store
            .execute(&rpc(request.clone(), Some(sender.clone())))
            .unwrap();
        assert_eq!(first["recipient_presence"]["listening"], true);

        // The recipient stops listening before the retry; a key-based
        // duplicate must retain the observation attached to the first send.
        store
            .db
            .execute(
                "UPDATE agents SET listening_until=0 WHERE id=?",
                [&recipient.agent],
            )
            .unwrap();
        let duplicate = store.execute(&rpc(request, Some(sender.clone()))).unwrap();
        assert_eq!(duplicate["duplicate"], true);
        assert_eq!(duplicate["recipient_presence"]["listening"], true);

        store
            .db
            .execute(
                "UPDATE agents SET active=1,listening_until=? WHERE id=?",
                params![now() - 1, recipient.agent],
            )
            .unwrap();
        let expired = store
            .execute(&rpc(
                send(&recipient.agent, "expired-key"),
                Some(sender.clone()),
            ))
            .unwrap();
        assert_eq!(expired["recipient_presence"]["listening"], false);

        // An inactive row cannot be reported as listening even if a stale
        // lease value remains in storage.
        store
            .db
            .execute(
                "UPDATE agents SET active=0,listening_until=? WHERE id=?",
                params![now() + 30, recipient.agent],
            )
            .unwrap();
        let inactive = store
            .execute(&rpc(
                send(&recipient.agent, "inactive-key"),
                Some(sender.clone()),
            ))
            .unwrap();
        assert_eq!(inactive["recipient_presence"]["listening"], false);

        // A direct reply observes the original sender's listener lease.
        store
            .db
            .execute(
                "UPDATE agents SET active=1,listening_until=? WHERE id=?",
                params![now() + 30, sender.agent],
            )
            .unwrap();
        let mut reply = send("unused", "reply-key");
        if let Operation::Send {
            to,
            reply_to,
            kind,
            request,
            ..
        } = &mut reply
        {
            *to = None;
            *reply_to = Some(first["message"]["id"].as_u64().unwrap());
            *kind = "reply".into();
            *request = false;
        }
        let replied = store
            .execute(&rpc(reply.clone(), Some(recipient.clone())))
            .unwrap();
        assert_eq!(replied["message"]["recipient"], sender.agent);
        assert_eq!(replied["recipient_presence"]["listening"], true);
        store
            .db
            .execute(
                "UPDATE agents SET listening_until=0 WHERE id=?",
                [&sender.agent],
            )
            .unwrap();
        let duplicate_reply = store.execute(&rpc(reply, Some(recipient))).unwrap();
        assert_eq!(duplicate_reply["duplicate"], true);
        assert_eq!(duplicate_reply["recipient_presence"]["listening"], true);

        let mut board_post = send("unused", "board-key");
        if let Operation::Send {
            to,
            project,
            kind,
            request,
            ..
        } = &mut board_post
        {
            *to = None;
            *project = Some("/test/project".into());
            *kind = "post".into();
            *request = false;
        }
        let posted = store.execute(&rpc(board_post, Some(sender))).unwrap();
        assert!(posted.get("recipient_presence").is_none());
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
                purpose: None,
            },
            None,
        );
        for old in [1, 2, 4, 6, 8] {
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

    #[test]
    fn projects_are_owned_listed_and_closed_without_losing_history() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        let owner = register(&mut store, "owner");
        let peer = register(&mut store, "peer");

        let projects = store
            .execute(&rpc(list_projects(false, false), Some(owner.clone())))
            .unwrap();
        assert_eq!(projects["projects"].as_array().unwrap().len(), 1);
        assert_eq!(projects["projects"][0]["owner_id"], owner.agent);

        let sent = store
            .execute(&rpc(
                send(&peer.agent, "project-message"),
                Some(owner.clone()),
            ))
            .unwrap();
        let thread_id = sent["message"]["thread_id"].as_str().unwrap().to_string();
        store
            .db
            .execute(
                "UPDATE projects SET last_activity=? WHERE project=?",
                params![now() - PROJECT_STALE_AFTER - 1, "/test/project"],
            )
            .unwrap();

        let hidden = store
            .execute(&rpc(list_projects(false, false), Some(owner.clone())))
            .unwrap();
        assert_eq!(hidden["projects"], json!([]));
        assert_eq!(hidden["hidden"]["stale"], 1);
        let stale = store
            .execute(&rpc(list_projects(true, false), Some(owner.clone())))
            .unwrap();
        assert_eq!(stale["projects"][0]["stale"], true);

        let err = store
            .execute(&rpc(Operation::CloseProject, Some(peer.clone())))
            .unwrap_err();
        assert_eq!(err.code, "NOT_PROJECT_OWNER");
        let closed = store
            .execute(&rpc(Operation::CloseProject, Some(owner.clone())))
            .unwrap();
        assert_eq!(closed["closed"], true);
        let visible_closed = store
            .execute(&rpc(list_projects(false, true), Some(owner.clone())))
            .unwrap();
        assert_eq!(visible_closed["projects"][0]["closed"], true);
        assert_eq!(visible_closed["projects"][0]["stale"], true);

        store
            .execute(&rpc(
                send(&peer.agent, "project-message-2"),
                Some(owner.clone()),
            ))
            .unwrap();
        let all = store
            .execute(&rpc(list_projects(true, true), Some(owner.clone())))
            .unwrap();
        assert_eq!(all["projects"][0]["closed"], true);
        assert_eq!(all["projects"][0]["stale"], false);

        store
            .db
            .execute(
                "UPDATE projects SET last_activity=? WHERE project=?",
                params![now() - PROJECT_STALE_AFTER - 1, "/test/project"],
            )
            .unwrap();
        store
            .execute(&rpc(Operation::ReopenProject, Some(owner.clone())))
            .unwrap();
        let reopened = store
            .execute(&rpc(list_projects(false, false), Some(owner.clone())))
            .unwrap();
        assert_eq!(reopened["projects"].as_array().unwrap().len(), 1);
        assert_eq!(reopened["projects"][0]["stale"], false);
        let history = store
            .execute(&rpc(
                Operation::History {
                    thread: thread_id,
                    after: 0,
                    limit: 50,
                },
                Some(owner),
            ))
            .unwrap();
        assert_eq!(history["messages"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn migration_backfills_project_owner_and_last_activity() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY, name TEXT NOT NULL, tool TEXT NOT NULL, context TEXT NOT NULL, token TEXT NOT NULL, last_seen INTEGER NOT NULL, active INTEGER NOT NULL DEFAULT 1, listening_until INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT, thread TEXT NOT NULL, sender TEXT NOT NULL REFERENCES agents(id), recipient TEXT REFERENCES agents(id), project TEXT, context TEXT NOT NULL, parts TEXT NOT NULL, kind TEXT NOT NULL, reply_to INTEGER REFERENCES messages(id), expects_reply INTEGER NOT NULL, created_at INTEGER NOT NULL, received_at INTEGER, answered_by INTEGER REFERENCES messages(id), cancelled INTEGER NOT NULL DEFAULT 0, key TEXT NOT NULL, canonical TEXT NOT NULL, UNIQUE(sender,key));
             PRAGMA user_version=1;",
        )
        .unwrap();
        let context = encode(&context()).unwrap();
        db.execute(
            "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES('agt_first','first','test',?,'secret-first',100)",
            [&context],
        )
        .unwrap();
        db.execute(
            "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES('agt_second','second','test',?,'secret-second',200)",
            [&context],
        )
        .unwrap();
        db.execute(
            "INSERT INTO messages(thread,sender,project,context,parts,kind,expects_reply,created_at,key,canonical) VALUES('thr_old','agt_second',NULL,?,'[]','message',0,300,'old','old')",
            [&context],
        )
        .unwrap();
        drop(db);

        let store = Store::open(&path).unwrap();
        let (owner, activity): (String, i64) = store
            .db
            .query_row(
                "SELECT owner_agent,last_activity FROM projects WHERE project='/test/project'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(owner, "agt_first");
        assert_eq!(activity, 300);
        assert_eq!(
            store
                .db
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            store
                .db
                .query_row("SELECT count(*) FROM agents", [], |r| r.get::<_, u64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            store
                .db
                .query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, u64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .db
                .query_row("SELECT supersedes FROM messages WHERE id=1", [], |r| r
                    .get::<_, Option<
                    u64,
                >>(
                    0
                ))
                .unwrap(),
            None
        );
    }

    #[test]
    fn migration_adds_supersedes_to_version_two_database() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY, name TEXT NOT NULL, tool TEXT NOT NULL, context TEXT NOT NULL, token TEXT NOT NULL, last_seen INTEGER NOT NULL, active INTEGER NOT NULL DEFAULT 1, listening_until INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT, thread TEXT NOT NULL, sender TEXT NOT NULL REFERENCES agents(id), recipient TEXT REFERENCES agents(id), project TEXT, context TEXT NOT NULL, parts TEXT NOT NULL, kind TEXT NOT NULL, reply_to INTEGER REFERENCES messages(id), expects_reply INTEGER NOT NULL, created_at INTEGER NOT NULL, received_at INTEGER, answered_by INTEGER REFERENCES messages(id), cancelled INTEGER NOT NULL DEFAULT 0, key TEXT NOT NULL, canonical TEXT NOT NULL, recipient_listening INTEGER, UNIQUE(sender,key));
             PRAGMA user_version=2;",
        )
        .unwrap();
        drop(db);

        let store = Store::open(&path).unwrap();
        assert_eq!(
            store
                .db
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT count(*) FROM pragma_table_info('messages') WHERE name='supersedes'",
                    [],
                    |r| r.get::<_, u64>(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='messages_supersedes_unique'",
                    [],
                    |r| r.get::<_, u64>(0),
                )
                .unwrap(),
            1
        );
    }

    #[test]
    fn migration_adds_purpose_to_version_three_without_changing_existing_rows() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY, name TEXT NOT NULL, tool TEXT NOT NULL, context TEXT NOT NULL, token TEXT NOT NULL, last_seen INTEGER NOT NULL, active INTEGER NOT NULL DEFAULT 1, listening_until INTEGER NOT NULL DEFAULT 0);
             CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT, thread TEXT NOT NULL, sender TEXT NOT NULL REFERENCES agents(id), recipient TEXT REFERENCES agents(id), project TEXT, context TEXT NOT NULL, parts TEXT NOT NULL, kind TEXT NOT NULL, reply_to INTEGER REFERENCES messages(id), expects_reply INTEGER NOT NULL, created_at INTEGER NOT NULL, received_at INTEGER, answered_by INTEGER REFERENCES messages(id), cancelled INTEGER NOT NULL DEFAULT 0, key TEXT NOT NULL, canonical TEXT NOT NULL, recipient_listening INTEGER, supersedes INTEGER REFERENCES messages(id), UNIQUE(sender,key));
             PRAGMA user_version=3;",
        )
        .unwrap();
        db.execute(
            "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES('agt_old','old','fixture','{\"project\":\"/legacy\",\"branch\":null,\"revision\":null}','secret-token',123)",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO messages(thread,sender,project,context,parts,kind,expects_reply,created_at,key,canonical) VALUES('thr_old','agt_old',NULL,'{\"project\":\"/legacy\",\"branch\":null,\"revision\":null}','[{\"type\":\"text\",\"text\":\"history survives\"}]','message',0,456,'old-key','old-canonical')",
            [],
        )
        .unwrap();
        drop(db);

        let mut store = Store::open(&path).unwrap();
        assert_eq!(
            store
                .db
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT name,purpose,purpose_updated_at,last_seen FROM agents WHERE id='agt_old'",
                    [],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?, row.get::<_, Option<i64>>(2)?, row.get::<_, i64>(3)?)),
                )
                .unwrap(),
            ("old".into(), None, None, 123)
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT count(*) FROM messages WHERE thread='thr_old' AND canonical='old-canonical'",
                    [],
                    |row| row.get::<_, u64>(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT count(*) FROM pragma_table_info('agents') WHERE name IN ('purpose','purpose_updated_at')",
                    [],
                    |row| row.get::<_, u64>(0),
                )
                .unwrap(),
            2
        );
        let old_agent = store
            .execute(&rpc(
                Operation::Whoami,
                Some(Credential {
                    agent: "agt_old".into(),
                    token: "secret-token".into(),
                }),
            ))
            .unwrap();
        assert!(old_agent["agent"]["purpose"].is_null());
        assert!(old_agent["agent"]["purpose_updated_at"].is_null());
    }

    #[test]
    fn migration_v4_backfills_thread_summaries_roles_and_attention() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch(
            "CREATE TABLE agents(id TEXT PRIMARY KEY,name TEXT NOT NULL,tool TEXT NOT NULL,context TEXT NOT NULL,token TEXT NOT NULL,last_seen INTEGER NOT NULL,active INTEGER NOT NULL DEFAULT 1,listening_until INTEGER NOT NULL DEFAULT 0,purpose TEXT,purpose_updated_at INTEGER);
             CREATE TABLE messages(id INTEGER PRIMARY KEY AUTOINCREMENT,thread TEXT NOT NULL,sender TEXT NOT NULL REFERENCES agents(id),recipient TEXT REFERENCES agents(id),project TEXT,context TEXT NOT NULL,parts TEXT NOT NULL,kind TEXT NOT NULL,reply_to INTEGER REFERENCES messages(id),expects_reply INTEGER NOT NULL,created_at INTEGER NOT NULL,received_at INTEGER,answered_by INTEGER REFERENCES messages(id),cancelled INTEGER NOT NULL DEFAULT 0,key TEXT NOT NULL,canonical TEXT NOT NULL,recipient_listening INTEGER,supersedes INTEGER REFERENCES messages(id),UNIQUE(sender,key));
             CREATE TABLE projects(project TEXT PRIMARY KEY,owner_agent TEXT NOT NULL REFERENCES agents(id),closed INTEGER NOT NULL DEFAULT 0,last_activity INTEGER NOT NULL);
             CREATE TABLE rpc_results(request_id TEXT PRIMARY KEY,canonical TEXT NOT NULL,response TEXT NOT NULL);
             CREATE TABLE participants(thread TEXT NOT NULL,agent TEXT NOT NULL REFERENCES agents(id),PRIMARY KEY(thread,agent));
             CREATE TABLE board_deliveries(message INTEGER NOT NULL REFERENCES messages(id),agent TEXT NOT NULL REFERENCES agents(id),received_at INTEGER,PRIMARY KEY(message,agent));
             PRAGMA user_version=4;",
        )
        .unwrap();
        for (id, name, token) in [
            ("agt_a", "author", "token-a"),
            ("agt_b", "recipient", "token-b"),
            ("agt_c", "handoff-only", "token-c"),
        ] {
            db.execute(
                "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES(?,?, 'test', '{\"project\":\"proj\"}', ?,100)",
                params![id, name, token],
            )
            .unwrap();
        }
        let parts = |text: &str| json!([{"type":"text","text":text}]).to_string();
        db.execute(
            "INSERT INTO messages(id,thread,sender,recipient,project,context,parts,kind,expects_reply,created_at,key,canonical)
             VALUES(1,'thr_direct','agt_a','agt_b',NULL,'{}',?,'message',1,100,'k1','c1')",
            [parts("direct request")],
        )
        .unwrap();
        db.execute(
            "INSERT INTO messages(id,thread,sender,recipient,project,context,parts,kind,expects_reply,created_at,key,canonical)
             VALUES(2,'thr_direct','agt_a','agt_c',NULL,'{}',?,'handoff',1,101,'k2','c2')",
            [parts("handoff summary")],
        )
        .unwrap();
        db.execute(
            "INSERT INTO messages(id,thread,sender,project,context,parts,kind,expects_reply,created_at,key,canonical)
             VALUES(3,'thr_board','agt_a','proj','{}',?,'post',0,102,'k3','c3')",
            [parts("board post")],
        )
        .unwrap();
        db.execute(
            "INSERT INTO messages(id,thread,sender,project,context,parts,kind,reply_to,expects_reply,created_at,key,canonical)
             VALUES(4,'thr_board','agt_b','proj','{}',?,'reply',3,0,103,'k4','c4')",
            [parts("board reply")],
        )
        .unwrap();
        db.execute(
            "INSERT INTO board_deliveries(message,agent) VALUES(4,'agt_a')",
            [],
        )
        .unwrap();
        drop(db);

        let store = Store::open(&path).unwrap();
        assert_eq!(
            store
                .db
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            5
        );
        let direct: (String, Option<String>, u64, u64, String) = store
            .db
            .query_row(
                "SELECT kind,board_project,latest_message_id,message_count,excerpt FROM thread_summaries WHERE thread='thr_direct'",
                [],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
            )
            .unwrap();
        assert_eq!(
            direct,
            ("direct".into(), None, 2, 2, "handoff summary".into())
        );
        let handoff_access: u64 = store
            .db
            .query_row(
                "SELECT latest_message_id FROM thread_list_access WHERE agent='agt_c' AND thread='thr_direct'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(handoff_access, 2);
        let board_replier_access: u64 = store
            .db
            .query_row(
                "SELECT latest_message_id FROM thread_list_access WHERE agent='agt_b' AND thread='thr_board'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(board_replier_access, 4);
        let board: (String, String, u64, u64) = store
            .db
            .query_row(
                "SELECT kind,board_project,latest_message_id,message_count FROM thread_summaries WHERE thread='thr_board'",
                [],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
            )
            .unwrap();
        assert_eq!(board, ("board".into(), "proj".into(), 4, 2));
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT group_concat(agent,',') FROM (SELECT agent FROM thread_conversation_participants WHERE thread='thr_direct' ORDER BY agent)",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "agt_a,agt_b"
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT agent FROM thread_handoff_grants WHERE thread='thr_direct'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "agt_c"
        );
        assert_eq!(
            store
                .db
                .query_row("SELECT COUNT(*) FROM thread_open_requests", [], |row| row
                    .get::<_, u64>(
                    0
                ))
                .unwrap(),
            4
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT unread_count FROM thread_unread WHERE thread='thr_direct' AND agent='agt_b'",
                    [],
                    |row| row.get::<_, u64>(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT unread_count FROM thread_unread WHERE thread='thr_board' AND agent='agt_a'",
                    [],
                    |row| row.get::<_, u64>(0),
                )
                .unwrap(),
            1
        );
        assert_eq!(
            store
                .db
                .query_row(
                    "SELECT received_at FROM board_deliveries WHERE message=4 AND agent='agt_a'",
                    [],
                    |row| row.get::<_, Option<i64>>(0)
                )
                .unwrap(),
            None
        );
    }

    #[test]
    fn thread_list_large_history_uses_indexed_summaries_and_keyset_page() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(&temp.path().join("db")).unwrap();
        for (id, name, token) in [
            ("agt_list", "list-agent", "list-token"),
            ("agt_peer", "peer", "peer-token"),
        ] {
            store
                .db
                .execute(
                    "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES(?,?,'test','{\"project\":\"/test/project\",\"branch\":null,\"revision\":null}',?,?)",
                    params![id,name,token,now()],
                )
                .unwrap();
        }
        store
            .db
            .execute_batch(
                "WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<1200)
                 INSERT INTO messages(id,thread,sender,recipient,project,context,parts,kind,expects_reply,created_at,key,canonical)
                 SELECT x,printf('thr_%04d',x),'agt_list','agt_peer',NULL,'{}','[{\"type\":\"text\",\"text\":\"history item\"}]','message',0,x,'k'||x,'c'||x FROM n;
                 WITH RECURSIVE n(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM n WHERE x<1200)
                 INSERT INTO thread_summaries(thread,kind,board_project,latest_message_id,latest_message_at,message_count,excerpt)
                 SELECT printf('thr_%04d',x),'direct',NULL,x,x,1,'history item' FROM n;
                 INSERT INTO thread_conversation_participants(thread,agent)
                 SELECT thread,'agt_list' FROM thread_summaries;
                 INSERT INTO thread_conversation_participants(thread,agent)
                 SELECT thread,'agt_peer' FROM thread_summaries;
                 INSERT INTO participants(thread,agent)
                 SELECT thread,'agt_list' FROM thread_summaries;
                 INSERT INTO participants(thread,agent)
                 SELECT thread,'agt_peer' FROM thread_summaries;
                 INSERT INTO thread_list_access(agent,thread,latest_message_id)
                 SELECT p.agent,p.thread,s.latest_message_id
                 FROM thread_conversation_participants p
                 JOIN thread_summaries s ON s.thread=p.thread;",
            )
            .unwrap();

        let plan = store
            .db
            .prepare(&format!("EXPLAIN QUERY PLAN {THREAD_LIST_SUMMARY_QUERY}"))
            .unwrap()
            .query_map(params!["agt_list", now() - STALE_AFTER as i64], |row| {
                row.get::<_, String>(3)
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap()
            .join("\n");
        assert!(plan.contains("thread_list_access_order"), "{plan}");
        assert!(plan.contains("thread_open_by_agent"), "{plan}");
        assert!(!plan.contains("SCAN messages"), "{plan}");
        assert!(!plan.contains("USE TEMP B-TREE FOR ORDER BY"), "{plan}");

        let data = store
            .execute(&rpc(
                Operation::ListThreads {
                    cursor: None,
                    limit: 50,
                },
                Some(Credential {
                    agent: "agt_list".into(),
                    token: "list-token".into(),
                }),
            ))
            .unwrap();
        assert_eq!(data["threads"].as_array().unwrap().len(), 50);
        assert_eq!(data["threads"][0]["last_message_id"], 1200);
        assert_eq!(data["snapshot_high_water_id"], 1200);
        assert_eq!(data["has_more"], true);
        let cursor = data["next_cursor"].as_str().unwrap();
        let second = store
            .execute(&rpc(
                Operation::ListThreads {
                    cursor: Some(cursor.into()),
                    limit: 50,
                },
                Some(Credential {
                    agent: "agt_list".into(),
                    token: "list-token".into(),
                }),
            ))
            .unwrap();
        assert_eq!(second["threads"].as_array().unwrap().len(), 50);
        assert_eq!(second["threads"][0]["last_message_id"], 1150);
        assert_eq!(second["snapshot_high_water_id"], 1200);

        for n in 0..100 {
            store
                .execute(&rpc(
                    Operation::Send {
                        to: Some("agt_peer".into()),
                        project: None,
                        thread: Some("thr_1200".into()),
                        reply_to: None,
                        parts: vec![Part::Text {
                            text: format!("reply {n}"),
                        }],
                        request: false,
                        kind: "message".into(),
                        key: format!("list-benchmark-{n}"),
                        context: context(),
                        supersedes: None,
                    },
                    Some(Credential {
                        agent: "agt_list".into(),
                        token: "list-token".into(),
                    }),
                ))
                .unwrap();
        }
    }

    #[test]
    fn fresh_database_runs_all_migrations_once() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(&temp.path().join("db")).unwrap();
        assert_eq!(
            store
                .db
                .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            5
        );
        for column in ["purpose", "purpose_updated_at"] {
            assert_eq!(
                store
                    .db
                    .query_row(
                        "SELECT count(*) FROM pragma_table_info('agents') WHERE name=?",
                        [column],
                        |row| row.get::<_, u64>(0),
                    )
                    .unwrap(),
                1
            );
        }
        drop(store);
        assert!(Store::open(&temp.path().join("db")).is_ok());
    }
}
