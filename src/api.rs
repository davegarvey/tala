use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio::time::timeout;

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
        .route("/api/sessions/:id/messages", post(send_message).get(get_messages))
        .route("/api/sessions/:id/wait", get(wait_for_message))
        .route("/api/sessions/:id/recap", get(recap_session))
        .route("/api/status", get(status))
        .layer(
            tower_http::cors::CorsLayer::permissive(),
        )
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

async fn list_sessions(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let sessions = state.store.list_sessions().await;
    (StatusCode::OK, Json(sessions))
}

async fn get_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.store.get_session(&id).await {
        Some(session) => (StatusCode::OK, Json(serde_json::to_value(session).unwrap()).into_response()),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("session '{}' not found", id),
            })
            .into_response(),
        ),
    }
}

async fn close_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if state.store.close_session(&id).await {
        (StatusCode::OK, Json(serde_json::json!({"status": "closed"}))).into_response()
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
    match state.store.add_message(&id, &req.sender, &req.content).await {
        Some(msg) => (StatusCode::CREATED, Json(SendMessageResponse {
            id: msg.id,
            session_id: msg.session_id,
            sender: msg.sender,
            content: msg.content,
            timestamp: msg.timestamp,
        }))
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
}

async fn wait_for_message(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<WaitParams>,
) -> impl IntoResponse {
    let since = params.since.unwrap_or(0);
    let wait_timeout = params.timeout_secs.unwrap_or(300);

    let existing = state.store.get_messages_since(&id, since).await;
    if !existing.is_empty() {
        return (
            StatusCode::OK,
            Json(WaitResponse {
                messages: existing,
                timeout: false,
                timeout_after: None,
                closed: false,
            }),
        )
            .into_response();
    }

    let session = state.store.get_session(&id).await;
    match session {
        Some(s) if s.closed => {
            return (
                StatusCode::OK,
                Json(WaitResponse {
                    messages: vec![],
                    timeout: false,
                    timeout_after: None,
                    closed: true,
                }),
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
                        return WaitResponse {
                            messages: vec![msg],
                            timeout: false,
                            timeout_after: None,
                            closed: false,
                        };
                    }
                }
                Ok(DaemonEvent::SessionClosed) => {
                    return WaitResponse {
                        messages: vec![],
                        timeout: false,
                        timeout_after: None,
                        closed: true,
                    };
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => {
                    return WaitResponse {
                        messages: vec![],
                        timeout: false,
                        timeout_after: None,
                        closed: true,
                    };
                }
            }
        }
    })
    .await;

    match result {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(_elapsed) => (
            StatusCode::OK,
            Json(WaitResponse {
                messages: vec![],
                timeout: true,
                timeout_after: Some(wait_timeout),
                closed: false,
            }),
        )
            .into_response(),
    }
}

async fn recap_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session = state.store.get_session(&id).await;
    match session {
        Some(session) => {
            let messages = state.store.get_all_messages(&id).await;
            (StatusCode::OK, Json(RecapResponse { session, messages })).into_response()
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

async fn status(
    State(state): State<AppState>,
) -> impl IntoResponse {
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
