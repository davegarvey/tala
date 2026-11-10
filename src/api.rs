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
    pub network: crate::listener::Manager,
}
type Payload = std::result::Result<Json<Rpc>, axum::extract::rejection::JsonRejection>;
type Reply = (StatusCode, Json<Envelope>);
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/rpc", post(rpc))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}
/// Router for the opt-in network listener. Every route requires the bearer
/// access token; the loopback instance header is never accepted here, and
/// `/health` does not disclose the instance identifier.
pub fn network_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(network_health))
        .route("/rpc", post(network_rpc))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}
/// Checks the bearer token against the current token, so rotation applies
/// to the next request.
fn bearer_ok(s: &AppState, headers: &HeaderMap) -> bool {
    let Some(token) = s.network.current_token() else {
        return false;
    };
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .is_some_and(|presented| crate::network::token_matches(&token, presented))
}
fn network_denied() -> Reply {
    (
        StatusCode::UNAUTHORIZED,
        Json(Envelope::failure(Failure::new(
            "NETWORK_AUTH",
            "Network access token missing or invalid",
            "Run tala network status --show-token on the host, then tala network connect again on this machine.",
        ))),
    )
}
async fn health(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"protocol_version":PROTOCOL_VERSION,"instance":s.info.instance,"pid":s.info.pid}))
}
async fn network_health(State(s): State<AppState>, headers: HeaderMap) -> Reply {
    if !bearer_ok(&s, &headers) {
        return network_denied();
    }
    (
        StatusCode::OK,
        Json(Envelope::success(
            json!({"protocol_version":PROTOCOL_VERSION}),
        )),
    )
}
async fn network_rpc(State(s): State<AppState>, headers: HeaderMap, payload: Payload) -> Reply {
    if !bearer_ok(&s, &headers) {
        return network_denied();
    }
    if let Ok(Json(request)) = &payload {
        if matches!(request.operation, Operation::Stop) || request.operation.is_network_management()
        {
            return (
                StatusCode::FORBIDDEN,
                Json(Envelope::failure(Failure::new(
                    "REMOTE_MODE",
                    "Stopping the daemon and managing the network window are not allowed over the network",
                    "Run the command on the host machine.",
                ))),
            );
        }
    }
    execute(s, payload).await
}
async fn rpc(State(s): State<AppState>, headers: HeaderMap, payload: Payload) -> Reply {
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
    execute(s, payload).await
}
async fn execute(s: AppState, payload: Payload) -> Reply {
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
    if request.operation.is_network_management() {
        let result = if request.protocol_version != PROTOCOL_VERSION {
            Err(Failure::new(
                "PROTOCOL_MISMATCH",
                "CLI and daemon protocol versions differ",
                "Stop the old daemon using its matching binary, then retry.",
            ))
        } else {
            s.network.execute(&s, &request.operation).await
        };
        return match result {
            Ok(data) => (StatusCode::OK, Json(Envelope::success(data))),
            Err(e) => (StatusCode::BAD_REQUEST, Json(Envelope::failure(e))),
        };
    }
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
            .execute_with_attention(&request)
    })
    .await;
    let response = match result {
        Ok(Ok((data, attention))) => Envelope {
            attention,
            ..Envelope::success(data)
        },
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
