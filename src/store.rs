use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
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

pub struct Store {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    messages: Arc<RwLock<HashMap<String, Vec<Message>>>>,
    broadcast: Arc<RwLock<HashMap<String, broadcast::Sender<DaemonEvent>>>>,
    next_msg_id: Arc<RwLock<HashMap<String, u64>>>,
    global_tx: broadcast::Sender<(String, DaemonEvent)>,
}

impl Store {
    pub fn new() -> Self {
        let (global_tx, _) = broadcast::channel(256);
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            broadcast: Arc::new(RwLock::new(HashMap::new())),
            next_msg_id: Arc::new(RwLock::new(HashMap::new())),
            global_tx,
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
            None
        };

        (id, first_msg_id)
    }

    pub async fn add_message(
        &self,
        session_id: &str,
        sender: &str,
        content: &str,
    ) -> Option<Message> {
        let mut sessions = self.sessions.write().await;
        let session = sessions.get_mut(session_id)?;
        if session.closed {
            return None;
        }
        session.last_activity = Utc::now();
        drop(sessions);

        let mut msg_ids = self.next_msg_id.write().await;
        let current_id = msg_ids.get_mut(session_id)?;
        let msg_id = *current_id;
        *current_id += 1;
        drop(msg_ids);

        let now = Utc::now();
        let msg = Message {
            id: msg_id,
            session_id: session_id.to_string(),
            sender: sender.to_string(),
            content: content.to_string(),
            timestamp: now,
        };

        let mut msgs = self.messages.write().await;
        msgs.entry(session_id.to_string())
            .or_default()
            .push(msg.clone());

        if let Some(tx) = self.broadcast.read().await.get(session_id) {
            let _ = tx.send(DaemonEvent::NewMessage(msg.clone()));
        }
        let _ = self
            .global_tx
            .send((session_id.to_string(), DaemonEvent::NewMessage(msg.clone())));

        Some(msg)
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
            result.into_iter().take(limit).collect()
        } else {
            result
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions = self.sessions.read().await;
        let msgs = self.messages.read().await;
        sessions
            .values()
            .map(|s| SessionSummary {
                id: s.id.clone(),
                name: s.name.clone(),
                created_at: s.created_at,
                closed: s.closed,
                message_count: msgs.get(&s.id).map(|v| v.len()).unwrap_or(0),
            })
            .collect()
    }

    pub async fn rename_session(
        &self,
        session_id: &str,
        name: &str,
        _force: bool,
    ) -> Result<bool, String> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            let old_name = session.name.clone().unwrap_or_default();
            session.name = Some(name.to_string());
            // Persist session name to disk
            let mut names: HashMap<String, String> = HashMap::new();
            for (sid, s) in sessions.iter() {
                if let Some(ref n) = s.name {
                    names.insert(sid.clone(), n.clone());
                }
            }
            drop(sessions);
            let _ = write_sessions_json(&names).await;

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

    pub async fn persist(&self) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        persist_sessions(&sessions).await
    }

    pub async fn load_persisted(&self) {
        let loaded = load_sessions().await;
        if loaded.is_empty() {
            return;
        }
        let mut sessions = self.sessions.write().await;
        let mut broadcast = self.broadcast.write().await;
        let mut msg_ids = self.next_msg_id.write().await;
        for (id, session) in loaded {
            if !sessions.contains_key(&id) {
                sessions.insert(id.clone(), session);
                let (tx, _) = broadcast::channel(32);
                broadcast.insert(id.clone(), tx);
                msg_ids.insert(id.clone(), 1);
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

pub async fn write_sessions_json(names: &HashMap<String, String>) -> anyhow::Result<()> {
    let path = sessions_path();
    let tmp = tala_home().join("sessions.json.tmp");
    let content = serde_json::to_string_pretty(names)?;
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&tmp, &content).await?;
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
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

pub fn local_cursor_path() -> PathBuf {
    PathBuf::from(".tala").join("cursor")
}

pub async fn read_cursor() -> u64 {
    let path = local_cursor_path();
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => content.trim().parse().unwrap_or(0),
        Err(_) => 0,
    }
}

pub async fn write_cursor(cursor: u64) -> anyhow::Result<()> {
    let path = local_cursor_path();
    tokio::fs::create_dir_all(path.parent().unwrap()).await?;
    tokio::fs::write(&path, cursor.to_string()).await?;
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
        assert_eq!(messages[0].content, "hello");
        assert_eq!(messages[1].content, "reply");
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
        assert_eq!(since_1[0].content, "second");

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
        assert_eq!(messages[0].content, "initial message");
    }

    #[test]
    fn test_get_default_sender() {
        let sender = get_default_sender();
        assert!(!sender.is_empty(), "default sender should not be empty");
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
