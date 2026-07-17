use std::io::{IsTerminal, Read};
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
#[command(
    name = "chit",
    about = "Agent-to-agent messaging for AI coding tools",
    long_about = "chit is a lightweight messaging tool for AI agents working across projects.\n\nStart a session with `chit start`, send messages with `chit send`,\nwait for replies with `chit wait`, or watch all sessions with `chit observe`.\n\nEvery command supports --json for structured output.",
    version,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize chit configuration for this project
    Init {
        #[arg(long, help = "Agent name for this project (defaults to directory name)")]
        name: Option<String>,
    },
    /// Start a new messaging session
    Start {
        #[arg(help = "Optional initial message to send")]
        message: Option<String>,
        #[arg(long, short = 'n', help = "Session name (shown in list and observe output)")]
        name: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Set or show the active session for this project directory
    Use {
        #[arg(help = "Session ID to set as active (omit to show current)")]
        session_id: Option<String>,
        #[arg(long, help = "Clear the active session")]
        clear: bool,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Send a message (alias: send)
    #[command(alias = "send")]
    Chat {
        #[arg(help = "Message content (omit to read from piped stdin)")]
        message: Option<String>,
        #[arg(long, help = "Read message content from a file")]
        file: Option<String>,
        #[arg(long, short, help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(long, short = 'w', help = "Wait for a reply after sending (default: return immediately)")]
        wait: bool,
        #[arg(long = "as", name = "sender_name", help = "Override the sender name")]
        sender_name: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, short = 'q', help = "Suppress confirmation output")]
        quiet: bool,
        #[arg(long, help = "Seconds to wait for a reply (default: 300)")]
        timeout: Option<u64>,
    },
    /// Wait for new messages in a session
    Wait {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(long, help = "Seconds to wait before timing out (default: 300)")]
        timeout: Option<u64>,
        #[arg(long, help = "Only return messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Maximum number of messages to return (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, help = "Only return messages from this sender")]
        from: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, help = "Wait for a new session to be created (ignores other args)")]
        r#new: bool,
    },
    /// Stream new messages as they arrive (SSE)
    Follow {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(long, help = "Only stream messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Maximum number of messages to stream (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, help = "Seconds to stay connected before disconnecting")]
        timeout: Option<u64>,
    },
    /// View conversation transcript
    Recap {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(long, help = "Only show messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Alias for --since (last seen cursor)")]
        cursor: Option<u64>,
        #[arg(long, help = "Only show messages from this sender")]
        from: Option<String>,
        #[arg(long, help = "Maximum number of messages to show (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Watch all sessions for interesting messages
    Observe {
        #[arg(long, help = "Only show messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Only show messages containing this text")]
        r#match: Option<String>,
        #[arg(long, help = "Only show messages from this sender")]
        from: Option<String>,
        #[arg(long, help = "Only show messages in sessions with matching name")]
        channel: Option<String>,
        #[arg(long, help = "Seconds to stay connected before disconnecting")]
        timeout: Option<u64>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// List all sessions
    List {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Close a session
    Close {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", conflicts_with = "session", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Show daemon status
    Status {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Stop the background daemon process
    Stop,
    #[command(hide = true)]
    Daemon,
    /// Manage sessions
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
}

#[derive(Subcommand)]
pub enum SessionCommands {
    /// List all sessions
    List {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Close a session by ID
    Close {
        #[arg(help = "Session ID to close")]
        session_id: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Show session details
    Show {
        #[arg(help = "Session ID to show")]
        session_id: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Rename a session
    Rename {
        #[arg(help = "Session ID to rename")]
        session_id: String,
        #[arg(help = "New name for the session")]
        name: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { name } => cmd_init(name).await,
        Commands::Start { message, name, json } => cmd_start(message, name, json).await,
        Commands::Use { session_id, clear, json } => cmd_use(session_id, clear, json).await,
        Commands::Chat { message, file, session, wait, sender_name, json, quiet, timeout } => {
            cmd_send(session, message, file, wait, sender_name.as_deref(), json, quiet, timeout).await
        }
        Commands::Wait { session, session_arg, timeout, since, limit, from, json, r#new } => {
            if r#new {
                cmd_wait_new(timeout, json).await
            } else {
                cmd_wait(session.or(session_arg), timeout, since, limit, from, json).await
            }
        }
        Commands::Follow { session, session_arg, since, limit, json, timeout } => {
            cmd_follow(session.or(session_arg), since, limit, json, timeout).await
        }
        Commands::Recap { session, session_arg, since, cursor, from, limit, json } => {
            cmd_recap(session.or(session_arg), since.or(cursor), from, limit, json).await
        }
        Commands::Observe { since, r#match, from, channel, timeout, json } => {
            cmd_observe(since, r#match, from, channel, timeout, json).await
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
description: Agent-to-agent messaging for AI coding tools. Use to communicate with agents across projects, terminals, or sessions.
license: MIT
compatibility: Requires chit CLI v0.23+
metadata:
  author: chit
  version: "2.0"
---
# chit — Agent-to-Agent Messaging

Send FYI messages with `chit send "msg"` (auto-creates session, returns immediately).
Request replies with `chit send --wait "question"`. Receive sessions with `chit wait --new`.
Pipe messages: `echo "msg" | chit send`. All commands support `--json`.

## Common Patterns

| Task | Command |
|---|---|
| Broadcast FYI | `chit send "status: done"` |
| Request + wait | `chit send --wait "need help" --timeout 300` |
| Wait for incoming | `sess=$(chit wait --new --timeout 600)` |
| Read transcript | `chit recap` |
| Named session | `chit start --name "my-project"` |
| Watch all | `chit observe` |
| Filtered watch | `chit observe --from "alpha" --match "urgent"` |

## Key Behaviors (v0.23+)
- Send returns immediately by default (fire-and-forget). Use `-w`/`--wait` to block.
- `chit send "msg"` auto-creates session if none exists.
- Active session is auto-set per project directory (`.chit/active-session`).
- `chit wait` without `--since` only waits for new messages (no history replay).
- `chit wait --new` blocks until another agent creates a session.
- `CHIT_HOME` env var overrides `~/.chit` for isolated daemon instances.

## Guidelines
- Use **markdown** in messages — code blocks, file refs `path/file:line`.
- Include relevant context: errors, stack traces, snippets.
- Sessions are ephemeral (in-memory daemon).
"#;
    tokio::fs::write(&skill_path, skill).await?;
    println!("Created .opencode/skills/chit/SKILL.md");

    let commands_dir = opencode_dir.join("commands");
    tokio::fs::create_dir_all(&commands_dir).await?;
    let command_path = commands_dir.join("chit.md");
    let command = r#"---
description: Use chit for agent-to-agent messaging — cross-project, cross-terminal, cross-agent communication.
---
Run chit for agent-to-agent messaging. Send FYI with `chit send "msg"` (auto-creates session, returns immediately). Request replies with `chit send --wait "question"`. Receive with `chit wait --new`. Watch all with `chit observe`. Pipe messages via stdin. Use `--json` for structured output.
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

async fn cmd_start(message: Option<String>, session_name: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/sessions");

    let req_body = CreateSessionRequest {
        message: message.clone(),
        sender: message.as_ref().map(|_| store::get_sender_name(None)),
        name: session_name,
    };

    let resp = client.post(&url).json(&req_body).send().await?;
    let session: CreateSessionResponse = resp.json().await?;

    // Auto-set as active session
    store::write_active_session(&session.id).await?;

    // Send initial message if provided (without waiting)
    if let Some(ref msg) = message {
        let sender = store::get_sender_name(None);
        let msg_url = daemon_url(&host, port, &format!("/api/sessions/{}/messages", session.id));
        let req = SendMessageRequest { sender, content: msg.to_string() };
        let _ = client.post(&msg_url).json(&req).send().await;
    }

    if json_output {
        println!("{}", serde_json::json!({"session_id": session.id}));
    } else {
        println!("{}", session.id);
    }
    Ok(())
}

async fn cmd_send(
    session_arg: Option<String>,
    message: Option<String>,
    file: Option<String>,
    should_wait: bool,
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
        msg.clone()
    } else if !std::io::stdin().is_terminal() {
        let read = tokio::task::spawn_blocking(|| {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok()?;
            let trimmed = buf.trim_end_matches('\n').to_string();
            if trimmed.is_empty() { None } else { Some(trimmed) }
        });
        match tokio::time::timeout(Duration::from_millis(500), read).await {
            Ok(Ok(Some(content))) => content,
            _ => anyhow::bail!("No message provided. Use a positional argument, --file <path>, or pipe to stdin"),
        }
    } else {
        anyhow::bail!("No message provided. Use a positional argument, --file <path>, or pipe to stdin");
    };

    let (host, port) = ensure_daemon_running().await?;

    // Resolve session or auto-create if none exists
    let session_id = if let Some(id) = session_arg.clone() {
        id
    } else if let Some(id) = store::read_active_session().await {
        // Validate active session still exists
        let check_url = daemon_url(&host, port, &format!("/api/sessions/{}", id));
        let check = reqwest::Client::new().get(&check_url).send().await;
        match check {
            Ok(r) if r.status().is_success() => id,
            _ => {
                store::clear_active_session().await?;
                let client = reqwest::Client::new();
                let url = daemon_url(&host, port, "/api/sessions");
                let sender = store::get_sender_name(sender_override);
                let resp = client.post(&url).json(&CreateSessionRequest { message: None, sender: Some(sender), name: None }).send().await?;
                let session: CreateSessionResponse = resp.json().await?;
                store::write_active_session(&session.id).await?;
                if !quiet {
                    eprintln!("Created session {}", session.id);
                }
                session.id
            }
        }
    } else {
        let client = reqwest::Client::new();
        let url = daemon_url(&host, port, "/api/sessions");
        let sender = store::get_sender_name(sender_override);
        let resp = client.post(&url).json(&CreateSessionRequest { message: None, sender: Some(sender), name: None }).send().await?;
        let session: CreateSessionResponse = resp.json().await?;
        store::write_active_session(&session.id).await?;
        if !quiet {
            eprintln!("Created session {}", session.id);
        }
        session.id
    };

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

    if !should_wait {
        if json_output {
            println!("{}", serde_json::to_string(&msg).unwrap());
        } else if !quiet {
            println!("✓ Sent message {} to session {}", msg.id, msg.session_id);
        }
        return Ok(());
    }

    if !json_output && !quiet {
        eprint!("⏎ Waiting for reply...");
        let _ = std::io::Write::flush(&mut std::io::stderr());
    }
    let mut wait_url = format!("/api/sessions/{}/wait?since={}", session_id, msg.id);
    if let Some(to) = chat_timeout {
        wait_url = format!("{}&timeout_secs={}", wait_url, to);
    }
    let wait_url = daemon_url(&host, port, &wait_url);
    let wait_resp = client.get(&wait_url).send().await?;
    let result: WaitResponse = wait_resp.json().await?;

    if !json_output && !quiet {
        eprintln!(); // clear the "Waiting for reply..." line
    }
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

    let since_id = if let Some(s) = since {
        s
    } else {
        let msgs_url = daemon_url(&host, port, &format!("/api/sessions/{}/messages?since=0", session_id));
        match reqwest::Client::new().get(&msgs_url).send().await {
            Ok(resp) => {
                let msgs: Vec<Message> = resp.json().await.unwrap_or_default();
                msgs.iter().map(|m| m.id).max().unwrap_or(0)
            }
            Err(_) => 0,
        }
    };

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

            let event_type = event_block.lines()
                .find_map(|line| line.strip_prefix("event: "))
                .unwrap_or("message");
            let mut data = String::new();

            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("data: ") {
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

async fn cmd_observe(
    since: Option<u64>,
    match_str: Option<String>,
    from: Option<String>,
    channel: Option<String>,
    _timeout: Option<u64>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let since_id = since.unwrap_or(0);
    let mut path = format!("/api/observe?since={}", since_id);
    if let Some(ref m) = match_str {
        path = format!("{}&match={}", path, urlencoding(m));
    }
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }
    if let Some(ref ch) = channel {
        path = format!("{}&channel={}", path, ch);
    }
    let url = daemon_url(&host, port, &path);

    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "OBSERVE_ERROR");
    }

    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let mut data = String::new();

            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("data: ") {
                    data = val.to_string();
                }
            }

            if json_output {
                println!("{}", data);
            } else if let Ok(evt) = serde_json::from_str::<ObserveEvent>(&data) {
                match evt.r#type.as_str() {
                    "message" => {
                        if let Some(msg) = evt.message {
                            let session_label = evt.session_name.unwrap_or(evt.session_id);
                            println!("[{}] {} ({}):\n    {}", session_label, msg.sender, msg.timestamp.format("%H:%M:%S"), msg.content);
                        }
                    }
                    "closed" => {
                        let session_label = evt.session_name.unwrap_or(evt.session_id);
                        println!("[{}] session closed", session_label);
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
     .replace('#', "%23")
     .replace('&', "%26")
     .replace('=', "%3D")
     .replace('+', "%2B")
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

async fn cmd_wait_new(timeout_secs: Option<u64>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let timeout = timeout_secs.unwrap_or(300);
    let url = daemon_url(&host, port, &format!("/api/sessions/wait-new?timeout_secs={}", timeout));
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let result: serde_json::Value = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
        println!("{}", sid);
    } else if result.get("timeout") == Some(&serde_json::json!(true)) {
        eprintln!("timeout after {}s, no new session", result["timeout_after"].as_u64().unwrap_or(timeout));
        process::exit(2);
    } else if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        fail(json_output, err, "WAIT_NEW_ERROR");
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
