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
#[command(
    name = "tala",
    about = "Agent-to-agent messaging for AI coding tools",
    long_about = "tala is a lightweight messaging tool for AI agents working across projects.\n\nSend messages with `tala send`, wait for replies with `tala wait`, stream a session with `tala stream`,\nor listen to all sessions with `tala listen`.\n\nUse `tala wait --new-session` to wait for another agent to create a session.\n\nEvery command supports --json for structured output.",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize tala config for this project directory (sets agent name used when sending messages)
    Init {
        #[arg(help = "Agent name for this project (defaults to directory name)")]
        name: Option<String>,
    },

    /// Set or show the active session for this project directory
    #[command(
        after_help = "See also: tala session (show, rename, reopen) for advanced session management"
    )]
    Use {
        #[arg(help = "Session ID to set as active (omit to show current)")]
        session_id: Option<String>,
        #[arg(long, help = "Clear the active session")]
        clear: bool,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Send a message to a session. Use `tala session create` to create a session without a message.
    #[command(
        after_help = "Multi-line messages (the common case): pipe a heredoc, no flag needed, e.g. `tala send <<'EOF'`.\nOne-line messages: inline argument, e.g. `tala send \"tests passing\"`.\nDraft-then-edit content: `--message-file <path>`.\nQuoted heredocs protect backticks, $variables and quotes; use `--stdin` to disambiguate stdin from a positional message.\nUse `--` to separate options from message content, e.g. `tala send -- --my-flags`.\nUse --wait / -w to block until a reply arrives.\n\nINTENT:\n  --intent <req|fyi|reply|out>  Declare what you expect (default: fyi; --wait implies req; --reply-to implies reply)\n  --reply-to <id>               Correlate this message as a reply to message <id> (same session)\n  --expect-reply                This message also expects a reply (modifier for reply/fyi)\n  With --wait --timeout N, recipients see the live countdown via the stamped waiting_until.\nUse `tala session create --name` to create a named session."
    )]
    Send {
        #[arg(
            allow_hyphen_values = true,
            help = "Session ID (positional, or use --session/-s)"
        )]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(
            allow_hyphen_values = true,
            help = "Message content (omit to read from piped stdin)"
        )]
        message: Option<String>,
        #[arg(
            long = "message-file",
            help = "Read message content from a file (use - for filename to use piped stdin)"
        )]
        message_file: Option<String>,
        #[arg(
            long,
            help = "Read message content from stdin (bypasses shell interpretation)"
        )]
        stdin: bool,
        #[arg(
            long,
            short = 'w',
            help = "Wait for a reply after sending (default: return immediately)"
        )]
        wait: bool,
        #[arg(long = "sender", help = "Override the sender name")]
        sender_name: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, short = 'q', help = "Suppress confirmation output")]
        quiet: bool,
        #[arg(long, help = "Seconds to wait for a reply (default: 60)")]
        timeout: Option<u64>,
        #[arg(long, help = "Declare message intent: req, fyi, reply, or out")]
        intent: Option<String>,
        #[arg(long, help = "Correlate this message as a reply to a message id")]
        reply_to: Option<u64>,
        #[arg(long, help = "This message expects a reply (valid with reply/fyi)")]
        expect_reply: bool,
    },
    /// Wait for new messages in a session (blocking poll — sends an HTTP request every few seconds).
    /// Use `tala stream` for real-time SSE on a single session, or `tala listen` to observe all sessions.
    /// Use `tala wait --new-session` to wait for another agent to create a session.
    #[command(
        after_help = "USAGE:\n  tala wait <session>          Blocking poll — sends periodic HTTP requests\n  tala wait --new-session     Wait for another agent to create a session\n\nCOMPARISON:\n  tala stream   Real-time SSE — stays connected, pushes messages immediately (single session)\n  tala listen   Real-time SSE — observe all sessions at once\n  tala check    Non-blocking — show new messages and return immediately\n\nSee also: tala history (transcript), tala session (manage sessions)"
    )]
    Wait {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, help = "Seconds to wait before timing out (default: 60)")]
        timeout: Option<u64>,
        #[arg(long, help = "Only return messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Maximum number of messages to return (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, help = "Only return messages from this sender")]
        from: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(
            long = "new-session",
            help = "Wait for a new session to be created (ignores other args)"
        )]
        r#new: bool,
    },
    /// Stream new messages as they arrive for a single session (real-time SSE — stays connected and pushes messages).
    /// Use `tala wait` for a blocking poll (request/response), or `tala listen` to observe all sessions.
    #[command(
        name = "stream",
        after_help = "USAGE:\n  tala stream <session>   Real-time SSE — stays connected, pushes messages immediately (single session)\n\nCOMPARISON:\n  tala wait     Blocking poll — sends periodic HTTP requests, good for scripts and CI\n  tala listen   Real-time SSE — observe all sessions at once\n  tala check    Non-blocking — show new messages and return immediately\n\nSee also: tala history (transcript)"
    )]
    Stream {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
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
    #[command(
        after_help = "See also: tala wait (blocking poll), tala listen (all sessions), tala stream (real-time SSE)"
    )]
    History {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, help = "Only show messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Only show messages from this sender")]
        from: Option<String>,
        #[arg(long, help = "Maximum number of messages to show (0 = unlimited)")]
        limit: Option<usize>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Observe all sessions for new messages (real-time SSE across all sessions).
    /// Use `tala stream` for a single session, or `tala wait` for a blocking poll.
    #[command(
        after_help = "USAGE:\n  tala listen                Real-time SSE — observe all sessions at once\n  tala listen --since <n>   Skip history replay (only messages with ID > n)\n  tala listen --from <name> Filter messages from a specific sender\n  tala listen --match <text> Filter messages containing text\n  tala listen --name <name> Filter by session name\n\nCOMPARISON:\n  tala stream   Real-time SSE — single session\n  tala wait     Blocking poll — sends periodic HTTP requests, good for scripts and CI\n  tala check    Non-blocking -- show new messages and return immediately\n\nSee also: tala history (transcript)"
    )]
    Listen {
        #[arg(long, help = "Only show messages with ID greater than this")]
        since: Option<u64>,
        #[arg(long, help = "Only show messages containing this text")]
        r#match: Option<String>,
        #[arg(long, help = "Only show messages from this sender")]
        from: Option<String>,
        #[arg(long, help = "Only show messages in sessions with matching name")]
        name: Option<String>,
        #[arg(
            long,
            help = "Seconds to stay connected before disconnecting (default: 60, 0 = no timeout)"
        )]
        timeout: Option<u64>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },

    /// Show new messages since last check (non-blocking)
    #[command(
        after_help = "See also: tala wait (blocking poll), tala listen (all sessions), tala stream (real-time SSE), tala history (transcript)"
    )]
    Check {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// List all sessions
    List {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Discover agents in other projects (scans parent directories for tala projects)
    #[command(
        after_help = "Scans up to 3 parent directories and their siblings for .tala/config.json files"
    )]
    Discover {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// List all active agents (unique senders across open sessions)
    #[command(after_help = "See also: tala discover (cross-project agent discovery)")]
    Agents {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Close a session
    Close {
        #[arg(help = "Session ID (uses active session if set)")]
        session: Option<String>,
        #[arg(
            long = "session",
            short,
            alias = "session-id",
            conflicts_with = "session",
            help = "Session ID"
        )]
        session_arg: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
        #[arg(long, short = 'q', help = "Suppress confirmation output")]
        quiet: bool,
    },
    /// List requests awaiting a reply (who owes whom)
    #[command(
        after_help = "Shows open obligations across your sessions: unanswered [REQ] messages and\nmessages sent with --expect-reply. Use `tala send --reply-to <id>` to answer one.\n\nSee also: tala list (sessions), tala check (new messages)"
    )]
    Pending {
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
    #[command(after_help = "Alias: tala list")]
    List {
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Close a session by ID
    #[command(after_help = "Alias: tala close")]
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
        #[arg(long, help = "Force rename even if session already has a name")]
        force: bool,
    },
    /// Reopen a closed session
    Reopen {
        #[arg(help = "Session ID to reopen")]
        session_id: String,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
    /// Create a new empty session
    Create {
        #[arg(
            long,
            short = 'n',
            help = "Session name (shown in list and check output)"
        )]
        name: Option<String>,
        #[arg(long, short = 'j', help = "Output in JSON format")]
        json: bool,
    },
}

pub async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Init { name } => cmd_init(name).await,
        Commands::Use {
            session_id,
            clear,
            json,
        } => cmd_use(session_id, clear, json).await,
        Commands::Send {
            session,
            session_arg,
            message,
            message_file,
            stdin,
            wait,
            sender_name,
            json,
            quiet,
            timeout,
            intent,
            reply_to,
            expect_reply,
        } => {
            let session_flag = session_arg.is_some();
            let resolved_session = session_arg
                .or_else(|| session.as_ref().filter(|s| s.starts_with("sess_")).cloned());
            let resolved_message = message.or_else(|| {
                if session_flag {
                    session
                } else {
                    session.filter(|s| !s.starts_with("sess_"))
                }
            });
            if stdin && resolved_message.is_some() {
                eprintln!("Warning: --stdin is set, ignoring positional message argument");
            }
            cmd_send(
                resolved_session,
                resolved_message,
                message_file,
                stdin,
                wait,
                sender_name.as_deref(),
                json,
                quiet,
                timeout,
                intent.as_deref(),
                reply_to,
                expect_reply,
            )
            .await
        }
        Commands::Wait {
            session,
            session_arg,
            timeout,
            since,
            limit,
            from,
            json,
            r#new,
        } => {
            if r#new {
                cmd_wait_new(timeout, json).await
            } else {
                cmd_wait(session.or(session_arg), timeout, since, limit, from, json).await
            }
        }
        Commands::Stream {
            session,
            session_arg,
            since,
            limit,
            json,
            timeout,
        } => cmd_watch(session.or(session_arg), since, limit, json, timeout).await,
        Commands::History {
            session,
            session_arg,
            since,
            from,
            limit,
            json,
        } => cmd_recap(session.or(session_arg), since, from, limit, json).await,
        Commands::Listen {
            since,
            r#match,
            from,
            name,
            timeout,
            json,
        } => cmd_listen(since, r#match, from, name, timeout, json).await,
        Commands::List { json } => cmd_list(json).await,
        Commands::Pending { json } => cmd_pending(json).await,
        Commands::Discover { json } => cmd_discover(json).await,
        Commands::Agents { json } => cmd_agents(json).await,
        Commands::Close {
            session,
            session_arg,
            json,
            quiet,
        } => cmd_close(session.or(session_arg), json, quiet).await,
        Commands::Check { json } => cmd_whatsup(json).await,
        Commands::Status { json } => cmd_status(json).await,
        Commands::Stop => cmd_stop().await,
        Commands::Daemon => crate::daemon::run_daemon().await,
        Commands::Session { command } => match command {
            SessionCommands::List { json } => cmd_list(json).await,
            SessionCommands::Close { session_id, json } => {
                cmd_close(Some(session_id), json, false).await
            }
            SessionCommands::Show { session_id, json } => cmd_session_show(session_id, json).await,
            SessionCommands::Rename {
                session_id,
                name,
                json,
                force,
            } => cmd_session_rename(session_id, name, json, force).await,
            SessionCommands::Reopen { session_id, json } => {
                cmd_session_reopen(session_id, json).await
            }
            SessionCommands::Create { name, json } => cmd_session_create(name, json).await,
        },
    }
}

fn daemon_home_display() -> String {
    let path = store::tala_home();
    if let Ok(th) = std::env::var("TALA_HOME") {
        format!("{} (from TALA_HOME={})", path.display(), th)
    } else {
        path.display().to_string()
    }
}

async fn ensure_daemon_running() -> anyhow::Result<(String, u16)> {
    // Check if daemon.json exists and daemon is reachable
    if let Ok(info) = store::read_daemon_json().await {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_default();
        let alive = client
            .get(format!("http://{}:{}/api/status", info.host, info.port))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false);
        if alive {
            return Ok((info.host, info.port));
        }
        // Stale daemon.json — clean up and restart
        store::remove_daemon_json().await;
    }

    // Start daemon
    let home = daemon_home_display();
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

    let daemon_path = store::tala_home().join("daemon.json");
    if !daemon_path.exists() {
        bail!(
            "Daemon not found at {}/daemon.json. Check TALA_HOME is set correctly.",
            home
        );
    } else {
        bail!("daemon failed to start within 5 seconds (daemon.json exists at {}/daemon.json but daemon is not reachable)", home);
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
        0 => bail!("No active sessions. Start one with `tala send`"),
        1 => Ok(active[0].id.clone()),
        _ => {
            let ids: Vec<&str> = active.iter().map(|s| s.id.as_str()).collect();
            bail!(
                "Multiple active sessions: {}. Specify one with `tala {} <session>` or set one with `tala use <session>`",
                ids.join(", "),
                cmd_name
            );
        }
    }
}

async fn cmd_init(name: Option<String>) -> anyhow::Result<()> {
    let tala_dir = std::path::PathBuf::from(".tala");
    tokio::fs::create_dir_all(&tala_dir).await?;

    let config_path = tala_dir.join("config.json");
    if config_path.exists() {
        eprintln!("./.tala/config.json already exists");
    } else {
        let project_name = name.unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "project".to_string())
        });
        let config = json!({ "name": project_name });
        tokio::fs::write(&config_path, serde_json::to_string_pretty(&config)?).await?;
        println!("Created ./.tala/config.json with name: {}", project_name);
    }

    install_opencode_skills().await?;
    Ok(())
}

async fn install_opencode_skills() -> anyhow::Result<()> {
    let opencode_dir = std::path::PathBuf::from(".opencode");
    if !opencode_dir.exists() {
        return Ok(());
    }

    let skill_dir = opencode_dir.join("skills").join("tala");
    tokio::fs::create_dir_all(&skill_dir).await?;

    let skill_path = skill_dir.join("SKILL.md");
    let skill = r#"---
name: tala
description: Agent-to-agent messaging for AI coding tools. Use to communicate with agents across projects, terminals, or sessions.
license: MIT
compatibility: Requires tala CLI v0.23+
metadata:
  author: tala
  version: "2.1"
---
# tala — Agent-to-Agent Messaging

Send messages with `tala send "msg"`. Request replies with `tala send --wait "question"`.
Wait for incoming sessions with `tala wait --new-session`. View history with `tala history`.
Pipe messages: `echo "msg" | tala send`. All commands support `--json`.

## Common Patterns

| Task | Command |
|---|---|
| Broadcast FYI | `tala send "status: done"` |
| Request + wait | `tala send --wait "need help" --timeout 60` |
| Correlated reply | `tala send --intent reply --reply-to 5 "fix is in parse_row"` |
| Wait for incoming | `sess=$(tala wait --new-session --timeout 600)` |
| What's unanswered | `tala pending` |
| Read transcript | `tala history` |
| Named session | `tala session create --name "my-project"` |
| Watch all | `tala listen` |
| Filtered watch | `tala listen --from "alpha" --match "urgent"` |
| Override sender | `tala send --sender "bot" "hello"` |
| Check messages | `tala check` |
| Discover agents | `tala agents` |
| Cross-project discovery | `tala discover` |

## Intent Protocol

Every message can declare its intent with `--intent <req|fyi|reply|out>`:
- `req` — a reply is expected from you (implied by `--wait`; `--reply-to` implies `reply`)
- `fyi` — informational, no reply needed (default)
- `reply` — answers a prior request; correlate with `--reply-to <id>`
- `out` — exchange over, no reply expected

Intents render as badges (`[REQ]`, `[REPLY→5]`) in history/check/wait/stream/listen,
and a `waiting_until` countdown ("waiting, 23s left") is stamped when you use
`send --wait --timeout N`. The countdown is computed at read time — never stale.

`tala pending` lists open obligations: unanswered `req` messages and anything
sent with `--expect-reply`. Answer one with `tala send --reply-to <id>`.

**Waiting visibility:** the daemon tracks active waits. When your wait overlaps
another agent's wait, you get a note (`⟳ note: alpha is waiting on sess_ab12
(13s left)`), and a timeout prints a hint when sessions hold unread messages.
`tala status` shows everyone waiting right now.

## Key Behaviors (v0.25+)
- Send returns immediately by default (fire-and-forget). Use `-w`/`--wait` to block.
- If no session exists and you provide a message, auto-creates a session.
- Use `tala session create` to create a session without a message.
- Active session is auto-set per project directory (`.tala/active-session`).
- `tala wait` without `--since` only waits for new messages (no history replay).
- `tala wait --new-session` blocks until another agent creates a session.
- `tala listen` watches all sessions.
- `tala check` shows new messages since last check (non-blocking).
- `tala agents` lists active participants.
- `tala discover` finds agents in other projects.
- `TALA_HOME` env var overrides `~/.tala` for isolated daemon instances.

## Guidelines
- Use **markdown** in messages — code blocks, file refs `path/file:line`.
- Include relevant context: errors, stack traces, snippets.
- Sessions are ephemeral (in-memory daemon).
- **Shell safety:** Multi-line messages (the common case) — pipe a quoted heredoc: `tala send <<'EOF' ... EOF` (no flag needed).
  For one-line messages with backticks or special chars, use single quotes: `tala send 'msg with \`code\`'`.
  Draft-then-edit content: `--message-file <path>`.
  If your message starts with `--`, add a `--` separator: `tala send -- --my-flag-value`.
"#;
    tokio::fs::write(&skill_path, skill).await?;
    println!("Created .opencode/skills/tala/SKILL.md");

    let commands_dir = opencode_dir.join("commands");
    tokio::fs::create_dir_all(&commands_dir).await?;
    let command_path = commands_dir.join("tala.md");
    let command = r#"---
description: Use tala for agent-to-agent messaging — cross-project, cross-terminal, cross-agent communication.
---
Run tala for agent-to-agent messaging. Send messages with `tala send "msg"`. Request replies with `tala send --wait "question"`. Receive sessions with `tala wait --new-session`. Watch all activity with `tala listen`. Read transcripts with `tala history`. Check for new messages with `tala check`. Discover cross-project agents with `tala discover`. Pipe messages via stdin. All commands support `--json`. By default, `tala send` returns immediately (use `-w`/`--wait` to block).
"#;
    tokio::fs::write(&command_path, command).await?;
    println!("Created .opencode/commands/tala.md");
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

    if let Some(input) = session_id {
        let (host, port) = ensure_daemon_running().await?;

        // Try name match first (more meaningful to users)
        let url = daemon_url(&host, port, "/api/sessions");
        let resp = reqwest::get(&url).await?;
        let sessions: Vec<SessionSummary> = resp.json().await?;
        let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

        let name_matches: Vec<&SessionSummary> = active
            .iter()
            .filter(|s| s.name.as_deref() == Some(&input))
            .copied()
            .collect();

        if name_matches.len() == 1 {
            let id = &name_matches[0].id;
            store::write_active_session(id).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({"session_id": id, "name": name_matches[0].name, "message_count": name_matches[0].message_count, "status": "active"})
                );
            } else {
                let name = name_matches[0].name.as_deref().unwrap_or("-");
                println!(
                    "Active session: {}  ({})  {} msgs",
                    id, name, name_matches[0].message_count
                );
            }
            return Ok(());
        } else if name_matches.len() > 1 {
            bail!(
                "Multiple sessions named '{}'. Use session ID instead.",
                input
            );
        }

        // Fall back to ID match (exact or prefix)
        let id_matches: Vec<&SessionSummary> = active
            .iter()
            .filter(|s| s.id == input || s.id.starts_with(&input))
            .copied()
            .collect();

        if id_matches.len() == 1 {
            let id = &id_matches[0].id;
            store::write_active_session(id).await?;
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({"session_id": id, "name": id_matches[0].name, "message_count": id_matches[0].message_count, "status": "active"})
                );
            } else {
                let name = id_matches[0].name.as_deref().unwrap_or("-");
                println!(
                    "Active session: {}  ({})  {} msgs",
                    id, name, id_matches[0].message_count
                );
            }
            return Ok(());
        } else if id_matches.len() > 1 {
            let ids: Vec<&str> = id_matches.iter().map(|s| s.id.as_str()).collect();
            bail!(
                "Multiple sessions match '{}': {}. Use a more specific ID.",
                input,
                ids.join(", ")
            );
        }

        // Check if input matches a closed session
        let closed_match: Vec<&SessionSummary> = sessions
            .iter()
            .filter(|s| s.closed && (s.id == input || s.id.starts_with(&input)))
            .collect();
        if !closed_match.is_empty() {
            bail!(
                "Session '{}' is closed. Use `tala session reopen` to continue",
                closed_match[0].id
            );
        }

        bail!("No active session named or matching '{}'", input);
    }

    match store::read_active_session().await {
        Some(id) => {
            let (host, port) = ensure_daemon_running().await?;
            let url = daemon_url(&host, port, &format!("/api/sessions/{}", id));
            let resp = reqwest::Client::new().get(&url).send().await;
            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(session) = r.json::<SessionSummary>().await {
                        if json_output {
                            println!(
                                "{}",
                                serde_json::json!({"session_id": id, "name": session.name, "message_count": session.message_count})
                            );
                        } else {
                            let name = session.name.as_deref().unwrap_or("-");
                            println!(
                                "Active session: {}  ({})  {} msgs",
                                id, name, session.message_count
                            );
                        }
                        return Ok(());
                    }
                }
                _ => {}
            }
            // Fallback if API call fails
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
                let (host, port) = ensure_daemon_running().await?;
                let url = daemon_url(&host, port, "/api/sessions");
                if let Ok(resp) = reqwest::get(&url).await {
                    if let Ok(sessions) = resp.json::<Vec<SessionSummary>>().await {
                        let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
                        if active.is_empty() {
                            println!("No active sessions. Start one with `tala send`.");
                        } else {
                            println!("Available sessions:\n");
                            for s in &active {
                                let name = s.name.as_deref().unwrap_or("-");
                                println!("  {}  {}  {} msgs", s.id, name, s.message_count);
                            }
                            println!("\nSet one with `tala use <session-id>`.");
                        }
                        return Ok(());
                    }
                }
                println!("No active session set. Use `tala use <session-id>` to set one.");
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn auto_create_session(
    host: &str,
    port: u16,
    sender_override: Option<&str>,
    quiet: bool,
    json_output: bool,
    session_name: Option<String>,
) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let url = daemon_url(host, port, "/api/sessions");
    let sender = store::get_sender_name(sender_override);
    let resp = client
        .post(&url)
        .json(&CreateSessionRequest {
            message: None,
            sender: Some(sender),
            name: session_name,
        })
        .send()
        .await?;
    let session: CreateSessionResponse = resp.json().await?;
    store::write_active_session(&session.id).await?;
    if !quiet && !json_output {
        println!("{}", session.id);
    }
    Ok(session.id)
}

async fn try_read_piped_stdin() -> Option<String> {
    tokio::time::timeout(Duration::from_millis(500), async {
        tokio::task::spawn_blocking(|| {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf).ok()?;
            let trimmed = buf.trim_end_matches('\n').to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        })
        .await
        .ok()?
    })
    .await
    .ok()?
}

#[allow(clippy::too_many_arguments)]
async fn cmd_send(
    session_arg: Option<String>,
    message: Option<String>,
    message_file: Option<String>,
    stdin_flag: bool,
    should_wait: bool,
    sender_override: Option<&str>,
    json_output: bool,
    quiet: bool,
    chat_timeout: Option<u64>,
    intent_arg: Option<&str>,
    reply_to: Option<u64>,
    expect_reply: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let has_content = message.is_some() || message_file.is_some() || stdin_flag;

    if !has_content && session_arg.is_none() && store::read_active_session().await.is_none() {
        let mut hint = String::new();
        if let Ok(req) = reqwest::get(&daemon_url(&host, port, "/api/sessions")).await {
            if let Ok(sessions) = req.json::<Vec<SessionSummary>>().await {
                let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
                if !active.is_empty() {
                    hint = format!("\nActive session: {} {}. Set it with `tala use` or target it with `tala send --session <id>`.",
                        active[0].id,
                        active[0].name.as_deref().unwrap_or(""),
                    );
                }
            }
        }
        anyhow::bail!(
            "Nothing to send. Use `tala session create` to create a session without a message.{}",
            hint
        );
    }

    // Warn if positional message starts with -- (likely shell confusion)
    if let Some(ref msg) = message {
        if msg.starts_with("--") && !quiet && !json_output {
            eprintln!("Warning: message starts with '--' which can be misinterpreted as a flag. Use '--' before the message, e.g. `tala send -- \"{}\"`, or use --stdin/--message-file.", msg);
        }
    }

    // Resolve content
    let content = if let Some(f) = message_file {
        if f == "-" {
            let piped = try_read_piped_stdin().await
                .ok_or_else(|| anyhow::anyhow!("No piped input. Use `--stdin` for explicit stdin, or provide a filename for --message-file"))?;
            piped
        } else {
            tokio::fs::read_to_string(&f)
                .await?
                .trim_end_matches('\n')
                .to_string()
        }
    } else if stdin_flag {
        try_read_piped_stdin().await.ok_or_else(|| {
            anyhow::anyhow!("No message provided via stdin (use `--stdin` flag with piped input)")
        })?
    } else if let Some(msg) = &message {
        if msg.is_empty() {
            anyhow::bail!("Message cannot be empty.");
        }
        msg.clone()
    } else {
        try_read_piped_stdin().await
            .ok_or_else(|| anyhow::anyhow!("No message provided. Use a positional argument, --message-file <path>, --stdin, or pipe to stdin"))?
    };

    // Resolve session: explicit, active, stale-replace, or auto-create
    let session_id = if let Some(id) = session_arg.clone() {
        id
    } else if let Some(id) = store::read_active_session().await {
        // Validate active session still exists and is open
        let check_url = daemon_url(&host, port, &format!("/api/sessions/{}", id));
        let check = reqwest::Client::new().get(&check_url).send().await;
        match check {
            Ok(r) if r.status().is_success() => {
                let session: Session = r.json().await?;
                if session.closed {
                    store::clear_active_session().await?;
                    let msg = format!(
                        "Session {} is closed. Use `tala session reopen {}` to reopen it.",
                        id, id
                    );
                    fail(json_output, &msg, "SESSION_CLOSED");
                }
                id
            }
            _ => {
                // Stale active session — replace with a new one
                store::clear_active_session().await?;
                auto_create_session(&host, port, sender_override, quiet, json_output, None).await?
            }
        }
    } else {
        // No active session — check if any sessions exist
        let client = reqwest::Client::new();
        let url = daemon_url(&host, port, "/api/sessions");
        let resp = client.get(&url).send().await?;
        let sessions: Vec<SessionSummary> = resp.json().await?;
        let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
        match active.len() {
            0 => {
                auto_create_session(&host, port, sender_override, quiet, json_output, None).await?
            }
            1 => {
                if !quiet && !json_output {
                    let name = active[0].name.as_deref().unwrap_or("-");
                    eprintln!(
                        "Sending to session {} ({})  {} msgs",
                        active[0].id, name, active[0].message_count
                    );
                }
                active[0].id.clone()
            }
            _ => {
                let mut msg = "No active session set.".to_string();
                for s in &active {
                    let name = s.name.as_deref().unwrap_or("-");
                    msg.push_str(&format!("\n  {}  {}", s.id, name));
                }
                msg.push_str("\nSet one with `tala use <id>`");
                fail(json_output, &msg, "NO_ACTIVE_SESSION");
            }
        }
    };

    send_content(
        session_id,
        &content,
        sender_override,
        should_wait,
        chat_timeout,
        json_output,
        quiet,
        &host,
        port,
        intent_arg,
        reply_to,
        expect_reply,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn send_content(
    session_id: String,
    content: &str,
    sender_override: Option<&str>,
    should_wait: bool,
    chat_timeout: Option<u64>,
    json_output: bool,
    quiet: bool,
    host: &str,
    port: u16,
    intent_arg: Option<&str>,
    reply_to: Option<u64>,
    expect_reply: bool,
) -> anyhow::Result<()> {
    let sender = store::get_sender_name(sender_override);
    let client = reqwest::Client::new();
    let url = daemon_url(
        host,
        port,
        &format!("/api/sessions/{}/messages", session_id),
    );

    // Resolve intent by precedence: explicit flag, then --reply-to implies reply,
    // then --wait implies req, else fyi. --reply-to + --wait implies reply+expect.
    let intent = if let Some(s) = intent_arg {
        match Intent::from_str(s) {
            Some(i) => i,
            None => fail(
                json_output,
                format!(
                    "invalid --intent '{}': must be one of req, fyi, reply, out",
                    s
                ),
                "INVALID_INTENT",
            ),
        }
    } else if reply_to.is_some() {
        Intent::Reply
    } else if should_wait {
        Intent::Req
    } else {
        Intent::Fyi
    };
    let expect_reply = expect_reply || (reply_to.is_some() && should_wait && intent_arg.is_none());
    if expect_reply && matches!(intent, Intent::Req | Intent::Out) {
        fail(
            json_output,
            "--expect-reply is only valid with intent reply or fyi",
            "INVALID_INTENT",
        );
    }

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(60);
    let effective_timeout = chat_timeout.or(Some(default_timeout));

    let req = SendMessageRequest {
        sender,
        content: content.to_string(),
        intent: Some(intent),
        reply_to,
        expect_reply,
        wait_timeout: if should_wait { effective_timeout } else { None },
    };
    let resp = client.post(&url).json(&req).send().await?;

    if !resp.status().is_success() {
        let err: ErrorResponse = resp.json().await?;
        let (msg, code) = if err.error.contains("closed") {
            (
                format!(
                    "Session {} is closed. Use `tala session reopen {}` to reopen it.",
                    session_id, session_id
                ),
                "SESSION_CLOSED",
            )
        } else {
            (err.error, "SESSION_NOT_FOUND")
        };
        fail(json_output, &msg, code);
    }

    let msg: SendMessageResponse = resp.json().await?;
    let _ = store::write_cursor(msg.id).await;

    if !should_wait {
        if json_output {
            println!("{}", serde_json::to_string(&msg).unwrap());
        } else if !quiet {
            println!("✓ Sent message {} to session {}", msg.id, msg.session_id);
        }
        return Ok(());
    }

    // Delivery receipt, then SSE wait for the reply to this message
    if json_output {
        println!(
            "{}",
            serde_json::json!({"sent": true, "message_id": msg.id, "session_id": msg.session_id})
        );
    } else if !quiet {
        println!("✓ sent (msg {}) — waiting for reply", msg.id);
    }

    let spinner = if !json_output && !quiet {
        eprint!("⏎ Waiting for reply");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        let spinner = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                eprint!(".");
                let _ = std::io::Write::flush(&mut std::io::stderr());
            }
        });
        Some(spinner)
    } else {
        if !json_output && !quiet {
            eprint!("⏎ Waiting for reply...");
            let _ = std::io::Write::flush(&mut std::io::stderr());
        }
        None
    };

    let identity = store::get_sender_name(sender_override);
    let mut wait_url = format!(
        "/api/sessions/{}/wait-stream?since={}&timeout_secs={}&reply_to={}&identity={}",
        session_id,
        msg.id,
        effective_timeout.unwrap_or(60),
        msg.id,
        identity
    );
    if let Some(to) = chat_timeout {
        wait_url = format!(
            "/api/sessions/{}/wait-stream?since={}&timeout_secs={}&reply_to={}&identity={}",
            session_id, msg.id, to, msg.id, identity
        );
    }
    let wait_url = daemon_url(host, port, &wait_url);
    let wait_resp = client.get(&wait_url).send().await?;

    let result: WaitResponse = consume_wait_stream(wait_resp, json_output).await?;

    if let Some(s) = spinner {
        s.abort();
        let _ = s.await;
    }

    if !json_output && !quiet {
        eprintln!();
    }
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
        if result.timeout {
            process::exit(2);
        }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!(
            "[timeout after {}s, no reply]",
            result.timeout_after.unwrap_or(0)
        );
        process::exit(2);
    } else {
        for m in &result.messages {
            println!("{}: {}", m.sender, m.content);
        }
    }
    Ok(())
}

/// Consumes an SSE wait stream, printing overlap/hint notes as they arrive and
/// buffering messages into a single WaitResponse. In JSON mode, overlap and hint
/// events are emitted as typed JSON lines and the WaitResponse document is
/// printed last, preserving the single-document contract.
async fn consume_wait_stream(
    resp: reqwest::Response,
    json_output: bool,
) -> anyhow::Result<WaitResponse> {
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let mut messages: Vec<Message> = Vec::new();
    let mut timeout = false;
    let mut timeout_after: Option<u64> = None;
    let mut closed = false;
    let mut cursor: Option<u64> = None;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let event_type = event_block
                .lines()
                .find_map(|line| line.strip_prefix("event: "))
                .unwrap_or("message");
            let mut data = String::new();
            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("data: ") {
                    data = val.to_string();
                }
            }

            match event_type {
                "message" => {
                    if let Ok(msg) = serde_json::from_str::<Message>(&data) {
                        cursor = Some(msg.id);
                        messages.push(msg);
                    }
                }
                "overlap" => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::json!({"event": "overlap", "overlap": serde_json::from_str::<serde_json::Value>(&data).unwrap_or_default()})
                        );
                    } else if let Ok(o) = serde_json::from_str::<WaitOverlap>(&data) {
                        let scope_desc = match &o.scope {
                            WaitScope::Session(s) => format!("session {}", s),
                            WaitScope::AnyNewSession => "a new session".to_string(),
                        };
                        eprintln!(
                            "⟳ note: {} is waiting on {} ({}s left)",
                            o.identity, scope_desc, o.remaining_secs
                        );
                    }
                }
                "result" => {
                    if let Ok(res) = serde_json::from_str::<WaitResponse>(&data) {
                        timeout = res.timeout;
                        timeout_after = res.timeout_after;
                        closed = res.closed;
                        if res.messages.is_empty() && !messages.is_empty() {
                            // buffered messages carry the result
                        } else if !res.messages.is_empty() {
                            messages = res.messages;
                        }
                        if let Some(c) = res.cursor {
                            cursor = Some(c);
                        }
                    } else if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        if v.get("timeout") == Some(&serde_json::json!(true)) {
                            timeout = true;
                            timeout_after = v["timeout_after"].as_u64();
                        }
                    }
                }
                "closed" => {
                    closed = true;
                }
                _ => {}
            }
        }
    }

    Ok(WaitResponse {
        messages,
        timeout,
        timeout_after,
        closed,
        cursor,
        overlaps: vec![],
    })
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
    let client = reqwest::Client::new();

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(60);
    let wait_timeout = timeout_secs.unwrap_or(default_timeout);

    loop {
        let sid = if let Some(id) = session_arg.clone() {
            if !json_output {
                eprintln!(
                    "Waiting for messages in session {} (timeout: {}s)...",
                    id, wait_timeout
                );
            }
            id
        } else if let Some(id) = store::read_active_session().await {
            if !json_output {
                eprintln!(
                    "Waiting for messages in session {} (timeout: {}s)...",
                    id, wait_timeout
                );
            }
            id
        } else {
            let url = daemon_url(&host, port, "/api/sessions");
            let resp = client.get(&url).send().await?;
            let sessions: Vec<SessionSummary> = resp.json().await?;
            let active: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();

            match active.len() {
                0 => {
                    if !json_output {
                        eprintln!(
                            "No active sessions. Waiting for a new session (timeout: {}s)...",
                            wait_timeout
                        );
                    }
                    let new_url = daemon_url(
                        &host,
                        port,
                        &format!("/api/sessions/wait-new?timeout_secs={}", wait_timeout),
                    );
                    let resp = client.get(&new_url).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    if json_output {
                        println!("{}", serde_json::to_string(&result).unwrap());
                        if result.get("timeout") == Some(&serde_json::json!(true)) {
                            process::exit(2);
                        }
                        return Ok(());
                    }
                    let sid_val = result
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            if result.get("timeout") == Some(&serde_json::json!(true)) {
                                anyhow::anyhow!(
                                    "timed out after {}s, no new session",
                                    result["timeout_after"].as_u64().unwrap_or(wait_timeout)
                                )
                            } else {
                                anyhow::anyhow!("failed to wait for new session")
                            }
                        })?
                        .to_string();
                    store::write_active_session(&sid_val).await?;
                    eprintln!("New session: {}", sid_val);
                    sid_val
                }
                1 => {
                    let sid_val = active[0].id.clone();
                    if !json_output {
                        eprintln!(
                            "Waiting for new messages in session {} (timeout: {}s)...",
                            sid_val, wait_timeout
                        );
                    }
                    sid_val
                }
                _ => {
                    if json_output {
                        let sessions_json: Vec<serde_json::Value> = active
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "id": s.id,
                                    "name": s.name,
                                    "message_count": s.message_count
                                })
                            })
                            .collect();
                        println!(
                            "{}",
                            serde_json::json!({
                                "sessions": sessions_json,
                                "error": "Use 'tala use <id>' to select a session"
                            })
                        );
                    } else {
                        println!("Multiple open sessions. Use `tala use <id>` to select one:\n");
                        for s in &active {
                            let name = s.name.as_deref().unwrap_or("-");
                            println!("  {}  {}  {} msgs", s.id, name, s.message_count);
                        }
                    }
                    process::exit(0);
                }
            }
        };

        let since_id = if let Some(s) = since {
            s
        } else {
            let msgs_url = daemon_url(
                &host,
                port,
                &format!("/api/sessions/{}/messages?since=0", sid),
            );
            match client.get(&msgs_url).send().await {
                Ok(resp) => {
                    let msgs: Vec<Message> = resp.json().await.unwrap_or_default();
                    msgs.iter().map(|m| m.id).max().unwrap_or(0)
                }
                Err(_) => 0,
            }
        };

        let mut path = format!(
            "/api/sessions/{}/wait-stream?since={}&timeout_secs={}&identity={}",
            sid,
            since_id,
            wait_timeout,
            store::get_sender_name(None)
        );
        if let Some(l) = limit.filter(|&l| l > 0) {
            path = format!("{}&limit={}", path, l);
        }
        if let Some(ref f) = from {
            path = format!("{}&from={}", path, f);
        }

        let url = daemon_url(&host, port, &path);

        let spinner = if !json_output {
            let spinner = tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(5));
                loop {
                    interval.tick().await;
                    eprint!(".");
                    let _ = std::io::Write::flush(&mut std::io::stderr());
                }
            });
            Some(spinner)
        } else {
            None
        };

        let resp = client.get(&url).send().await?;

        if let Some(s) = spinner {
            s.abort();
            let _ = s.await;
        }

        if !resp.status().is_success() {
            let err: ErrorResponse = resp.json().await?;
            if session_arg.is_none() && err.error.to_lowercase().contains("session not found") {
                store::clear_active_session().await?;
                if !json_output {
                    eprintln!("Active session was stale. Re-discovering...");
                }
                continue;
            }
            fail(json_output, &err.error, "SESSION_NOT_FOUND");
        }

        let result: WaitResponse = consume_wait_stream(resp, json_output).await?;

        if json_output {
            println!("{}", serde_json::to_string(&result).unwrap());
            if result.timeout {
                process::exit(2);
            }
        } else if result.closed {
            println!("[session closed]");
        } else if result.timeout {
            println!(
                "timeout after {}s, no new messages",
                result.timeout_after.unwrap_or(0)
            );
            let _ = print_unread_hint(&host, port).await;
            process::exit(2);
        } else {
            let _ = store::write_active_session(&sid).await;
            for msg in &result.messages {
                println!(
                    "[sess {}] [{}] {}{} ({}):\n    {}",
                    sid,
                    msg.id,
                    intent_badge(msg),
                    msg.sender,
                    msg.timestamp.format("%H:%M:%S"),
                    msg.content
                );
            }
        }

        break;
    }
    Ok(())
}

async fn cmd_watch(
    session_arg: Option<String>,
    since: Option<u64>,
    limit: Option<usize>,
    json_output: bool,
    timeout: Option<u64>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "stream").await?;

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

    let timeout_dur = timeout.filter(|&t| t > 0).map(Duration::from_secs);

    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let mut message_count: u64 = 0;

    loop {
        let chunk = if let Some(dur) = timeout_dur {
            match tokio::time::timeout(dur, stream.next()).await {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(_) => break,
            }
        } else {
            match stream.next().await {
                Some(chunk) => chunk,
                None => break,
            }
        };
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let event_type = event_block
                .lines()
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
                    message_count += 1;
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
                        println!(
                            "[{}] {} {} ({}):{}",
                            msg.id,
                            intent_badge(&msg),
                            msg.sender,
                            msg.timestamp.format("%H:%M:%S"),
                            render_deadline(&msg)
                        );
                        println!("    {}", msg.content);
                    }
                }
                _ => {}
            }
        }
    }

    if message_count == 0 {
        if json_output {
            println!("[]");
        } else {
            println!("[no messages received]");
        }
    }

    Ok(())
}

async fn cmd_listen(
    since: Option<u64>,
    match_str: Option<String>,
    from: Option<String>,
    name: Option<String>,
    timeout: Option<u64>,
    json_output: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let since_id = if let Some(s) = since {
        s
    } else {
        store::read_cursor().await
    };
    let mut path = format!("/api/observe?since={}", since_id);
    if let Some(ref m) = match_str {
        path = format!("{}&match={}", path, urlencoding(m));
    }
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }
    if let Some(ref n) = name {
        path = format!("{}&channel={}", path, n);
    }
    // Default timeout to 60s if not specified, unless explicitly set to 0
    let timeout_secs = timeout.filter(|&t| t != 0).or(Some(60u64));
    if let Some(t) = timeout_secs {
        path = format!("{}&timeout_secs={}", path, t);
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
    let mut max_msg_id = since_id;

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
                            if msg.id > max_msg_id {
                                max_msg_id = msg.id;
                            }
                            let session_label = evt.session_name.unwrap_or(evt.session_id);
                            println!(
                                "[{}] {} {} ({}):{}",
                                session_label,
                                intent_badge(&msg),
                                msg.sender,
                                msg.timestamp.format("%H:%M:%S"),
                                render_deadline(&msg)
                            );
                            println!("    {}", msg.content);
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

    if max_msg_id > since_id {
        let _ = store::write_cursor(max_msg_id).await;
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
        println!(
            "session: {}  |  created: {}  |  closed: {}",
            recap.session.id,
            recap.session.created_at.format("%Y-%m-%d %H:%M:%S"),
            recap.session.closed
        );
        if let Some(c) = recap.cursor {
            println!("cursor: {}", c);
        }
        println!();
        for msg in &recap.messages {
            println!(
                "[{}] {} {} ({}):{}",
                msg.id,
                intent_badge(msg),
                msg.sender,
                msg.timestamp.format("%H:%M:%S"),
                render_deadline(msg)
            );
            println!("    {}\n", msg.content);
        }
    }
    store::write_cursor(recap.cursor.unwrap_or(0)).await?;
    Ok(())
}

async fn cmd_pending(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/pending");
    let resp = client.get(&url).send().await?;
    let obligations: Vec<PendingObligation> = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&obligations).unwrap());
    } else if obligations.is_empty() {
        println!("Nothing pending — every request has been answered.");
    } else {
        for o in &obligations {
            let label = o.session_name.as_deref().unwrap_or(&o.session_id);
            let deadline = match o.waiting_until {
                Some(until) => {
                    let remaining = (until - chrono::Utc::now()).num_seconds();
                    if remaining >= 0 {
                        format!(" (waiting, {}s left)", remaining)
                    } else {
                        " (wait expired)".to_string()
                    }
                }
                None => String::new(),
            };
            println!(
                "[{}] [{}] [{}] {}{}: {}",
                label,
                o.message_id,
                o.intent.badge(),
                o.sender,
                deadline,
                o.content
            );
            let mins = o.elapsed_seconds / 60;
            let secs = o.elapsed_seconds % 60;
            println!(
                "      unanswered for {}{} — answer with `tala send --reply-to {}`",
                if mins > 0 {
                    format!("{}m ", mins)
                } else {
                    String::new()
                },
                secs,
                o.message_id
            );
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

    let cursor = store::read_cursor().await;
    let active_session = store::read_active_session().await;
    let pending: Vec<PendingObligation> = {
        let client = reqwest::Client::new();
        let url = daemon_url(&host, port, "/api/pending");
        match client.get(&url).send().await {
            Ok(resp) => resp.json().await.unwrap_or_default(),
            Err(_) => vec![],
        }
    };
    let waits: Vec<ActiveWaitInfo> = {
        let client = reqwest::Client::new();
        let url = daemon_url(&host, port, "/api/waits");
        match client.get(&url).send().await {
            Ok(resp) => resp.json().await.unwrap_or_default(),
            Err(_) => vec![],
        }
    };

    if json_output {
        let mut enriched: Vec<serde_json::Value> = Vec::new();
        for s in &sessions {
            let unread = if s.closed {
                0
            } else {
                compute_session_unread(&host, port, s, cursor).await
            };
            let mut entry = serde_json::to_value(s).unwrap_or_default();
            if let Some(obj) = entry.as_object_mut() {
                obj.insert("unread_count".to_string(), serde_json::json!(unread));
                obj.insert(
                    "active".to_string(),
                    serde_json::json!(active_session.as_deref() == Some(&s.id)),
                );
                let pending_count = pending.iter().filter(|p| p.session_id == s.id).count();
                obj.insert(
                    "pending_count".to_string(),
                    serde_json::json!(pending_count),
                );
                let waiting = waits
                    .iter()
                    .filter(|w| matches!(&w.scope, WaitScope::Session(sid) if sid == &s.id))
                    .count();
                obj.insert("waiting".to_string(), serde_json::json!(waiting));
            }
            enriched.push(entry);
        }
        println!("{}", serde_json::to_string(&enriched).unwrap());
    } else if sessions.is_empty() {
        println!("No sessions");
    } else {
        let name_width = sessions
            .iter()
            .map(|s| s.name.as_deref().unwrap_or("-").len())
            .max()
            .unwrap_or(1)
            .max(4);
        for s in &sessions {
            let status = if s.closed { "closed" } else { "active" };
            let name = s.name.as_deref().unwrap_or("-");
            let marker = if active_session.as_deref() == Some(&s.id) {
                " *"
            } else {
                "  "
            };
            let pending_count = pending.iter().filter(|p| p.session_id == s.id).count();
            let waiting = waits
                .iter()
                .filter(|w| matches!(&w.scope, WaitScope::Session(sid) if sid == &s.id))
                .count();
            if s.closed {
                println!(
                    "{}  {:width$}  {}  {} msgs{}",
                    s.id,
                    name,
                    status,
                    s.message_count,
                    marker,
                    width = name_width
                );
            } else {
                let unread = compute_session_unread(&host, port, s, cursor).await;
                let mut extra = Vec::new();
                if unread > 0 {
                    extra.push(format!("{} new", unread));
                }
                if pending_count > 0 {
                    extra.push(format!("{} pending", pending_count));
                }
                if waiting > 0 {
                    extra.push(format!("{} waiting", waiting));
                }
                let extra_str = if extra.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", extra.join(", "))
                };
                println!(
                    "{}  {:width$}  {}  {} msgs{}{}",
                    s.id,
                    name,
                    status,
                    s.message_count,
                    extra_str,
                    marker,
                    width = name_width
                );
            }
        }
    }
    Ok(())
}

async fn compute_session_unread(
    host: &str,
    port: u16,
    session: &SessionSummary,
    cursor: u64,
) -> usize {
    if cursor == 0 && session.message_count == 0 {
        return 0;
    }
    let local_agent = store::read_project_config()
        .await
        .or_else(|| Some(store::get_default_sender()));
    let client = reqwest::Client::new();
    let msgs_url = daemon_url(
        host,
        port,
        &format!("/api/sessions/{}/messages?since={}", session.id, cursor),
    );
    match client.get(&msgs_url).send().await {
        Ok(resp) => {
            let msgs: Vec<Message> = resp.json().await.unwrap_or_default();
            if let Some(ref agent) = local_agent {
                msgs.iter().filter(|m| m.sender != *agent).count()
            } else {
                msgs.len()
            }
        }
        Err(_) => 0,
    }
}

async fn check_tcp_port(host: &str, port: u16) -> bool {
    tokio::net::TcpStream::connect((host, port)).await.is_ok()
}

async fn probe_daemon(host: &str, port: u16, agents: &mut Vec<AgentSummary>) -> bool {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    let url = format!("http://{}:{}/api/agents", host, port);
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            *agents = resp.json::<Vec<AgentSummary>>().await.unwrap_or_default();
            true
        }
        _ => check_tcp_port(host, port).await,
    }
}

async fn try_read_json(path: &std::path::Path) -> Option<serde_json::Value> {
    tokio::fs::read_to_string(path)
        .await
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[derive(serde::Serialize)]
struct DiscoveredProject {
    project: String,
    agent_name: String,
    daemon_running: bool,
    agents: Vec<AgentSummary>,
}

async fn cmd_discover(json_output: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let mut discovered: Vec<DiscoveredProject> = Vec::new();

    // Walk up parent directories looking for .tala/config.json
    let mut current = Some(cwd.as_path());
    let mut checked = std::collections::HashSet::new();
    for _ in 0..4 {
        let dir = match current {
            Some(d) => d,
            None => break,
        };
        let tala_config = dir.join(".tala").join("config.json");
        if tala_config.exists() && checked.insert(dir.to_path_buf()) {
            if let Some(config) = try_read_json(&tala_config).await {
                let agent_name = config["name"].as_str().unwrap_or("unknown").to_string();
                let daemon_path = store::tala_home().join("daemon.json");
                let mut daemon_running = false;
                let mut agents: Vec<AgentSummary> = Vec::new();
                if let Some(dinfo) = try_read_json(&daemon_path).await {
                    let host = dinfo["host"].as_str().unwrap_or("127.0.0.1");
                    let port = dinfo["port"].as_u64().unwrap_or(0) as u16;
                    if port > 0 {
                        daemon_running = probe_daemon(host, port, &mut agents).await;
                    }
                }
                discovered.push(DiscoveredProject {
                    project: dir.display().to_string(),
                    agent_name,
                    daemon_running,
                    agents,
                });
            }
        }

        // Check siblings
        if let Some(parent) = dir.parent() {
            if let Ok(mut entries) = tokio::fs::read_dir(parent).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_dir() && path != dir && checked.insert(path.clone()) {
                        let sibling_config = path.join(".tala").join("config.json");
                        if sibling_config.exists() {
                            if let Some(config) = try_read_json(&sibling_config).await {
                                let agent_name =
                                    config["name"].as_str().unwrap_or("unknown").to_string();
                                let daemon_path = store::tala_home().join("daemon.json");
                                let mut daemon_running = false;
                                let mut agents: Vec<AgentSummary> = Vec::new();
                                if let Some(dinfo) = try_read_json(&daemon_path).await {
                                    let host = dinfo["host"].as_str().unwrap_or("127.0.0.1");
                                    let port = dinfo["port"].as_u64().unwrap_or(0) as u16;
                                    if port > 0 {
                                        daemon_running =
                                            probe_daemon(host, port, &mut agents).await;
                                    }
                                }
                                discovered.push(DiscoveredProject {
                                    project: path.display().to_string(),
                                    agent_name,
                                    daemon_running,
                                    agents,
                                });
                            }
                        }
                    }
                }
            }
        }

        current = dir.parent();
    }

    if json_output {
        println!("{}", serde_json::to_string(&discovered).unwrap());
    } else if discovered.is_empty() {
        println!("No other tala projects discovered in parent directories.");
    } else {
        for p in &discovered {
            let daemon_status = if p.daemon_running {
                "running"
            } else {
                "stopped"
            };
            println!(
                "{}  ({})  [daemon: {}]",
                p.project, p.agent_name, daemon_status
            );
            if p.daemon_running && !p.agents.is_empty() {
                for a in &p.agents {
                    println!(
                        "  └ {}  last: {}  {} msgs",
                        a.sender,
                        a.last_seen.format("%Y-%m-%d %H:%M:%S UTC"),
                        a.message_count
                    );
                }
            }
        }
    }

    Ok(())
}

async fn cmd_agents(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, "/api/agents");
    let resp = client.get(&url).send().await?;
    let agents: Vec<AgentSummary> = resp.json().await?;

    if json_output {
        println!("{}", serde_json::to_string(&agents).unwrap());
    } else if agents.is_empty() {
        println!("No active agents found. Start a session with `tala send`, or try `tala discover` to find agents in other projects.");
    } else {
        for a in &agents {
            println!(
                "{}  last: {}  {} msgs",
                a.sender,
                a.last_seen.format("%Y-%m-%d %H:%M:%S UTC"),
                a.message_count
            );
        }
    }
    Ok(())
}

async fn cmd_close(
    session_arg: Option<String>,
    json_output: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_id(&host, port, session_arg.as_deref(), "close").await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.delete(&url).send().await?;

    if resp.status().is_success() {
        let result: CloseSessionResponse = resp.json().await?;
        let was_active = session_arg.is_none()
            && store::read_active_session().await.as_deref() == Some(&session_id);
        if was_active {
            store::clear_active_session().await?;
        }
        if json_output {
            let mut out = serde_json::json!({"session_id": session_id, "status": result.status});
            if was_active {
                out["active_cleared"] = serde_json::json!(true);
            }
            println!("{}", out);
        } else if !quiet {
            println!("Session {}: {}", session_id, result.status);
            if was_active {
                eprintln!("Active session was closed and cleared. Use `tala use <session-id>` to set a new one.");
            }
        }
    } else {
        let err: ErrorResponse = resp.json().await?;
        let code = if err.error.contains("closed") {
            "SESSION_CLOSED"
        } else {
            "SESSION_NOT_FOUND"
        };
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
        println!(
            "  Created: {}",
            session.created_at.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "  Last activity: {}",
            session.last_activity.format("%Y-%m-%d %H:%M:%S")
        );
        println!(
            "  Status: {}",
            if session.closed { "closed" } else { "active" }
        );
    }
    Ok(())
}

async fn cmd_session_rename(
    session_id: String,
    name: String,
    json_output: bool,
    force: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/rename", session_id));
    let resp = client
        .post(&url)
        .json(&json!({"name": name, "force": force}))
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        let err: ErrorResponse = resp.json().await?;
        match status.as_u16() {
            409 => fail(json_output, &err.error, "SESSION_ALREADY_NAMED"),
            _ => fail(json_output, &err.error, "SESSION_NOT_FOUND"),
        }
    }

    let result: serde_json::Value = resp.json().await?;
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!(
            "Session {} renamed to '{}'",
            session_id,
            result["name"].as_str().unwrap_or("")
        );
    }
    Ok(())
}

async fn cmd_session_reopen(session_id: String, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/reopen", session_id));
    let resp = client.post(&url).send().await?;

    let status = resp.status();
    if !status.is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let result: serde_json::Value = resp.json().await?;
    store::write_active_session(&session_id).await?;
    if json_output {
        let mut out = result;
        out["active"] = serde_json::json!(true);
        println!("{}", serde_json::to_string(&out).unwrap());
    } else {
        println!("Session {} reopened (now active)", session_id);
    }
    Ok(())
}

async fn cmd_session_create(session_name: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    auto_create_session(&host, port, None, false, json_output, session_name).await?;
    Ok(())
}

async fn cmd_wait_new(timeout_secs: Option<u64>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let timeout = timeout_secs.unwrap_or(60);
    if !json_output {
        eprintln!("Waiting for a new session (timeout: {}s)...", timeout);
    }
    let _ = print_unread_hint(&host, port).await;
    let url = daemon_url(
        &host,
        port,
        &format!(
            "/api/sessions/wait-new-stream?timeout_secs={}&identity={}",
            timeout,
            store::get_sender_name(None)
        ),
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let result: serde_json::Value = consume_wait_new_stream(resp, json_output).await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
        println!("{}", sid);
    } else if result.get("timeout") == Some(&serde_json::json!(true)) {
        eprintln!(
            "timeout after {}s, no new session",
            result["timeout_after"].as_u64().unwrap_or(timeout)
        );
        let _ = print_unread_hint(&host, port).await;
        process::exit(2);
    } else if let Some(err) = result.get("error").and_then(|v| v.as_str()) {
        fail(json_output, err, "WAIT_NEW_ERROR");
    }
    Ok(())
}

/// Consumes the SSE wait-new stream; returns the terminal result JSON
/// (session_id payload or timeout marker). Overlap notes are printed live.
async fn consume_wait_new_stream(
    resp: reqwest::Response,
    json_output: bool,
) -> anyhow::Result<serde_json::Value> {
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let mut result: serde_json::Value = serde_json::Value::Null;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(pos) = buffer.find("\n\n") {
            let event_block = buffer[..pos].to_string();
            buffer = buffer[pos + 2..].to_string();

            let event_type = event_block
                .lines()
                .find_map(|line| line.strip_prefix("event: "))
                .unwrap_or("message");
            let mut data = String::new();
            for line in event_block.lines() {
                if let Some(val) = line.strip_prefix("data: ") {
                    data = val.to_string();
                }
            }

            match event_type {
                "overlap" => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::json!({"event": "overlap", "overlap": serde_json::from_str::<serde_json::Value>(&data).unwrap_or_default()})
                        );
                    } else if let Ok(o) = serde_json::from_str::<WaitOverlap>(&data) {
                        let scope_desc = match &o.scope {
                            WaitScope::Session(s) => format!("session {}", s),
                            WaitScope::AnyNewSession => "a new session".to_string(),
                        };
                        eprintln!(
                            "⟳ note: {} is waiting on {} ({}s left) — this wait will not receive that session's messages",
                            o.identity, scope_desc, o.remaining_secs
                        );
                    }
                }
                "result" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                        result = v;
                    }
                }
                _ => {}
            }
        }
    }

    Ok(result)
}

/// Renders the intent badge + reply target, e.g. "[REQ]" or "[REPLY→3]".
fn intent_badge(msg: &Message) -> String {
    let base = msg.intent.badge();
    match msg.reply_to {
        Some(target) => format!("[{}→{}]", base, target),
        None => format!("[{}]", base),
    }
}

/// Renders the waiting_until deadline relative to now, e.g. " (waiting, 83s left)".
fn render_deadline(msg: &Message) -> String {
    match msg.waiting_until {
        Some(until) => {
            let now = chrono::Utc::now();
            let remaining = (until - now).num_seconds();
            if remaining >= 0 {
                format!(" (waiting, {}s left)", remaining)
            } else {
                let mins = (-remaining) / 60;
                let secs = -remaining % 60;
                if mins > 0 {
                    format!(" (wait expired {}m{}s ago)", mins, secs)
                } else {
                    format!(" (wait expired {}s ago)", secs)
                }
            }
        }
        None => String::new(),
    }
}

/// Prints a hint when sessions exist with unread messages from other senders.
async fn print_unread_hint(host: &str, port: u16) -> anyhow::Result<()> {
    let cursor = store::read_cursor().await;
    let local_agent = store::read_project_config()
        .await
        .or_else(|| Some(store::get_default_sender()));
    let client = reqwest::Client::new();
    let url = daemon_url(host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;
    let mut named: Vec<String> = Vec::new();
    for s in &sessions {
        if s.closed {
            continue;
        }
        let msgs_url = daemon_url(
            host,
            port,
            &format!("/api/sessions/{}/messages?since={}", s.id, cursor),
        );
        if let Ok(resp) = client.get(&msgs_url).send().await {
            if let Ok(msgs) = resp.json::<Vec<Message>>().await {
                let unread = match &local_agent {
                    Some(agent) => msgs.iter().filter(|m| m.sender != *agent).count(),
                    None => msgs.len(),
                };
                if unread > 0 {
                    named.push(s.id.clone());
                }
            }
        }
    }
    if !named.is_empty() {
        let list = named.join(", ");
        eprintln!(
            "⟳ hint: {} session(s) with an unread message ({}) — run `tala check`",
            named.len(),
            list
        );
    }
    Ok(())
}

async fn cmd_status(json_output: bool) -> anyhow::Result<()> {
    let info = match store::read_daemon_json().await {
        Ok(info) => info,
        Err(_) => {
            let home = daemon_home_display();
            if json_output {
                println!("{}", serde_json::json!({"running": false, "home": home}));
            } else {
                println!("no daemon running (checked {}/daemon.json)", home);
                println!("Start the daemon by running any tala command, or set TALA_HOME if using a custom location");
            }
            return Ok(());
        }
    };

    let status_url = daemon_url(&info.host, info.port, "/api/status");
    let alive = reqwest::Client::new()
        .get(&status_url)
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if alive {
        let cursor = store::read_cursor().await;
        let total_unread = compute_total_unread(&info.host, info.port, cursor).await;
        let waits: Vec<ActiveWaitInfo> = {
            let client = reqwest::Client::new();
            let url = daemon_url(&info.host, info.port, "/api/waits");
            match client.get(&url).send().await {
                Ok(resp) => resp.json().await.unwrap_or_default(),
                Err(_) => vec![],
            }
        };

        if json_output {
            let resp = serde_json::json!({
                "running": true,
                "pid": info.pid,
                "port": info.port,
                "host": info.host,
                "started_at": info.started_at,
                "total_unread": total_unread,
                "active_waits": waits,
            });
            println!("{}", serde_json::to_string(&resp).unwrap());
        } else {
            println!("daemon running:");
            println!("  PID:  {}", info.pid);
            println!("  Port: {}", info.port);
            println!("  Host: {}", info.host);
            println!("  Since: {}", info.started_at.format("%Y-%m-%d %H:%M:%S"));
            if total_unread > 0 {
                println!(
                    "  Unread: {} new message(s) across all sessions",
                    total_unread
                );
            } else {
                println!("  Unread: 0 new messages");
            }
            if !waits.is_empty() {
                println!("  Waiting now:");
                for w in &waits {
                    let scope_desc = match &w.scope {
                        WaitScope::Session(s) => format!("session {}", s),
                        WaitScope::AnyNewSession => "a new session".to_string(),
                    };
                    println!(
                        "    {}  → {}  ({}s left)",
                        w.identity, scope_desc, w.remaining_secs
                    );
                }
            }
        }
    } else {
        let home = daemon_home_display();
        if json_output {
            println!(
                "{}",
                serde_json::json!({"running": false, "stale_daemon_json": true, "home": home})
            );
        } else {
            println!("daemon.json found at {}/daemon.json but daemon is not reachable (may have crashed)", home);
            println!("Try `tala stop` to clean up stale daemon.json, then run your command again.");
        }
    }
    Ok(())
}

async fn compute_total_unread(host: &str, port: u16, cursor: u64) -> usize {
    let local_agent = store::read_project_config().await;
    let client = reqwest::Client::new();
    let url = daemon_url(host, port, "/api/sessions");
    match client.get(&url).send().await {
        Ok(resp) => {
            let sessions: Vec<SessionSummary> = resp.json().await.unwrap_or_default();
            let mut total = 0;
            for s in &sessions {
                let msgs_url = daemon_url(
                    host,
                    port,
                    &format!("/api/sessions/{}/messages?since={}", s.id, cursor),
                );
                if let Ok(resp) = client.get(&msgs_url).send().await {
                    if let Ok(msgs) = resp.json::<Vec<Message>>().await {
                        if let Some(ref agent) = local_agent {
                            total += msgs.iter().filter(|m| m.sender != *agent).count();
                        } else {
                            total += msgs.len();
                        }
                    }
                }
            }
            total
        }
        Err(_) => 0,
    }
}

async fn cmd_whatsup(json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let cursor = store::read_cursor().await;
    let client = reqwest::Client::new();

    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    let mut all_messages: Vec<Message> = Vec::new();

    for s in &sessions {
        let msgs_url = daemon_url(
            &host,
            port,
            &format!("/api/sessions/{}/messages?since={}", s.id, cursor),
        );
        if let Ok(resp) = client.get(&msgs_url).send().await {
            if let Ok(msgs) = resp.json::<Vec<Message>>().await {
                all_messages.extend(msgs);
            }
        }
    }

    all_messages.sort_by_key(|m| m.id);

    let new_cursor = all_messages.iter().map(|m| m.id).max().unwrap_or(cursor);

    if json_output {
        let result = serde_json::json!({
            "cursor": new_cursor,
            "messages": all_messages,
        });
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if all_messages.is_empty() {
        println!("No new messages since last check (cursor: {})", cursor);
    } else {
        // Group messages by session
        let mut by_session: std::collections::BTreeMap<String, Vec<&Message>> =
            std::collections::BTreeMap::new();
        for msg in &all_messages {
            by_session
                .entry(msg.session_id.clone())
                .or_default()
                .push(msg);
        }
        for (sid, msgs) in &by_session {
            // Find session name
            let session_name = sessions
                .iter()
                .find(|s| s.id == *sid)
                .and_then(|s| s.name.clone())
                .unwrap_or_else(|| sid.clone());
            println!("[{}] ({} new message(s))", session_name, msgs.len());
            for msg in msgs {
                println!(
                    "  [{}] {}{} ({}):{}",
                    msg.id,
                    intent_badge(msg),
                    msg.sender,
                    msg.timestamp.format("%H:%M:%S"),
                    render_deadline(msg)
                );
                println!("    {}", msg.content);
            }
            println!();
        }
    }

    store::write_cursor(new_cursor).await?;

    if !json_output && !all_messages.is_empty() {
        println!("(cursor updated to {})", new_cursor);
    }

    Ok(())
}

async fn cmd_stop() -> anyhow::Result<()> {
    #[cfg(not(unix))]
    {
        let _ = store::read_daemon_json().await;
        bail!("stop is not supported on this platform");
    }

    #[cfg(unix)]
    {
        let info = match store::read_daemon_json().await {
            Ok(info) => info,
            Err(_) => {
                println!("daemon is not running");
                return Ok(());
            }
        };
        use std::process::Command;
        let kill_status = Command::new("kill")
            .arg(info.pid.to_string())
            .status()
            .context("failed to run kill")?;

        if !kill_status.success() {
            // Process already gone — clean up stale daemon.json
            store::remove_daemon_json().await;
            println!("daemon stopped");
            return Ok(());
        }

        for _ in 0..20 {
            if store::read_daemon_json().await.is_err() {
                println!("daemon stopped");
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        // Timed out — clean up stale daemon.json and report success
        store::remove_daemon_json().await;
        println!("daemon stopped (stale daemon.json cleaned up)");
        Ok(())
    }
}
