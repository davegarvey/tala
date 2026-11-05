//! Per-home coordination for intentional daemon shutdown and startup.
//!
//! The lifecycle file is only a local coordination aid; the RPC protocol
//! remains authoritative for daemon operations. Every mutation is protected
//! by a short file lock and atomically replaces the state file.

use crate::{daemon, models::*, store};
use fs2::FileExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    #[serde(default)]
    pub generation: u64,
    #[serde(default)]
    pub instance: Option<String>,
    #[serde(default)]
    pub stop_in_progress: bool,
    #[serde(default)]
    pub stopping: Option<String>,
}

fn locked<T>(f: impl FnOnce(&mut State) -> Result<T>) -> Result<T> {
    let lock = daemon::lock_file("lifecycle.lock")?;
    lock.lock_exclusive()?;
    let path = store::home().join("lifecycle.json");
    let mut state: State = match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
            Failure::new(
                "LIFECYCLE_STATE_ERROR",
                format!("Could not read lifecycle state: {e}"),
                "Inspect TALA_HOME and retry after resolving the local state file.",
            )
        })?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => State::default(),
        Err(e) => return Err(e.into()),
    };
    let before = state.clone();
    let result = f(&mut state)?;
    if state != before {
        store::private_write(
            &path,
            &serde_json::to_vec(&state).expect("lifecycle state serializes"),
        )?;
    }
    let _ = lock.unlock();
    Ok(result)
}

pub fn snapshot() -> Result<State> {
    locked(|state| Ok(state.clone()))
}

pub fn stop_operation_active() -> Result<bool> {
    let lock = daemon::lock_file("stop.lock")?;
    match lock.try_lock_exclusive() {
        Ok(()) => {
            let _ = lock.unlock();
            Ok(false)
        }
        Err(error) if crate::client::startup_lock_contended(&error) => Ok(true),
        Err(error) => Err(error.into()),
    }
}

pub fn reserve_for_wait() -> Result<u64> {
    locked(|state| {
        if state.stop_in_progress || state.stopping.is_some() {
            return Err(stopped());
        }
        Ok(state.generation)
    })
}

pub fn reserve_for_command() -> Result<u64> {
    locked(|state| {
        if state.stop_in_progress || state.stopping.is_some() {
            return Err(Failure::new(
                "DAEMON_STOPPING",
                "The daemon is shutting down",
                "Retry after the current tala stop finishes.",
            ));
        }
        Ok(state.generation)
    })
}

pub fn can_publish(generation: u64) -> Result<bool> {
    locked(|state| {
        Ok(state.generation == generation && !state.stop_in_progress && state.stopping.is_none())
    })
}

pub fn publish(generation: u64, instance: &str) -> Result<bool> {
    locked(|state| {
        if state.generation != generation || state.stop_in_progress || state.stopping.is_some() {
            return Ok(false);
        }
        state.instance = Some(instance.to_owned());
        Ok(true)
    })
}

pub fn accepts(generation: u64) -> Result<bool> {
    locked(|state| {
        Ok(state.generation == generation && !state.stop_in_progress && state.stopping.is_none())
    })
}

/// Marks the currently published instance as stopping before any RPC is sent.
/// The caller must acquire startup.lock as a barrier after this returns.
pub fn accept_stop() -> Result<(u64, Option<String>)> {
    locked(|state| {
        if let Some(target) = &state.stopping {
            state.stop_in_progress = true;
            return Ok((state.generation, Some(target.clone())));
        }
        let target = state
            .instance
            .clone()
            .or_else(|| daemon::read_info().map(|info| info.instance));
        state.generation = state.generation.saturating_add(1);
        state.stop_in_progress = true;
        state.stopping = target.clone();
        Ok((state.generation, target))
    })
}

pub fn finish_stop(target: Option<&str>) -> Result<()> {
    locked(|state| {
        if target.is_none() || state.stopping.as_deref() == target {
            state.stop_in_progress = false;
            state.stopping = None;
            if let Some(target) = target {
                if state.instance.as_deref() == Some(target) {
                    state.instance = None;
                }
            }
        }
        Ok(())
    })
}

/// Clears a shutdown marker only after the target is known to be gone. A
/// reachable instance keeps the marker; an unreachable instance is unknown
/// and also keeps it for a later retry.
pub async fn recover_stopping() -> Result<State> {
    let state = snapshot()?;
    if !state.stop_in_progress && state.stopping.is_none() {
        return Ok(state);
    }
    if stop_operation_active()? {
        return Ok(state);
    }
    let Some(target) = state.stopping.as_deref() else {
        finish_stop(None)?;
        return snapshot();
    };
    let info = daemon::read_info().filter(|info| info.instance == target);
    let exited = match info {
        None => daemon::daemon_lock_available()?,
        Some(info) => {
            matches!(
                crate::client::probe(&info).await,
                crate::client::Liveness::Stale
            ) && daemon::daemon_lock_available()?
        }
    };
    if exited {
        finish_stop(Some(target))?;
    }
    snapshot()
}

pub fn stopping_error() -> Failure {
    stopped()
}

pub fn stopped() -> Failure {
    Failure::new(
        "DAEMON_STOPPED",
        "The daemon was intentionally stopped while this wait was active",
        "Start a new wait after the stop completes.",
    )
}
