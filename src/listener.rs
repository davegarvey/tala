//! Runtime management of the daemon's time-limited network listener.
//!
//! The loopback listener is never touched here. The network listener can be
//! opened, moved, extended and closed while the daemon runs. Expiry is
//! checked against wall-clock time once a second, because Tokio's monotonic
//! timers do not advance while macOS sleeps. The listener speaks only TLS
//! 1.3 with the host certificate (see `tls`).

use crate::{
    api::{self, AppState},
    models::*,
    network, store,
};
use axum_server::{tls_rustls::RustlsConfig, Handle};
use serde_json::{json, Value};
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::task::JoinHandle;

/// How long in-flight requests may finish after a window closes; remaining
/// connections are then closed.
const GRACE: Duration = Duration::from_secs(3);

struct Running {
    bind: String,
    address: String,
    expires_at: i64,
    tls: RustlsConfig,
    handle: Handle<std::net::SocketAddr>,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct Inner {
    running: Option<Running>,
    last_error: Option<String>,
}

#[derive(Clone, Default)]
pub struct Manager {
    inner: Arc<tokio::sync::Mutex<Inner>>,
    token: Arc<RwLock<Option<Arc<str>>>>,
}

fn config_error(message: impl Into<String>, hint: &str) -> Failure {
    Failure::new("NETWORK_CONFIG", message, hint)
}

/// Expiry for a window of `seconds` starting at `from`, capped at the maximum
/// window measured from now.
fn capped_expiry(from: i64, seconds: u64, now: i64) -> (i64, bool) {
    let wanted = from.saturating_add(seconds.min(i64::MAX as u64) as i64);
    let limit = now + network::MAX_WINDOW_SECONDS as i64;
    if wanted > limit {
        (limit, true)
    } else {
        (wanted, false)
    }
}

impl Manager {
    /// The token network requests must present, if a window has ever opened.
    pub fn current_token(&self) -> Option<Arc<str>> {
        self.token.read().ok().and_then(|t| t.clone())
    }

    fn set_token(&self, token: String) {
        if let Ok(mut slot) = self.token.write() {
            *slot = Some(token.into());
        }
    }

    fn describe(inner: &Inner) -> Value {
        let now = store::now();
        match &inner.running {
            Some(r) => json!({
                "open": true,
                "bind": r.bind,
                "address": r.address,
                "expires_at": r.expires_at,
                "remaining_seconds": (r.expires_at - now).max(0),
                "last_error": inner.last_error,
            }),
            None => json!({
                "open": false,
                "bind": Value::Null,
                "address": Value::Null,
                "expires_at": Value::Null,
                "remaining_seconds": Value::Null,
                "last_error": inner.last_error,
            }),
        }
    }

    pub async fn status(&self) -> Value {
        Self::describe(&*self.inner.lock().await)
    }

    /// Stops a listener. A graceful stop lets in-flight requests finish for
    /// the grace period and then closes every connection; an immediate stop
    /// frees the port before returning.
    async fn stop(running: Running, graceful: bool) {
        if graceful {
            running.handle.graceful_shutdown(Some(GRACE));
        } else {
            running.handle.shutdown();
            let _ = running.task.await;
        }
    }

    async fn bind(address: &str) -> std::io::Result<tokio::net::TcpListener> {
        tokio::net::TcpListener::bind(address).await
    }

    /// Opens the window on `bind` for `seconds`, or moves or re-times an open one.
    pub async fn open(&self, app: &AppState, bind: &str, seconds: u64) -> Result<Value> {
        network::validate_bind(bind)?;
        let identity = network::host_identity_or_create()?;
        let tls_config = crate::tls::server_config(&identity)?;
        self.set_token(identity.token);
        let now = store::now();
        let (expires_at, capped) = capped_expiry(now, seconds, now);
        let mut inner = self.inner.lock().await;
        if let Some(running) = inner.running.as_mut().filter(|r| r.bind == bind) {
            running.expires_at = expires_at;
        } else {
            let listener = match Self::bind(bind).await {
                Ok(listener) => listener,
                Err(first) => {
                    // The old listener may hold the port on another address.
                    let Some(old) = inner.running.take() else {
                        inner.last_error = Some(first.to_string());
                        return Err(bind_failed(bind, &first));
                    };
                    Self::stop(old, false).await;
                    let _ = network::remove_window();
                    match Self::bind(bind).await {
                        Ok(listener) => listener,
                        Err(e) => {
                            inner.last_error = Some(e.to_string());
                            return Err(bind_failed(bind, &e));
                        }
                    }
                }
            };
            let address = listener.local_addr()?.to_string();
            if let Some(old) = inner.running.take() {
                Self::stop(old, true).await;
            }
            let tls = RustlsConfig::from_config(tls_config);
            let handle = Handle::new();
            let server = axum_server::from_tcp_rustls(listener.into_std()?, tls.clone())?
                .handle(handle.clone());
            let router = api::network_router(app.clone());
            let task = tokio::spawn(async move {
                if let Err(e) = server.serve(router.into_make_service()).await {
                    eprintln!("Tala network listener stopped: {e}");
                }
            });
            inner.running = Some(Running {
                bind: bind.to_owned(),
                address,
                expires_at,
                tls,
                handle,
                task,
            });
        }
        inner.last_error = None;
        network::save_window(&network::Window {
            bind: bind.to_owned(),
            expires_at,
        })?;
        let mut result = Self::describe(&inner);
        result["capped"] = json!(capped);
        Ok(result)
    }

    pub async fn extend(&self, seconds: u64) -> Result<Value> {
        let mut inner = self.inner.lock().await;
        let Some(running) = inner.running.as_mut() else {
            return Err(config_error(
                "No network window is open",
                "Open one with tala network open.",
            ));
        };
        let now = store::now();
        let (expires_at, capped) = capped_expiry(running.expires_at.max(now), seconds, now);
        running.expires_at = expires_at;
        network::save_window(&network::Window {
            bind: running.bind.clone(),
            expires_at,
        })?;
        let mut result = Self::describe(&inner);
        result["capped"] = json!(capped);
        Ok(result)
    }

    pub async fn close(&self) -> Result<Value> {
        let mut inner = self.inner.lock().await;
        let closed = match inner.running.take() {
            Some(running) => {
                Self::stop(running, true).await;
                eprintln!("Tala network window closed");
                true
            }
            None => false,
        };
        network::remove_window()?;
        let mut result = Self::describe(&inner);
        result["closed"] = json!(closed);
        Ok(result)
    }

    /// Replaces token, certificate and key; an open window keeps running and
    /// presents the new certificate from the next handshake.
    pub async fn rotate_token(&self) -> Result<Value> {
        let inner = self.inner.lock().await;
        let identity = network::rotate_host_identity()?;
        if let Some(running) = &inner.running {
            running
                .tls
                .reload_from_config(crate::tls::server_config(&identity)?);
        }
        self.set_token(identity.token);
        let mut result = Self::describe(&inner);
        result["rotated"] = json!(true);
        Ok(result)
    }

    /// Closes the window once its wall-clock deadline has passed.
    pub async fn expire_if_due(&self) {
        let mut inner = self.inner.lock().await;
        if inner
            .running
            .as_ref()
            .is_some_and(|r| r.expires_at <= store::now())
        {
            if let Some(running) = inner.running.take() {
                Self::stop(running, true).await;
            }
            let _ = network::remove_window();
            eprintln!("Tala network window expired");
        }
    }

    /// Restores an unexpired window after a daemon start and starts the
    /// expiry check. A window never outlives its original deadline.
    pub async fn start(&self, app: &AppState) {
        match network::window() {
            Ok(Some(window)) => {
                let remaining = window.expires_at - store::now();
                if remaining > 0 {
                    if let Err(e) = self.open(app, &window.bind, remaining as u64).await {
                        eprintln!("Tala network window not restored: {}", e.message);
                        let _ = network::remove_window();
                    }
                } else {
                    let _ = network::remove_window();
                }
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("Tala network window ignored: {}", e.message);
                self.inner.lock().await.last_error = Some(e.message);
            }
        }
        let manager = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs(1));
            loop {
                tick.tick().await;
                manager.expire_if_due().await;
            }
        });
    }

    /// Executes a loopback network-management operation.
    pub async fn execute(&self, app: &AppState, operation: &Operation) -> Result<Value> {
        match operation {
            Operation::NetworkOpen { bind, seconds } => self.open(app, bind, *seconds).await,
            Operation::NetworkExtend { seconds } => self.extend(*seconds).await,
            Operation::NetworkClose => self.close().await,
            Operation::NetworkRotateToken => self.rotate_token().await,
            Operation::NetworkStatus => Ok(self.status().await),
            _ => Err(Failure::new(
                "INVALID_OPERATION",
                "Not a network operation",
                "Report this failure.",
            )),
        }
    }
}

fn bind_failed(bind: &str, error: &std::io::Error) -> Failure {
    config_error(
        format!("Could not listen on {bind}: {error}"),
        "Choose another address or port with --bind=IP:PORT; local agents are unaffected.",
    )
}

#[cfg(test)]
mod tests {
    use super::capped_expiry;

    #[test]
    fn expiry_is_capped_one_day_from_now() {
        assert_eq!(capped_expiry(100, 60, 100), (160, false));
        assert_eq!(capped_expiry(100, 200_000, 100), (100 + 86_400, true));
        // Extending from a later expiry still caps relative to now.
        assert_eq!(capped_expiry(80_000, 10_000, 100), (100 + 86_400, true));
    }
}
