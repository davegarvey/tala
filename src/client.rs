use crate::{daemon, models::*, store};
use fs2::FileExt;
use serde_json::Value;
use std::{
    fs::OpenOptions,
    process::{Command, Stdio},
    time::Duration,
};
use uuid::Uuid;

fn network(e: impl std::fmt::Display) -> Failure {
    Failure::new("CONNECTION_ERROR",e.to_string(),"Inspect tala status and daemon.log; retry with the same --key if a send may have been stored.")
}
fn http() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(network)
}
pub async fn live(info: &DaemonInfo) -> bool {
    let Ok(client) = http() else { return false };
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
        Ok(r) if r.status().is_success() => r.json::<Value>().await.ok().is_some_and(|v| {
            if info.instance.is_empty() {
                v["pid"].as_u64() == Some(info.pid as u64)
            } else {
                v["instance"] == info.instance
            }
        }),
        _ => false,
    }
}
pub async fn ensure() -> Result<DaemonInfo> {
    if let Some(i) = daemon::read_info() {
        if live(&i).await {
            return compatible(i);
        }
    }
    let lock = daemon::lock_file("startup.lock")?;
    lock.lock_exclusive()?;
    if let Some(i) = daemon::read_info() {
        if live(&i).await {
            return compatible(i);
        }
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let log = options.open(store::home().join("daemon.log"))?;
    Command::new(std::env::current_exe()?)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()?;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if let Some(i) = daemon::read_info() {
            if live(&i).await {
                return compatible(i);
            }
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
pub fn credential(id: Option<&str>) -> Result<Credential> {
    let id = id.ok_or_else(|| {
        Failure::new(
            "IDENTITY_REQUIRED",
            "No agent selected",
            "Run tala register; set TALA_AGENT to its returned ID or pass --agent=<id>.",
        )
    })?;
    if !id.starts_with("agt_") || !id[4..].chars().all(|c| c.is_ascii_hexdigit()) || id.len() != 36
    {
        return Err(Failure::new(
            "IDENTITY_ERROR",
            "--agent requires a complete agent ID",
            "Use the ID returned by tala register; names are for recipient addressing only.",
        ));
    }
    serde_json::from_slice(&std::fs::read(
        store::home().join("identities").join(format!("{id}.json")),
    )?)
    .map_err(network)
}
pub async fn call(op: Operation, credential: Option<Credential>) -> Result<Value> {
    let request = Rpc {
        protocol_version: PROTOCOL_VERSION,
        request_id: Uuid::new_v4().to_string(),
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
    let Some(info) = daemon::read_info() else {
        return Ok(None);
    };
    if !live(&info).await {
        return Ok(None);
    }
    // Stop is deliberately allowed against a compatible daemon only; no stale PID signalling.
    compatible(info.clone())?;
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
