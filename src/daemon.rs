use crate::{
    api::{self, AppState},
    models::*,
    store::{self, Store},
};
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    sync::{Arc, Mutex},
};
use tokio::sync::watch;
use uuid::Uuid;

pub fn lock_file(name: &str) -> std::io::Result<File> {
    store::private_dir(&store::home())?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(store::home().join(name))
}
pub fn daemon_lock_available() -> std::io::Result<bool> {
    let lock = lock_file("daemon.lock")?;
    match lock.try_lock_exclusive() {
        Ok(()) => {
            let _ = lock.unlock();
            Ok(true)
        }
        Err(error) if crate::client::startup_lock_contended(&error) => Ok(false),
        Err(error) => Err(error),
    }
}
pub fn read_info() -> Option<DaemonInfo> {
    serde_json::from_slice(&std::fs::read(store::home().join("daemon.json")).ok()?).ok()
}
pub async fn run() -> Result<()> {
    let generation = std::env::var("TALA_START_GENERATION")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| {
            Failure::new(
                "INVALID_OPERATION",
                "The daemon process must be started by a Tala client",
                "Run a normal Tala command to start the daemon safely.",
            )
        })?;
    if !crate::lifecycle::can_publish(generation)? {
        return Err(crate::lifecycle::stopped());
    }
    if let Some(info) = read_info() {
        if info.protocol_version != PROTOCOL_VERSION && crate::client::live(&info).await {
            return Err(Failure::new(
                "PROTOCOL_MISMATCH",
                "An incompatible daemon already owns this Tala home",
                "Stop it with its matching binary before upgrading.",
            ));
        }
    }
    let lock = lock_file("daemon.lock")?;
    lock.try_lock_exclusive().map_err(|_| {
        Failure::new(
            "DAEMON_ALREADY_RUNNING",
            "A daemon already owns this Tala home",
            "Use tala status or stop.",
        )
    })?;
    let store = Store::open(&store::home().join("tala.sqlite3"))?;
    if !crate::lifecycle::can_publish(generation)? {
        return Err(crate::lifecycle::stopped());
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let info = DaemonInfo {
        pid: std::process::id(),
        port: listener.local_addr()?.port(),
        protocol_version: PROTOCOL_VERSION,
        instance: Uuid::new_v4().to_string(),
    };
    if !crate::lifecycle::publish(generation, &info.instance)? {
        return Err(crate::lifecycle::stopped());
    }
    store::private_write(
        &store::home().join("daemon.json"),
        &serde_json::to_vec(&info).expect("daemon metadata serializes"),
    )?;
    let (stop, mut rx) = watch::channel(false);
    let app = api::router(AppState {
        store: Arc::new(Mutex::new(store)),
        info: info.clone(),
        stop,
    });
    eprintln!(
        "Tala daemon {} started on localhost:{}",
        info.instance, info.port
    );
    let result=axum::serve(listener,app).with_graceful_shutdown(async move{
        #[cfg(unix)] {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut sig)=>{tokio::select!{_ = rx.changed()=>{}, _ = tokio::signal::ctrl_c()=>{}, _ = sig.recv()=>{}}}
                Err(_)=>{tokio::select!{_ = rx.changed()=>{}, _ = tokio::signal::ctrl_c()=>{}}}
            }
        }
        #[cfg(not(unix))]{tokio::select!{_ = rx.changed()=>{}, _ = tokio::signal::ctrl_c()=>{}}}
    }).await;
    if read_info().is_some_and(|i| i.instance == info.instance) {
        let _ = std::fs::remove_file(store::home().join("daemon.json"));
    }
    drop(lock);
    result.map_err(Into::into)
}
