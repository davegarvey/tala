//! `tala ui`: a read-only web viewer for the current Tala home.
//!
//! The viewer runs in its own process, reads the store through read-only
//! connections, and never contacts the daemon. It listens only on loopback,
//! requires a per-run bearer token on every data route, and rejects requests
//! whose `Host` header is not this listener.

pub mod query;

use crate::models::{Failure, Result};
use axum::{
    extract::{Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use query::Filter;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{net::SocketAddr, path::PathBuf, sync::Arc};

const INDEX: &str = include_str!("assets/index.html");
const SCRIPT: &str = include_str!("assets/app.js");
const STYLE: &str = include_str!("assets/app.css");
// Style attributes carry per-agent hues and bar widths; scripts and
// stylesheets must come from this origin.
const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
                   style-src-attr 'unsafe-inline'; img-src 'self' data:; \
                   connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

#[derive(Clone)]
pub struct Viewer {
    inner: Arc<Inner>,
}
struct Inner {
    store: PathBuf,
    home: PathBuf,
    token: String,
    port: u16,
}

/// A bound, not yet serving, viewer.
pub struct Bound {
    pub viewer: Viewer,
    pub listener: tokio::net::TcpListener,
}
impl Bound {
    pub fn url(&self) -> String {
        format!(
            "http://127.0.0.1:{}/#t={}",
            self.viewer.inner.port, self.viewer.inner.token
        )
    }
    pub fn port(&self) -> u16 {
        self.viewer.inner.port
    }
}

fn new_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Checks the store and binds the loopback listener.
pub async fn bind(home: PathBuf, port: u16) -> Result<Bound> {
    let store = home.join("tala.sqlite3");
    let check = store.clone();
    tokio::task::spawn_blocking(move || query::open(&check).map(drop))
        .await
        .map_err(|e| Failure::new("IO_ERROR", e.to_string(), "Retry tala ui."))??;
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port)))
        .await
        .map_err(|e| {
            Failure::new(
                "IO_ERROR",
                format!("Cannot listen on 127.0.0.1:{port}: {e}"),
                "Choose another --port, or omit it to use a free port.",
            )
        })?;
    let port = listener
        .local_addr()
        .map_err(|e| Failure::new("IO_ERROR", e.to_string(), "Retry tala ui."))?
        .port();
    Ok(Bound {
        viewer: Viewer {
            inner: Arc::new(Inner {
                store,
                home,
                token: new_token(),
                port,
            }),
        },
        listener,
    })
}

pub fn router(viewer: Viewer) -> Router {
    let api = Router::new()
        .route("/api/head", get(head))
        .route("/api/meta", get(meta))
        .route("/api/messages", get(messages))
        .route("/api/threads", get(threads))
        .route("/api/threads/:id", get(transcript))
        .route("/api/agents", get(agents))
        .route("/api/attention", get(attention))
        .route_layer(middleware::from_fn_with_state(
            viewer.clone(),
            require_token,
        ));
    Router::new()
        .route(
            "/",
            get(|| async { asset(INDEX, "text/html; charset=utf-8") }),
        )
        .route(
            "/app.js",
            get(|| async { asset(SCRIPT, "text/javascript; charset=utf-8") }),
        )
        .route(
            "/app.css",
            get(|| async { asset(STYLE, "text/css; charset=utf-8") }),
        )
        .merge(api)
        .fallback(|| async { (StatusCode::NOT_FOUND, Json(json!({"error": "not_found"}))) })
        .layer(middleware::from_fn_with_state(viewer.clone(), require_host))
        .layer(middleware::from_fn(security_headers))
        .with_state(viewer)
}

fn asset(body: &'static str, content_type: &'static str) -> Response {
    ([(header::CONTENT_TYPE, content_type)], body).into_response()
}

async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    for (name, value) in [
        (header::CONTENT_SECURITY_POLICY, CSP),
        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        (header::REFERRER_POLICY, "no-referrer"),
        (header::CACHE_CONTROL, "no-store"),
        (header::X_FRAME_OPTIONS, "DENY"),
    ] {
        headers.insert(name, HeaderValue::from_static(value));
    }
    response
}

fn denied(status: StatusCode, reason: &str) -> Response {
    (status, Json(json!({"error": reason}))).into_response()
}

/// Rejects requests whose Host header is not this loopback listener, which
/// defeats DNS rebinding from other web pages.
async fn require_host(State(v): State<Viewer>, request: Request, next: Next) -> Response {
    let port = v.inner.port;
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or_default();
    let allowed = host == format!("127.0.0.1:{port}") || host == format!("localhost:{port}");
    if !allowed {
        return denied(StatusCode::FORBIDDEN, "host");
    }
    next.run(request).await
}

fn token_ok(v: &Viewer, headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .is_some_and(|presented| crate::network::token_matches(&v.inner.token, presented))
}

async fn require_token(State(v): State<Viewer>, request: Request, next: Next) -> Response {
    if !token_ok(&v, request.headers()) {
        return denied(StatusCode::UNAUTHORIZED, "token");
    }
    next.run(request).await
}

/// Runs one read on a fresh read-only connection off the async executor.
async fn read<F>(v: &Viewer, f: F) -> Response
where
    F: FnOnce(&rusqlite::Connection) -> Result<Value> + Send + 'static,
{
    let path = v.inner.store.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut db = query::open(&path)?;
        let tx = db
            .transaction()
            .map_err(|e| Failure::new("STORAGE_ERROR", e.to_string(), "Retry the request."))?;
        let value = f(&tx)?;
        drop(tx);
        Ok::<_, Failure>(value)
    })
    .await;
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(failure)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": failure})),
        )
            .into_response(),
        Err(e) => denied(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

#[derive(Deserialize, Default)]
struct Params {
    project: Option<String>,
    agent: Option<String>,
    q: Option<String>,
    before: Option<i64>,
    limit: Option<usize>,
}
impl Params {
    fn filter(&self) -> Filter {
        let keep = |s: &Option<String>| s.as_ref().filter(|s| !s.trim().is_empty()).cloned();
        Filter {
            project: keep(&self.project),
            agent: keep(&self.agent),
            text: keep(&self.q),
        }
    }
}

async fn head(State(v): State<Viewer>) -> Response {
    read(&v, |db| Ok(json!({"head": query::head(db)?}))).await
}

async fn meta(State(v): State<Viewer>) -> Response {
    let home = v.inner.home.display().to_string();
    read(&v, move |db| {
        let mut meta = query::meta(db)?;
        meta["home"] = json!(home);
        meta["active_window_seconds"] = json!(query::ACTIVE_WINDOW);
        Ok(meta)
    })
    .await
}

async fn messages(State(v): State<Viewer>, Query(p): Query<Params>) -> Response {
    let filter = p.filter();
    let limit = p.limit.unwrap_or(query::DEFAULT_LIMIT);
    read(&v, move |db| query::messages(db, &filter, p.before, limit)).await
}

async fn threads(State(v): State<Viewer>, Query(p): Query<Params>) -> Response {
    let filter = p.filter();
    read(&v, move |db| query::threads(db, &filter, p.before)).await
}

async fn transcript(State(v): State<Viewer>, Path(id): Path<String>) -> Response {
    read(&v, move |db| query::transcript(db, &id)).await
}

async fn agents(State(v): State<Viewer>, Query(p): Query<Params>) -> Response {
    let filter = p.filter();
    read(&v, move |db| query::agents(db, &filter)).await
}

async fn attention(State(v): State<Viewer>, Query(p): Query<Params>) -> Response {
    let filter = p.filter();
    read(&v, move |db| query::attention(db, &filter)).await
}

/// Opens `url` in the default browser. Failure is reported, not fatal.
pub fn open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    #[cfg(windows)]
    let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    command
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(drop)
}

/// A future that resolves on Ctrl-C or termination. The handlers are
/// registered when this is called, so a signal that arrives before serving
/// starts still shuts the viewer down cleanly.
pub type Shutdown = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
pub fn shutdown_signal() -> Result<Shutdown> {
    let failed = |e: std::io::Error| {
        Failure::new(
            "IO_ERROR",
            format!("Cannot register signal handlers: {e}"),
            "Retry tala ui.",
        )
    };
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut interrupt = signal(SignalKind::interrupt()).map_err(failed)?;
        let mut terminate = signal(SignalKind::terminate()).map_err(failed)?;
        Ok(Box::pin(async move {
            tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
        }))
    }
    #[cfg(not(unix))]
    {
        let _ = failed;
        Ok(Box::pin(async {
            let _ = tokio::signal::ctrl_c().await;
        }))
    }
}

/// Serves until `shutdown` resolves.
pub async fn serve(bound: Bound, shutdown: Shutdown) -> Result<()> {
    axum::serve(bound.listener, router(bound.viewer))
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| Failure::new("IO_ERROR", e.to_string(), "Retry tala ui."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    struct Reply {
        status: u16,
        headers: String,
    }
    impl Reply {
        fn status(&self) -> StatusCode {
            StatusCode::from_u16(self.status).unwrap()
        }
        fn header(&self, name: &str) -> Option<String> {
            self.headers.lines().find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.eq_ignore_ascii_case(name).then(|| v.trim().to_string())
            })
        }
    }

    /// Serves a viewer over a freshly migrated store on an ephemeral port.
    async fn fixture() -> (tempfile::TempDir, Viewer) {
        let dir = tempfile::TempDir::new().unwrap();
        drop(crate::store::Store::open(&dir.path().join("tala.sqlite3")).unwrap());
        let bound = bind(dir.path().to_path_buf(), 0).await.unwrap();
        let viewer = bound.viewer.clone();
        tokio::spawn(async move { axum::serve(bound.listener, router(bound.viewer)).await });
        (dir, viewer)
    }

    async fn get(v: &Viewer, path: &str, host: Option<&str>, token: Option<&str>) -> Reply {
        let port = v.inner.port;
        let default_host = format!("127.0.0.1:{port}");
        let mut request = format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
            host.unwrap_or(&default_host)
        );
        if let Some(token) = token {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        request.push_str("\r\n");
        let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).await.unwrap();
        let raw = String::from_utf8_lossy(&raw);
        let (head, _) = raw.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        Reply {
            status,
            headers: head.to_string(),
        }
    }

    #[tokio::test]
    async fn binds_loopback_and_reports_url() {
        let dir = tempfile::TempDir::new().unwrap();
        drop(crate::store::Store::open(&dir.path().join("tala.sqlite3")).unwrap());
        let bound = bind(dir.path().to_path_buf(), 0).await.unwrap();
        let addr = bound.listener.local_addr().unwrap();
        assert!(addr.ip().is_loopback());
        assert!(bound
            .url()
            .starts_with(&format!("http://127.0.0.1:{}/#t=", addr.port())));
        assert!(bound.viewer.inner.token.len() >= 32);
    }

    #[tokio::test]
    async fn token_and_host_are_enforced() {
        let (_dir, v) = fixture().await;
        let token = v.inner.token.clone();
        assert_eq!(
            get(&v, "/api/head", None, None).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get(&v, "/api/head", None, Some("wrong")).await.status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get(&v, "/api/head", None, Some(&new_token()))
                .await
                .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            get(&v, "/api/head", Some("evil.example:80"), Some(&token))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            get(&v, "/", Some("evil.example"), None).await.status(),
            StatusCode::FORBIDDEN
        );
        let port = v.inner.port;
        assert_eq!(
            get(
                &v,
                "/api/head",
                Some(&format!("localhost:{port}")),
                Some(&token)
            )
            .await
            .status(),
            StatusCode::OK
        );
        for path in [
            "/api/head",
            "/api/meta",
            "/api/messages?limit=5",
            "/api/threads",
            "/api/threads/none",
            "/api/agents",
            "/api/attention?agent=&q=",
        ] {
            assert_eq!(
                get(&v, path, None, Some(&token)).await.status(),
                StatusCode::OK,
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn page_needs_no_token_and_sends_security_headers() {
        let (_dir, v) = fixture().await;
        for path in ["/", "/app.js", "/app.css"] {
            let r = get(&v, path, None, None).await;
            assert_eq!(r.status(), StatusCode::OK, "{path}");
            assert_eq!(r.header("content-security-policy").as_deref(), Some(CSP));
            assert_eq!(
                r.header("x-content-type-options").as_deref(),
                Some("nosniff")
            );
            assert_eq!(r.header("referrer-policy").as_deref(), Some("no-referrer"));
            assert_eq!(r.header("cache-control").as_deref(), Some("no-store"));
        }
    }

    #[test]
    fn assets_reference_no_other_origin() {
        for asset in [INDEX, SCRIPT, STYLE] {
            for needle in ["http://", "https://", "//cdn", "@import", "fonts.g"] {
                assert!(!asset.contains(needle), "asset references {needle}");
            }
        }
        assert!(!SCRIPT.contains("innerHTML = m.text"));
    }

    #[tokio::test]
    async fn reads_do_not_change_the_store() {
        let (dir, v) = fixture().await;
        let path = dir.path().join("tala.sqlite3");
        let snapshot = |p: &std::path::Path| {
            let db = rusqlite::Connection::open(p).unwrap();
            let version: i64 = db
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            let counts: Vec<i64> = ["agents", "messages", "agent_events", "projects"]
                .iter()
                .map(|t| {
                    db.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                        .unwrap()
                })
                .collect();
            (version, counts)
        };
        {
            let db = rusqlite::Connection::open(&path).unwrap();
            db.execute(
                "INSERT INTO agents(id,name,tool,context,token,last_seen) VALUES('agt_a','a','t','{\"project\":\"/p\"}','h',1)",
                [],
            )
            .unwrap();
        }
        let before = snapshot(&path);
        let token = v.inner.token.clone();
        for p in [
            "/api/head",
            "/api/meta",
            "/api/messages",
            "/api/threads",
            "/api/threads/x",
            "/api/agents",
            "/api/attention",
        ] {
            assert_eq!(
                get(&v, p, None, Some(&token)).await.status(),
                StatusCode::OK
            );
        }
        assert_eq!(snapshot(&path), before);
    }
}
