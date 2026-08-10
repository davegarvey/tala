use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Utc;
use rand::Rng;
use tokio::sync::{broadcast, RwLock};

use crate::models::*;

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

fn generate_session_id() -> String {
    let mut rng = rand::thread_rng();
    let id: String = (0..5)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect();
    format!("sess_{}", id)
}

pub fn tala_home() -> PathBuf {
    if let Some(th) = std::env::var_os("TALA_HOME") {
        PathBuf::from(th)
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".tala")
    } else {
        PathBuf::from("/tmp/.tala")
    }
}

pub fn local_config_path() -> PathBuf {
    PathBuf::from(".tala").join("config.json")
}

/// Idempotency index key: (sender, idempotency key).
type DedupKey = (String, String);
/// Idempotency index value: (session_id, message_id).
type DedupLoc = (String, u64);

pub struct Store {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    messages: Arc<RwLock<HashMap<String, Vec<Message>>>>,
    broadcast: Arc<RwLock<HashMap<String, broadcast::Sender<DaemonEvent>>>>,
    next_msg_id: Arc<RwLock<HashMap<String, u64>>>,
    read_state: Arc<RwLock<HashMap<(String, String), u64>>>,
    global_tx: broadcast::Sender<(String, DaemonEvent)>,
    pub wait_registry: Arc<Mutex<WaitRegistry>>,
    /// Idempotency index: key → location. Rebuilt from persisted messages at
    /// load; entries live on the Message itself so dedup survives restarts.
    dedup: Arc<RwLock<HashMap<DedupKey, DedupLoc>>>,
}

/// Parameters for a new message, mirroring the intent metadata fields.
#[derive(Debug, Clone, Default)]
pub struct AddMessageParams {
    pub sender: String,
    pub parts: Vec<Part>,
    pub intent: Intent,
    pub reply_to: Option<u64>,
    pub expect_reply: bool,
    pub waiting_until: Option<chrono::DateTime<chrono::Utc>>,
    pub idempotency_key: Option<String>,
}

/// Outcome of an add-message attempt.
#[derive(Debug)]
pub enum AddMessageResult {
    /// A new message was stored.
    Stored(Message),
    /// A retry with a known idempotency key; the original message is returned
    /// and nothing new was stored or broadcast.
    Duplicate(Message),
    /// The idempotency key was already used by this sender with different
    /// content; the conflicting original is returned.
    KeyConflict(Message),
    /// Session is closed or does not exist.
    Unavailable,
}

/// Tracks active waits so overlapping waits can be surfaced (deadlock visibility).
#[derive(Debug, Default)]
pub struct WaitRegistry {
    waits: Vec<ActiveWait>,
    next_id: u64,
}

/// An active wait entry in the registry.
#[derive(Debug, Clone)]
pub struct ActiveWait {
    pub id: u64,
    pub scope: WaitScope,
    pub identity: String,
    pub since: chrono::DateTime<chrono::Utc>,
    pub deadline: chrono::DateTime<chrono::Utc>,
}

impl WaitRegistry {
    fn prune(&mut self) {
        let now = Utc::now();
        self.waits.retain(|w| w.deadline > now);
    }

    pub fn list(&mut self) -> Vec<ActiveWaitInfo> {
        self.prune();
        let now = Utc::now();
        self.waits
            .iter()
            .map(|w| ActiveWaitInfo {
                identity: w.identity.clone(),
                scope: w.scope.clone(),
                since: w.since,
                deadline: w.deadline,
                remaining_secs: (w.deadline - now).num_seconds().max(0),
            })
            .collect()
    }

    fn unregister(&mut self, id: u64) {
        self.waits.retain(|w| w.id != id);
    }
}

/// Drop guard that removes a wait from the registry, including on disconnect/error.
pub struct WaitGuard {
    pub registry: Arc<Mutex<WaitRegistry>>,
    pub id: u64,
}

impl WaitGuard {
    pub fn new(registry: Arc<Mutex<WaitRegistry>>, id: u64) -> Self {
        Self { registry, id }
    }
}

impl Drop for WaitGuard {
    fn drop(&mut self) {
        if let Ok(mut reg) = self.registry.lock() {
            reg.unregister(self.id);
        }
    }
}

/// Returns true when two wait scopes overlap: same session, or either is "any new session".
/// New-session↔new-session pairs are excluded (normal multi-agent startup pattern).
pub fn scopes_overlap(a: &WaitScope, b: &WaitScope) -> bool {
    match (a, b) {
        (WaitScope::AnyNewSession, WaitScope::AnyNewSession) => false,
        (WaitScope::Session(x), WaitScope::Session(y)) => x == y,
        (WaitScope::AnyNewSession, WaitScope::Session(_))
        | (WaitScope::Session(_), WaitScope::AnyNewSession) => true,
    }
}

impl Store {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(256);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            broadcast: Arc::new(RwLock::new(HashMap::new())),
            next_msg_id: Arc::new(RwLock::new(HashMap::new())),
            read_state: Arc::new(RwLock::new(HashMap::new())),
            global_tx,
            wait_registry: Arc::new(Mutex::new(WaitRegistry::default())),
            dedup: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        initial_message: Option<(String, String)>,
        name: Option<String>,
    ) -> (String, Option<u64>) {
        let id = loop {
            let candidate = generate_session_id();
            let sessions = self.sessions.read().await;
            if !sessions.contains_key(&candidate) {
                break candidate;
            }
        };

        let now = Utc::now();
        let session = Session {
            id: id.clone(),
            name,
            created_at: now,
            last_activity: now,
            closed: false,
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(id.clone(), session);

        let (tx, _) = broadcast::channel(32);
        self.broadcast.write().await.insert(id.clone(), tx);
        self.next_msg_id.write().await.insert(id.clone(), 1);

        let _ = self
            .global_tx
            .send((id.clone(), DaemonEvent::SessionCreated(id.clone())));

        let first_msg_id = if let Some((sender, content)) = initial_message {
            drop(sessions);
            self.add_message(&id, &sender, &content).await.map(|m| m.id)
        } else {
            drop(sessions);
            None
        };

        // Persist the new session immediately so a crash cannot orphan any
        // messages that follow (messages are persisted on send).
        self.persist_state().await;

        (id, first_msg_id)
    }

    pub async fn add_message(
        &self,
        session_id: &str,
        sender: &str,
        content: &str,
    ) -> Option<Message> {
        match self
            .add_message_with(
                session_id,
                AddMessageParams {
                    sender: sender.to_string(),
                    parts: vec![Part::Text {
                        content: content.to_string(),
                    }],
                    ..AddMessageParams::default()
                },
            )
            .await
        {
            AddMessageResult::Stored(m) | AddMessageResult::Duplicate(m) => Some(m),
            AddMessageResult::KeyConflict(_) | AddMessageResult::Unavailable => None,
        }
    }

    pub async fn add_message_with(
        &self,
        session_id: &str,
        params: AddMessageParams,
    ) -> AddMessageResult {
        // Closed/unknown sessions are rejected before any dedup bookkeeping.
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id).map(|s| s.closed);
        match session {
            Some(false) => {
                let s = sessions.get_mut(session_id).unwrap();
                s.last_activity = Utc::now();
            }
            Some(true) => return AddMessageResult::Unavailable,
            None => return AddMessageResult::Unavailable,
        }
        drop(sessions);

        let mut msg_ids = self.next_msg_id.write().await;
        let Some(current_id) = msg_ids.get_mut(session_id) else {
            return AddMessageResult::Unavailable;
        };
        let msg_id = *current_id;
        *current_id += 1;
        drop(msg_ids);

        let now = Utc::now();
        let msg = Message {
            id: msg_id,
            session_id: session_id.to_string(),
            sender: params.sender.clone(),
            parts: params.parts.clone(),
            timestamp: now,
            intent: params.intent,
            reply_to: params.reply_to,
            expect_reply: params.expect_reply,
            waiting_until: params.waiting_until,
            idempotency_key: params.idempotency_key.clone(),
        };

        // Dedup check: the messages + dedup write locks are held together with
        // no awaits in between, so check+insert is atomic against concurrent
        // sends with the same key.
        let mut msgs = self.messages.write().await;
        let mut dedup = self.dedup.write().await;
        if let Some(key) = &params.idempotency_key {
            let lookup = (params.sender.clone(), key.clone());
            if let Some((orig_sid, orig_mid)) = dedup.get(&lookup).cloned() {
                let existing = msgs
                    .get(&orig_sid)
                    .and_then(|v| v.iter().find(|m| m.id == orig_mid))
                    .cloned();
                if let Some(existing) = existing {
                    // Canonical serialization (serde_json map ordering is
                    // deterministic) — same key + same parts is a retry.
                    let same = serde_json::to_string(&existing.parts)
                        .ok()
                        .zip(serde_json::to_string(&msg.parts).ok())
                        .map(|(a, b)| a == b)
                        .unwrap_or(false);
                    if same {
                        return AddMessageResult::Duplicate(existing);
                    }
                    return AddMessageResult::KeyConflict(existing);
                }
                // Stale index entry (message evicted) — fall through and store.
                dedup.remove(&lookup);
            }
        }

        msgs.entry(session_id.to_string())
            .or_default()
            .push(msg.clone());
        if let Some(key) = &params.idempotency_key {
            dedup.insert(
                (params.sender.clone(), key.clone()),
                (msg.session_id.clone(), msg.id),
            );
        }
        drop(dedup);

        if let Some(tx) = self.broadcast.read().await.get(session_id) {
            let _ = tx.send(DaemonEvent::NewMessage(msg.clone()));
        }
        let _ = self
            .global_tx
            .send((session_id.to_string(), DaemonEvent::NewMessage(msg.clone())));

        drop(msgs);

        // Best-effort durability: persist the transcript so a daemon restart
        // or crash does not lose it (B024).
        self.persist_state().await;

        AddMessageResult::Stored(msg)
    }

    pub async fn get_messages_since(&self, session_id: &str, since: u64) -> Vec<Message> {
        let msgs = self.messages.read().await;
        msgs.get(session_id)
            .map(|v| v.iter().filter(|m| m.id > since).cloned().collect())
            .unwrap_or_default()
    }

    pub async fn get_messages_filtered(
        &self,
        session_id: &str,
        since: u64,
        limit: Option<usize>,
        from: Option<&str>,
    ) -> Vec<Message> {
        let msgs = self.messages.read().await;
        let result: Vec<Message> = msgs
            .get(session_id)
            .map(|v| {
                v.iter()
                    .filter(|m| {
                        if m.id <= since {
                            return false;
                        }
                        if let Some(sender) = from {
                            if m.sender != sender {
                                return false;
                            }
                        }
                        true
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if let Some(limit) = limit {
            // Return the NEWEST N matching messages (tail semantics): keep the
            // last `limit` items in ascending-id order (B016).
            let len = result.len();
            result.into_iter().skip(len.saturating_sub(limit)).collect()
        } else {
            result
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// True when any session already carries `name` (B017: names are an
    /// addressing key and must be unique).
    pub async fn session_name_exists(&self, name: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.values().any(|s| s.name.as_deref() == Some(name))
    }

    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().await;
        let msgs = self.messages.read().await;
        let read_state = self.read_state.read().await;
        sessions
            .values()
            .map(|s| {
                let read_by = read_state
                    .iter()
                    .filter(|((sid, _), _)| sid == &s.id)
                    .map(|((_, sender), id)| (sender.clone(), *id))
                    .collect();
                SessionSummary {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    created_at: s.created_at,
                    closed: s.closed,
                    message_count: msgs.get(&s.id).map(|v| v.len()).unwrap_or(0),
                    read_by,
                }
            })
            .collect()
    }

    /// Record that `sender` has read messages up to (and including) `up_to`
    /// in `session_id`. Monotonic: a lower value never overwrites a higher one
    /// (B021 — sender read receipts).
    pub async fn record_read(&self, session_id: &str, sender: &str, up_to: u64) {
        let mut read_state = self.read_state.write().await;
        let entry = read_state
            .entry((session_id.to_string(), sender.to_string()))
            .or_insert(0);
        if up_to > *entry {
            *entry = up_to;
        }
    }

    pub async fn rename_session(
        &self,
        session_id: &str,
        name: &str,
        _force: bool,
    ) -> Result<bool, String> {
        let mut sessions = self.sessions.write().await;
        // B017: reject collisions — another session already owns this name.
        // Renaming a session to its OWN current name stays a no-op success.
        let collision = sessions
            .iter()
            .any(|(sid, s)| *sid != session_id && s.name.as_deref() == Some(name));
        if collision {
            return Err(format!("A session named '{}' already exists", name));
        }
        if let Some(session) = sessions.get_mut(session_id) {
            let old_name = session.name.clone().unwrap_or_default();
            session.name = Some(name.to_string());
            // Persist session name to disk (full sessions map — the legacy
            // name-only format could not be parsed back by load_sessions).
            drop(sessions);
            self.persist_state().await;

            let sid = session_id.to_string();
            let event = DaemonEvent::SessionRenamed {
                id: sid.clone(),
                old_name: old_name.clone(),
                new_name: name.to_string(),
            };
            if let Some(tx) = self.broadcast.read().await.get(&sid) {
                let _ = tx.send(event);
            }
            let _ = self.global_tx.send((
                sid.clone(),
                DaemonEvent::SessionRenamed {
                    id: sid,
                    old_name,
                    new_name: name.to_string(),
                },
            ));

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn close_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if session.closed {
                return false;
            }
            session.closed = true;
            session.last_activity = Utc::now();

            let sid = session_id.to_string();
            if let Some(tx) = self.broadcast.read().await.get(&sid) {
                let _ = tx.send(DaemonEvent::SessionClosed);
            }
            let _ = self.global_tx.send((sid, DaemonEvent::SessionClosed));
            true
        } else {
            false
        }
    }

    pub async fn reopen_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if !session.closed {
                return true;
            }
            session.closed = false;
            session.last_activity = Utc::now();

            let sid = session_id.to_string();
            let event = DaemonEvent::SessionReopened(sid.clone());
            if let Some(tx) = self.broadcast.read().await.get(&sid) {
                let _ = tx.send(event);
            }
            let _ = self
                .global_tx
                .send((sid, DaemonEvent::SessionReopened(session_id.to_string())));
            true
        } else {
            false
        }
    }

    pub async fn has_recent_activity(&self, max_idle: Duration) -> bool {
        let sessions = self.sessions.read().await;
        let now = Utc::now();
        sessions.values().any(|s| {
            let elapsed = now - s.last_activity;
            elapsed.num_seconds() as u64 <= max_idle.as_secs()
        })
    }

    pub async fn subscribe(&self, session_id: &str) -> Option<broadcast::Receiver<DaemonEvent>> {
        self.broadcast
            .read()
            .await
            .get(session_id)
            .map(|tx| tx.subscribe())
    }

    pub fn subscribe_global(&self) -> broadcast::Receiver<(String, DaemonEvent)> {
        self.global_tx.subscribe()
    }

    /// Registers an active wait and returns its id, overlaps found at registration,
    /// and a guard that deregisters on drop. Prunes expired entries first.
    pub fn register_wait(
        &self,
        scope: WaitScope,
        identity: String,
        timeout_secs: u64,
    ) -> (u64, Vec<WaitOverlap>) {
        let mut reg = match self.wait_registry.lock() {
            Ok(r) => r,
            Err(_) => return (0, vec![]),
        };
        reg.prune();
        let now = Utc::now();
        let id = reg.next_id;
        reg.next_id += 1;
        let overlaps: Vec<WaitOverlap> = reg
            .waits
            .iter()
            .filter(|w| scopes_overlap(&w.scope, &scope))
            .map(|w| WaitOverlap {
                identity: w.identity.clone(),
                scope: w.scope.clone(),
                remaining_secs: (w.deadline - now).num_seconds().max(0),
            })
            .collect();
        reg.waits.push(ActiveWait {
            id,
            scope,
            identity,
            since: now,
            deadline: now + chrono::Duration::seconds(timeout_secs as i64),
        });
        (id, overlaps)
    }

    /// Lists active waits (pruned), for status surfacing.
    pub fn list_active_waits(&self) -> Vec<ActiveWaitInfo> {
        match self.wait_registry.lock() {
            Ok(mut reg) => reg.list(),
            Err(_) => vec![],
        }
    }

    /// Derives which message ids in a session are answered: by explicit `reply_to`,
    /// by an uncorrelated `reply` answering the oldest open `req`, or closed by the
    /// sender's `out`. Returns (answered, closed) id sets; `closed` ids are reqs
    /// that were explicitly abandoned via `out` and must not surface as pending.
    pub fn derive_answered(messages: &[Message]) -> (HashSet<u64>, HashSet<u64>) {
        let mut answered = HashSet::new();
        let mut closed = HashSet::new();
        let mut open: Vec<(u64, String)> = Vec::new();
        for m in messages {
            if m.intent == Intent::Req {
                open.push((m.id, m.sender.clone()));
            }
            if let Some(t) = m.reply_to {
                answered.insert(t);
                open.retain(|(id, _)| *id != t);
            }
            if m.intent == Intent::Reply && m.reply_to.is_none() {
                if let Some((rid, _)) = open.first().cloned() {
                    open.remove(0);
                    answered.insert(rid);
                }
            }
            if m.intent == Intent::Out {
                let sender = m.sender.clone();
                open.retain(|(id, s)| {
                    if *s == sender {
                        closed.insert(*id);
                        false
                    } else {
                        true
                    }
                });
            }
        }
        (answered, closed)
    }

    /// Returns all open obligations across open sessions: unanswered `req` messages
    /// and messages sent with `expect_reply`, excluding closed sessions.
    pub async fn pending_obligations(&self) -> Vec<PendingObligation> {
        let sessions = self.sessions.read().await;
        let msgs = self.messages.read().await;
        let now = Utc::now();
        let mut out = Vec::new();
        for (sid, session) in sessions.iter() {
            if session.closed {
                continue;
            }
            let Some(smsgs) = msgs.get(sid) else {
                continue;
            };
            let (answered, closed) = Self::derive_answered(smsgs);
            for m in smsgs {
                let is_obligation = match m.intent {
                    Intent::Req => !answered.contains(&m.id) && !closed.contains(&m.id),
                    Intent::Reply | Intent::Fyi => m.expect_reply && !answered.contains(&m.id),
                    Intent::Out => false,
                };
                if is_obligation {
                    out.push(PendingObligation {
                        session_id: sid.clone(),
                        session_name: session.name.clone(),
                        message_id: m.id,
                        sender: m.sender.clone(),
                        content: m.snippet(),
                        content_full: m
                            .parts
                            .iter()
                            .find(|p| matches!(p, Part::Text { .. }))
                            .and_then(|p| match p {
                                Part::Text { content } => Some(content.clone()),
                                _ => None,
                            }),
                        elapsed_seconds: (now - m.timestamp).num_seconds(),
                        intent: m.intent,
                        waiting_until: m.waiting_until,
                    });
                }
            }
        }
        out
    }

    /// Broadcasts a WaitUpdate event (new overlapping waiter registered) to the
    /// global channel and every session channel; wait handlers filter by relevance.
    pub async fn broadcast_wait_update(&self, identity: String, scope: WaitScope) {
        let event = DaemonEvent::WaitUpdate { identity, scope };
        let sessions = self.broadcast.read().await;
        for tx in sessions.values() {
            let _ = tx.send(event.clone());
        }
        let _ = self.global_tx.send((String::new(), event));
    }

    /// Whether `candidate` answers `target` in `session_id`: the target becomes
    /// answered only after processing the candidate (per derive_answered).
    pub async fn message_answers(
        &self,
        session_id: &str,
        candidate: &Message,
        target: u64,
    ) -> bool {
        let msgs = self.get_messages_since(session_id, 0).await;
        let without: Vec<Message> = msgs
            .iter()
            .filter(|m| m.id != candidate.id)
            .cloned()
            .collect();
        let (before, _) = Self::derive_answered(&without);
        if before.contains(&target) {
            return false;
        }
        let (after, _) = Self::derive_answered(&msgs);
        after.contains(&target)
    }

    pub async fn persist(&self) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await.clone();
        let messages = self.messages.read().await.clone();
        let next_msg_id = self.next_msg_id.read().await.clone();
        persist_sessions(&sessions).await?;
        persist_messages(&messages, &next_msg_id).await
    }

    /// Best-effort persistence of the full daemon state. Locks are taken one
    /// at a time (and released before the next) to avoid lock-ordering
    /// deadlocks with writers that hold multiple locks (e.g. create_session).
    async fn persist_state(&self) {
        let sessions = { self.sessions.read().await.clone() };
        let messages = { self.messages.read().await.clone() };
        let next_msg_id = { self.next_msg_id.read().await.clone() };
        let _ = persist_sessions(&sessions).await;
        let _ = persist_messages(&messages, &next_msg_id).await;
    }

    pub async fn load_persisted(&self) {
        let loaded = load_sessions().await;
        let (messages, next_ids) = load_messages().await;

        let mut sessions = self.sessions.write().await;
        let mut broadcast = self.broadcast.write().await;
        let mut msg_ids = self.next_msg_id.write().await;
        let mut msgs = self.messages.write().await;
        for (id, session) in loaded {
            if !sessions.contains_key(&id) {
                sessions.insert(id.clone(), session);
                let (tx, _) = broadcast::channel(32);
                broadcast.insert(id.clone(), tx);
            }
        }

        // Restore per-session next ids first (stored values are authoritative).
        for (sid, next) in next_ids {
            if sessions.contains_key(&sid) {
                msg_ids.insert(sid, next);
            }
        }
        // Then restore transcripts; derive next ids for sessions that lacked one.
        for (sid, list) in messages {
            if sessions.contains_key(&sid) {
                msgs.entry(sid.clone()).or_insert_with(|| list.clone());
                msg_ids
                    .entry(sid)
                    .or_insert_with(|| list.iter().map(|m| m.id).max().unwrap_or(0) + 1);
            }
        }
        // Sessions with no messages and no stored next id start at 1 (a missing
        // entry makes add_message return None and the API misreports it as
        // "session is closed").
        let session_ids: Vec<String> = sessions.keys().cloned().collect();
        for sid in session_ids {
            msg_ids.entry(sid).or_insert(1);
        }
        drop(msg_ids);

        // Rebuild the idempotency index from persisted messages so dedup
        // survives daemon restarts.
        let mut dedup = self.dedup.write().await;
        for (sid, list) in msgs.iter() {
            for m in list {
                if let Some(key) = &m.idempotency_key {
                    dedup.insert((m.sender.clone(), key.clone()), (sid.clone(), m.id));
                }
            }
        }
    }
}

pub async fn read_daemon_json() -> anyhow::Result<DaemonInfo> {
    let path = tala_home().join("daemon.json");
    let content = tokio::fs::read_to_string(&path).await?;
    let info: DaemonInfo = serde_json::from_str(&content)?;
    Ok(info)
}

pub async fn write_daemon_json(port: u16) -> anyhow::Result<()> {
    let home = tala_home();
    tokio::fs::create_dir_all(&home).await?;

    let info = DaemonInfo {
        pid: std::process::id(),
        port,
        host: "127.0.0.1".to_string(),
        started_at: chrono::Utc::now(),
        protocol_version: crate::models::PROTOCOL_VERSION,
    };

    let path = home.join("daemon.json");
    let tmp = home.join("daemon.json.tmp");
    let content = serde_json::to_string_pretty(&info)?;
    tokio::fs::write(&tmp, &content).await?;
    tokio::fs::rename(&tmp, &path).await?;

    Ok(())
}

pub async fn remove_daemon_json() {
    let path = tala_home().join("daemon.json");
    let _ = tokio::fs::remove_file(&path).await;
}

fn sessions_path() -> PathBuf {
    tala_home().join("sessions.json")
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SessionsFile {
    sessions: HashMap<String, Session>,
}

pub async fn persist_sessions(sessions: &HashMap<String, Session>) -> anyhow::Result<()> {
    let path = sessions_path();
    let tmp = tala_home().join("sessions.json.tmp");
    let data = SessionsFile {
        sessions: sessions.clone(),
    };
    let content = serde_json::to_string_pretty(&data)?;
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&tmp, &content).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

pub async fn load_sessions() -> HashMap<String, Session> {
    let path = sessions_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str::<SessionsFile>(&content) {
            Ok(data) => data.sessions,
            Err(_) => HashMap::new(),
        },
        Err(_) => HashMap::new(),
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct MessagesFile {
    messages: HashMap<String, Vec<Message>>,
    next_msg_id: HashMap<String, u64>,
}

fn messages_path() -> PathBuf {
    tala_home().join("messages.json")
}

pub async fn persist_messages(
    messages: &HashMap<String, Vec<Message>>,
    next_msg_id: &HashMap<String, u64>,
) -> anyhow::Result<()> {
    let path = messages_path();
    let tmp = tala_home().join("messages.json.tmp");
    let data = MessagesFile {
        messages: messages.clone(),
        next_msg_id: next_msg_id.clone(),
    };
    let content = serde_json::to_string_pretty(&data)?;
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&tmp, &content).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
}

pub async fn load_messages() -> (HashMap<String, Vec<Message>>, HashMap<String, u64>) {
    let path = messages_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => match serde_json::from_str::<MessagesFile>(&content) {
            Ok(data) => (data.messages, data.next_msg_id),
            Err(_) => (HashMap::new(), HashMap::new()),
        },
        Err(_) => (HashMap::new(), HashMap::new()),
    }
}

pub fn local_active_session_path() -> PathBuf {
    PathBuf::from(".tala").join("active-session")
}

pub async fn read_active_session() -> Option<String> {
    let path = local_active_session_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => {
            let s = content.trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }
        Err(_) => None,
    }
}

pub async fn write_active_session(session_id: &str) -> anyhow::Result<()> {
    let path = local_active_session_path();
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&path, session_id).await?;
    Ok(())
}

pub async fn clear_active_session() -> anyhow::Result<()> {
    let path = local_active_session_path();
    if path.exists() {
        tokio::fs::remove_file(&path).await?;
    }
    Ok(())
}

/// Per-session read cursors: map of session_id -> last-seen message id.
///
/// Message ids are PER-SESSION (every session's ids start at 1), so a single
/// global cursor compared against per-session ids is unsound (backlog B014,
/// B023, B025). The legacy `.tala/cursor` single-value file is ignored;
/// `.tala/cursors.json` holds the per-session map.
pub fn cursors_path() -> PathBuf {
    PathBuf::from(".tala").join("cursors.json")
}

pub async fn read_cursors() -> HashMap<String, u64> {
    let path = cursors_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => HashMap::new(),
    }
}

pub async fn read_cursor(session_id: &str) -> u64 {
    read_cursors().await.get(session_id).copied().unwrap_or(0)
}

pub async fn write_cursor(session_id: &str, cursor: u64) -> anyhow::Result<()> {
    let mut cursors = read_cursors().await;
    cursors.insert(session_id.to_string(), cursor);
    let path = cursors_path();
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    let content = serde_json::to_string(&cursors)?;
    tokio::fs::write(&path, content).await?;
    Ok(())
}

pub async fn read_project_config() -> Option<String> {
    let path = local_config_path();
    let content = tokio::fs::read_to_string(&path).await.ok()?;
    let config: serde_json::Value = serde_json::from_str(&content).ok()?;
    config["name"].as_str().map(|s| s.to_string())
}

pub fn get_default_sender() -> String {
    if let Ok(dir) = std::env::current_dir() {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    }
}

pub fn get_sender_name(override_name: Option<&str>) -> String {
    if let Some(name) = override_name {
        return name.to_string();
    }
    tokio::runtime::Handle::try_current()
        .ok()
        .and_then(|_| {
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(read_project_config())
            })
        })
        .unwrap_or_else(get_default_sender)
}

pub async fn read_user_config() -> serde_json::Value {
    let path = tala_home().join("config.json");
    tokio::fs::read_to_string(&path)
        .await
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "default_timeout": 60,
                "idle_timeout": 86400,
                "default_host": "127.0.0.1"
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_create_session() {
        let store = Store::new();
        let (id, _) = store.create_session(None, None).await;
        assert!(
            id.starts_with("sess_"),
            "session ID should start with sess_"
        );

        let session = store.get_session(&id).await;
        assert!(session.is_some());
        assert_eq!(session.unwrap().id, id);
    }

    #[tokio::test]
    async fn test_store_add_and_retrieve_messages() {
        let store = Store::new();
        let (id, _) = store.create_session(None, None).await;

        let msg = store.add_message(&id, "agent-a", "hello").await;
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, 1);

        let msg = store.add_message(&id, "agent-b", "reply").await;
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().id, 2);

        let messages = store.get_messages_since(&id, 0).await;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text_content(), "hello");
        assert_eq!(messages[1].text_content(), "reply");
    }

    #[tokio::test]
    async fn test_store_messages_since() {
        let store = Store::new();
        let (id, _) = store.create_session(None, None).await;

        store.add_message(&id, "a", "first").await;
        store.add_message(&id, "b", "second").await;
        store.add_message(&id, "a", "third").await;

        let since_0 = store.get_messages_since(&id, 0).await;
        assert_eq!(since_0.len(), 3);

        let since_1 = store.get_messages_since(&id, 1).await;
        assert_eq!(since_1.len(), 2);
        assert_eq!(since_1[0].text_content(), "second");

        let since_3 = store.get_messages_since(&id, 3).await;
        assert!(since_3.is_empty());
    }

    #[tokio::test]
    async fn test_store_close_session() {
        let store = Store::new();
        let (id, _) = store.create_session(None, None).await;

        assert!(store.close_session(&id).await);
        assert!(!store.close_session(&id).await);

        let session = store.get_session(&id).await.unwrap();
        assert!(session.closed);

        let msg = store.add_message(&id, "a", "after close").await;
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn test_store_list_sessions() {
        let store = Store::new();
        assert!(store.list_sessions().await.is_empty());

        store.create_session(None, None).await;
        store.create_session(None, None).await;

        let sessions = store.list_sessions().await;
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_store_create_with_initial_message() {
        let store = Store::new();
        let (id, first_msg_id) = store
            .create_session(Some(("init-agent".into(), "initial message".into())), None)
            .await;
        assert_eq!(first_msg_id, Some(1), "first message should have ID 1");

        let messages = store.get_messages_since(&id, 0).await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].sender, "init-agent");
        assert_eq!(messages[0].text_content(), "initial message");
    }

    #[test]
    fn test_get_default_sender() {
        let sender = get_default_sender();
        assert!(!sender.is_empty(), "default sender should not be empty");
    }

    fn msg(id: u64, sender: &str, intent: Intent, reply_to: Option<u64>) -> Message {
        Message {
            id,
            session_id: "s".into(),
            sender: sender.into(),
            parts: vec![Part::Text {
                content: format!("m{}", id),
            }],
            timestamp: Utc::now(),
            intent,
            reply_to,
            expect_reply: false,
            waiting_until: None,
            idempotency_key: None,
        }
    }

    #[test]
    fn test_derive_answered_correlated() {
        let msgs = vec![
            msg(1, "alpha", Intent::Req, None),
            msg(2, "beta", Intent::Reply, Some(1)),
        ];
        let (answered, closed) = Store::derive_answered(&msgs);
        assert!(answered.contains(&1));
        assert!(closed.is_empty());
    }

    #[test]
    fn test_derive_answered_uncorrelated_reply_takes_oldest() {
        let msgs = vec![
            msg(1, "alpha", Intent::Req, None),
            msg(2, "alpha", Intent::Req, None),
            msg(3, "beta", Intent::Reply, None),
        ];
        let (answered, _) = Store::derive_answered(&msgs);
        assert!(answered.contains(&1));
        assert!(
            !answered.contains(&2),
            "oldest req answered, not the newest"
        );
    }

    #[test]
    fn test_derive_answered_out_closes_senders_requests() {
        let msgs = vec![
            msg(1, "alpha", Intent::Req, None),
            msg(2, "beta", Intent::Req, None),
            msg(3, "alpha", Intent::Out, None),
        ];
        let (_answered, closed) = Store::derive_answered(&msgs);
        assert!(closed.contains(&1), "alpha's out closes alpha's req");
        assert!(!closed.contains(&2), "beta's req untouched by alpha's out");
    }

    #[test]
    fn test_scopes_overlap_rules() {
        let a = WaitScope::Session("x".into());
        let b = WaitScope::Session("x".into());
        let c = WaitScope::Session("y".into());
        let any = WaitScope::AnyNewSession;
        assert!(scopes_overlap(&a, &b));
        assert!(!scopes_overlap(&a, &c));
        assert!(scopes_overlap(&any, &a));
        assert!(scopes_overlap(&a, &any));
        assert!(!scopes_overlap(&any, &any), "new-session pairs suppressed");
    }

    #[tokio::test]
    async fn test_wait_registry_registers_prunes_and_guards() {
        let store = Store::new();
        let (id, overlaps) =
            store.register_wait(WaitScope::Session("sess_a".into()), "alpha".into(), 60);
        assert!(overlaps.is_empty());
        assert_eq!(store.list_active_waits().len(), 1);

        let (_, overlaps2) =
            store.register_wait(WaitScope::Session("sess_a".into()), "beta".into(), 60);
        assert_eq!(overlaps2.len(), 1);
        assert_eq!(overlaps2[0].identity, "alpha");

        let (_, overlaps3) =
            store.register_wait(WaitScope::Session("sess_b".into()), "gamma".into(), 60);
        assert!(overlaps3.is_empty(), "different session: no overlap");

        let _guard = WaitGuard::new(Arc::clone(&store.wait_registry), id);
        drop(_guard);
        assert_eq!(store.list_active_waits().len(), 2);

        // Expired entries are pruned on the next registration
        {
            let mut reg = store.wait_registry.lock().unwrap();
            for w in reg.waits.iter_mut() {
                w.deadline = Utc::now() - chrono::Duration::seconds(1);
            }
        }
        let _ = store.register_wait(WaitScope::AnyNewSession, "delta".into(), 60);
        assert_eq!(store.list_active_waits().len(), 1);
    }

    #[tokio::test]
    async fn test_pending_obligations() {
        let store = Store::new();
        let (sid, _) = store.create_session(None, None).await;

        store
            .add_message_with(
                &sid,
                AddMessageParams {
                    sender: "alpha".into(),
                    parts: vec![Part::Text {
                        content: "help".into(),
                    }],
                    intent: Intent::Req,
                    ..AddMessageParams::default()
                },
            )
            .await;
        store
            .add_message_with(
                &sid,
                AddMessageParams {
                    sender: "alpha".into(),
                    parts: vec![Part::Text {
                        content: "answered req".into(),
                    }],
                    intent: Intent::Req,
                    ..AddMessageParams::default()
                },
            )
            .await;
        store
            .add_message_with(
                &sid,
                AddMessageParams {
                    sender: "beta".into(),
                    parts: vec![Part::Text {
                        content: "fixed".into(),
                    }],
                    intent: Intent::Reply,
                    reply_to: Some(2),
                    ..AddMessageParams::default()
                },
            )
            .await;
        store
            .add_message_with(
                &sid,
                AddMessageParams {
                    sender: "beta".into(),
                    parts: vec![Part::Text {
                        content: "expecting more".into(),
                    }],
                    intent: Intent::Fyi,
                    expect_reply: true,
                    ..AddMessageParams::default()
                },
            )
            .await;

        let pending = store.pending_obligations().await;
        assert_eq!(pending.len(), 2, "req 1 + expect_reply msg 4");
        let ids: Vec<u64> = pending.iter().map(|p| p.message_id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&4));

        store.close_session(&sid).await;
        let pending = store.pending_obligations().await;
        assert!(pending.is_empty(), "closed sessions excluded");
    }

    #[tokio::test]
    async fn test_idempotency_dedup_and_conflict() {
        let store = Store::new();
        let (sid, _) = store.create_session(None, None).await;

        let params = || AddMessageParams {
            sender: "alpha".into(),
            parts: vec![Part::Text {
                content: "retry me".into(),
            }],
            idempotency_key: Some("k1".into()),
            ..AddMessageParams::default()
        };

        let first = store.add_message_with(&sid, params()).await;
        let AddMessageResult::Stored(first) = first else {
            panic!("first send must store");
        };

        // Retry with the same key and content: deduplicated, nothing stored.
        let dup = store.add_message_with(&sid, params()).await;
        match dup {
            AddMessageResult::Duplicate(m) => assert_eq!(m.id, first.id),
            other => panic!("expected Duplicate, got {:?}", other),
        }
        assert_eq!(store.get_messages_since(&sid, 0).await.len(), 1);

        // Same key, different content: conflict.
        let conflict = store
            .add_message_with(
                &sid,
                AddMessageParams {
                    parts: vec![Part::Text {
                        content: "different".into(),
                    }],
                    ..params()
                },
            )
            .await;
        assert!(matches!(conflict, AddMessageResult::KeyConflict(_)));
        assert_eq!(store.get_messages_since(&sid, 0).await.len(), 1);
    }

    #[test]
    fn test_tala_home() {
        let home = tala_home();
        assert!(home.ends_with(".tala"), "tala home should end with .tala");
    }

    #[tokio::test]
    async fn test_read_user_config_defaults() {
        let config = read_user_config().await;
        assert_eq!(config["default_timeout"], 60);
        assert_eq!(config["idle_timeout"], 86400);
        assert_eq!(config["default_host"], "127.0.0.1");
    }
}
