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
        .route("/api/sessions/:id/events", get(stream_events))
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
    let id = state.store.create_session(initial).await;
    (StatusCode::CREATED, Json(CreateSessionResponse { id }))
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
    if state.store.close_session(&id).await {
        (
            StatusCode::OK,
            Json(serde_json::json!({"status": "closed"})),
        )
            .into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("session '{}' not found or already closed", id),
            }),
        )
            .into_response()
    }
}

async fn send_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl IntoResponse {
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

fn wrap_wait(messages: Vec<Message>, timeout: bool, timeout_after: Option<u64>, closed: bool) -> WaitResponse {
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
    let wait_timeout = params.timeout_secs.unwrap_or(300);
    let limit = params.limit;
    let from = params.from.as_deref();

    let existing = state
        .store
        .get_messages_filtered(&id, since, limit, from)
        .await;
    if !existing.is_empty() {
        return (StatusCode::OK, Json(wrap_wait(existing, false, None, false))).into_response();
    }

    let session = state.store.get_session(&id).await;
    match session {
        Some(s) if s.closed => {
            return (
                StatusCode::OK,
                Json(wrap_wait(vec![], false, None, true)),
            )
                .into_response();
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
                        let msgs = if limit.unwrap_or(1) > 1 {
                            state
                                .store
                                .get_messages_filtered(&id, since, limit, from)
                                .await
                        } else {
                            vec![msg]
                        };
                        return wrap_wait(msgs, false, None, false);
                    }
                }
                Ok(DaemonEvent::SessionClosed) => {
                    return wrap_wait(vec![], false, None, true);
                }
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
            Json(wrap_wait(vec![], true, Some(wait_timeout), false)),
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
    if state.store.rename_session(&id, &req.name).await {
        (
            StatusCode::OK,
            Json(serde_json::json!({"session_id": id, "name": req.name, "status": "renamed"})),
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

#[derive(Deserialize)]
struct EventsParams {
    since: Option<u64>,
    limit: Option<usize>,
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
            let event: Result<Event, Infallible> = Ok(Event::default().data("{\"event\":\"closed\"}"));
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

    let max_count = params.limit.unwrap_or(usize::MAX);
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
                    Some(Ok::<_, Infallible>(Event::default().event("message").data(data)))
                } else {
                    None
                }
            }
            Ok(DaemonEvent::SessionClosed) => {
                Some(Ok::<_, Infallible>(Event::default().event("closed").data("{}")))
            }
            Err(_) => None,
        }
    });

    (StatusCode::OK, Sse::new(stream)).into_response()
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
