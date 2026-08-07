use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream;
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::models::*;
use crate::store::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
}

pub fn create_router(store: Arc<Store>) -> Router {
    let state = AppState { store };

    Router::new()
        .route("/api/sessions", post(create_session).get(list_sessions))
        .route("/api/sessions/:id", get(get_session).delete(close_session))
        .route(
            "/api/sessions/:id/messages",
            post(send_message).get(get_messages),
        )
        .route("/api/sessions/:id/wait", get(wait_for_message))
        .route("/api/sessions/:id/recap", get(recap_session))
        .route("/api/sessions/:id/rename", post(rename_session))
        .route("/api/sessions/:id/reopen", post(reopen_session))
        .route("/api/sessions/:id/events", get(stream_events))
        .route("/api/sessions/wait-new", get(wait_new_session))
        .route("/api/sessions/wait-all", get(wait_all))
        .route("/api/observe", get(observe_events))
        .route("/api/agents", get(agents))
        .route("/api/status", get(status))
        .layer(tower_http::cors::CorsLayer::permissive())
        .with_state(state)
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let sender = req.sender.unwrap_or_else(|| "unknown".to_string());
    let initial = req.message.map(|msg| (sender, msg));
    let (id, first_message_id) = state.store.create_session(initial, req.name).await;
    (
        StatusCode::CREATED,
        Json(CreateSessionResponse {
            id,
            first_message_id,
        }),
    )
}

async fn list_sessions(State(state): State<AppState>) -> impl IntoResponse {
    let sessions = state.store.list_sessions().await;
    (StatusCode::OK, Json(sessions))
}

async fn get_session(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    match state.store.get_session(&id).await {
        Some(session) => (
            StatusCode::OK,
            Json(serde_json::to_value(session).unwrap()).into_response(),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("session '{}' not found", id),
            })
            .into_response(),
        ),
    }
}

async fn close_session(State(state): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let exists = state.store.get_session(&id).await;
    match exists {
        Some(s) if s.closed => (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: format!("session '{}' is already closed", id),
            }),
        )
            .into_response(),
        Some(_) => {
            state.store.close_session(&id).await;
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "closed"})),
            )
                .into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("session '{}' not found", id),
            }),
        )
            .into_response(),
    }
}

async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
    if req.content.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "message content cannot be empty".to_string(),
            }),
        )
            .into_response();
    }
    match state
        .store
        .add_message(&id, &req.sender, &req.content)
        .await
    {
        Some(msg) => (
            StatusCode::CREATED,
            Json(SendMessageResponse {
                cursor: Some(msg.id),
                id: msg.id,
                session_id: msg.session_id,
                sender: msg.sender,
                content: msg.content,
                timestamp: msg.timestamp,
            }),
        )
            .into_response(),
        None => {
            let session = state.store.get_session(&id).await;
            match session {
                Some(_) => (
                    StatusCode::CONFLICT,
                    Json(ErrorResponse {
                        error: "session is closed".to_string(),
                    }),
                )
                    .into_response(),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse {
                        error: format!("session '{}' not found", id),
                    }),
                )
                    .into_response(),
            }
        }
    }
}

#[derive(Deserialize)]
struct GetMessagesParams {
    since: Option<u64>,
}

async fn get_messages(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<GetMessagesParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    let messages = state.store.get_messages_since(&id, since).await;
    (StatusCode::OK, Json(messages))
}

#[derive(Deserialize)]
struct WaitParams {
    since: Option<u64>,
    timeout_secs: Option<u64>,
    limit: Option<usize>,
    from: Option<String>,
}

fn compute_cursor(messages: &[Message]) -> Option<u64> {
    messages.iter().map(|m| m.id).max()
}

fn wrap_wait(
    messages: Vec<Message>,
    timeout: bool,
    timeout_after: Option<u64>,
    closed: bool,
) -> WaitResponse {
    let cursor = compute_cursor(&messages);
    WaitResponse {
        messages,
        timeout,
        timeout_after,
        closed,
        cursor,
    }
}

async fn wait_for_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<WaitParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    let wait_timeout = params.timeout_secs.unwrap_or(60);
    let limit = params.limit.filter(|&l| l > 0);
    let from = params.from.as_deref();

    let session = state.store.get_session(&id).await;
    let is_closed = match &session {
        Some(s) => s.closed,
        None => false,
    };

    let existing = state
        .store
        .get_messages_filtered(&id, since, limit, from)
        .await;
    if !existing.is_empty() {
        return (
            StatusCode::OK,
            Json(wrap_wait(existing, false, None, is_closed)),
        )
            .into_response();
    }

    match session {
        Some(s) if s.closed => {
            return (StatusCode::OK, Json(wrap_wait(vec![], false, None, true))).into_response();
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("session '{}' not found", id),
                }),
            )
                .into_response();
        }
        _ => {}
    }

    let mut rx = match state.store.subscribe(&id).await {
        Some(rx) => rx,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "failed to subscribe to session".to_string(),
                }),
            )
                .into_response();
        }
    };

    // Re-check for messages or close that arrived between our initial check and subscribe
    let session = state.store.get_session(&id).await;
    let is_closed = match &session {
        Some(s) => s.closed,
        None => false,
    };
    if is_closed {
        return (StatusCode::OK, Json(wrap_wait(vec![], false, None, true))).into_response();
    }
    let existing = state
        .store
        .get_messages_filtered(&id, since, limit, from)
        .await;
    if !existing.is_empty() {
        return (
            StatusCode::OK,
            Json(wrap_wait(existing, false, None, is_closed)),
        )
            .into_response();
    }

    let timeout_dur = Duration::from_secs(wait_timeout);
    let result = timeout(timeout_dur, async {
        loop {
            match rx.recv().await {
                Ok(DaemonEvent::NewMessage(msg)) => {
                    if msg.id > since {
                        if let Some(sender) = from {
                            if msg.sender != sender {
                                continue;
                            }
                        }
                        let effective_limit = limit.filter(|&l| l > 0);
                        let msgs = if effective_limit.unwrap_or(1) > 1 {
                            state
                                .store
                                .get_messages_filtered(&id, since, limit, from)
                                .await
                        } else {
                            vec![msg]
                        };
                        let session = state.store.get_session(&id).await;
                        let closed = session.map(|s| s.closed).unwrap_or(false);
                        return wrap_wait(msgs, false, None, closed);
                    }
                }
                Ok(DaemonEvent::SessionClosed) => {
                    return wrap_wait(vec![], false, None, true);
                }
                Ok(
                    DaemonEvent::SessionCreated(_)
                    | DaemonEvent::SessionReopened(_)
                    | DaemonEvent::SessionRenamed { .. },
                ) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return wrap_wait(vec![], false, None, true);
                }
            }
        }
    })
    .await;

    match result {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(_elapsed) => {
            let mut resp = wrap_wait(vec![], true, Some(wait_timeout), false);
            resp.cursor = Some(since);
            (StatusCode::OK, Json(resp)).into_response()
        }
    }
}

#[derive(Deserialize)]
struct WaitNewParams {
    timeout_secs: Option<u64>,
    /// The waiting agent's own identity (from .tala/config.json or the default
    /// sender). When present, the wait only returns sessions that carry a
    /// message from a DIFFERENT agent, and ignores the waiter's own creates.
    sender: Option<String>,
    /// The waiter's per-session read cursors (URL-encoded JSON map of
    /// session_id -> last-seen message id, from .tala/cursors.json). Sessions
    /// the waiter has a cursor entry for (created/sent-in/read) are never
    /// "new" to it (B029) and are excluded from the pre-existing scan.
    seen: Option<String>,
}

async fn wait_new_session(
    State(state): State<AppState>,
    Query(params): Query<WaitNewParams>,
) -> impl IntoResponse {
    let timeout_secs = params.timeout_secs.unwrap_or(60);
    let caller = params.sender;
    // B029: the waiter's per-session read state. Sessions with a cursor entry
    // are known to the waiter; only never-seen sessions can satisfy the
    // pre-existing scan.
    let seen: HashMap<String, u64> = params
        .seen
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    // B003: when the waiter identifies itself, first scan for sessions that
    // ALREADY exist with an incoming message from another agent. The event loop
    // below only reacts to future events, so a session created before the wait
    // started would otherwise be missed forever. B029: the scan only returns
    // NEVER-SEEN sessions (no cursor entry), freshest first — stale backlog in
    // sessions the waiter already knows never satisfies `wait --new-session`.
    if let Some(ref me) = caller {
        if let Some((sid, msg)) = find_incoming_session(&state.store, me, &seen).await {
            let mut resp = serde_json::json!({"session_id": sid});
            resp["message"] = serde_json::to_value(&msg).unwrap_or_default();
            return (StatusCode::OK, Json(resp)).into_response();
        }
    }

    let mut rx = state.store.subscribe_global();

    let existing_count = state.store.list_sessions().await.len();

    let timeout_dur = Duration::from_secs(timeout_secs);
    let result = timeout(timeout_dur, async {
        loop {
            match rx.recv().await {
                Ok((_sid, DaemonEvent::SessionCreated(id))) => {
                    // When the waiter identifies itself, a session create alone
                    // must NOT satisfy the wait (B003: it fires on your OWN
                    // `session create`). Wait for an incoming NewMessage from
                    // another agent instead. Without identity, keep legacy
                    // behavior (any new session counts).
                    if caller.is_some() {
                        continue;
                    }
                    let msgs = state.store.get_messages_since(&id, 0).await;
                    let first = msgs.first().cloned();
                    let mut resp = serde_json::json!({"session_id": id});
                    if let Some(msg) = first {
                        resp["message"] = serde_json::to_value(msg).unwrap_or_default();
                    }
                    return resp;
                }
                Ok((_sid, DaemonEvent::NewMessage(msg))) => {
                    if let Some(ref me) = caller {
                        // Only incoming messages from OTHER agents satisfy the
                        // wait; our own sends (including the initial message of
                        // a session we create) are ignored.
                        if msg.sender == *me {
                            continue;
                        }
                        let mut resp = serde_json::json!({"session_id": msg.session_id});
                        resp["message"] = serde_json::to_value(&msg).unwrap_or_default();
                        return resp;
                    }
                    let sessions = state.store.list_sessions().await;
                    if sessions.len() > existing_count {
                        let mut resp = serde_json::json!({"session_id": msg.session_id});
                        resp["message"] = serde_json::to_value(&msg).unwrap_or_default();
                        return resp;
                    }
                }
                Ok((_sid, DaemonEvent::SessionClosed)) => continue,
                Ok((_sid, DaemonEvent::SessionReopened(_))) => continue,
                Ok((_sid, DaemonEvent::SessionRenamed { .. })) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return serde_json::json!({"error": "daemon shutting down"});
                }
            }
        }
    })
    .await;

    match result {
        Ok(json) => (StatusCode::OK, Json(json)).into_response(),
        Err(_elapsed) => (
            StatusCode::OK,
            Json(serde_json::json!({"timeout": true, "timeout_after": timeout_secs})),
        )
            .into_response(),
    }
}

/// Find the session (if any) that the waiter has NEVER seen (no cursor entry)
/// and that already has a message from an agent other than `me`, preferring the
/// most recently active one. Used by `wait_new_session` so a pre-existing
/// session with an incoming question is returned immediately (B003) while
/// stale backlog in sessions the waiter already knows is ignored (B029).
async fn find_incoming_session(
    store: &Arc<Store>,
    me: &str,
    seen: &HashMap<String, u64>,
) -> Option<(String, Message)> {
    let sessions = store.list_sessions().await;
    let mut best: Option<(String, Message)> = None;
    for s in sessions {
        if s.closed {
            continue;
        }
        // B029: any cursor entry (created, sent in, or read) marks the session
        // as known to the waiter — never a candidate for `--new-session`.
        if seen.contains_key(&s.id) {
            continue;
        }
        let msgs = store.get_messages_since(&s.id, 0).await;
        // Freshest incoming message from another agent (the waiter cannot have
        // sent here: sending records a cursor entry, excluding the session).
        let freshest_incoming = msgs.iter().filter(|m| m.sender != me).next_back();
        if let Some(m) = freshest_incoming {
            let is_better = match &best {
                Some((_, bm)) => m.timestamp > bm.timestamp,
                None => true,
            };
            if is_better {
                best = Some((s.id.clone(), m.clone()));
            }
        }
    }
    best
}

async fn wait_all(
    State(state): State<AppState>,
    Query(params): Query<WaitNewParams>,
) -> impl IntoResponse {
    let timeout_secs = params.timeout_secs.unwrap_or(60);
    let mut rx = state.store.subscribe_global();

    let timeout_dur = Duration::from_secs(timeout_secs);
    let result = timeout(timeout_dur, async {
        loop {
            match rx.recv().await {
                Ok((_sid, DaemonEvent::NewMessage(msg))) => {
                    return wrap_wait(vec![msg], false, None, false);
                }
                Ok((
                    _sid,
                    DaemonEvent::SessionCreated(_)
                    | DaemonEvent::SessionReopened(_)
                    | DaemonEvent::SessionRenamed { .. },
                )) => continue,
                Ok((_sid, DaemonEvent::SessionClosed)) => continue,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return wrap_wait(vec![], false, None, true);
                }
            }
        }
    })
    .await;

    match result {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(_elapsed) => (
            StatusCode::OK,
            Json(wrap_wait(vec![], true, Some(timeout_secs), false)),
        )
            .into_response(),
    }
}

async fn recap_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<RecapQuery>,
) -> impl IntoResponse {
    let session = match state.store.get_session(&id).await {
        Some(s) => s,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("session '{}' not found", id),
                }),
            )
                .into_response();
        }
    };

    let since = params.cursor.or(params.since).unwrap_or(0);
    let from = params.from.as_deref();
    let messages = state
        .store
        .get_messages_filtered(&id, since, params.limit, from)
        .await;
    let cursor = compute_cursor(&messages);

    (
        StatusCode::OK,
        Json(RecapResponse {
            session,
            messages,
            cursor,
        }),
    )
        .into_response()
}

async fn rename_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<RenameSessionRequest>,
) -> impl IntoResponse {
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "session name cannot be empty".to_string(),
            }),
        )
            .into_response();
    }
    match state.store.rename_session(&id, &req.name, req.force).await {
        Ok(true) => (
            StatusCode::OK,
            Json(serde_json::json!({"session_id": id, "name": req.name, "status": "renamed"})),
        )
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("session '{}' not found", id),
            }),
        )
            .into_response(),
        Err(msg) => (StatusCode::CONFLICT, Json(ErrorResponse { error: msg })).into_response(),
    }
}

#[derive(Deserialize)]
struct EventsParams {
    since: Option<u64>,
    limit: Option<usize>,
}

async fn reopen_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.store.reopen_session(&id).await {
        (
            StatusCode::OK,
            Json(serde_json::json!({"session_id": id, "status": "reopened"})),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("session '{}' not found", id),
            }),
        )
            .into_response()
    }
}

async fn stream_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<EventsParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);

    let session = state.store.get_session(&id).await;
    match session {
        Some(s) if s.closed => {
            let event: Result<Event, Infallible> =
                Ok(Event::default().data("{\"event\":\"closed\"}"));
            return (StatusCode::OK, Sse::new(stream::iter(vec![event]))).into_response();
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("session '{}' not found", id),
                }),
            )
                .into_response();
        }
        _ => {}
    }

    let rx = match state.store.subscribe(&id).await {
        Some(rx) => rx,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "failed to subscribe".to_string(),
                }),
            )
                .into_response();
        }
    };

    let max_count = params.limit.filter(|&l| l > 0).unwrap_or(usize::MAX);
    let mut count = 0usize;

    let stream = BroadcastStream::new(rx).filter_map(move |result| {
        if count >= max_count {
            return None;
        }
        match result {
            Ok(DaemonEvent::NewMessage(msg)) => {
                if msg.id > since {
                    count += 1;
                    let data = serde_json::to_string(&msg).unwrap_or_default();
                    Some(Ok::<_, Infallible>(
                        Event::default().event("message").data(data),
                    ))
                } else {
                    None
                }
            }
            Ok(DaemonEvent::SessionClosed) => Some(Ok::<_, Infallible>(
                Event::default().event("closed").data("{}"),
            )),
            Ok(
                DaemonEvent::SessionCreated(_)
                | DaemonEvent::SessionReopened(_)
                | DaemonEvent::SessionRenamed { .. },
            ) => None,
            Err(_) => None,
        }
    });

    (StatusCode::OK, Sse::new(stream)).into_response()
}

#[derive(Deserialize)]
struct ObserveParams {
    since: Option<u64>,
    since_map: Option<String>,
    r#match: Option<String>,
    from: Option<String>,
    channel: Option<String>,
    timeout_secs: Option<u64>,
}

async fn observe_events(
    State(state): State<AppState>,
    Query(params): Query<ObserveParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    // Optional per-session since map (URL-encoded JSON) for read-cursor-aware
    // replay: each session is replayed from ITS OWN last-seen id (B014).
    let since_map: HashMap<String, u64> = params
        .since_map
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let since_for = |sid: &str| since_map.get(sid).copied().unwrap_or(since);
    let match_str = params.r#match;
    let from = params.from;
    let channel = params.channel;

    let sessions = state.store.list_sessions().await;

    // First, replay historical messages from all sessions
    let mut history: Vec<Result<Event, Infallible>> = Vec::new();
    for session in &sessions {
        if let Some(ref ch) = channel {
            match session.name {
                Some(ref name) if !name.contains(ch) => continue,
                None => continue,
                _ => {}
            }
        }
        let msgs = state
            .store
            .get_messages_since(&session.id, since_for(&session.id))
            .await;
        for msg in &msgs {
            if let Some(ref f) = from {
                if msg.sender != *f {
                    continue;
                }
            }
            if let Some(ref m) = match_str {
                if !msg.content.contains(m.as_str()) {
                    continue;
                }
            }
            let session_name = session.name.clone();
            let observe = ObserveEvent {
                session_id: session.id.clone(),
                session_name,
                r#type: "message".to_string(),
                message: Some(msg.clone()),
            };
            history.push(Ok(Event::default()
                .event("message")
                .data(serde_json::to_string(&observe).unwrap())));
        }
    }

    let mut rx = state.store.subscribe_global();
    let store_for_task = state.store.clone();

    let (tx, rx_channel) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);

    // Send historical messages first
    for event in history {
        if tx.send(event).await.is_err() {
            return (
                StatusCode::OK,
                Sse::new(tokio_stream::wrappers::ReceiverStream::new(rx_channel)),
            )
                .into_response();
        }
    }

    // Then stream new events
    let timeout_dur = params
        .timeout_secs
        .filter(|&t| t > 0)
        .map(Duration::from_secs);
    tokio::spawn(async move {
        loop {
            let result = if let Some(dur) = timeout_dur {
                match tokio::time::timeout(dur, rx.recv()).await {
                    Ok(result) => result,
                    Err(_) => break, // timeout expired
                }
            } else {
                rx.recv().await
            };

            match result {
                Ok((session_id, event)) => {
                    let session = store_for_task.get_session(&session_id).await;
                    let session_name = session.as_ref().and_then(|s| s.name.clone());

                    if let Some(ref ch) = channel {
                        match session_name {
                            Some(ref name) if !name.contains(ch) => continue,
                            None => continue,
                            _ => {}
                        }
                    }

                    let opt = match event {
                        DaemonEvent::NewMessage(msg) => {
                            let msg_since =
                                since_map.get(&msg.session_id).copied().unwrap_or(since);
                            if msg.id <= msg_since {
                                continue;
                            }
                            if let Some(ref f) = from {
                                if msg.sender != *f {
                                    continue;
                                }
                            }
                            if let Some(ref m) = match_str {
                                if !msg.content.contains(m.as_str()) {
                                    continue;
                                }
                            }
                            let observe = ObserveEvent {
                                session_id,
                                session_name,
                                r#type: "message".to_string(),
                                message: Some(msg),
                            };
                            Some(
                                Event::default()
                                    .event("message")
                                    .data(serde_json::to_string(&observe).unwrap()),
                            )
                        }
                        DaemonEvent::SessionClosed => {
                            let observe = ObserveEvent {
                                session_id,
                                session_name,
                                r#type: "closed".to_string(),
                                message: None,
                            };
                            Some(
                                Event::default()
                                    .event("closed")
                                    .data(serde_json::to_string(&observe).unwrap()),
                            )
                        }
                        DaemonEvent::SessionCreated(id) => {
                            let session = state.store.get_session(&id).await;
                            let name = session.and_then(|s| s.name);
                            let observe = ObserveEvent {
                                session_id: id,
                                session_name: name,
                                r#type: "created".to_string(),
                                message: None,
                            };
                            Some(
                                Event::default()
                                    .event("created")
                                    .data(serde_json::to_string(&observe).unwrap()),
                            )
                        }
                        DaemonEvent::SessionReopened(id) => {
                            let session = state.store.get_session(&id).await;
                            let name = session.and_then(|s| s.name);
                            let observe = ObserveEvent {
                                session_id: id,
                                session_name: name,
                                r#type: "reopened".to_string(),
                                message: None,
                            };
                            Some(
                                Event::default()
                                    .event("reopened")
                                    .data(serde_json::to_string(&observe).unwrap()),
                            )
                        }
                        DaemonEvent::SessionRenamed {
                            id,
                            old_name: _,
                            new_name: _,
                        } => {
                            let session = state.store.get_session(&id).await;
                            let name = session.and_then(|s| s.name);
                            let observe = ObserveEvent {
                                session_id: id,
                                session_name: name,
                                r#type: "renamed".to_string(),
                                message: None,
                            };
                            Some(
                                Event::default()
                                    .event("renamed")
                                    .data(serde_json::to_string(&observe).unwrap()),
                            )
                        }
                    };

                    if let Some(event) = opt {
                        if tx.send(Ok(event)).await.is_err() {
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    use tokio_stream::wrappers::ReceiverStream;
    let stream = ReceiverStream::new(rx_channel);
    (StatusCode::OK, Sse::new(stream)).into_response()
}

async fn agents(State(state): State<AppState>) -> impl IntoResponse {
    use std::collections::BTreeMap;
    let sessions = state.store.list_sessions().await;
    let mut agent_map: BTreeMap<String, (chrono::DateTime<chrono::Utc>, usize)> = BTreeMap::new();

    // Include the local agent as a participant in any open session
    if let Some(local_agent) = crate::store::read_project_config().await {
        for summary in &sessions {
            if !summary.closed {
                agent_map
                    .entry(local_agent.clone())
                    .or_insert_with(|| (summary.created_at, 0));
            }
        }
    }

    for summary in &sessions {
        if summary.closed {
            continue;
        }
        let msgs = state.store.get_messages_since(&summary.id, 0).await;
        for msg in &msgs {
            let entry = agent_map
                .entry(msg.sender.clone())
                .or_insert((msg.timestamp, 0));
            if msg.timestamp > entry.0 {
                entry.0 = msg.timestamp;
            }
            entry.1 += 1;
        }
    }

    let agents: Vec<AgentSummary> = agent_map
        .into_iter()
        .map(|(sender, (last_seen, message_count))| AgentSummary {
            sender,
            last_seen,
            message_count,
        })
        .collect();

    (StatusCode::OK, Json(agents))
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let sessions = state.store.list_sessions().await;
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let response = StatusResponse {
        pid: std::process::id(),
        port: 0,
        uptime_seconds: started_at,
        session_count: sessions.len(),
    };

    (StatusCode::OK, Json(response))
}
