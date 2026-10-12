use crate::{daemon, models::*, store};
use fs2::FileExt;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    process::{Command, Stdio},
    sync::Mutex,
    time::Duration,
};
use uuid::Uuid;

fn network(e: impl std::fmt::Display) -> Failure {
    Failure::new("CONNECTION_ERROR",e.to_string(),"Inspect tala status and daemon.log; retry with the same --key if a send may have been stored.")
}
fn unreachable(port: u16, reason: &str) -> Failure {
    Failure::new(
        "DAEMON_UNREACHABLE",
        format!("Cannot reach the Tala daemon on 127.0.0.1:{port}: {reason}"),
        "The daemon may still be running: sandbox or network restrictions can block loopback connections, and a paused daemon stops answering. Tala will not start a second daemon. Request permission for local network access or inspect tala status.",
    )
}
fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(network)
}
/// Result of probing the daemon recorded in daemon.json.
pub enum Liveness {
    Live,
    /// Nothing usable answers at the recorded port: safe to start a daemon.
    Stale,
    /// The daemon may be alive but cannot be reached: never start another.
    Unreachable(String),
}
fn io_kind(error: &reqwest::Error) -> Option<std::io::ErrorKind> {
    let mut source = std::error::Error::source(error);
    while let Some(e) = source {
        if let Some(io) = e.downcast_ref::<std::io::Error>() {
            return Some(io.kind());
        }
        source = e.source();
    }
    None
}
fn classify(error: &reqwest::Error) -> Liveness {
    match io_kind(error) {
        Some(std::io::ErrorKind::ConnectionRefused) => Liveness::Stale,
        _ if error.is_timeout() || error.is_connect() => Liveness::Unreachable(
            io_kind(error)
                .map(|k| k.to_string())
                .unwrap_or_else(|| "timed out".into()),
        ),
        // Connected, but the peer answered badly: not a Tala daemon.
        _ => Liveness::Stale,
    }
}
pub async fn probe(info: &DaemonInfo) -> Liveness {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Liveness::Unreachable(e.to_string()),
    };
    let endpoint = if info.instance.is_empty() {
        "/api/status"
    } else {
        "/health"
    };
    let result = client
        .get(format!("http://127.0.0.1:{}{}", info.port, endpoint))
        .send()
        .await;
    match result {
        Ok(r) if r.status().is_success() => match r.json::<Value>().await {
            Ok(v)
                if (info.instance.is_empty() && v["pid"].as_u64() == Some(info.pid as u64))
                    || (!info.instance.is_empty() && v["instance"] == info.instance) =>
            {
                Liveness::Live
            }
            Ok(_) => Liveness::Stale,
            Err(e) if e.is_timeout() => Liveness::Unreachable("timed out".into()),
            Err(_) => Liveness::Stale,
        },
        Ok(_) => Liveness::Stale,
        Err(e) => classify(&e),
    }
}
pub async fn live(info: &DaemonInfo) -> bool {
    matches!(probe(info).await, Liveness::Live)
}
/// Probes recorded metadata: Some(info) when live, None when stale or absent.
async fn usable() -> Result<Option<DaemonInfo>> {
    let Some(info) = daemon::read_info() else {
        return Ok(None);
    };
    match probe(&info).await {
        Liveness::Live => compatible(info).map(Some),
        Liveness::Stale => Ok(None),
        Liveness::Unreachable(reason) => Err(unreachable(info.port, &reason)),
    }
}
pub async fn ensure() -> Result<DaemonInfo> {
    if let Some(i) = usable().await? {
        return Ok(i);
    }
    let lock = daemon::lock_file("startup.lock")?;
    lock.lock_exclusive()?;
    if let Some(i) = usable().await? {
        return Ok(i);
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let log = options.open(store::home().join("daemon.log"))?;
    // Windows inherits every inheritable handle, including the CLI's capture
    // pipes. A long-lived daemon must not keep those pipes open after CLI exit.
    #[cfg(windows)]
    clear_standard_handle_inheritance()?;
    let mut command = Command::new(std::env::current_exe()?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x00000008 | 0x00000200); // Detached, new process group.
    }
    command
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()?;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(i) = usable().await? {
            return Ok(i);
        }
    }
    Err(Failure::new(
        "DAEMON_START_FAILED",
        "Daemon did not start within five seconds",
        "Read TALA_HOME/daemon.log for storage, lock or startup errors.",
    ))
}
fn compatible(i: DaemonInfo) -> Result<DaemonInfo> {
    if i.protocol_version != PROTOCOL_VERSION {
        return Err(Failure::new(
            "PROTOCOL_MISMATCH",
            format!(
                "Daemon protocol {} differs from CLI protocol {}",
                i.protocol_version, PROTOCOL_VERSION
            ),
            "Stop the old daemon with its matching binary, then retry.",
        ));
    }
    Ok(i)
}
fn handle_path(handle: &str) -> std::path::PathBuf {
    let digest = Sha256::digest(handle.as_bytes());
    let name: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    store::home().join("handles").join(format!("{name}.json"))
}
/// Issues a new private handle for a credential and stores it under the
/// handle's hash, so neither the handle nor the token appears in file names.
pub fn issue_handle(credential: &Credential) -> Result<String> {
    let handle = format!("th_{}", Uuid::new_v4().simple());
    let path = handle_path(&handle);
    store::private_dir(path.parent().expect("handles directory"))?;
    store::private_write(&path, &serde_json::to_vec(credential).map_err(network)?)?;
    Ok(handle)
}
/// Credential stored by a protocol 2 registration, keyed by public ID.
pub fn legacy_credential_path(agent: &str) -> std::path::PathBuf {
    store::home()
        .join("identities")
        .join(format!("{agent}.json"))
}
pub fn credential(handle: Option<&str>) -> Result<Credential> {
    let handle = handle.ok_or_else(|| {
        Failure::new(
            "IDENTITY_REQUIRED",
            "No agent selected",
            "Run tala register; pass the returned handle with --agent=<handle> or TALA_AGENT.",
        )
    })?;
    if handle.starts_with("agt_") {
        return Err(Failure::new(
            "IDENTITY_ERROR",
            "--agent takes your private handle, not a public agent ID",
            "Public agt_ IDs are for addressing others. Use the th_ handle that tala register returned to you. For your own identity created before handles existed, run tala register --reissue=<your agt_ ID> once.",
        ));
    }
    if !handle.starts_with("th_")
        || handle.len() != 35
        || !handle[3..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(Failure::new(
            "IDENTITY_ERROR",
            "--agent requires a complete th_ handle",
            "Use the handle returned by tala register.",
        ));
    }
    match std::fs::read(handle_path(handle)) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(network),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(Failure::new(
            "IDENTITY_ERROR",
            "Unknown handle for this Tala home",
            "Use the handle tala register returned in this TALA_HOME, or register a new identity.",
        )),
        Err(e) => Err(e.into()),
    }
}
static ATTENTION: Mutex<Option<Value>> = Mutex::new(None);
/// Attention summary from the most recent successful daemon response.
pub fn take_attention() -> Option<Value> {
    ATTENTION.lock().ok().and_then(|mut a| a.take())
}
pub async fn call(op: Operation, credential: Option<Credential>) -> Result<Value> {
    call_as(Uuid::new_v4().to_string(), op, credential).await
}
/// Calls with a caller-chosen request ID; repeating an ID replays the
/// daemon's recorded result for state-changing operations.
pub async fn call_as(
    request_id: String,
    op: Operation,
    credential: Option<Credential>,
) -> Result<Value> {
    let request = Rpc {
        protocol_version: PROTOCOL_VERSION,
        request_id,
        credential,
        operation: op,
    };
    let mut last = None;
    for _ in 0..3 {
        let info = ensure().await?;
        match http()?
            .post(format!("http://127.0.0.1:{}/rpc", info.port))
            .header("x-tala-instance", &info.instance)
            .json(&request)
            .send()
            .await
        {
            Ok(response) => match response.json::<Envelope>().await {
                Ok(e) => {
                    return if e.ok {
                        if let Ok(mut slot) = ATTENTION.lock() {
                            *slot = e.attention;
                        }
                        e.data.ok_or_else(|| network("Empty response"))
                    } else {
                        Err(e.error.unwrap_or_else(|| network("Missing error")))
                    }
                }
                Err(e) => last = Some(network(e)),
            },
            Err(e) => last = Some(network(e)),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(last.unwrap_or_else(|| network("Request failed")))
}
pub async fn existing(op: Operation) -> Result<Option<Value>> {
    // Stop is deliberately allowed against a compatible daemon only; no stale PID signalling.
    let Some(info) = usable().await? else {
        return Ok(None);
    };
    let request = Rpc {
        protocol_version: PROTOCOL_VERSION,
        request_id: Uuid::new_v4().to_string(),
        credential: None,
        operation: op,
    };
    let response = http()?
        .post(format!("http://127.0.0.1:{}/rpc", info.port))
        .header("x-tala-instance", &info.instance)
        .json(&request)
        .send()
        .await
        .map_err(network)?
        .json::<Envelope>()
        .await
        .map_err(network)?;
    if !response.ok {
        return Err(response.error.unwrap_or_else(|| network("Missing error")));
    }
    Ok(response.data)
}

#[cfg(windows)]
fn clear_standard_handle_inheritance() -> std::io::Result<()> {
    use std::{ffi::c_void, os::windows::io::AsRawHandle};
    #[link(name = "kernel32")]
    extern "system" {
        fn SetHandleInformation(handle: *mut c_void, mask: u32, flags: u32) -> i32;
    }
    for handle in [
        std::io::stdin().as_raw_handle(),
        std::io::stdout().as_raw_handle(),
        std::io::stderr().as_raw_handle(),
    ] {
        if !handle.is_null() && handle as isize != -1 {
            // These are borrowed process handles; clearing inheritance does not
            // close them or interfere with the CLI's own input/output.
            if unsafe { SetHandleInformation(handle, 1, 0) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
        }
    }
    Ok(())
}
