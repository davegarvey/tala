use crate::{models::*, store::Store};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Mutex<Store>>,
    pub info: DaemonInfo,
    pub stop: watch::Sender<bool>,
}
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/rpc", post(rpc))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}
async fn health(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"protocol_version":PROTOCOL_VERSION,"instance":s.info.instance,"pid":s.info.pid}))
}
async fn rpc(
    State(s): State<AppState>,
    headers: HeaderMap,
    payload: std::result::Result<Json<Rpc>, axum::extract::rejection::JsonRejection>,
) -> (StatusCode, Json<Envelope>) {
    if headers.get("x-tala-instance").and_then(|v| v.to_str().ok())
        != Some(s.info.instance.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(Envelope::failure(Failure::new(
                "DAEMON_IDENTITY",
                "Daemon instance header missing or invalid",
                "Use the matching Tala CLI.",
            ))),
        );
    }
    let request = match payload {
        Ok(Json(r)) => r,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(Envelope::failure(Failure::new(
                    "INVALID_REQUEST",
                    e.body_text(),
                    "Use the matching Tala CLI and valid request JSON.",
                ))),
            )
        }
    };
    let stopping = matches!(request.operation, Operation::Stop);
    let store = s.store.clone();
    let result = tokio::task::spawn_blocking(move || {
        store
            .lock()
            .map_err(|_| {
                Failure::new(
                    "STORAGE_ERROR",
                    "Store lock poisoned",
                    "Inspect daemon.log and restart the daemon.",
                )
            })?
            .execute(&request)
    })
    .await;
    let response = match result {
        Ok(Ok(data)) => Envelope::success(data),
        Ok(Err(e)) => Envelope::failure(e),
        Err(e) => Envelope::failure(Failure::new(
            "DAEMON_ERROR",
            e.to_string(),
            "Inspect daemon.log.",
        )),
    };
    if stopping && response.ok {
        let _ = s.stop.send(true);
    }
    (
        if response.ok {
            StatusCode::OK
        } else {
            StatusCode::BAD_REQUEST
        },
        Json(response),
    )
}
