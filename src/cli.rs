use std::time::Duration;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use serde_json::json;

use crate::models::*;
use crate::store;

#[derive(Parser)]
#[command(name = "chit", about = "Agent-to-agent messaging", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize chit in the current project
    Init {
        /// Custom project name
        #[arg(long)]
        name: Option<String>,
        /// Generate opencode skill file
        #[arg(long)]
        opencode: bool,
    },
    /// Start daemon and create a new session
    Start {
        /// Optional initial message (sends and blocks for reply)
        message: Option<String>,
    },
    /// Send a message to a session (blocks for reply by default)
    Send {
        /// Message content (markdown)
        message: String,
        /// Session ID (auto-targets if single session exists)
        #[arg(long, short)]
        session: Option<String>,
        /// Fire-and-forget: don't wait for reply
        #[arg(long, short)]
        ff: bool,
        /// Override sender name
        #[arg(long = "as", name = "sender_name")]
        sender_name: Option<String>,
    },
    /// Wait for the next message in a session
    Wait {
        /// Session ID (auto-targets if single session exists)
        session: Option<String>,
        /// Timeout in seconds
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// Show full conversation transcript
    Recap {
        /// Session ID (auto-targets if single session exists)
        session: Option<String>,
    },
    /// List active sessions
    List,
    /// Close a session
    Close {
        /// Session ID (auto-targets if single session exists)
        session: Option<String>,
    },
    /// Show daemon status
    Status,
    /// Stop the daemon
    Stop,
    /// Run the daemon server (internal)
    #[command(hide = true)]
    Daemon,
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { name, opencode } => cmd_init(name, opencode).await,
        Commands::Start { message } => cmd_start(message).await,
        Commands::Send { message, session, ff, sender_name } => {
            cmd_send(session, &message, ff, sender_name.as_deref()).await
        }
        Commands::Wait { session, timeout } => cmd_wait(session, timeout).await,
        Commands::Recap { session } => cmd_recap(session).await,
        Commands::List => cmd_list().await,
        Commands::Close { session } => cmd_close(session).await,
        Commands::Status => cmd_status().await,
        Commands::Stop => cmd_stop().await,
        Commands::Daemon => crate::daemon::run_daemon().await,
    }
}

async fn ensure_daemon_running() -> anyhow::Result<(String, u16)> {
    match store::read_daemon_json().await {
        Ok(info) => Ok((info.host, info.port)),
        Err(_) => {
            std::process::Command::new(std::env::current_exe()?)
                .arg("daemon")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .stdin(std::process::Stdio::null())
                .spawn()
                .context("failed to start daemon")?;

            for _ in 0..50 {
                tokio::time::sleep(Duration::from_millis(100)).await;
                if let Ok(info) = store::read_daemon_json().await {
                    return Ok((info.host, info.port));
                }
            }

            bail!("daemon failed to start within 5 seconds");
        }
    }
}

fn daemon_url(host: &str, port: u16, path: &str) -> String {
    format!("http://{}:{}{}", host, port, path)
}

async fn resolve_session_id(
    host: &str,
    port: u16,
    session_arg: Option<&str>,
    cmd_name: &str,
) -> anyhow::Result<String> {
    if let Some(id) = session_arg {
        return Ok(id.to_string());
    }

    let url = daemon_url(host, port, "/api/sessions");
    let resp = reqwest::get(&url).await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;
    let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

    match active.len() {
        0 => bail!("No active sessions. Start one with `chit start`"),
        1 => Ok(active[0].id.clone()),
        _ => {
            let ids: Vec<&str> = active.iter().map(|s| s.id.as_str()).collect();
            bail!(
                "Multiple active sessions: {}. Specify one with `chit {} <session>`",
                ids.join(", "),
                cmd_name
            );
        }
    }
}

async fn cmd_init(name: Option<String>, opencode: bool) -> anyhow::Result<()> {
    let chit_dir = std::path::PathBuf::from(".chit");
    tokio::fs::create_dir_all(&chit_dir).await?;

    let config_path = chit_dir.join("config.json");
    if config_path.exists() {
        eprintln!("./.chit/config.json already exists");
    } else {
        let project_name = name.unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "project".to_string())
        });
        let config = json!({ "name": project_name });
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&config)?).await?;
        println!("Created ./.chit/config.json with name: {}", project_name);
    }

    if opencode {
        let skill_path = chit_dir.join("opencode-skill.md");
        let skill = r#"# chit — Agent-to-Agent Messaging

You have access to `chit`, a CLI tool for communicating with agents in other sessions.

## Commands

- `chit start [message]` — Start a new session (optionally with initial message). Outputs a session ID like `sess_abc12`.
- `chit send [session] <message>` — Send a message in markdown format. Blocks for a reply by default. Use `--ff` to fire-and-forget.
- `chit wait [session]` — Block until a new message arrives. Use `--timeout <secs>` to set a timeout.
- `chit recap [session]` — View the full conversation transcript.
- `chit close [session]` — Close a session.

## Guidelines

- Format messages in **markdown** — use code blocks with language tags, file references as `path/file:line`, and links where useful.
- Include relevant context: error messages, file paths, stack traces, code snippets.
- Use `chit send` when you need to ask something or provide information to another agent.
- Use `chit wait` when you're expecting a response.
"#;
        tokio::fs::write(&skill_path, skill).await?;
        println!("Created ./.chit/opencode-skill.md");
    }

    Ok(())
}

async fn cmd_start(message: Option<String>) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");

    let req_body = if let Some(ref msg) = message {
        let sender = store::get_sender_name(None);
        CreateSessionRequest {
            message: Some(msg.clone()),
            sender: Some(sender),
        }
    } else {
        CreateSessionRequest {
            message: None,
            sender: None,
        }
    };

    let resp = client
        .post(&url)
        .json(&req_body)
        .send()
        .await?;

    let session: CreateSessionResponse = resp.json().await?;
    println!("{}", session.id);

    if message.is_some() {
        cmd_wait(Some(session.id.clone()), None).await?;
    }

    Ok(())
}

async fn cmd_send(
    session_arg: Option<String>,
    content: &str,
    fire_and_forget: bool,
    sender_override: Option<&str>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "send").await?;

    let sender = store::get_sender_name(sender_override);
    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/messages", session_id));

    let req = SendMessageRequest {
        sender,
        content: content.to_string(),
    };

    let resp = client.post(&url).json(&req).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        bail!("{}", err.error);
    }

    let msg: SendMessageResponse = resp.json().await?;

    if fire_and_forget {
        return Ok(());
    }

    let wait_url = daemon_url(
        &host,
        port,
        &format!("/api/sessions/{}/wait?since={}", session_id, msg.id),
    );
    let wait_resp = client.get(&wait_url).send().await?;
    let result: WaitResponse = wait_resp.json().await?;

    if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!("[timeout after {}s, no reply]", result.timeout_after.unwrap_or(0));
    } else {
        for m in &result.messages {
            println!("{}: {}", m.sender, m.content);
        }
    }

    Ok(())
}

async fn cmd_wait(session_arg: Option<String>, timeout_secs: Option<u64>) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "wait").await?;

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(300);
    let wait_timeout = timeout_secs.unwrap_or(default_timeout);

    let client = reqwest::Client::new();
    let url = daemon_url(
        &host,
        port,
        &format!("/api/sessions/{}/wait?since=0&timeout_secs={}", session_id, wait_timeout),
    );

    let resp = client.get(&url).send().await?;
    let result: WaitResponse = resp.json().await?;

    if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!("timeout after {}s, no new messages", result.timeout_after.unwrap_or(0));
    } else {
        for msg in &result.messages {
            println!("[{}] {} ({}):\n    {}", msg.id, msg.sender, msg.timestamp.format("%H:%M:%S"), msg.content);
        }
    }

    Ok(())
}

async fn cmd_recap(session_arg: Option<String>) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "recap").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/recap", session_id));
    let resp = client.get(&url).send().await?;
    let recap: RecapResponse = resp.json().await?;

    println!(
        "session: {}  |  created: {}  |  closed: {}",
        recap.session.id,
        recap.session.created_at.format("%Y-%m-%d %H:%M:%S"),
        recap.session.closed,
    );
    println!();

    for msg in &recap.messages {
        println!(
            "[{}] {} ({}):\n    {}\n",
            msg.id,
            msg.sender,
            msg.timestamp.format("%H:%M:%S"),
            msg.content,
        );
    }

    Ok(())
}

async fn cmd_list() -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    if sessions.is_empty() {
        println!("No active sessions");
    } else {
        for s in &sessions {
            let status = if s.closed { "closed" } else { "active" };
            println!("{}  {}  {} msgs", s.id, status, s.message_count);
        }
    }

    Ok(())
}

async fn cmd_close(session_arg: Option<String>) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "close").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.delete(&url).send().await?;

    if resp.status().is_success() {
        let result: CloseSessionResponse = resp.json().await?;
        println!("Session {}: {}", session_id, result.status);
    } else {
        let err: ErrorResponse = resp.json().await?;
        bail!("{}", err.error);
    }

    Ok(())
}

async fn cmd_status() -> anyhow::Result<()> {
    match store::read_daemon_json().await {
        Ok(info) => {
            println!("daemon running:");
            println!("  PID:  {}", info.pid);
            println!("  Port: {}", info.port);
            println!("  Host: {}", info.host);
            println!("  Since: {}", info.started_at.format("%Y-%m-%d %H:%M:%S"));
            Ok(())
        }
        Err(_) => {
            println!("no daemon running");
            Ok(())
        }
    }
}

async fn cmd_stop() -> anyhow::Result<()> {
    match store::read_daemon_json().await {
        Ok(info) => {
            #[cfg(unix)]
            {
                use std::process::Command;
                Command::new("kill")
                    .arg(info.pid.to_string())
                    .status()
                    .context("failed to send SIGTERM")?;
            }
            #[cfg(not(unix))]
            {
                let _ = info;
                bail!("stop is not supported on this platform");
            }

            for _ in 0..20 {
                if store::read_daemon_json().await.is_err() {
                    println!("daemon stopped");
                    return Ok(());
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            bail!("daemon did not stop in time");
        }
        Err(_) => {
            println!("no daemon running");
            Ok(())
        }
    }
}
