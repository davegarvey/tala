use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
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

/// Exit code for benign blocking timeouts (wait/send --wait running out of
/// time with no new messages). Deliberately distinct from clap's usage-error
/// code 2: a timeout is not a usage error, and scripts must be able to tell
/// "nothing happened yet" apart from "you called me wrong".
const EXIT_TIMEOUT: i32 = 3;

const TALA_SKILL_MIN_VERSION: &str = "0.27.3";
const TALA_CLI_MIN_VERSION_PLACEHOLDER: &str = "__TALA_CLI_MIN_VERSION__";
const TALA_CLI_GENERATED_VERSION_PLACEHOLDER: &str = "__TALA_CLI_GENERATED_VERSION__";

#[derive(Debug, Eq, PartialEq)]
struct SemanticVersion {
    major: u64,
    minor: u64,
    patch: u64,
    prerelease: Vec<PrereleaseIdentifier>,
}

#[derive(Debug, Eq, PartialEq)]
enum PrereleaseIdentifier {
    Numeric(u64),
    Text(String),
}

impl Ord for PrereleaseIdentifier {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Numeric(left), Self::Numeric(right)) => left.cmp(right),
            (Self::Numeric(_), Self::Text(_)) => Ordering::Less,
            (Self::Text(_), Self::Numeric(_)) => Ordering::Greater,
            (Self::Text(left), Self::Text(right)) => left.cmp(right),
        }
    }
}

impl PartialOrd for PrereleaseIdentifier {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemanticVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
            .then_with(
                || match (self.prerelease.is_empty(), other.prerelease.is_empty()) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    (false, false) => self
                        .prerelease
                        .iter()
                        .zip(&other.prerelease)
                        .map(|(left, right)| left.cmp(right))
                        .find(|ordering| *ordering != Ordering::Equal)
                        .unwrap_or_else(|| self.prerelease.len().cmp(&other.prerelease.len())),
                },
            )
    }
}

impl PartialOrd for SemanticVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_semantic_version(value: &str) -> anyhow::Result<SemanticVersion> {
    let (without_build, build) = match value.split_once('+') {
        Some((core, build)) if !build.is_empty() => (core, build),
        Some(_) => bail!("invalid semantic version '{}': empty build metadata", value),
        None => (value, ""),
    };
    validate_version_identifiers(build, "build metadata")?;

    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, prerelease)) if !prerelease.is_empty() => (core, prerelease),
        Some(_) => bail!("invalid semantic version '{}': empty prerelease", value),
        None => (without_build, ""),
    };
    let core_parts: Vec<&str> = core.split('.').collect();
    if core_parts.len() != 3 {
        bail!(
            "invalid semantic version '{}': expected MAJOR.MINOR.PATCH",
            value
        );
    }

    let major = parse_version_number(core_parts[0], value)?;
    let minor = parse_version_number(core_parts[1], value)?;
    let patch = parse_version_number(core_parts[2], value)?;
    let prerelease = if prerelease.is_empty() {
        Vec::new()
    } else {
        validate_version_identifiers(prerelease, "prerelease")?;
        prerelease
            .split('.')
            .map(|identifier| {
                if identifier.chars().all(|c| c.is_ascii_digit()) {
                    if identifier.len() > 1 && identifier.starts_with('0') {
                        bail!(
                            "invalid semantic version '{}': numeric prerelease identifiers cannot have leading zeroes",
                            value
                        );
                    }
                    Ok(PrereleaseIdentifier::Numeric(identifier.parse().map_err(
                        |_| anyhow::anyhow!("numeric prerelease identifier is too large"),
                    )?))
                } else {
                    Ok(PrereleaseIdentifier::Text(identifier.to_string()))
                }
            })
            .collect::<anyhow::Result<Vec<_>>>()?
    };

    Ok(SemanticVersion {
        major,
        minor,
        patch,
        prerelease,
    })
}

fn parse_version_number(value: &str, full_version: &str) -> anyhow::Result<u64> {
    if value.is_empty()
        || !value.chars().all(|character| character.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        bail!(
            "invalid semantic version '{}': invalid numeric component",
            full_version
        );
    }
    value
        .parse()
        .map_err(|_| anyhow::anyhow!("numeric component is too large"))
}

fn validate_version_identifiers(value: &str, label: &str) -> anyhow::Result<()> {
    if value.is_empty() {
        return Ok(());
    }
    if value.split('.').any(|identifier| {
        identifier.is_empty()
            || !identifier
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
    }) {
        bail!("invalid {} in semantic version", label);
    }
    Ok(())
}

fn replace_placeholder_once(
    template: &str,
    placeholder: &str,
    replacement: &str,
) -> anyhow::Result<String> {
    match template.matches(placeholder).count() {
        0 => bail!("template is missing required placeholder '{}'", placeholder),
        1 => Ok(template.replace(placeholder, replacement)),
        count => bail!(
            "template contains {} copies of required placeholder '{}'; expected one",
            count,
            placeholder
        ),
    }
}

fn render_integration_document(
    template: &str,
    min_version: &str,
    generated_version: &str,
) -> anyhow::Result<String> {
    let min = parse_semantic_version(min_version)?;
    let generated = parse_semantic_version(generated_version)?;
    if min > generated {
        bail!(
            "skill minimum CLI version {} is newer than generating CLI version {}",
            min_version,
            generated_version
        );
    }

    let with_min =
        replace_placeholder_once(template, TALA_CLI_MIN_VERSION_PLACEHOLDER, min_version)?;
    replace_placeholder_once(
        &with_min,
        TALA_CLI_GENERATED_VERSION_PLACEHOLDER,
        generated_version,
    )
}

fn render_integration_documents(
    skill_template: &str,
    command_template: &str,
    min_version: &str,
    generated_version: &str,
) -> anyhow::Result<(String, String)> {
    let skill = render_integration_document(skill_template, min_version, generated_version)?;
    let command = render_integration_document(command_template, min_version, generated_version)?;
    Ok((skill, command))
}

#[derive(Parser)]
#[command(
    name = "tala",
    about = "Agent-to-agent messaging for AI coding tools",
    long_about = "tala is a lightweight messaging tool for AI agents working across projects.\n\nSend messages with `tala send`, wait for replies with `tala wait`, or listen to all sessions with `tala listen`.\n\nUse `tala wait --new-session` to wait for a session with an unread incoming message from another agent (new sessions first, then sessions you have participated in).\n\nEvery command supports --json for structured output.",
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
        after_help = "See also: tala session (create, rename, reopen) for advanced session management"
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
        after_help = "Use --wait / -w to block until a reply arrives.\nUse `tala send --name <label>` to create a named session in one command, or `tala session create --name` for an empty one.\nUse --stdin or pipe content for messages with special characters (backticks, quotes, leading dashes).\nUse `--` to separate options from message content, e.g. `tala send -- --my-flags`.\n\nINTENT PRECEDENCE: explicit --intent wins; --reply-to implies reply; --wait implies req; default fyi. With --wait --timeout N, recipients see the live countdown via the stamped waiting_until.\n\nEXIT CODES: 0 = sent (or reply received with --wait); 3 = --wait timed out; 2 = usage error; 1 = error"
    )]
    Send {
        #[arg(help = "Session ID (positional, or use --session/-s)")]
        session: Option<String>,
        #[arg(long = "session", short, alias = "session-id", help = "Session ID")]
        session_arg: Option<String>,
        #[arg(help = "Message content (omit to read from piped stdin)")]
        message: Option<String>,
        #[arg(
            long,
            help = "Create a new named session, send there, and set it active (one-command named start)"
        )]
        name: Option<String>,
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
            long = "part",
            value_name = "KIND:VALUE",
            help = "Structured message part, repeatable: text:<value>, file:<path>, data:<json>. Cannot be combined with positional content, --message-file, or --stdin"
        )]
        parts: Vec<String>,
        #[arg(
            long,
            short = 'w',
            help = "Wait for a reply after sending (default: return immediately)"
        )]
        wait: bool,
        #[arg(
            long = "sender",
            help = "Override the sender name (warns if it differs from the configured agent name)"
        )]
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
    /// Use `tala listen` to observe all sessions, or `tala wait` for a blocking poll.
    /// Use `tala wait --new-session` to wait for a session with an unread incoming message from another agent (new sessions first, then sessions you have participated in).
    #[command(
        after_help = "USAGE:\n  tala wait <session>          Blocking poll — sends periodic HTTP requests\n  tala wait --new-session     Wait for a session with an incoming message from another agent\n\nCOMPARISON:\n  tala listen   Real-time SSE — observe all sessions at once\n  tala check    Non-blocking — show new messages and return immediately\n\nEXIT CODES: 0 = messages received (or new session found); 3 = benign timeout; 2 = usage error; 1 = error\n\nSee also: tala history (transcript), tala session (manage sessions)"
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
        #[arg(
            long,
            help = "Seconds to wait before timing out (default: 60, 0 = no timeout)"
        )]
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
            help = "Wait for a session with an unread incoming message from another agent — new sessions first, then sessions you have participated in (ignores other args)"
        )]
        r#new: bool,
    },
    /// View conversation transcript
    #[command(after_help = "See also: tala wait (blocking poll), tala listen (all sessions)")]
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
    /// Use `tala wait` for a blocking poll.
    #[command(
        after_help = "USAGE:\n  tala listen                Real-time SSE — observe all sessions at once\n  tala listen --since <n>   Skip history replay (only messages with ID > n)\n  tala listen --from <name> Filter messages from a specific sender\n  tala listen --match <text> Filter messages containing text\n  tala listen --name <name> Filter by session name\n\nCOMPARISON:\n  tala wait     Blocking poll — sends periodic HTTP requests, good for scripts and CI\n  tala check    Non-blocking -- show new messages and return immediately\n\nEXIT CODES: 0 = received messages; 3 = timed out with no messages; 1 = error\n\nSee also: tala history (transcript)"
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
        after_help = "See also: tala wait (blocking poll), tala listen (all sessions), tala history (transcript)"
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
    /// Close a session (clears the active marker if it was active)
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
    /// Create a new empty session (sets it active for this project)
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
    let _ = precheck_daemon_compat(&cli.command).await;
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
            name,
            message,
            message_file,
            stdin,
            parts,
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
            // A positional session ref may be an id OR a name (B035). A lone
            // positional is a message unless it starts with sess_; when a
            // message positional is also present, the first positional is a
            // session ref (id or name).
            let resolved_session = session_arg.or_else(|| {
                session
                    .clone()
                    .filter(|s| s.starts_with("sess_") || message.is_some())
            });
            let resolved_message = message.or_else(|| {
                if session_flag {
                    session
                } else if resolved_session.is_some() {
                    None
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
                parts,
                wait,
                sender_name.as_deref(),
                json,
                quiet,
                timeout,
                intent.as_deref(),
                reply_to,
                expect_reply,
                name,
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

/// Read-only commands may inspect a stale daemon (with a warning); everything
/// else must fail fast before issuing any command to an incompatible daemon.
fn command_is_read_only(cmd: &Commands) -> bool {
    matches!(
        cmd,
        Commands::Status { .. }
            | Commands::Discover { .. }
            | Commands::Stop
            | Commands::Init { .. }
            | Commands::Use { .. }
            | Commands::Daemon
    )
}

fn command_json_output(cmd: &Commands) -> bool {
    match cmd {
        Commands::Use { json, .. }
        | Commands::Send { json, .. }
        | Commands::Wait { json, .. }
        | Commands::History { json, .. }
        | Commands::Listen { json, .. }
        | Commands::List { json }
        | Commands::Pending { json }
        | Commands::Discover { json }
        | Commands::Close { json, .. }
        | Commands::Check { json }
        | Commands::Status { json } => *json,
        Commands::Session { command } => match command {
            SessionCommands::Rename { json, .. }
            | SessionCommands::Reopen { json, .. }
            | SessionCommands::Create { json, .. } => *json,
        },
        _ => false,
    }
}

/// daemon-compat gate: refuse to talk to a stale daemon (live `/api/status`
/// version differs from PROTOCOL_VERSION) before any command runs. Read-only
/// commands warn instead of failing. Skips when no daemon is running — the
/// command's own ensure_daemon_running spawns a fresh, same-binary daemon.
async fn precheck_daemon_compat(cmd: &Commands) -> anyhow::Result<()> {
    let info = match store::read_daemon_json().await {
        Ok(info) => info,
        Err(_) => return Ok(()),
    };
    let client = reqwest::Client::new();
    let url = daemon_url(&info.host, info.port, "/api/status");
    let Ok(resp) = client.get(&url).send().await else {
        return Ok(());
    };
    if !resp.status().is_success() {
        return Ok(());
    }
    let Ok(status) = resp.json::<StatusResponse>().await else {
        return Ok(());
    };
    if status.protocol_version == PROTOCOL_VERSION {
        return Ok(());
    }
    let msg = format!(
        "daemon protocol version {} is incompatible with this tala (requires {}). Restart the daemon with `tala stop` and retry, or upgrade tala.",
        status.protocol_version, PROTOCOL_VERSION
    );
    if command_is_read_only(cmd) {
        eprintln!("warning: {}", msg);
    } else {
        fail(command_json_output(cmd), &msg, "VERSION_MISMATCH");
    }
    Ok(())
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

/// Build the `&seen=` query suffix for wait-new requests: the waiter's
/// per-session read cursors (URL-encoded JSON map) so the daemon can exclude
/// sessions the waiter already knows (B029). Empty when no cursors exist.
async fn wait_new_seen_param() -> String {
    let cursors = store::read_cursors().await;
    if cursors.is_empty() {
        return String::new();
    }
    match serde_json::to_string(&cursors) {
        Ok(json) => format!("&seen={}", percent_encode(&json)),
        Err(_) => String::new(),
    }
}

/// Resolve a session ref for a command, emitting a proper (possibly JSON)
/// error instead of a plain anyhow error (B031-adjacent: --json paths must
/// not leak human text).
async fn resolve_session_id_or_fail(
    host: &str,
    port: u16,
    session_arg: Option<&str>,
    cmd_name: &str,
    json_output: bool,
) -> String {
    match resolve_session_id(host, port, session_arg, cmd_name).await {
        Ok(id) => id,
        Err(e) => fail(json_output, e.to_string(), "SESSION_NOT_FOUND"),
    }
}

async fn resolve_session_id(
    host: &str,
    port: u16,
    session_arg: Option<&str>,
    cmd_name: &str,
) -> anyhow::Result<String> {
    if let Some(id) = session_arg {
        return resolve_session_ref(host, port, id, cmd_name).await;
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
                "Multiple open sessions: {}. Specify one with `tala {} <session>` or set one with `tala use <session>`",
                ids.join(", "),
                cmd_name
            );
        }
    }
}

/// Resolve a user-supplied session reference (name, full id, or unique id
/// prefix) to a session id. Errors loudly on no-match or ambiguity — never
/// silently falls back to another session (B035).
async fn resolve_session_ref(
    host: &str,
    port: u16,
    input: &str,
    cmd_name: &str,
) -> anyhow::Result<String> {
    let url = daemon_url(host, port, "/api/sessions");
    let resp = reqwest::get(&url).await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    // 1. Exact name match
    let name_matches: Vec<&SessionSummary> = sessions
        .iter()
        .filter(|s| s.name.as_deref() == Some(input))
        .collect();
    if name_matches.len() == 1 {
        return Ok(name_matches[0].id.clone());
    }
    if name_matches.len() > 1 {
        let ids: Vec<&str> = name_matches.iter().map(|s| s.id.as_str()).collect();
        bail!(
            "Multiple sessions named '{}': {}. Use session ID instead.",
            input,
            ids.join(", ")
        );
    }

    // 2. Exact id match
    if let Some(s) = sessions.iter().find(|s| s.id == input) {
        return Ok(s.id.clone());
    }

    // 3. Unique id-prefix match
    let prefix_matches: Vec<&SessionSummary> = sessions
        .iter()
        .filter(|s| s.id.starts_with(input))
        .collect();
    if prefix_matches.len() == 1 {
        return Ok(prefix_matches[0].id.clone());
    }
    if prefix_matches.len() > 1 {
        let ids: Vec<&str> = prefix_matches.iter().map(|s| s.id.as_str()).collect();
        bail!(
            "Multiple sessions match '{}': {}. Use the full session ID.",
            input,
            ids.join(", ")
        );
    }

    bail!(
        "session '{}' not found. Use `tala {} <session-id>` or `tala use <session-id>` to target an existing session.",
        input,
        cmd_name
    )
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

    let skill_path = opencode_dir.join("skills").join("tala").join("SKILL.md");
    let command_path = opencode_dir.join("commands").join("tala.md");
    install_rendered_documents(
        &skill_path,
        &command_path,
        include_str!("../.opencode/skills/tala/SKILL.md"),
        include_str!("../.opencode/commands/tala.md"),
        TALA_SKILL_MIN_VERSION,
        env!("CARGO_PKG_VERSION"),
    )
    .await?;

    println!("Created .opencode/skills/tala/SKILL.md");
    println!("Created .opencode/commands/tala.md");
    Ok(())
}

async fn install_rendered_documents(
    skill_path: &Path,
    command_path: &Path,
    skill_template: &str,
    command_template: &str,
    min_version: &str,
    generated_version: &str,
) -> anyhow::Result<()> {
    // Render and validate both documents before changing either destination.
    let (skill, command) = render_integration_documents(
        skill_template,
        command_template,
        min_version,
        generated_version,
    )?;

    if let Some(parent) = skill_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if let Some(parent) = command_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    write_file_atomically(skill_path, &skill).await?;
    write_file_atomically(command_path, &command).await?;
    Ok(())
}

async fn write_file_atomically(path: &Path, contents: &str) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("cannot atomically write a path without a parent directory")?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("cannot atomically write a path without a valid file name")?;
    let temporary_path = parent.join(format!(".{}.{}.tmp", file_name, uuid::Uuid::new_v4()));

    let result = async {
        tokio::fs::write(&temporary_path, contents).await?;
        tokio::fs::rename(&temporary_path, path).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;

    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    result
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
                "Session '{}' is closed. Use `tala session reopen {}` to open it, then `tala use {}` to make it active",
                closed_match[0].id,
                closed_match[0].id,
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
                            println!("No open sessions. Start one with `tala send`.");
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
/// Counts open (not-closed) sessions — used by the cycle-19 ambiguity guard.
async fn session_count_open(host: &str, port: u16) -> usize {
    let url = daemon_url(host, port, "/api/sessions");
    if let Ok(resp) = reqwest::get(&url).await {
        if let Ok(sessions) = resp.json::<Vec<SessionSummary>>().await {
            return sessions.iter().filter(|s| !s.closed).count();
        }
    }
    1
}

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
    if !resp.status().is_success() {
        if resp.status().as_u16() == 409 {
            // B017: duplicate session name — surface the daemon's message.
            let err: ErrorResponse = resp.json().await?;
            fail(json_output, &err.error, "SESSION_NAME_TAKEN");
        }
        fail(
            json_output,
            format!("failed to create session (HTTP {})", resp.status().as_u16()),
            "SESSION_CREATE_FAILED",
        );
    }
    let session: CreateSessionResponse = resp.json().await?;
    store::write_active_session(&session.id).await?;
    // B029: a session the waiter itself creates is "seen" from birth (cursor
    // entry) — never a candidate for its own `wait --new-session` scan.
    let _ = store::write_cursor(&session.id, 0).await;
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
    parts_arg: Vec<String>,
    should_wait: bool,
    sender_override: Option<&str>,
    json_output: bool,
    quiet: bool,
    chat_timeout: Option<u64>,
    intent_arg: Option<&str>,
    reply_to: Option<u64>,
    expect_reply: bool,
    session_name: Option<String>,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;

    // --part is the structured content form; it must not be mixed with the
    // legacy content sources.
    let legacy_content = message.is_some() || message_file.is_some() || stdin_flag;
    if !parts_arg.is_empty() && legacy_content {
        fail(
            json_output,
            "--part cannot be combined with positional content, --message-file, or --stdin",
            "INVALID_PART",
        );
    }

    let has_content = legacy_content || !parts_arg.is_empty();

    if !has_content && session_arg.is_none() && store::read_active_session().await.is_none() {
        let mut hint = String::new();
        if let Ok(req) = reqwest::get(&daemon_url(&host, port, "/api/sessions")).await {
            if let Ok(sessions) = req.json::<Vec<SessionSummary>>().await {
                let open: Vec<_> = sessions.iter().filter(|s| !s.closed).collect();
                if !open.is_empty() {
                    hint = format!("\nOpen session: {} {}. Set it with `tala use` or target it with `tala send --session <id>`.",
                        open[0].id,
                        open[0].name.as_deref().unwrap_or(""),
                    );
                }
            }
        }
        fail(
            json_output,
            format!(
                "Nothing to send. Use `tala session create` to create a session without a message.{}",
                hint
            ),
            "NOTHING_TO_SEND",
        );
    }

    // Note: unknown/typo'd flags are now rejected by clap (no allow_hyphen_values
    // on send positionals), so a positional message starting with '--' can only
    // arrive via the explicit `--` separator — which is the documented correct
    // usage and needs no warning (B015).

    // B004 (PO decision 2026-08-07): --sender is RESTRICTED to the project's
    // configured agent name (`.tala/config.json`). A mismatched identity is a
    // hard error — nothing is sent. The honest way to speak as another agent
    // is to operate from that agent's project dir.
    if let Some(s) = sender_override {
        let configured = store::get_sender_name(None);
        if s != configured {
            let msg = format!(
                "Cannot send as '{}': this project is configured as agent '{}'. Use the configured identity or run from the other agent's project directory.",
                s, configured
            );
            fail(json_output, &msg, "SENDER_MISMATCH");
        }
    }

    // Resolve content: structured parts take the --part path; legacy content
    // sources (positional, --message-file, --stdin, piped stdin) produce a
    // single text part.
    let parts: Vec<Part> = if !parts_arg.is_empty() {
        parse_parts(&parts_arg, json_output)
    } else {
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
                anyhow::anyhow!(
                    "No message provided via stdin (use `--stdin` flag with piped input)"
                )
            })?
        } else if let Some(msg) = &message {
            if msg.is_empty() {
                fail(json_output, "Message cannot be empty.", "EMPTY_MESSAGE");
            }
            msg.clone()
        } else {
            try_read_piped_stdin().await
                .ok_or_else(|| anyhow::anyhow!("No message provided. Use a positional argument, --message-file <path>, --stdin, or pipe to stdin"))?
        };
        vec![Part::Text { content }]
    };

    // Resolve session: explicit (id or name, B035), --name (new named
    // session), active, stale-replace, or auto-create
    let session_id = if let Some(id) = session_arg.clone() {
        if session_name.is_some() {
            fail(
                json_output,
                "--name creates a new session; it cannot be combined with an explicit session target",
                "INVALID_SESSION_NAME",
            );
        }
        // Explicit ref: resolve name/prefix to an id; error loudly if it does
        // not match anything (never silently fall back to the active session).
        match resolve_session_ref(&host, port, &id, "send").await {
            Ok(sid) => sid,
            Err(e) => fail(json_output, e.to_string(), "SESSION_NOT_FOUND"),
        }
    } else if let Some(name) = session_name {
        // Golden path (cycle-19): one-command named start — create the
        // session with the given name, send there, set it active.
        auto_create_session(&host, port, sender_override, quiet, json_output, Some(name)).await?
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
                // Cycle-19 guard: warn when the active session is an ambiguous
                // choice (multiple open sessions, no explicit target).
                if !quiet {
                    let warn = session_count_open(&host, port).await;
                    if warn > 1 {
                        eprintln!(
                            "warning: targeting active session {} ({} open sessions) — use -s <id> or `tala use` to be explicit",
                            id, warn
                        );
                    }
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
        &parts,
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

/// Parses `--part KIND:VALUE` flags into an ordered part list. Values are
/// split on the FIRST colon so text values may contain colons. `data:` values
/// must parse as JSON; unknown kinds and empty text parts are usage errors.
fn parse_parts(arg: &[String], json_output: bool) -> Vec<Part> {
    let mut out = Vec::with_capacity(arg.len());
    for a in arg {
        let (kind, value) = match a.split_once(':') {
            Some((k, v)) => (k, v),
            None => fail(
                json_output,
                format!(
                    "invalid --part '{}': expected KIND:VALUE with kind text, file, or data",
                    a
                ),
                "INVALID_PART",
            ),
        };
        match kind {
            "text" => {
                if value.trim().is_empty() {
                    fail(json_output, "empty text part", "INVALID_PART");
                }
                out.push(Part::Text {
                    content: value.to_string(),
                });
            }
            "file" => {
                if value.is_empty() {
                    fail(json_output, "empty file part", "INVALID_PART");
                }
                out.push(Part::File {
                    path: value.to_string(),
                    label: None,
                });
            }
            "data" => match serde_json::from_str::<serde_json::Value>(value) {
                Ok(v) => out.push(Part::Data {
                    value: v,
                    label: None,
                }),
                Err(e) => fail(
                    json_output,
                    format!("invalid --part data value '{}': {}", value, e),
                    "INVALID_PART",
                ),
            },
            other => fail(
                json_output,
                format!(
                    "invalid --part kind '{}': must be one of text, file, data",
                    other
                ),
                "INVALID_PART",
            ),
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
async fn send_content(
    session_id: String,
    parts: &[Part],
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
    // B040: a reply that is not correlated to anything defeats the intent model.
    if intent == Intent::Reply && reply_to.is_none() {
        eprintln!(
            "warning: --intent reply without --reply-to — this reply is not correlated to a message"
        );
    }
    let expect_reply = expect_reply || (reply_to.is_some() && should_wait && intent_arg.is_none());
    if expect_reply && matches!(intent, Intent::Req | Intent::Out) {
        fail(
            json_output,
            "--expect-reply is only valid with intent reply or fyi; `--intent req` (or `--wait`) already expects a reply",
            "INVALID_INTENT",
        );
    }

    let config = store::read_user_config().await;
    let default_timeout = config["default_timeout"].as_u64().unwrap_or(60);
    let effective_timeout = chat_timeout.or(Some(default_timeout));

    // Idempotency key: generated once per invocation, reused across every
    // retry below, so a retried send can never double-post (send-idempotency).
    let idempotency_key = uuid::Uuid::new_v4().to_string();
    let req = SendMessageRequest {
        sender,
        content: None,
        parts: Some(parts.to_vec()),
        idempotency_key: Some(idempotency_key.clone()),
        intent: Some(intent),
        reply_to,
        expect_reply,
        wait_timeout: if should_wait { effective_timeout } else { None },
    };

    // Retry connection failures only (up to two additional attempts) with the
    // same key; HTTP error responses are never retried.
    let mut resp: Option<reqwest::Response> = None;
    for attempt in 0..3u32 {
        match client.post(&url).json(&req).send().await {
            Ok(r) => {
                resp = Some(r);
                break;
            }
            Err(_) if attempt < 2 => {
                tokio::time::sleep(Duration::from_millis(200 * (attempt as u64 + 1))).await;
            }
            Err(e) => return Err(e.into()),
        }
    }
    let resp = resp.unwrap();

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
    let _ = store::write_cursor(&msg.session_id, msg.id).await;

    // A deduplicated replay: report the original message and stop — nothing
    // new was stored, so there is nothing to wait on.
    if msg.duplicate {
        if json_output {
            println!("{}", serde_json::to_string(&msg).unwrap());
        } else if !quiet {
            println!("duplicate suppressed (msg {})", msg.id);
        }
        return Ok(());
    }

    if !should_wait {
        if json_output {
            let val = serde_json::to_value(&msg).unwrap();
            println!("{}", serde_json::to_string(&val).unwrap());
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
            process::exit(EXIT_TIMEOUT);
        }
    } else if result.closed {
        println!("[session closed]");
    } else if result.timeout {
        println!(
            "[timeout after {}s, no reply]",
            result.timeout_after.unwrap_or(0)
        );
        let _ = print_unread_hint(host, port).await;
        process::exit(EXIT_TIMEOUT);
    } else {
        for m in &result.messages {
            println!("{}: {}", m.sender, m.render());
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
    let timeout_label = if wait_timeout == 0 {
        "no timeout".to_string()
    } else {
        format!("{}s", wait_timeout)
    };

    // B021: identify the reader so the daemon records read receipts.
    let sender_param = format!(
        "&sender={}",
        store::read_project_config()
            .await
            .or_else(|| Some(store::get_default_sender()))
            .unwrap_or_else(|| "unknown".to_string())
    );

    // Resolve an explicit session ref (id or name, B035) once up front.
    let session_arg = match session_arg {
        Some(id) => Some(
            resolve_session_ref(&host, port, &id, "wait")
                .await
                .unwrap_or_else(|e| fail(json_output, e.to_string(), "SESSION_NOT_FOUND")),
        ),
        None => None,
    };

    // Cycle-19 guard: warn once when a bare wait targets the active session
    // while several sessions are open (ambiguous without -s).
    if session_arg.is_none() && store::read_active_session().await.is_some() && !json_output {
        let open = session_count_open(&host, port).await;
        if open > 1 {
            eprintln!(
                "warning: waiting on active session ({} open sessions) — use -s <id> to target a specific session",
                open
            );
        }
    }

    // Plateau: a stale active session must not produce a bare SESSION_NOT_FOUND
    // (beta, plateau eval). Validate once; on missing/closed, clear it and fall
    // through to the no-active path, which waits for a new session.
    if session_arg.is_none() {
        if let Some(id) = store::read_active_session().await {
            let check_url = daemon_url(&host, port, &format!("/api/sessions/{}", id));
            let valid = match client.get(&check_url).send().await {
                Ok(r) if r.status().is_success() => r
                    .json::<Session>()
                    .await
                    .map(|s| !s.closed)
                    .unwrap_or(false),
                _ => false,
            };
            if !valid {
                store::clear_active_session().await?;
                if !json_output {
                    eprintln!(
                        "Active session {} is gone or closed — cleared. Use `tala use --clear` or target explicitly with -s <id>.",
                        id
                    );
                }
            }
        }
    }

    loop {
        let sid = if let Some(id) = session_arg.clone() {
            if !json_output {
                eprintln!(
                    "Waiting for messages in session {} (timeout: {})...",
                    id, timeout_label
                );
            }
            id
        } else if let Some(id) = store::read_active_session().await {
            if !json_output {
                eprintln!(
                    "Waiting for messages in session {} (timeout: {})...",
                    id, timeout_label
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
                            "No active sessions. Waiting for a new session (timeout: {})...",
                            timeout_label
                        );
                    }
                    let sender = store::read_project_config()
                        .await
                        .or_else(|| Some(store::get_default_sender()));
                    let sender_param = match sender {
                        Some(s) => format!("&sender={}", s),
                        None => String::new(),
                    };
                    let seen_param = wait_new_seen_param().await;
                    let new_url = daemon_url(
                        &host,
                        port,
                        &format!(
                            "/api/sessions/wait-new?timeout_secs={}{}{}",
                            wait_timeout, sender_param, seen_param
                        ),
                    );
                    let resp = client.get(&new_url).send().await?;
                    let result: serde_json::Value = resp.json().await?;
                    if json_output {
                        println!("{}", serde_json::to_string(&result).unwrap());
                        if result.get("timeout") == Some(&serde_json::json!(true)) {
                            process::exit(EXIT_TIMEOUT);
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
                            "Waiting for new messages in session {} (timeout: {})...",
                            sid_val, timeout_label
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
            "/api/sessions/{}/wait-stream?since={}&timeout_secs={}&identity={}{}",
            sid,
            since_id,
            wait_timeout,
            store::get_sender_name(None),
            sender_param
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
                process::exit(EXIT_TIMEOUT);
            }
        } else if result.closed {
            println!("[session closed]");
        } else if result.timeout {
            println!(
                "timeout after {}s, no new messages",
                result.timeout_after.unwrap_or(0)
            );
            let _ = print_unread_hint(&host, port).await;
            process::exit(EXIT_TIMEOUT);
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
                    msg.render()
                );
            }
        }

        break;
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

    let since_id = since.unwrap_or_default();
    let mut path = format!("/api/observe?since={}", since_id);
    if since.is_none() {
        // No explicit --since: replay each session since ITS OWN read cursor.
        // A single global since cannot represent per-session id spaces (B014).
        let cursors = store::read_cursors().await;
        let since_map = serde_json::to_string(&cursors).unwrap_or_else(|_| "{}".to_string());
        path = format!("{}&since_map={}", path, percent_encode(&since_map));
    }
    if let Some(ref m) = match_str {
        path = format!("{}&match={}", path, urlencoding(m));
    }
    if let Some(ref f) = from {
        path = format!("{}&from={}", path, f);
    }
    if let Some(ref n) = name {
        path = format!("{}&channel={}", path, n);
    }
    // Default timeout to 60s if not specified; --timeout 0 = indefinite
    // (B046: the old filter turned Some(0) into 60 — 0 must pass through).
    let timeout_secs = timeout.or(Some(60u64));
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

    // B007: visible connection status. Text banner goes to stdout (matches the
    // "Waiting for a new session…" convention in wait); in --json mode it goes
    // to stderr so stdout stays a pure JSON event stream. B046b: the since
    // label must be truthful — cursor mode replays from last-read, not id 0.
    let since_label = if since.is_none() {
        "from last-read cursors".to_string()
    } else {
        format!("since id {}", since_id)
    };
    if json_output {
        eprintln!(
            "[listen] connected to tala daemon at {}:{} ({})",
            host, port, since_label
        );
    } else {
        println!(
            "Listening on tala daemon at {}:{} (all sessions, {})...",
            host, port, since_label
        );
    }

    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();
    let mut message_count: u64 = 0;
    let mut max_by_session: HashMap<String, u64> = HashMap::new();
    let mut timed_out = false;

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

            let evt = serde_json::from_str::<ObserveEvent>(&data);
            if let Ok(ref evt) = evt {
                if evt.r#type == "message" {
                    message_count += 1;
                }
            }

            // B046: advance the per-session cursor as messages are delivered
            // (both modes), so check agrees and a killed/reconnected listener
            // never replays. Explicit --since is replay mode — leave cursors
            // untouched there.
            if let Ok(ref evt) = evt {
                if evt.r#type == "message" {
                    if let Some(msg) = &evt.message {
                        if since.is_none() {
                            let _ = store::write_cursor(&evt.session_id, msg.id).await;
                        }
                    }
                }
            }

            // B046: a lagged broadcast warns instead of dropping silently.
            if data.contains("\"overload\"") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&data) {
                    let skipped = v["skipped"].as_u64().unwrap_or(0);
                    eprintln!(
                        "warning: missed {} message(s) (daemon overload) — run `tala check` to catch up",
                        skipped
                    );
                }
                continue;
            }

            // Plateau: benign timeout is exit 3, matching the wait family
            // (the daemon emits a terminal timeout event before closing).
            if data.contains("\"timeout\"") {
                timed_out = true;
                if !json_output {
                    eprintln!(
                        "[listen] timed out after {}s — no new messages",
                        timeout_secs.unwrap_or(60)
                    );
                }
            }

            if json_output {
                println!("{}", data);
            } else if let Ok(evt) = evt {
                match evt.r#type.as_str() {
                    "message" => {
                        if let Some(msg) = evt.message {
                            let sid = evt.session_id.clone();
                            let entry = max_by_session.entry(sid).or_insert(0);
                            if msg.id > *entry {
                                *entry = msg.id;
                            }
                            let session_label = evt.session_name.unwrap_or(evt.session_id);
                            println!(
                                "[{}] {} {} ({}):{}",
                                session_label,
                                intent_badge(&msg),
                                msg.sender,
                                msg.timestamp.format("%H:%M:%S"),
                                render_deadline(&msg, true)
                            );
                            println!("    {}", msg.render());
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

    // B007: end-of-stream note with a message tally so connected-but-quiet is
    // distinguishable from a dead listener.
    if json_output {
        eprintln!("[listen] connection closed ({} message(s))", message_count);
    } else {
        println!("[connection closed] ({} message(s))", message_count);
    }

    // Advance the per-session read cursor for each session we saw messages in.
    for (sid, mid) in &max_by_session {
        if *mid > store::read_cursor(sid).await {
            let _ = store::write_cursor(sid, *mid).await;
        }
    }

    // Plateau: benign timeout exits 3 ONLY when nothing was received
    // (family contract: 0 = messages received; 3 = timed out empty).
    if timed_out && message_count == 0 {
        process::exit(EXIT_TIMEOUT);
    }

    Ok(())
}

fn percent_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
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
    let session_id =
        resolve_session_id_or_fail(&host, port, session_arg.as_deref(), "recap", json_output).await;

    // B021: identify the reader so the daemon records read receipts.
    let sender_param = format!(
        "&sender={}",
        store::read_project_config()
            .await
            .or_else(|| Some(store::get_default_sender()))
            .unwrap_or_else(|| "unknown".to_string())
    );

    let since_id = since.unwrap_or(0);
    let mut path = format!(
        "/api/sessions/{}/recap?since={}{}",
        session_id, since_id, sender_param
    );
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
        if recap.messages.is_empty() {
            println!("(no messages yet)");
        } else {
            for msg in &recap.messages {
                println!(
                    "[{}] {} {} ({}):{}",
                    msg.id,
                    intent_badge(msg),
                    msg.sender,
                    msg.timestamp.format("%H:%M:%S"),
                    render_deadline(msg, false)
                );
                println!("    {}\n", msg.render());
            }
        }
    }
    // Mark THIS session read only. Empty sessions have cursor: None — skip the
    // write so an empty history can never reset read state (B025).
    if let Some(c) = recap.cursor {
        store::write_cursor(&session_id, c).await?;
    }
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
            // The actionable hint depends on who owes whom: my own unanswered
            // request is owed BY the recipient; someone else's is owed BY me.
            let hint = if o.sender == store::get_sender_name(None) {
                format!("awaiting reply from {}", o.sender)
            } else {
                format!("answer with `tala send --reply-to {}`", o.message_id)
            };
            println!(
                "      unanswered for {}{} — {}",
                if mins > 0 {
                    format!("{}m ", mins)
                } else {
                    String::new()
                },
                secs,
                hint
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

    let cursors = store::read_cursors().await;
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
                let since_id = cursors.get(&s.id).copied().unwrap_or(0);
                compute_session_unread(&host, port, s, since_id).await
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
            let status = if s.closed { "closed" } else { "open" };
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
            // B021: show readers OTHER than the local identity (self-reads are
            // noise in text; --json exposes the full read_by map).
            let local_identity = store::read_project_config()
                .await
                .or_else(|| Some(store::get_default_sender()));
            let mut readers: Vec<String> = s
                .read_by
                .iter()
                .filter(|(sender, _)| local_identity.as_deref() != Some(sender.as_str()))
                .map(|(sender, id)| format!("{}@{}", sender, id))
                .collect();
            readers.sort();
            let read_suffix = if readers.is_empty() {
                String::new()
            } else {
                format!("  read: {}", readers.join(", "))
            };
            if s.closed {
                println!(
                    "{}  {:width$}  {}  {} msgs{}{}",
                    s.id,
                    name,
                    status,
                    s.message_count,
                    marker,
                    read_suffix,
                    width = name_width
                );
            } else {
                let since_id = cursors.get(&s.id).copied().unwrap_or(0);
                let unread = compute_session_unread(&host, port, s, since_id).await;
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
                    "{}  {:width$}  {}  {} msgs{}{}{}",
                    s.id,
                    name,
                    status,
                    s.message_count,
                    extra_str,
                    marker,
                    read_suffix,
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

async fn cmd_close(
    session_arg: Option<String>,
    json_output: bool,
    quiet: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id =
        resolve_session_id_or_fail(&host, port, session_arg.as_deref(), "close", json_output).await;

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}", session_id));
    let resp = client.delete(&url).send().await?;

    if resp.status().is_success() {
        let result: CloseSessionResponse = resp.json().await?;
        // Clear the active marker whenever the CLOSED session is the active one,
        // regardless of how it was addressed (positional, -s, or `session close`
        // alias which always passes Some(id)). Previously the alias path computed
        // was_active=false and left a dangling * marker + stale active-session
        // file, breaking bare `send` (B028).
        let was_active = store::read_active_session().await.as_deref() == Some(&session_id);
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

async fn cmd_session_rename(
    session_id: String,
    name: String,
    json_output: bool,
    force: bool,
) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    let session_id = resolve_session_ref(&host, port, &session_id, "rename")
        .await
        .unwrap_or_else(|e| fail(json_output, e.to_string(), "SESSION_NOT_FOUND"));

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
            409 => fail(json_output, &err.error, "SESSION_NAME_TAKEN"),
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
    let session_id = resolve_session_ref(&host, port, &session_id, "reopen")
        .await
        .unwrap_or_else(|e| fail(json_output, e.to_string(), "SESSION_NOT_FOUND"));

    let client = reqwest::Client::new();
    let url = daemon_url(&host, port, &format!("/api/sessions/{}/reopen", session_id));
    let resp = client.post(&url).send().await?;

    let status = resp.status();
    if !status.is_success() {
        let err: ErrorResponse = resp.json().await?;
        fail(json_output, &err.error, "SESSION_NOT_FOUND");
    }

    let result: serde_json::Value = resp.json().await?;
    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else {
        println!(
            "Session {} reopened (use `tala use {}` to make it active)",
            session_id, session_id
        );
    }
    Ok(())
}

async fn cmd_session_create(session_name: Option<String>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    // B039: --json success must emit a typed document, never silence.
    let id = auto_create_session(&host, port, None, false, json_output, session_name).await?;
    if json_output {
        println!("{}", serde_json::json!({"session_id": id}));
    }
    Ok(())
}

async fn cmd_wait_new(timeout_secs: Option<u64>, json_output: bool) -> anyhow::Result<()> {
    let (host, port) = ensure_daemon_running().await?;
    // B048: --timeout 0 = wait indefinitely (same semantics as listen).
    let timeout = timeout_secs.unwrap_or(60);
    let label = if timeout == 0 {
        "no timeout".to_string()
    } else {
        format!("{}s", timeout)
    };
    if !json_output {
        eprintln!("Waiting for a new session (timeout: {})...", label);
    }
    let _ = print_unread_hint(&host, port).await;
    // B003: identify ourselves so the daemon only delivers sessions with an
    // incoming message from ANOTHER agent (not our own creates).
    let sender = store::read_project_config()
        .await
        .or_else(|| Some(store::get_default_sender()));
    let sender_param = match sender {
        Some(s) => format!("&sender={}", s),
        None => String::new(),
    };
    let seen_param = wait_new_seen_param().await;
    let url = daemon_url(
        &host,
        port,
        &format!(
            "/api/sessions/wait-new-stream?timeout_secs={}&identity={}{}{}",
            timeout,
            store::get_sender_name(None),
            sender_param,
            seen_param
        ),
    );
    let client = reqwest::Client::new();
    let resp = client.get(&url).send().await?;
    let result: serde_json::Value = consume_wait_new_stream(resp, json_output).await?;

    if json_output {
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if let Some(sid) = result.get("session_id").and_then(|v| v.as_str()) {
        // B041: stdout stays the bare id (agents capture it via $(tala wait --new-session));
        // the session name is context, so it goes to stderr.
        println!("{}", sid);
        if let Some(name) = result.get("name").and_then(|v| v.as_str()) {
            if !name.is_empty() {
                eprintln!("session: {}", name);
            }
        }
    } else if result.get("timeout") == Some(&serde_json::json!(true)) {
        eprintln!(
            "timeout after {}s, no new session",
            result["timeout_after"].as_u64().unwrap_or(timeout)
        );
        let _ = print_unread_hint(&host, port).await;
        process::exit(EXIT_TIMEOUT);
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
/// Renders the waiting_until deadline relative to now, e.g. " (waiting, 83s left)".
/// Past deadlines render as "expired" only in live surfaces (check/listen/wait);
/// history suppresses them — a completed exchange must not read like a failure.
fn render_deadline(msg: &Message, show_expired: bool) -> String {
    match msg.waiting_until {
        Some(until) => {
            let now = chrono::Utc::now();
            let remaining = (until - now).num_seconds();
            if remaining >= 0 {
                format!(" (waiting, {}s left)", remaining)
            } else if !show_expired {
                String::new()
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
    let cursors = store::read_cursors().await;
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
        let cursor = cursors.get(&s.id).copied().unwrap_or(0);
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
    let tala_home_set = std::env::var("TALA_HOME").is_ok();
    let home_path = store::tala_home();
    let info = match store::read_daemon_json().await {
        Ok(info) => info,
        Err(_) => {
            let home = daemon_home_display();
            if json_output {
                println!(
                    "{}",
                    serde_json::json!({
                        "running": false,
                        "home": home_path.display().to_string(),
                        "tala_home_set": tala_home_set,
                    })
                );
            } else {
                println!("no daemon running (checked {}/daemon.json)", home);
                println!("Start the daemon by running any tala command, or set TALA_HOME if using a custom location");
            }
            return Ok(());
        }
    };

    let status_url = daemon_url(&info.host, info.port, "/api/status");
    // Live protocol version from the daemon's own status response (the
    // on-disk daemon.json claim can be stale if the daemon restarted).
    let live_version = {
        let client = reqwest::Client::new();
        match client.get(&status_url).send().await {
            Ok(r) if r.status().is_success() => r
                .json::<StatusResponse>()
                .await
                .ok()
                .map(|s| s.protocol_version),
            _ => None,
        }
    };
    let alive = live_version.is_some();
    let version = live_version.or(Some(info.protocol_version));

    if alive {
        let cursors = store::read_cursors().await;
        let total_unread = compute_total_unread(&info.host, info.port, &cursors).await;
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
                "protocol_version": version,
                "total_unread": total_unread,
                "active_waits": waits,
                "home": home_path.display().to_string(),
                "tala_home_set": tala_home_set,
            });
            println!("{}", serde_json::to_string(&resp).unwrap());
        } else {
            println!("daemon running:");
            println!("  PID:  {}", info.pid);
            println!("  Port: {}", info.port);
            println!("  Host: {}", info.host);
            println!("  Protocol: {}", version.unwrap_or(0));
            println!("  Since: {}", info.started_at.format("%Y-%m-%d %H:%M:%S"));
            println!("  Home: {}", daemon_home_display());
            if !tala_home_set {
                eprintln!(
                    "warning: TALA_HOME is not set — using default daemon home {}",
                    home_path.display()
                );
            }
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
                serde_json::json!({"running": false, "stale_daemon_json": true, "home": home_path.display().to_string(), "tala_home_set": tala_home_set})
            );
        } else {
            println!("daemon.json found at {}/daemon.json but daemon is not reachable (may have crashed)", home);
            println!("Try `tala stop` to clean up stale daemon.json, then run your command again.");
        }
    }
    Ok(())
}

async fn compute_total_unread(host: &str, port: u16, cursors: &HashMap<String, u64>) -> usize {
    let local_agent = store::read_project_config().await;
    let client = reqwest::Client::new();
    let url = daemon_url(host, port, "/api/sessions");
    match client.get(&url).send().await {
        Ok(resp) => {
            let sessions: Vec<SessionSummary> = resp.json().await.unwrap_or_default();
            let mut total = 0;
            for s in &sessions {
                let since_id = cursors.get(&s.id).copied().unwrap_or(0);
                let msgs_url = daemon_url(
                    host,
                    port,
                    &format!("/api/sessions/{}/messages?since={}", s.id, since_id),
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
    let mut cursors = store::read_cursors().await;
    let client = reqwest::Client::new();

    let url = daemon_url(&host, port, "/api/sessions");
    let resp = client.get(&url).send().await?;
    let sessions: Vec<SessionSummary> = resp.json().await?;

    let mut all_messages: Vec<Message> = Vec::new();

    // B021: identify the reader so the daemon records read receipts.
    let sender_param = format!(
        "&sender={}",
        store::read_project_config()
            .await
            .or_else(|| Some(store::get_default_sender()))
            .unwrap_or_else(|| "unknown".to_string())
    );

    // Fetch per session since THAT session's own read cursor (B014).
    for s in &sessions {
        let since_id = cursors.get(&s.id).copied().unwrap_or(0);
        let msgs_url = daemon_url(
            &host,
            port,
            &format!(
                "/api/sessions/{}/messages?since={}{}",
                s.id, since_id, sender_param
            ),
        );
        if let Ok(resp) = client.get(&msgs_url).send().await {
            if let Ok(msgs) = resp.json::<Vec<Message>>().await {
                if let Some(max_id) = msgs.iter().map(|m| m.id).max() {
                    cursors.insert(s.id.clone(), max_id);
                }
                all_messages.extend(msgs);
            }
        }
    }

    all_messages.sort_by_key(|m| m.id);

    // "cursor" is kept for backward compatibility: max of the per-session cursors.
    let max_cursor = cursors.values().copied().max().unwrap_or(0);

    if json_output {
        let result = serde_json::json!({
            "cursor": max_cursor,
            "cursors": cursors,
            "messages": all_messages,
        });
        println!("{}", serde_json::to_string(&result).unwrap());
    } else if all_messages.is_empty() {
        println!("No new messages since last check");
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
                    render_deadline(msg, true)
                );
                println!("    {}", msg.render());
            }
            println!();
        }
    }

    // Persist the per-session read cursors.
    for (sid, mid) in &cursors {
        store::write_cursor(sid, *mid).await?;
    }

    if !json_output && !all_messages.is_empty() {
        println!("(read markers updated for {} session(s))", cursors.len());
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
        use libc::kill;
        let pid = info.pid as libc::pid_t;
        // Send SIGTERM directly via the syscall — no external `kill` binary needed.
        // (Containers/slim systems may not ship /bin/kill.)
        let ret = unsafe { kill(pid, libc::SIGTERM) };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::ESRCH) {
                // Process already gone — clean up stale daemon.json
                store::remove_daemon_json().await;
                println!("daemon stopped");
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "failed to kill daemon (pid {}): {}",
                pid,
                err
            ));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_cli_version_output(output: &str) -> anyhow::Result<SemanticVersion> {
        let mut fields = output.split_whitespace();
        if fields.next() != Some("tala") {
            bail!("version output must start with 'tala'");
        }
        let version = fields
            .next()
            .ok_or_else(|| anyhow::anyhow!("version output is missing a version"))?;
        if fields.next().is_some() {
            bail!("version output has unexpected trailing fields");
        }
        parse_semantic_version(version)
    }

    #[test]
    fn semantic_versions_follow_semver_precedence() {
        assert!(
            parse_semantic_version("1.0.0-alpha.1").unwrap()
                < parse_semantic_version("1.0.0-alpha.beta").unwrap()
        );
        assert!(
            parse_semantic_version("1.0.0-alpha").unwrap()
                < parse_semantic_version("1.0.0").unwrap()
        );
        assert_eq!(
            parse_semantic_version("1.0.0+build.1").unwrap(),
            parse_semantic_version("1.0.0+build.2").unwrap()
        );
    }

    #[test]
    fn compatibility_fixtures_cover_each_version_direction() {
        let minimum = parse_semantic_version("0.27.3").unwrap();
        let generated = parse_semantic_version("0.28.0").unwrap();

        assert!(parse_semantic_version("0.26.0").unwrap() < minimum);
        assert!(minimum < generated);
        assert_eq!(generated, parse_semantic_version("0.28.0").unwrap());
        assert!(generated < parse_semantic_version("0.29.0").unwrap());
        assert!(
            parse_semantic_version("0.28.0-rc.1").unwrap()
                < parse_semantic_version("0.28.0").unwrap()
        );
    }

    #[test]
    fn semantic_version_rejects_invalid_values() {
        for value in ["1.0", "01.0.0", "1.0.0-", "1.0.0+"] {
            assert!(
                parse_semantic_version(value).is_err(),
                "{} should be rejected",
                value
            );
        }
    }

    #[test]
    fn renderer_replaces_each_version_placeholder() {
        let rendered = render_integration_document(
            "min=__TALA_CLI_MIN_VERSION__ generated=__TALA_CLI_GENERATED_VERSION__",
            "0.27.3",
            "0.28.0",
        )
        .unwrap();
        assert_eq!(rendered, "min=0.27.3 generated=0.28.0");
    }

    #[test]
    fn renderer_rejects_missing_or_duplicate_placeholders() {
        assert!(render_integration_document("no placeholders", "0.27.3", "0.28.0").is_err());
        assert!(render_integration_document(
            "__TALA_CLI_MIN_VERSION__ __TALA_CLI_MIN_VERSION__ __TALA_CLI_GENERATED_VERSION__",
            "0.27.3",
            "0.28.0"
        )
        .is_err());
    }

    #[test]
    fn renderer_rejects_invalid_and_inconsistent_versions() {
        assert!(render_integration_document(
            "__TALA_CLI_MIN_VERSION__ __TALA_CLI_GENERATED_VERSION__",
            "not-a-version",
            "0.28.0"
        )
        .is_err());
        assert!(render_integration_document(
            "__TALA_CLI_MIN_VERSION__ __TALA_CLI_GENERATED_VERSION__",
            "0.28.0",
            "0.27.3"
        )
        .is_err());
    }

    #[tokio::test]
    async fn rendering_failure_leaves_existing_documents_unchanged() {
        let directory = tempfile::tempdir().unwrap();
        let skill_path = directory.path().join("skills/SKILL.md");
        let command_path = directory.path().join("commands/tala.md");
        tokio::fs::create_dir_all(skill_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(command_path.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&skill_path, "old skill document")
            .await
            .unwrap();
        tokio::fs::write(&command_path, "old command document")
            .await
            .unwrap();

        let result = install_rendered_documents(
            &skill_path,
            &command_path,
            "__TALA_CLI_MIN_VERSION__ __TALA_CLI_GENERATED_VERSION__",
            "missing generated placeholder",
            "0.27.3",
            "0.28.0",
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            tokio::fs::read_to_string(&skill_path).await.unwrap(),
            "old skill document"
        );
        assert_eq!(
            tokio::fs::read_to_string(&command_path).await.unwrap(),
            "old command document"
        );
    }

    #[test]
    fn version_output_fixtures_are_parsed_or_rejected() {
        assert!(parse_cli_version_output("tala 0.28.0").is_ok());
        assert!(parse_cli_version_output("").is_err());
        assert!(parse_cli_version_output("tala development").is_err());
        assert!(parse_cli_version_output("tala 0.28.0 extra").is_err());
    }
}
