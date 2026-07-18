use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

use crate::api::create_router;
use crate::store::{read_user_config, remove_daemon_json, tala_home, write_daemon_json, Store};

pub async fn run_daemon() -> anyhow::Result<()> {
    let store = Arc::new(Store::new());

    // Load persisted sessions from disk
    store.load_persisted().await;

    let app = create_router(store.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let port = addr.port();

    write_daemon_json(port).await?;
    info!("tala daemon started on port {}", port);

    let idle_timeout = {
        let config = read_user_config().await;
        config["idle_timeout"].as_u64().unwrap_or(86400)
    };

    let store_clone = store.clone();
    let idle_handle = tokio::spawn(async move {
        let check_interval = Duration::from_secs(60);
        let max_idle = Duration::from_secs(idle_timeout);

        loop {
            tokio::time::sleep(check_interval).await;

            let has_recent_activity = store_clone.has_recent_activity(max_idle).await;
            if !has_recent_activity {
                let daemon_path = tala_home().join("daemon.json");
                if daemon_path.exists() {
                    let metadata = match tokio::fs::metadata(&daemon_path).await {
                        Ok(m) => m,
                        Err(_) => continue,
                    };

                    let elapsed = match metadata.modified() {
                        Ok(time) => match time.elapsed() {
                            Ok(d) => d,
                            Err(_) => continue,
                        },
                        Err(_) => continue,
                    };

                    if elapsed > max_idle {
                        info!("idle timeout reached, shutting down");
                        // Persist open sessions before exit
                        let _ = store_clone.persist().await;
                        std::process::exit(0);
                    }
                }
            }
        }
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    idle_handle.abort();
    // Persist open sessions on graceful shutdown
    let _ = store.persist().await;
    remove_daemon_json().await;
    info!("tala daemon stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
