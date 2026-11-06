use crate::{models::*, store::Store};
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Duration;
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
    let stopping = matches!(&request.operation, Operation::Stop);
    if matches!(&request.operation, Operation::Wait { .. }) {
        return wait_rpc(s, request).await;
    }
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

async fn wait_rpc(s: AppState, mut request: Rpc) -> (StatusCode, Json<Envelope>) {
    let Operation::Wait { timeout, .. } = &request.operation else {
        unreachable!()
    };
    let deadline =
        (*timeout != 0).then(|| tokio::time::Instant::now() + Duration::from_secs(*timeout));
    let mut stop = s.stop.subscribe();
    loop {
        let store = s.store.clone();
        let rpc = request.clone();
        let result = tokio::task::spawn_blocking(move || {
            store
                .lock()
                .map_err(|_| {
                    Failure::new(
                        "STORAGE_ERROR",
                        "Store lock poisoned",
                        "Inspect daemon.log and restart the daemon.",
                    )
                })
                .and_then(|mut store| store.execute_with_attention(&rpc))
        })
        .await;
        let snapshot = match result {
            Ok(Ok((data, attention))) => {
                if data["inbox"]["messages"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                    || data["board"]["messages"]
                        .as_array()
                        .is_some_and(|v| !v.is_empty())
                    || data["durable_events"]
                        .as_array()
                        .is_some_and(|v| !v.is_empty())
                {
                    return (
                        StatusCode::OK,
                        Json(Envelope {
                            attention,
                            ..Envelope::success(data)
                        }),
                    );
                }
                data
            }
            Ok(Err(e)) => return (StatusCode::BAD_REQUEST, Json(Envelope::failure(e))),
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(Envelope::failure(Failure::new(
                        "DAEMON_ERROR",
                        e.to_string(),
                        "Inspect daemon.log.",
                    ))),
                )
            }
        };
        if let Some(deadline) = deadline {
            if tokio::time::Instant::now() >= deadline {
                let mut data = snapshot;
                data["timed_out"] = json!(true);
                return (StatusCode::OK, Json(Envelope::success(data)));
            }
        }
        tokio::select! {
            _=tokio::time::sleep(Duration::from_millis(250))=>{},
            _=stop.changed()=>return (StatusCode::SERVICE_UNAVAILABLE,Json(Envelope::failure(Failure::new("DAEMON_STOPPED","Wait ended because the daemon was intentionally stopped","Run tala wait after the daemon is restarted.")))),
        }
        if let Operation::Wait {
            inbox_after,
            board_after,
            event_after,
            ..
        } = &mut request.operation
        {
            *inbox_after = snapshot["inbox"]["next_after"]
                .as_u64()
                .unwrap_or(*inbox_after);
            *board_after = Some(
                snapshot["board"]["next_after"]
                    .as_u64()
                    .unwrap_or(board_after.unwrap_or(0)),
            );
            *event_after = snapshot["event_after"].as_u64().unwrap_or(*event_after);
        }
    }
}
