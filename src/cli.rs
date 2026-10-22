use crate::{client, daemon, integration, models::*, store};
use clap::{Args, Parser, Subcommand};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::{self, IsTerminal, Read},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "tala",
    version,
    about = "Local messaging for distinct coding agents. Without a command, shows what needs your attention.",
    after_help = help_text()
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        short = 'j',
        hide = true,
        help = "Accepted for compatibility; JSON is the default output"
    )]
    pub json: bool,
    #[arg(
        long,
        global = true,
        help = "Render text for people instead of one JSON envelope"
    )]
    pub human: bool,
    #[arg(
        long,
        global = true,
        env = "TALA_AGENT",
        help = "Your private handle from tala register (independent of CWD)"
    )]
    pub agent: Option<String>,
    #[command(subcommand)]
    pub command: Option<Commands>,
}
#[derive(Subcommand)]
pub enum Commands {
    /// Generate agent instructions without creating a project identity
    Init {
        #[arg(long, conflicts_with = "refresh")]
        check: bool,
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Register a distinct agent instance; keep the returned private handle for this run
    Register {
        #[arg(long, required_unless_present = "reissue")]
        name: Option<String>,
        #[arg(long, required_unless_present = "reissue")]
        tool: Option<String>,
        #[arg(
            long,
            conflicts_with = "reissue",
            help = "Checkout path; defaults to the current project root"
        )]
        project: Option<PathBuf>,
        #[arg(
            long,
            value_name = "AGENT_ID",
            conflicts_with_all = ["name", "tool"],
            hide = true,
            help = "Issue a handle, once, for your own identity created before handles existed"
        )]
        reissue: Option<String>,
    },
    /// Mark your instance inactive; preserve inbox and history
    Unregister,
    /// Show your selected identity and project context
    Whoami,
    /// Find registered peers, with observed presence and exact IDs
    Agents {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long, default_value = "")]
        after: String,
        #[arg(long,default_value_t=50,value_parser=limit)]
        limit: usize,
    },
    /// Send to an explicit recipient; --wait consumes only its correlated reply
    Send {
        #[arg(long)]
        to: String,
        #[arg(long, help = "Continue a visible thread")]
        thread: Option<String>,
        #[arg(long)]
        request: bool,
        #[arg(long)]
        wait: bool,
        #[arg(
            long,
            default_value_t = 60,
            help = "Reply deadline in seconds; 0 waits indefinitely"
        )]
        timeout: u64,
        #[arg(long, help = "Stable retry key; reuse only for an identical send")]
        key: Option<String>,
        #[command(flatten)]
        input: Input,
    },
    /// Consume your unread direct messages; --peek leaves receipts unchanged
    Inbox {
        #[arg(long)]
        peek: bool,
        #[arg(long)]
        wait: bool,
        #[arg(
            long,
            value_name = "MESSAGE_ID",
            help = "Only the reply to your request with this ID"
        )]
        reply_to: Option<u64>,
        #[arg(
            long,
            default_value_t = 60,
            help = "Overall wait deadline in seconds; 0 waits indefinitely"
        )]
        timeout: u64,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Reply to a message ID; Tala supplies recipient and thread
    Reply {
        #[arg(id = "message_id")]
        message: u64,
        #[arg(long)]
        request: bool,
        #[arg(long)]
        key: Option<String>,
        #[command(flatten)]
        input: Input,
    },
    /// Publish an informational update to a project board
    Post {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        key: Option<String>,
        #[command(flatten)]
        input: Input,
    },
    /// Read shared project updates without consuming any inbox
    Board {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(
            long,
            help = "Wait for messages newer than --after, or newer than the latest when the wait starts"
        )]
        wait: bool,
        #[arg(
            long,
            default_value_t = 60,
            help = "Overall wait deadline in seconds; 0 waits indefinitely"
        )]
        timeout: u64,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Read a visible thread without acknowledging inbox messages
    History {
        thread: String,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Search visible direct messages and a project board
    Search {
        query: String,
        #[arg(long)]
        project: Option<PathBuf>,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Show received or unread requests that still need an answer
    Pending {
        #[arg(long,default_value="all",value_parser=["all","incoming","outgoing"])]
        direction: String,
        #[arg(
            long,
            help = "List only stale requests (unanswered for 24 hours, or from unregistered agents)"
        )]
        stale: bool,
        #[command(flatten)]
        page: PageArgs,
    },
    /// Cancel your own outstanding request explicitly
    Resolve { message: u64 },
    /// Give a peer a summary and read access to an existing thread
    Handoff {
        #[arg(long)]
        to: String,
        #[arg(long)]
        thread: String,
        #[arg(long)]
        key: Option<String>,
        #[command(flatten)]
        input: Input,
    },
    /// Inspect daemon/storage or a message's delivery; does not start a daemon
    Status {
        #[arg(long)]
        message: Option<u64>,
    },
    /// Stop the live daemon gracefully; never signal a recorded PID
    Stop,
    #[command(hide = true)]
    Daemon,
}
fn limit(s: &str) -> std::result::Result<usize, String> {
    let n = s
        .parse::<usize>()
        .map_err(|_| "Expected an integer".to_string())?;
    if !(1..=500).contains(&n) {
        return Err("Limit must be between 1 and 500".into());
    }
    Ok(n)
}
#[derive(Args)]
pub struct PageArgs {
    #[arg(
        long,
        default_value_t = 0,
        help = "Return message IDs greater than this cursor"
    )]
    after: u64,
    #[arg(long,default_value_t=50,value_parser=limit)]
    limit: usize,
}
#[derive(Args)]
pub struct Input {
    #[arg(conflicts_with_all=["message_file","stdin","parts"])]
    message: Option<String>,
    #[arg(long,conflicts_with_all=["message","stdin","parts"],help="Read UTF-8 text from a file; - reads stdin")]
    message_file: Option<PathBuf>,
    #[arg(long,conflicts_with_all=["message","message_file","parts"])]
    stdin: bool,
    #[arg(long="part",conflicts_with_all=["message","message_file","stdin"],help="Repeat text:<text>, file:<reference-path>, or data:<json>")]
    parts: Vec<String>,
}
impl Input {
    fn read(self) -> Result<Vec<Part>> {
        let parts = if !self.parts.is_empty() {
            self.parts
                .iter()
                .map(|p| {
                    let (kind, value) = p
                        .split_once(':')
                        .ok_or_else(|| usage("Part must be text:, file:, or data:"))?;
                    match kind {
                        "text" => Ok(Part::Text { text: value.into() }),
                        "file" => Ok(Part::File { path: value.into() }),
                        "data" => Ok(Part::Data {
                            value: serde_json::from_str(value).map_err(|e| usage(e.to_string()))?,
                        }),
                        _ => Err(usage("Unknown part type")),
                    }
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            let text = if let Some(m) = self.message {
                m
            } else if let Some(p) = self.message_file {
                if p == Path::new("-") {
                    read_stdin()?
                } else {
                    std::fs::read_to_string(p)?
                }
            } else if self.stdin || !io::stdin().is_terminal() {
                read_stdin()?
            } else {
                return Err(usage(
                    "No message provided; supply text, --message-file or piped stdin",
                ));
            };
            vec![Part::Text { text }]
        };
        if parts.iter().any(|p| !p.valid()) {
            return Err(usage("Message parts must be nonempty"));
        }
        if serde_json::to_vec(&parts)
            .map_err(|e| usage(e.to_string()))?
            .len()
            > 1024 * 1024
        {
            return Err(usage("Message exceeds the 1 MiB limit"));
        }
        Ok(parts)
    }
}
fn read_stdin() -> Result<String> {
    if io::stdin().is_terminal() {
        return Err(usage("Pipe text to stdin, or supply a message argument"));
    }
    let mut text = String::new();
    io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_string(&mut text)?;
    if text.len() > 1024 * 1024 {
        return Err(usage("Message exceeds the 1 MiB limit"));
    }
    Ok(text)
}
fn usage(m: impl Into<String>) -> Failure {
    Failure::new("USAGE_ERROR", m, "Run tala <command> --help.")
}
pub fn context(path: Option<&Path>) -> Result<Context> {
    let dir = std::fs::canonicalize(path.map(PathBuf::from).unwrap_or(std::env::current_dir()?))?;
    if !dir.is_dir() {
        return Err(usage("Project must be a directory"));
    }
    let git = |args: &[&str]| -> Option<String> {
        Command::new("git")
            .args(args)
            .current_dir(&dir)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().into())
    };
    let worktree = git(&["rev-parse", "--show-toplevel"])
        .map(std::fs::canonicalize)
        .transpose()?;
    // Linked worktrees share one repository, and so one project: the main
    // worktree root, found from the common git directory.
    let common = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .map(PathBuf::from)
        .or_else(|| git(&["rev-parse", "--git-common-dir"]).map(|p| dir.join(p)))
        .and_then(|p| std::fs::canonicalize(p).ok());
    let project = match (&common, &worktree) {
        (Some(c), _) if c.file_name().is_some_and(|n| n == ".git") => c
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| dir.clone()),
        (_, Some(w)) => w.clone(),
        _ => dir.clone(),
    };
    Ok(Context {
        project: project.display().to_string(),
        worktree: worktree.map(|w| w.display().to_string()),
        working_directory: Some(dir.display().to_string()),
        branch: git(&["symbolic-ref", "--short", "HEAD"]),
        revision: git(&["rev-parse", "HEAD"]),
    })
}
pub struct Outcome {
    pub data: Value,
    pub code: i32,
}
fn done(data: Value) -> Outcome {
    Outcome { data, code: 0 }
}
async fn identity_context(credential: &Credential) -> Result<Context> {
    let identity = client::call(Operation::Whoami, Some(credential.clone())).await?;
    let registered: Agent =
        serde_json::from_value(identity["agent"].clone()).map_err(|e| usage(e.to_string()))?;
    // Rebuild from the registered worktree, not the project root: in a linked
    // worktree the project is the main checkout, but messages must record the
    // sender's own worktree, branch and revision.
    let mut context = registered.context;
    context.working_directory = Some(
        std::fs::canonicalize(std::env::current_dir()?)?
            .display()
            .to_string(),
    );
    // Preserve identity and board context while recording where this specific
    // message command was invoked.
    Ok(context)
}
/// Fills an empty send key with one derived from sender, routing and
/// content, so an identical resend is recognised as a duplicate.
fn keyed(sender: &str, mut op: Operation) -> Operation {
    if let Operation::Send {
        to,
        project,
        thread,
        reply_to,
        parts,
        request,
        kind,
        key,
        ..
    } = &mut op
    {
        if key.is_empty() {
            let material = json!([sender, kind, to, project, thread, reply_to, request, parts]);
            let digest = Sha256::digest(material.to_string().as_bytes());
            *key = format!(
                "auto:{}",
                digest
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
        }
    }
    op
}
fn shell_word(arg: &str) -> String {
    if !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_=./:@,+".contains(c))
    {
        arg.into()
    } else {
        format!("'{}'", arg.replace('\'', r"'\''"))
    }
}
/// The current command with an explicit key, for retrying a send that may
/// not have reached the daemon. Identity flags are left to the caller.
fn retry_command(argv: impl IntoIterator<Item = String>, key: &str) -> Option<String> {
    let mut args: Vec<String> = Vec::new();
    let mut skip = false;
    let mut has_key = false;
    for arg in argv {
        if skip {
            skip = false;
            continue;
        }
        if arg == "--agent" {
            skip = true;
            continue;
        }
        if arg.starts_with("--agent=") {
            continue;
        }
        has_key |= arg == "--key" || arg.starts_with("--key=");
        args.push(shell_word(&arg));
    }
    if !has_key {
        args.push(shell_word(&format!("--key={key}")));
    }
    let command = format!("tala {}", args.join(" "));
    (command.len() <= 4096).then_some(command)
}
/// Sends one message; on a transport failure the error's next command
/// repeats it with the same key.
async fn send(c: Credential, op: Operation) -> Result<Value> {
    let op = keyed(&c.agent, op);
    let key = match &op {
        Operation::Send { key, .. } => key.clone(),
        _ => unreachable!(),
    };
    client::call(op, Some(c)).await.map_err(|e| {
        if e.code == "CONNECTION_ERROR" {
            match retry_command(std::env::args().skip(1), &key) {
                Some(next) => e.with_next(next),
                None => e,
            }
        } else {
            e
        }
    })
}
/// Deadline, backoff and transport-failure bookkeeping shared by waits. A
/// transport failure is transient until the deadline; if the deadline passes
/// while failing, the failure is reported rather than a quiet timeout.
struct Waiter {
    start: Instant,
    timeout: u64,
    failure: Option<Failure>,
    backoff: Duration,
}
enum Attempt {
    Data(Value),
    Retry,
    Deadline,
}
impl Waiter {
    fn new(timeout: u64) -> Self {
        Self {
            start: Instant::now(),
            timeout,
            failure: None,
            backoff: Duration::from_millis(200),
        }
    }
    /// Time left, or None for an indefinite wait.
    fn remaining(&self) -> Option<Duration> {
        (self.timeout != 0)
            .then(|| Duration::from_secs(self.timeout).saturating_sub(self.start.elapsed()))
    }
    fn expired(&self) -> bool {
        self.remaining() == Some(Duration::ZERO)
    }
    /// Ends a wait at its deadline: the last transport failure, or a timeout.
    fn finish(&mut self, timed_out: Value) -> Result<Outcome> {
        match self.failure.take() {
            Some(f) => Err(f),
            None => Ok(Outcome {
                data: timed_out,
                code: 3,
            }),
        }
    }
    /// Runs one read within the deadline.
    async fn attempt(
        &mut self,
        call: impl std::future::Future<Output = Result<Value>>,
    ) -> Result<Attempt> {
        let result = match self.remaining() {
            None => call.await,
            Some(r) => match tokio::time::timeout(r, call).await {
                Ok(result) => result,
                Err(_) => return Ok(Attempt::Deadline),
            },
        };
        match result {
            Ok(data) => {
                self.failure = None;
                self.backoff = Duration::from_millis(200);
                Ok(Attempt::Data(data))
            }
            Err(e) if e.is_transport() => {
                self.failure = Some(e);
                Ok(Attempt::Retry)
            }
            Err(e) => Err(e),
        }
    }
    /// Sleeps before the next read, backing off while transport fails.
    async fn pause(&mut self) {
        let mut delay = Duration::from_millis(200);
        if self.failure.is_some() {
            delay = self.backoff;
            self.backoff = (self.backoff * 2).min(Duration::from_secs(2));
        }
        tokio::time::sleep(self.remaining().map_or(delay, |r| delay.min(r))).await;
    }
}
async fn wait_inbox(
    credential: Credential,
    peek: bool,
    after: u64,
    limit: usize,
    reply_to: Option<u64>,
    timeout: u64,
) -> Result<Outcome> {
    let mut waiter = Waiter::new(timeout);
    let timed_out = json!({"messages":[],"next_after":after,"has_more":false,"timed_out":true});
    // A consuming read is replayed with the same request ID until it
    // succeeds, so a lost response cannot drop the batch it acknowledged.
    let mut consume_id: Option<String> = None;
    loop {
        if waiter.expired() {
            return waiter.finish(timed_out);
        }
        let operation = |peek: bool| Operation::Inbox {
            peek,
            after,
            limit,
            reply_to,
            listening: peek,
        };
        let (request_id, op) = match &consume_id {
            Some(id) => (id.clone(), operation(false)),
            None => (Uuid::new_v4().to_string(), operation(true)),
        };
        let call = client::call_as(request_id, op, Some(credential.clone()));
        match waiter.attempt(call).await? {
            Attempt::Deadline => return waiter.finish(timed_out),
            Attempt::Retry => {}
            Attempt::Data(data) => {
                let found = data["messages"].as_array().is_some_and(|m| !m.is_empty());
                if consume_id.take().is_some() {
                    if found {
                        return Ok(done(data));
                    }
                } else if found {
                    if peek {
                        return Ok(done(data));
                    }
                    consume_id = Some(Uuid::new_v4().to_string());
                    continue;
                }
            }
        }
        if waiter.expired() {
            return waiter.finish(timed_out);
        }
        waiter.pause().await;
    }
}
/// Waits for board messages newer than `after`, or, when `after` is 0, newer
/// than the board's latest message when the wait starts. Never acknowledges.
async fn wait_board(
    credential: Credential,
    project: String,
    after: u64,
    limit: usize,
    timeout: u64,
) -> Result<Outcome> {
    let read = |after: u64, limit: usize| {
        client::call(
            Operation::Board {
                project: project.clone(),
                after,
                limit,
            },
            Some(credential.clone()),
        )
    };
    let mut cursor = after;
    if cursor == 0 {
        loop {
            let page = read(cursor, 500).await?;
            cursor = page["next_after"].as_u64().unwrap_or(cursor);
            if page["has_more"] != true {
                break;
            }
        }
    }
    let mut waiter = Waiter::new(timeout);
    let timed_out = json!({"messages":[],"next_after":cursor,"has_more":false,"timed_out":true});
    loop {
        if waiter.expired() {
            return waiter.finish(timed_out);
        }
        match waiter.attempt(read(cursor, limit)).await? {
            Attempt::Deadline => return waiter.finish(timed_out),
            Attempt::Retry => {}
            Attempt::Data(data) => {
                if data["messages"].as_array().is_some_and(|m| !m.is_empty()) {
                    return Ok(done(data));
                }
            }
        }
        if waiter.expired() {
            return waiter.finish(timed_out);
        }
        waiter.pause().await;
    }
}
async fn reissue(id: &str) -> Result<Value> {
    if !id.starts_with("agt_") || id.len() != 36 || !id[4..].chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(usage("--reissue requires a complete agt_ agent ID"));
    }
    let path = client::legacy_credential_path(id);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(Failure::new(
                "IDENTITY_ERROR",
                format!("No reissuable credential for {id}"),
                "A handle is issued once per identity. Use the handle you already have, or register a new identity.",
            ))
        }
        Err(e) => return Err(e.into()),
    };
    let c: Credential = serde_json::from_slice(&bytes).map_err(|e| usage(e.to_string()))?;
    if c.agent != id {
        return Err(usage("Stored credential does not match the agent ID"));
    }
    let mut data = client::call(Operation::Whoami, Some(c.clone())).await?;
    let handle = client::issue_handle(&c)?;
    std::fs::remove_file(&path)?;
    data["handle"] = json!(handle);
    data["select"] = json!(format!("--agent={handle}"));
    Ok(data)
}

pub async fn run(cli: Cli) -> Result<Outcome> {
    let selected = cli.agent.as_deref();
    let human = cli.human;
    let Some(command) = cli.command else {
        return Ok(done(
            client::call(Operation::Attention, Some(client::credential(selected)?)).await?,
        ));
    };
    let data = match command {
        Commands::Daemon => {
            daemon::run().await?;
            return Ok(done(json!({"stopped":true})));
        }
        Commands::Init {
            check,
            refresh,
            dry_run,
        } => integration::init(check, refresh, dry_run)?,
        Commands::Register {
            reissue: Some(id), ..
        } => reissue(&id).await?,
        Commands::Register {
            name,
            tool,
            project,
            reissue: None,
        } => {
            let mut data = client::call(
                Operation::Register {
                    name: name.unwrap_or_default(),
                    tool: tool.unwrap_or_default(),
                    context: context(project.as_deref())?,
                },
                None,
            )
            .await?;
            let c: Credential = serde_json::from_value(data["credential"].clone())
                .map_err(|e| usage(e.to_string()))?;
            let handle = client::issue_handle(&c)?;
            data.as_object_mut()
                .expect("register object")
                .remove("credential");
            data["handle"] = json!(handle);
            data["select"] = json!(format!("--agent={handle}"));
            data
        }
        Commands::Agents {
            project,
            after,
            limit,
        } => {
            client::call(
                Operation::Agents {
                    project: project
                        .as_deref()
                        .map(|p| context(Some(p)).map(|c| c.project))
                        .transpose()?,
                    after,
                    limit,
                },
                selected.map(|h| client::credential(Some(h))).transpose()?,
            )
            .await?
        }
        Commands::Status { message } => {
            let mut status = match client::existing(Operation::Status).await {
                Ok(Some(status)) => status,
                Ok(None) => json!({
                    "running": false,
                    "protocol_version": PROTOCOL_VERSION,
                    "stale_metadata": daemon::read_info().is_some(),
                }),
                // Unreachable is not proof of absence: report it as unknown.
                Err(e) if e.code == "PROTOCOL_MISMATCH" => json!({
                    "running": true,
                    "compatible": false,
                    "protocol_version": PROTOCOL_VERSION,
                    "error": e,
                }),
                Err(e) if e.code == "DAEMON_UNREACHABLE" => json!({
                    "running": null,
                    "reachable": false,
                    "protocol_version": PROTOCOL_VERSION,
                    "error": e,
                }),
                Err(e) => return Err(e),
            };
            status["home"] = json!(store::home());
            status["database"] = json!(store::home().join("tala.sqlite3"));
            status["legacy_files"] = json!(["sessions.json", "messages.json"]
                .iter()
                .filter(|name| store::home().join(name).exists())
                .collect::<Vec<_>>());
            if let Some(id) = message {
                if status["running"] != true {
                    return Err(Failure::new(
                        "DAEMON_NOT_RUNNING",
                        "Start the daemon to inspect message delivery",
                        "Run tala whoami, then retry status --message.",
                    ));
                }
                status["message"] = client::call(
                    Operation::MessageStatus { message: id },
                    Some(client::credential(selected)?),
                )
                .await?["message"]
                    .clone();
            }
            status
        }
        Commands::Stop => {
            let data = client::existing(Operation::Stop)
                .await?
                .unwrap_or_else(|| json!({"stopped":false,"running":false}));
            if data["stopped"] == true {
                for _ in 0..50 {
                    if daemon::read_info().is_none() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                if daemon::read_info().is_some() {
                    return Err(Failure::new(
                        "STOP_TIMEOUT",
                        "Daemon has not completed shutdown",
                        "Check tala status and daemon.log.",
                    ));
                }
            }
            data
        }
        Commands::Whoami => {
            client::call(Operation::Whoami, Some(client::credential(selected)?)).await?
        }
        Commands::Unregister => {
            client::call(Operation::Unregister, Some(client::credential(selected)?)).await?
        }
        Commands::Inbox {
            peek,
            wait,
            reply_to,
            timeout,
            page,
        } => {
            let c = client::credential(selected)?;
            if wait {
                if human {
                    eprintln!(
                        "Waiting for messages addressed to {} (timeout: {}s; 0 = indefinite)",
                        c.agent, timeout
                    );
                }
                return wait_inbox(c, peek, page.after, page.limit, reply_to, timeout).await;
            }
            client::call(
                Operation::Inbox {
                    peek,
                    after: page.after,
                    limit: page.limit,
                    reply_to,
                    listening: false,
                },
                Some(c),
            )
            .await?
        }
        Commands::Send {
            to,
            thread,
            request,
            wait,
            timeout,
            key,
            input,
        } => {
            let parts = input.read()?;
            let c = client::credential(selected)?;
            let context = identity_context(&c).await?;
            let data = send(
                c.clone(),
                Operation::Send {
                    to: Some(to),
                    project: None,
                    thread,
                    reply_to: None,
                    parts,
                    request: request || wait,
                    kind: "message".into(),
                    key: key.unwrap_or_default(),
                    context,
                },
            )
            .await?;
            if wait {
                let id = data["message"]["id"].as_u64().expect("stored message ID");
                if human {
                    eprintln!("Stored message {id}; waiting for its correlated reply.");
                }
                let mut result = match wait_inbox(c, false, 0, 50, Some(id), timeout).await {
                    Ok(result) => result,
                    Err(e) => {
                        let mut failure = Failure::new(
                            "WAIT_FAILED",
                            format!(
                                "Message {id} was stored, but waiting for its reply failed: {} ({})",
                                e.message, e.code
                            ),
                            "Do not resend. Resume waiting for the reply with the next command once the daemon is reachable.",
                        )
                        .with_next(format!("tala inbox --wait --reply-to={id} --timeout={timeout}"));
                        failure.stored = Some(id);
                        return Err(failure);
                    }
                };
                result.data["sent"] = data;
                if !result.data["request"].is_null() {
                    result.data["sent"]["message"] = result.data["request"].take();
                    result.data["sent"]["snapshot"] = json!("at_reply_receipt");
                    result
                        .data
                        .as_object_mut()
                        .expect("wait result object")
                        .remove("request");
                } else {
                    result.data["sent"]["snapshot"] = json!("at_send");
                }
                if result.data["timed_out"] == true {
                    result.data["next"] = json!(format!(
                        "tala inbox --wait --reply-to={id} --timeout={timeout}"
                    ));
                }
                return Ok(result);
            }
            data
        }
        Commands::Reply {
            message,
            request,
            key,
            input,
        } => {
            let parts = input.read()?;
            let c = client::credential(selected)?;
            let context = identity_context(&c).await?;
            send(
                c,
                Operation::Send {
                    to: None,
                    project: None,
                    thread: None,
                    reply_to: Some(message),
                    parts,
                    request,
                    kind: "reply".into(),
                    key: key.unwrap_or_default(),
                    context,
                },
            )
            .await?
        }
        Commands::Post {
            project,
            key,
            input,
        } => {
            let parts = input.read()?;
            let c = client::credential(selected)?;
            let ctx = identity_context(&c).await?;
            let p = project
                .as_deref()
                .map(|p| context(Some(p)).map(|c| c.project))
                .transpose()?
                .unwrap_or_else(|| ctx.project.clone());
            send(
                c,
                Operation::Send {
                    to: None,
                    project: Some(p),
                    thread: None,
                    reply_to: None,
                    parts,
                    request: false,
                    kind: "post".into(),
                    key: key.unwrap_or_default(),
                    context: ctx,
                },
            )
            .await?
        }
        Commands::Board {
            project,
            wait,
            timeout,
            page,
        } => {
            let c = client::credential(selected)?;
            let p = project
                .as_deref()
                .map(|p| context(Some(p)).map(|c| c.project))
                .transpose()?
                .unwrap_or(identity_context(&c).await?.project);
            if wait {
                return wait_board(c, p, page.after, page.limit, timeout).await;
            }
            client::call(
                Operation::Board {
                    project: p,
                    after: page.after,
                    limit: page.limit,
                },
                Some(c),
            )
            .await?
        }
        Commands::History { thread, page } => {
            client::call(
                Operation::History {
                    thread,
                    after: page.after,
                    limit: page.limit,
                },
                Some(client::credential(selected)?),
            )
            .await?
        }
        Commands::Search {
            query,
            project,
            page,
        } => {
            client::call(
                Operation::Search {
                    query,
                    project: project
                        .as_deref()
                        .map(|p| context(Some(p)).map(|c| c.project))
                        .transpose()?,
                    after: page.after,
                    limit: page.limit,
                },
                Some(client::credential(selected)?),
            )
            .await?
        }
        Commands::Pending {
            direction,
            stale,
            page,
        } => {
            client::call(
                Operation::Pending {
                    direction,
                    stale,
                    after: page.after,
                    limit: page.limit,
                },
                Some(client::credential(selected)?),
            )
            .await?
        }
        Commands::Resolve { message } => {
            client::call(
                Operation::Resolve { message },
                Some(client::credential(selected)?),
            )
            .await?
        }
        Commands::Handoff {
            to,
            thread,
            key,
            input,
        } => {
            let parts = input.read()?;
            let c = client::credential(selected)?;
            let context = identity_context(&c).await?;
            send(
                c,
                Operation::Send {
                    to: Some(to),
                    project: None,
                    thread: Some(thread),
                    reply_to: None,
                    parts,
                    request: true,
                    kind: "handoff".into(),
                    key: key.unwrap_or_default(),
                    context,
                },
            )
            .await?
        }
    };
    Ok(done(data))
}
/// Age in days, hours and minutes for people.
pub fn human_age(seconds: i64) -> String {
    let (d, h, m) = (seconds / 86400, seconds % 86400 / 3600, seconds % 3600 / 60);
    match (d, h, m) {
        (0, 0, 0) => "<1m".into(),
        (0, 0, m) => format!("{m}m"),
        (0, h, m) => format!("{h}h {m}m"),
        (d, h, _) => format!("{d}d {h}h"),
    }
}
pub fn render(data: &Value, attention: Option<&Value>) {
    if data["incoming_requests"].is_array() {
        render_summary(data);
    } else if let Some(messages) = data["messages"].as_array() {
        if messages.is_empty() {
            println!("No messages.");
        }
        for message in messages {
            render_message(message);
        }
        if data["has_more"] == true {
            println!("More messages: repeat with --after={}", data["next_after"]);
        }
        if data["stale_count"].as_u64().is_some_and(|n| n > 0) {
            println!(
                "{} stale requests hidden: {}",
                data["stale_count"],
                data["stale_next"].as_str().unwrap_or("")
            );
        }
    } else if data["message"].is_object() {
        render_message(&data["message"]);
        if data["duplicate"] == true {
            println!("Duplicate suppressed.");
        }
    } else if let Some(handle) = data["handle"].as_str() {
        println!(
            "Registered {} ({}). Pass --agent={handle} on every tala command; keep the handle private.",
            data["agent"]["id"].as_str().unwrap_or(""),
            data["agent"]["name"].as_str().unwrap_or("")
        );
    } else if let Some(agents) = data["agents"].as_array() {
        for a in agents {
            println!(
                "{}  {} ({})  {}  {}",
                a["id"].as_str().unwrap_or(""),
                a["name"].as_str().unwrap_or(""),
                a["tool"].as_str().unwrap_or(""),
                a["context"]["project"].as_str().unwrap_or(""),
                if a["listening"] == true {
                    "listening"
                } else if a["active"] == true {
                    "recently seen"
                } else {
                    "inactive"
                }
            );
        }
        if agents.is_empty() {
            println!("No registered agents. Run tala register.");
        }
        if data["has_more"] == true {
            println!(
                "More agents: repeat with --after={}",
                data["next_after"].as_str().unwrap_or("")
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(data).expect("JSON value")
        );
    }
    if data["timed_out"] == true {
        if !data["sent"].is_null() {
            println!("Wait timed out; the request was stored. Do not resend it.");
            if let Some(next) = data["next"].as_str() {
                println!("Resume waiting with: {next}");
            }
        } else {
            println!("Wait timed out; no messages were returned.");
        }
    }
    if let Some(a) = attention {
        let requests: Vec<String> = a["requests"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| {
                format!(
                    "{} from {} ({})",
                    r["id"],
                    r["from"].as_str().unwrap_or(""),
                    human_age(r["age_seconds"].as_i64().unwrap_or(0))
                )
            })
            .collect();
        println!(
            "Attention: {} unread, {} open requests{}{}; next: {}",
            a["unread"],
            a["open_requests"],
            if requests.is_empty() {
                String::new()
            } else {
                format!(" [{}]", requests.join(", "))
            },
            if a["stale"].as_u64().is_some_and(|n| n > 0) {
                format!(", {} stale", a["stale"])
            } else {
                String::new()
            },
            a["next"].as_str().unwrap_or("")
        );
    }
}
fn render_summary(data: &Value) {
    let requests = data["incoming_requests"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let unread = &data["unread"];
    let waiting = data["outgoing_requests"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    println!(
        "{} requests to you, {} unread, {} of your requests awaiting replies.",
        requests.len(),
        unread["count"],
        waiting.len()
    );
    for m in &requests {
        render_message(m);
        println!("  next: {}", m["next"].as_str().unwrap_or(""));
    }
    for m in unread["messages"].as_array().into_iter().flatten() {
        if m["expects_reply"] != true {
            render_message(m);
        }
    }
    for m in &waiting {
        println!(
            "Awaiting reply to {} ({}); next: {}",
            m["id"],
            human_age(m["age_seconds"].as_i64().unwrap_or(0)),
            m["next"].as_str().unwrap_or("")
        );
    }
    let stale = data["stale"]["incoming"].as_u64().unwrap_or(0)
        + data["stale"]["outgoing"].as_u64().unwrap_or(0);
    if stale > 0 {
        println!(
            "{stale} stale requests hidden: {}",
            data["stale"]["next"].as_str().unwrap_or("")
        );
    }
    if let Some(next) = data["next"].as_str() {
        println!("Next: {next}");
    }
}
fn render_message(v: &Value) {
    println!(
        "[{}] {} → {}  {}{}  {} ago  thread {}",
        v["id"],
        v["sender"].as_str().unwrap_or(""),
        v["recipient"]
            .as_str()
            .or(v["project"].as_str())
            .unwrap_or(""),
        v["delivery"].as_str().unwrap_or(""),
        if v["expects_reply"] == true {
            "; reply requested"
        } else {
            ""
        },
        human_age(v["age_seconds"].as_i64().unwrap_or(0)),
        v["thread_id"].as_str().unwrap_or("")
    );
    if let Some(parts) = v["parts"].as_array() {
        for p in parts {
            match p["type"].as_str() {
                Some("text") => println!("{}", p["text"].as_str().unwrap_or("")),
                Some("file") => println!("[file reference: {}]", p["path"].as_str().unwrap_or("")),
                Some("data") => println!("[data: {}]", p["value"]),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn clap_contract_has_unique_arguments() {
        Cli::command().debug_assert();
    }
    #[test]
    fn retry_command_adds_key_quotes_text_and_drops_identity() {
        let argv = [
            "--agent",
            "th_x",
            "send",
            "--to=b",
            "it's done",
            "--agent=th_y",
        ];
        assert_eq!(
            retry_command(argv.map(String::from), "auto:ab").as_deref(),
            Some("tala send --to=b 'it'\\''s done' --key=auto:ab")
        );
        let keyed = ["send", "--to=b", "--key=k", "hi"];
        assert_eq!(
            retry_command(keyed.map(String::from), "k").as_deref(),
            Some("tala send --to=b --key=k hi")
        );
        assert!(retry_command(["x".repeat(5000)], "k").is_none());
    }
    #[test]
    fn content_keys_depend_on_sender_routing_and_content() {
        let op = |text: &str| Operation::Send {
            to: Some("b".into()),
            project: None,
            thread: None,
            reply_to: None,
            parts: vec![Part::Text { text: text.into() }],
            request: false,
            kind: "message".into(),
            key: String::new(),
            context: Context {
                project: "/p".into(),
                worktree: None,
                working_directory: None,
                branch: None,
                revision: None,
            },
        };
        let key = |sender: &str, text: &str| match keyed(sender, op(text)) {
            Operation::Send { key, .. } => key,
            _ => unreachable!(),
        };
        assert!(key("a", "hi").starts_with("auto:"));
        assert_eq!(key("a", "hi"), key("a", "hi"));
        assert_ne!(key("a", "hi"), key("c", "hi"));
        assert_ne!(key("a", "hi"), key("a", "bye"));
    }
    #[test]
    fn ages_render_in_days_hours_and_minutes() {
        assert_eq!(human_age(30), "<1m");
        assert_eq!(human_age(5400), "1h 30m");
        assert_eq!(human_age(91104 * 60), "63d 6h");
    }
}
