use std::io::Read;
use std::process;
use std::time::Duration;

use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use futures::StreamExt;
use serde_json::json;

use crate::models::*;
use crate::store;

fn fail(json: bool, msg: impl std::fmt::Display, code: &str) -> ! {
    if json {
        eprintln!(
            "{}",
            serde_json::json!({"error": format!("{}", msg), "code": code})
        );
    } else {
        eprintln!("Error: {}", msg);
    }
    process::exit(1);
}

#[derive(Parser)]
#[command(name = "chit", about = "Agent-to-agent messaging", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    Start {
        message: Option<String>,
    },
    Use {
        session_id: Option<String>,
        #[arg(long)]
        clear: bool,
        #[arg(long, short = 'j')]
        json: bool,
    },
    #[command(alias = "send")]
    Chat {
        message: Option<String>,
        #[arg(long)]
        file: Option<String>,
        #[arg(long, short)]
        session: Option<String>,
        #[arg(long = "no-wait", short = 'n', alias = "ff")]
        no_wait: bool,
        #[arg(long = "as", name = "sender_name")]
        sender_name: Option<String>,
        #[arg(long, short = 'j')]
        json: bool,
        #[arg(long, short = 'q')]
        quiet: bool,
        #[arg(long)]
        timeout: Option<u64>,
    },
    Wait {
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        #[arg(long)]
        timeout: Option<u64>,
        #[arg(long)]
        since: Option<u64>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long, short = 'j')]
        json: bool,
    },
    Follow {
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        #[arg(long)]
        since: Option<u64>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, short = 'j')]
        json: bool,
        #[arg(long)]
        timeout: Option<u64>,
    },
    Recap {
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        #[arg(long)]
        since: Option<u64>,
        #[arg(long)]
        cursor: Option<u64>,
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long, short = 'j')]
        json: bool,
    },
    List {
        #[arg(long, short = 'j')]
        json: bool,
    },
    Close {
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session")]
        session_arg: Option<String>,
        #[arg(long, short = 'j')]
        json: bool,
    },
    Status {
        #[arg(long, short = 'j')]
        json: bool,
    },
    Stop,
    #[command(hide = true)]
    Daemon,
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
}

#[derive(Subcommand)]
pub enum SessionCommands {
    List {
        #[arg(long, short = 'j')]
        json: bool,
    },
    Close {
        session_id: String,
        #[arg(long, short = 'j')]
        json: bool,
    },
    Show {
        session_id: String,
        #[arg(long, short = 'j')]
        json: bool,
    },
    Rename {
        session_id: String,
        name: String,
        #[arg(long, short = 'j')]
        json: bool,
    },
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { name } => cmd_init(name).await,
        Commands::Start { message } => cmd_start(message).await,
        Commands::Use { session_id, clear, json } => cmd_use(session_id, clear, json).await,
        Commands::Chat { message, file, session, no_wait, sender_name, json, quiet, timeout } => {
            cmd_send(session, message, file, no_wait, sender_name.as_deref(), json, quiet, timeout).await
        }
        Commands::Wait { session, session_arg, timeout, since, limit, from, json } => {
            cmd_wait(session.or(session_arg), timeout, since, limit, from, json).await
        }
        Commands::Follow { session, session_arg, since, limit, json, timeout } => {
            cmd_follow(session.or(session_arg), since, limit, json, timeout).await
        }
        Commands::Recap { session, session_arg, since, cursor, from, limit, json } => {
            cmd_recap(session.or(session_arg), since.or(cursor), from, limit, json).await
        }
        Commands::List { json } => cmd_list(json).await,
        Commands::Close { session, session_arg, json } => cmd_close(session.or(session_arg), json).await,
        Commands::Status { json } => cmd_status(json).await,
        Commands::Stop => cmd_stop().await,
        Commands::Daemon => crate::daemon::run_daemon().await,
        Commands::Session { command } => match command {
            SessionCommands::List { json } => cmd_list(json).await,
            SessionCommands::Close { session_id, json } => cmd_close(Some(session_id), json).await,
            SessionCommands::Show { session_id, json } => cmd_session_show(session_id, json).await,
            SessionCommands::Rename { session_id, name, json } => cmd_session_rename(session_id, name, json).await,
        },
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

    if let Some(id) = store::read_active_session().await {
        return Ok(id);
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
                "Multiple active sessions: {}. Specify one with `chit {} <session>` or set one with `chit use <session>`",
                ids.join(", "),
                cmd_name
            );
        }
    }
}

async fn cmd_init(name: Option<String>) -> anyhow::Result<()> {
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

    install_opencode_skills().await?;
    Ok(())
}

async fn install_opencode_skills() -> anyhow::Result<()> {
    let opencode_dir = std::path::PathBuf::from(".opencode");
    if !opencode_dir.exists() {
        return Ok(());
    }

    let skill_dir = opencode_dir.join("skills").join("chit");
    tokio::fs::create_dir_all(&skill_dir).await?;

    let skill_path = skill_dir.join("SKILL.md");
    let skill = r#"---
name: chit
description: Agent-to-agent messaging for AI coding tools. Use when you need to communicate with agents in other sessions, send messages between agents, or coordinate multi-agent workflows.
license: MIT
compatibility: Requires chit CLI (agent-to-agent messaging tool)
metadata:
  author: chit
  version: "1.0"
---
# chit — Agent-to-Agent Messaging

You have access to `chit`, a CLI tool for communicating with agents in other sessions.

## Commands

- `chit start [message]` — Start a new session (optionally with initial message). Outputs a session ID like `sess_abc12`.
- `chit chat [session] <message>` — Send a message in markdown format. Blocks for a reply by default. Use `--no-wait` (`-n`, or `--ff`) to fire-and-forget.
- `chit wait [session]` — Block until a new message arrives. Use `--timeout <secs>` to set a timeout. Use `--since <id>` for delta reads, `--from <sender>` to filter by sender, `--limit <n>` to cap results.
- `chit follow [session]` — Stream new messages as they arrive (SSE). Use `--since <id>` to catch up, `--timeout <secs>` to auto-disconnect.
- `chit recap [session]` — View the full conversation transcript. Use `--since <id>` and `--limit <n>` for pagination.
- `chit close [session]` — Close a session.
- `chit session list` — List all sessions (alias for chit list).
- `chit session show <id>` — Show session details.
- `chit session close <id>` — Close a session by ID.
- `chit use [session-id]` — Set or show the active session for this project. Use `chit use --clear` to unset.

## JSON Output

All commands support `--json` for structured output.

## Guidelines

- Format messages in **markdown** — use code blocks with language tags, file references as `path/file:line`, and links where useful.
- Include relevant context: error messages, file paths, stack traces, code snippets.
- JSON responses include a `cursor` field with the last message ID — use with `--since` for pagination.
"#;
    tokio::fs::write(&skill_path, skill).await?;
    println!("Created .opencode/skills/chit/SKILL.md");

    let commands_dir = opencode_dir.join("commands");
    tokio::fs::create_dir_all(&commands_dir).await?;
    let command_path = commands_dir.join("chit.md");
    let command = r#"---
description: Use chit for agent-to-agent messaging - start sessions, send messages, wait for replies, follow streams, and view transcripts.
---
Run chit commands for agent-to-agent messaging. Use `chit start` to create a session, `chit chat` to send a message (use `--no-wait`/`-n` to fire-and-forget, `--file` to read from file), `chit wait` to wait for a reply, `chit follow` to stream messages, `chit recap` to view a transcript, or `chit use` to set the active session. Use `--json` for structured output.
"#;
    tokio::fs::write(&command_path, command).await?;
    println!("Created .opencode/commands/chit.md");
    Ok(())
}

async fn cmd_use(session_id: Option<String>, clear: bool, json_output: bool) -> anyhow::Result<()> {
    if clear {
        store::clear_active_session().await?;
        if json_output {
            println!("{}", serde_json::json!({"status": "cleared"}));
        } else {
            println!("Active session cleared");
        }
        return Ok(());
    }

    if let Some(id) = session_id {
        store::write_active_session(&id).await?;
        if json_output {
            println!("{}", serde_json::json!({"session_id": id, "status": "active"}));
        } else {
            println!("Active session set to {}", id);
        }
        return Ok(());
    }

    match store::read_active_session().await {
        Some(id) => {
            if json_output {
                println!("{}", serde_json::json!({"session_id": id}));
            } else {
                println!("Active session: {}", id);
            }
        }
        None => {
            if json_output {
                println!("{}", serde_json::json!({"session_id": null}));
            } else {
                println!("No active session set. Use `chit use <session-id>` to set one.");
            }
        }
    }
    Ok(())
}

async fn cmd_start(message: Option<String>) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");

    let req_body = if let Some(ref msg) = message {
        let sender = store::get_sender_name(None);
        CreateSessionRequest { message: Some(msg.clone()), sender: Some(sender) }
    } else {
        CreateSessionRequest { message: None, sender: None }
    };

    let resp = client.post(&url).json(&req_body).send().await?;
    let session: CreateSessionResponse = resp.json().await?;
    println!("{}", session.id);

    if message.is_some() {
        cmd_wait(Some(session.id.clone()), None, None, None, None, false).await?;
    }
    Ok(())
}

async fn cmd_send(
    session_arg: Option<String>,
    message: Option<String>,
    file: Option<String>,
    fire_and_forget: bool,
    sender_override: Option<&str>,
    json_output: bool,
    quiet: bool,
    chat_timeout: Option<u64>,
) -> anyhow::Result<()> {
    let content = if let Some(f) = file {
        tokio::fs::read_to_string(&f).await?
            .trim_end_matches('\n')
            .to_string()
    } else if let Some(msg) = &message {
        if msg == "-" {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf.trim_end_matches('\n').to_string()
        } else {
            msg.clone()
        }
    } else {
        anyhow::bail!("No message provided. Use a positional argument, --file <path>, or pipe to stdin with `-`");
    };

    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "send").await?;

    let sender = store::get_sender_name(sender_override);
    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/messages", session_id));

    let req = SendMessageRequest { sender, content: content.to_string() };
    let resp = client.post(&url).json(&req).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        let code = if err.error.contains("closed") { "SESSION_CLOSED" } else { "SESSION_NOT_FOUND" };
        fail(json_output, &err.error, code);
    }

    let msg: SendMessageResponse = resp.json().await?;

    if fire_and_forget {
        if json_output {
            println!("{}", serde_json::to_string(&msg).unwrap());
        } else if !quiet {
            println!("✓ Sent message {} to session {}", msg.id, msg.session_id);
        }
        return Ok(());
    }

    let mut wait_url = format!("/api/sessions/{}/wait?since={}", session_id, msg.id);
    if let Some(to) = chat_timeout {
        wait_url = format!("{}&timeout_secs={}", wait_url, to);
    }
    let wait_url = daemon_url(&host, port, &wait_url);
    let wait_resp = client.get(&wait_url).send().await?;
    let result: WaitResponse = wait_resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
        if result.timeout { process::exit(2); }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!("[timeout after {}s, no reply]", result.timeout_after.unwrap_or(0));
        process::exit(2);
    } else {
        for m in &result.messages {
            println!("{}: {}", m.sender, m.content);
        }
    }
    Ok(())
}

async fn cmd_wait(
    session_arg: Option<String>,
    timeout_secs: Option<u64>,
    since: Option<u64>,
    limit: Option<usize>,
    from: Option<String>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "wait").await?;

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(300);
    let wait_timeout = timeout_secs.unwrap_or(default_timeout);
    let since_id = since.unwrap_or(0);

    let mut path = format!("/api/sessions/{}/wait?since={}&timeout_secs={}", session_id, since_id, wait_timeout);
    if let Some(l) = limit.filter(|&l| l > 0) {
        path = format!("{}&limit={}", path, l);
    }
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &path);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let result: WaitResponse = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
        if result.timeout { process::exit(2); }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!("timeout after {}s, no new messages", result.timeout_after.unwrap_or(0));
        process::exit(2);
    } else {
        for msg in &result.messages {
            println!("[{}] {} ({}):\n    {}", msg.id, msg.sender, msg.timestamp.format("%H:%M:%S"), msg.content);
        }
    }
    Ok(())
}

async fn cmd_follow(
    session_arg: Option<String>,
    since: Option<u64>,
    limit: Option<usize>,
    json_output: bool,
    _timeout: Option<u64>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "follow").await?;

    let since_id = since.unwrap_or(0);
    let mut path = format!("/api/sessions/{}/events?since={}", session_id, since_id);
    if let Some(l) = limit.filter(|&l| l > 0) {
        path = format!("{}&limit={}", path, l);
    }
    let url = daemon_url(&host, port, &path);

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let mut event_type = "message";
            let mut data = String::new();

            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("event: ") {
                    event_type = val;
                } else if let Some(val) = line.strip_prefix("data: ") {
                    data = val.to_string();
                }
            }

            match event_type {
                "closed" => {
                    if json_output {
                        println!("{}", json!({"event": "closed"}));
                    } else {
                        println!("[session closed]");
                    }
                    return Ok(());
                }
                "message" => {
                    if json_output {
                        if let Ok(msg) = serde_json::from_str::<Message>(&data) {
                            let mut obj: serde_json::Value =
                                serde_json::from_str(&data).unwrap_or_default();
                            obj["cursor"] = serde_json::json!(msg.id);
                            println!("{}", serde_json::to_string(&obj).unwrap());
                        } else {
                            println!("{}", data);
                        }
                    } else if let Ok(msg) = serde_json::from_str::<Message>(&data) {
                        println!("[{}] {} ({}):\n    {}", msg.id, msg.sender, msg.timestamp.format("%H:%M:%S"), msg.content);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

async fn cmd_recap(
    session_arg: Option<String>,
    since: Option<u64>,
    from: Option<String>,
    limit: Option<usize>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "recap").await?;

    let since_id = since.unwrap_or(0);
    let mut path = format!("/api/sessions/{}/recap?since={}", session_id, since_id);
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }
    if let Some(l) = limit.filter(|&l| l > 0) {
        path = format!("{}&limit={}", path, l);
    }

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &path);
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let recap: RecapResponse = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&recap).unwrap());
    } else {
        println!("session: {}  |  created: {}  |  closed: {}", recap.session.id, recap.session.created_at.format("%Y-%m-%d %H:%M:%S"), recap.session.closed);
        if let Some(c) = recap.cursor {
            println!("cursor: {}", c);
        }
        println!();
        for msg in &recap.messages {
            println!("[{}] {} ({}):\n    {}\n", msg.id, msg.sender, msg.timestamp.format("%H:%M:%S"), msg.content);
        }
    }
    Ok(())
}

async fn cmd_list(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&sessions).unwrap());
    } else if sessions.is_empty() {
        println!("No active sessions");
    } else {
        for s in &sessions {
            let status = if s.closed { "closed" } else { "active" };
            println!("{}  {}  {} msgs", s.id, status, s.message_count);
        }
    }
    Ok(())
}

async fn cmd_close(session_arg: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "close").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.delete(&url).send().await?;

    if resp.status().is_success() {
        let result: CloseSessionResponse = resp.json().await?;
        if json_output {
            println!("{}", serde_json::json!({"session_id": session_id, "status": result.status}));
        } else {
            println!("Session {}: {}", session_id, result.status);
        }
    } else {
        let err: ErrorResponse = resp.json().await?;
        let code = if err.error.contains("closed") { "SESSION_CLOSED" } else { "SESSION_NOT_FOUND" };
        fail(json_output, &err.error, code);
    }
    Ok(())
}

async fn cmd_session_show(session_id: String, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let session: Session = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&session).unwrap());
    } else {
        println!("Session: {}", session.id);
        if let Some(ref n) = session.name {
            println!("  Name: {}", n);
        }
        println!("  Created: {}", session.created_at.format("%Y-%m-%d %H:%M:%S"));
        println!("  Last activity: {}", session.last_activity.format("%Y-%m-%d %H:%M:%S"));
        println!("  Status: {}", if session.closed { "closed" } else { "active" });
    }
    Ok(())
}

async fn cmd_session_rename(session_id: String, name: String, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/rename", session_id));
    let resp = client.post(&url).json(&json!({"name": name})).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let result: serde_json::Value = resp.json().await?;
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!("Session {} renamed to '{}'", session_id, result["name"]);
    }
    Ok(())
}

async fn cmd_status(json_output: bool) -> anyhow::Result<()> {
    match store::read_daemon_json().await {
        Ok(info) => {
            if json_output {
                println!("{}", serde_json::to_string(&info).unwrap());
            } else {
                println!("daemon running:");
                println!("  PID:  {}", info.pid);
                println!("  Port: {}", info.port);
                println!("  Host: {}", info.host);
                println!("  Since: {}", info.started_at.format("%Y-%m-%d %H:%M:%S"));
            }
            Ok(())
        }
        Err(_) => {
            if json_output {
                println!("{}", serde_json::json!({"running": false}));
            } else {
                println!("no daemon running");
            }
            Ok(())
        }
    }
}

async fn cmd_stop() -> anyhow::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = store::read_daemon_json().await;
        bail!("stop is not supported on this platform");
    }

    #[cfg(unix)]
    {
        let info = store::read_daemon_json().await?;
        use std::process::Command;
        Command::new("kill")
            .arg(info.pid.to_string())
            .status()
            .context("failed to send SIGTERM")?;

        for _ in 0..20 {
            if store::read_daemon_json().await.is_err() {
                println!("daemon stopped");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        bail!("daemon did not stop in time");
    }
}
