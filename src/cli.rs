use crate::{client, daemon, integration, models::*, store};
use clap::{Args, Parser, Subcommand};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
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
        help = "Your private handle from tala register (or TALA_AGENT; independent of CWD)"
    )]
    pub agent: Option<String>,
    #[arg(
        long,
        global = true,
        conflicts_with = "agent",
        value_name = "PATH",
        help = "Read your private handle from a protected selector file (or TALA_AGENT_FILE)"
    )]
    pub agent_file: Option<PathBuf>,
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
    /// Register a distinct agent instance; optionally save its private handle to a selector file
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
            value_name = "PATH",
            conflicts_with = "reissue",
            help = "Save the private selector in a new, protected file instead of displaying it"
        )]
        save_handle_to: Option<PathBuf>,
        #[arg(
            long,
            conflicts_with = "reissue",
            help = "Short public purpose visible to peers; do not include secrets, personal data, or instructions"
        )]
        purpose: Option<String>,
        #[arg(
            long,
            value_name = "AGENT_ID",
            conflicts_with_all = ["name", "tool"],
            hide = true,
            help = "Issue a handle, once, for your own identity created before handles existed"
        )]
        reissue: Option<String>,
    },
    /// Read the bundled agent guide, optionally for one topic
    Guide {
        #[arg(value_parser = ["listening", "requests", "identity", "worktrees", "errors"])]
        topic: Option<String>,
    },
    /// Mark your instance inactive; preserve inbox and history
    Unregister,
    /// Change the name of your selected identity without changing its history
    Rename {
        #[arg(long)]
        name: String,
    },
    /// Set or clear the public purpose of your selected identity
    UpdatePurpose {
        #[arg(
            long,
            conflicts_with = "clear",
            required_unless_present = "clear",
            help = "Short public purpose visible to peers; do not include secrets, personal data, or instructions"
        )]
        purpose: Option<String>,
        #[arg(long, conflicts_with = "purpose", required_unless_present = "purpose")]
        clear: bool,
    },
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
    /// List server-wide projects before or after registration; stale and closed projects are hidden by default
    ListProjects {
        #[arg(long, help = "Include stale projects")]
        stale: bool,
        #[arg(long, help = "Include closed projects")]
        closed: bool,
        #[arg(long, conflicts_with_all = ["stale", "closed"], help = "Include stale and closed projects")]
        all: bool,
        #[arg(
            long,
            default_value = "",
            help = "Return project paths after this cursor"
        )]
        after: String,
        #[arg(long, default_value_t=50,value_parser=limit)]
        limit: usize,
    },
    /// Close the project owned by your selected agent
    CloseProject,
    /// Reopen the project owned by your selected agent
    ReopenProject,
    /// Send to an explicit recipient; --wait consumes only its correlated reply
    Send {
        #[arg(long)]
        to: String,
        #[arg(long, help = "Continue a visible thread")]
        thread: Option<String>,
        #[arg(long)]
        request: bool,
        #[arg(
            long,
            value_name = "MESSAGE_ID",
            help = "Supersede an earlier message in the same direct thread"
        )]
        supersedes: Option<u64>,
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
    /// Wait for unread inbox messages and project-board activity without acknowledging messages
    Wait {
        #[arg(
            long,
            help = "Watch this project's board instead of your registered project's board"
        )]
        project: Option<PathBuf>,
        #[arg(
            long,
            default_value_t = 0,
            help = "Wait deadline in seconds; 0 waits indefinitely"
        )]
        timeout: u64,
        #[arg(
            long,
            default_value_t = 0,
            help = "Resume inbox notifications after this message ID"
        )]
        inbox_after: u64,
        #[arg(
            long,
            help = "Resume board notifications after this message ID; omitted means start at the latest post"
        )]
        board_after: Option<u64>,
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
    /// List recent visible threads or read one visible thread without acknowledging inbox messages
    History {
        thread: Option<String>,
        #[arg(
            long,
            requires = "thread",
            conflicts_with = "cursor",
            value_name = "MESSAGE_ID"
        )]
        after: Option<u64>,
        #[arg(long, conflicts_with = "thread", value_name = "CURSOR")]
        cursor: Option<String>,
        #[arg(long, default_value_t = 50, value_parser = limit)]
        limit: usize,
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
    identity_context_from_value(identity)
}
async fn wait_identity_context(credential: &Credential, generation: u64) -> Result<Context> {
    let identity =
        client::call_bound(Operation::Whoami, Some(credential.clone()), generation).await?;
    identity_context_from_value(identity)
}
fn identity_context_from_value(identity: Value) -> Result<Context> {
    let registered: Agent =
        serde_json::from_value(identity["agent"].clone()).map_err(|e| usage(e.to_string()))?;
    // Keep the registered project as the agent's identity and board context,
    // but record Git metadata from the directory where this message command
    // runs. In particular, an agent may move between linked worktrees.
    let invocation = context(None)?;
    let mut context = registered.context;
    context.working_directory = invocation.working_directory;
    context.worktree = invocation.worktree;
    context.branch = invocation.branch;
    context.revision = invocation.revision;
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
        supersedes,
        ..
    } = &mut op
    {
        if key.is_empty() {
            let material = if let Some(target) = supersedes {
                json!([sender, kind, to, project, thread, reply_to, request, parts, target])
            } else {
                json!([sender, kind, to, project, thread, reply_to, request, parts])
            };
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
        Self::from_start(timeout, Instant::now())
    }
    fn from_start(timeout: u64, start: Instant) -> Self {
        Self {
            start,
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
fn finish_wait(waiter: &mut Waiter, timed_out: Outcome, resume: String) -> Result<Outcome> {
    match waiter.finish(timed_out.data) {
        Err(error) if error.is_transport() => Err(error.with_next(resume)),
        result => result,
    }
}

fn inbox_wait_result(
    data: Value,
    consumed: bool,
    generation_active: bool,
) -> Result<Option<Outcome>> {
    let found = data["messages"]
        .as_array()
        .is_some_and(|messages| !messages.is_empty());
    if !found {
        return Ok(None);
    }
    if consumed || generation_active {
        // A consumed batch is already acknowledged and must be returned even
        // if stop raced with the response.
        Ok(Some(done(data)))
    } else {
        Err(crate::lifecycle::stopped())
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
    let (_, generation) = client::begin_wait().await?;
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
        let call = client::call_as_bound(request_id, op, Some(credential.clone()), generation);
        match waiter.attempt(call).await? {
            Attempt::Deadline => return waiter.finish(timed_out),
            Attempt::Retry => {}
            Attempt::Data(data) => {
                let consumed = consume_id.take().is_some();
                if let Some(result) =
                    inbox_wait_result(data, consumed, crate::lifecycle::accepts(generation)?)?
                {
                    if consumed || peek {
                        return Ok(result);
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
    let (_, generation) = client::begin_wait().await?;
    let read = |after: u64, limit: usize| {
        client::call_bound(
            Operation::Board {
                project: project.clone(),
                after,
                limit,
            },
            Some(credential.clone()),
            generation,
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
                    if !crate::lifecycle::accepts(generation)? {
                        return Err(crate::lifecycle::stopped());
                    }
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
const WAIT_PAGE_LIMIT: usize = 50;

fn wait_command(inbox_after: u64, board_after: Option<u64>, timeout: u64, project: &str) -> String {
    let mut command = format!("tala wait --inbox-after={inbox_after} --timeout={timeout}");
    if let Some(board_after) = board_after {
        command.push_str(&format!(" --board-after={board_after}"));
    }
    if !project.is_empty() {
        command.push_str(&format!(
            " --project={}",
            shell_quote_selector_path(project)
        ));
    }
    command
}

struct WaitResultMeta {
    inbox_before: u64,
    inbox_after: u64,
    inbox_has_more: bool,
    board_before: u64,
    board_after: Option<u64>,
    board_has_more: bool,
    project: String,
    timeout: u64,
    timed_out: bool,
}

struct WaitLifecycle {
    started: Instant,
    generation: u64,
}

fn wait_result(events: BTreeMap<u64, (String, Vec<String>)>, meta: WaitResultMeta) -> Outcome {
    let event_values: Vec<Value> = events
        .iter()
        .map(|(id, (kind, sources))| json!({"id": id, "type": kind, "sources": sources}))
        .collect();
    let inbox_present = events
        .values()
        .any(|(_, sources)| sources.iter().any(|s| s == "inbox"));
    let board_present = events
        .values()
        .any(|(_, sources)| sources.iter().any(|s| s == "board"));
    let inbox_next =
        inbox_present.then(|| format!("tala inbox --peek --after={}", meta.inbox_before));
    let mut board_next = board_present.then(|| format!("tala board --after={}", meta.board_before));
    if let Some(command) = &mut board_next {
        if !meta.project.is_empty() {
            command.push_str(&format!(
                " --project={}",
                shell_quote_selector_path(&meta.project)
            ));
        }
    }
    let listen_again = wait_command(
        meta.inbox_after,
        meta.board_after,
        meta.timeout,
        &meta.project,
    );
    let next = if events.is_empty() {
        json!({"listen_again": listen_again})
    } else {
        json!({
            "inbox": inbox_next,
            "board": board_next,
            "listen_again": listen_again
        })
    };
    Outcome {
        data: json!({
            "events": event_values,
            "inbox_after": meta.inbox_after,
            "inbox_has_more": meta.inbox_has_more,
            "board_after": meta.board_after,
            "board_has_more": meta.board_has_more,
            "timed_out": meta.timed_out,
            "next": next,
        }),
        code: if meta.timed_out { 3 } else { 0 },
    }
}

fn wait_event_type(message: &Value) -> String {
    let is_board = message["project"].is_string();
    let reply = message["reply_to"].as_u64().is_some();
    let request = message["expects_reply"] == true;
    match (is_board, reply, request) {
        (true, true, _) => "board_reply",
        (true, false, _) => "board_post",
        (false, true, _) => "direct_reply",
        (false, false, true) => "direct_request",
        (false, false, false) => "direct_message",
    }
    .into()
}

fn board_latest_id(page: &Value) -> Result<u64> {
    page["latest_id"].as_u64().ok_or_else(|| {
        Failure::new(
            "PROTOCOL_MISMATCH",
            "The running daemon does not provide an atomic board high-water mark",
            "Restart the daemon with the current Tala binary, then run the wait command again.",
        )
        .with_next("tala stop")
    })
}

/// Waits for unread messages and project-board activity. Inbox pages are
/// peeked, so notifications never acknowledge receipts. The two cursors are
/// independent because inbox and board messages share global IDs.
async fn wait_events(
    credential: Credential,
    project: String,
    project_arg: Option<String>,
    timeout: u64,
    inbox_cursor: u64,
    board_after: Option<u64>,
    lifecycle: WaitLifecycle,
) -> Result<Outcome> {
    let generation = lifecycle.generation;
    let mut waiter = Waiter::from_start(timeout, lifecycle.started);
    let resume_command = |board_after| {
        wait_command(
            inbox_cursor,
            board_after,
            timeout,
            project_arg.as_deref().unwrap_or(""),
        )
    };
    let timed_out = |board_after: Option<u64>, board_has_more: bool| {
        wait_result(
            BTreeMap::new(),
            WaitResultMeta {
                inbox_before: inbox_cursor,
                inbox_after: inbox_cursor,
                inbox_has_more: false,
                board_before: board_after.unwrap_or(0),
                board_after,
                board_has_more,
                project: project_arg.clone().unwrap_or_default(),
                timeout,
                timed_out: true,
            },
        )
    };
    let mut board_cursor = if let Some(cursor) = board_after {
        cursor
    } else {
        loop {
            if waiter.expired() {
                return finish_wait(&mut waiter, timed_out(None, false), resume_command(None));
            }
            let attempt = waiter
                .attempt(client::call_bound(
                    Operation::Board {
                        project: project.clone(),
                        after: 0,
                        limit: 1,
                    },
                    Some(credential.clone()),
                    generation,
                ))
                .await?;
            match attempt {
                Attempt::Deadline => {
                    return finish_wait(&mut waiter, timed_out(None, false), resume_command(None));
                }
                Attempt::Retry => waiter.pause().await,
                Attempt::Data(page) => {
                    break board_latest_id(&page)?;
                }
            }
        }
    };
    let mut board_has_more = false;
    loop {
        if waiter.expired() {
            return finish_wait(
                &mut waiter,
                timed_out(Some(board_cursor), board_has_more),
                resume_command(Some(board_cursor)),
            );
        }
        let inbox_call = client::call_bound(
            Operation::Inbox {
                peek: true,
                after: inbox_cursor,
                limit: WAIT_PAGE_LIMIT,
                reply_to: None,
                listening: true,
            },
            Some(credential.clone()),
            generation,
        );
        let board_call = client::call_bound(
            Operation::Board {
                project: project.clone(),
                after: board_cursor,
                limit: WAIT_PAGE_LIMIT,
            },
            Some(credential.clone()),
            generation,
        );
        let attempt = waiter
            .attempt(async {
                let (inbox, board) = tokio::join!(inbox_call, board_call);
                Ok(json!({"inbox": inbox?, "board": board?}))
            })
            .await?;
        match attempt {
            Attempt::Deadline => {
                return finish_wait(
                    &mut waiter,
                    timed_out(Some(board_cursor), board_has_more),
                    resume_command(Some(board_cursor)),
                );
            }
            Attempt::Retry => {}
            Attempt::Data(pages) => {
                let inbox = pages["inbox"].clone();
                let board = pages["board"].clone();
                let inbox_messages = inbox["messages"].as_array().cloned().unwrap_or_default();
                let board_messages = board["messages"].as_array().cloned().unwrap_or_default();
                let mut events: BTreeMap<u64, (String, Vec<String>)> = BTreeMap::new();
                for message in &inbox_messages {
                    if let Some(id) = message["id"].as_u64() {
                        events
                            .entry(id)
                            .or_insert_with(|| (wait_event_type(message), Vec::new()))
                            .1
                            .push("inbox".into());
                    }
                }
                for message in &board_messages {
                    let Some(id) = message["id"].as_u64() else {
                        continue;
                    };
                    if message["sender"].as_str() == Some(credential.agent.as_str()) {
                        continue;
                    }
                    let entry = events
                        .entry(id)
                        .or_insert_with(|| (wait_event_type(message), Vec::new()));
                    if !entry.1.iter().any(|source| source == "board") {
                        entry.1.push("board".into());
                    }
                }
                let next_inbox_cursor = if inbox_messages.is_empty() {
                    inbox_cursor
                } else {
                    inbox["next_after"].as_u64().unwrap_or(inbox_cursor)
                };
                let next_board_cursor = if board_messages.is_empty() {
                    board_cursor
                } else {
                    board["next_after"].as_u64().unwrap_or(board_cursor)
                };
                let inbox_has_more = inbox["has_more"] == true;
                board_has_more = board["has_more"] == true;
                if !events.is_empty() {
                    if !crate::lifecycle::accepts(generation)? {
                        return Err(crate::lifecycle::stopped());
                    }
                    return Ok(wait_result(
                        events,
                        WaitResultMeta {
                            inbox_before: inbox_cursor,
                            inbox_after: next_inbox_cursor,
                            inbox_has_more,
                            board_before: board_cursor,
                            board_after: Some(next_board_cursor),
                            board_has_more,
                            project: project_arg.clone().unwrap_or_default(),
                            timeout,
                            timed_out: false,
                        },
                    ));
                }
                // Advancing over self-authored board entries is safe: the
                // cursor is returned on the next notification or timeout.
                board_cursor = next_board_cursor;
            }
        }
        if waiter.expired() {
            return finish_wait(
                &mut waiter,
                timed_out(Some(board_cursor), board_has_more),
                resume_command(Some(board_cursor)),
            );
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
    data["next"] = json!("tala guide listening");
    Ok(data)
}

fn resolve_selector(agent: Option<String>, agent_file: Option<PathBuf>) -> Result<Option<String>> {
    if let Some(handle) = agent {
        if handle.is_empty() {
            return Err(Failure::new(
                "IDENTITY_ERROR",
                "The --agent selector is empty",
                "Pass a complete private handle with --agent or select one with --agent-file.",
            ));
        }
        return Ok(Some(handle));
    }
    if let Some(path) = agent_file {
        if path.as_os_str().is_empty() {
            return Err(Failure::new(
                "IDENTITY_ERROR",
                "The --agent-file path is empty",
                "Pass a path to a protected selector file.",
            ));
        }
        return client::read_selector_file(&path).map(Some);
    }

    let env_agent = std::env::var_os("TALA_AGENT");
    let env_file = std::env::var_os("TALA_AGENT_FILE");
    match (env_agent, env_file) {
        (Some(_), Some(_)) => Err(Failure::new(
            "IDENTITY_ERROR",
            "Both TALA_AGENT and TALA_AGENT_FILE are set",
            "Set exactly one identity selector, or pass one explicit --agent or --agent-file option.",
        )),
        (Some(value), None) => {
            let value = value.into_string().map_err(|_| Failure::new(
                "IDENTITY_ERROR",
                "TALA_AGENT is not valid UTF-8",
                "Set TALA_AGENT to the complete private handle returned by tala register.",
            ))?;
            if value.is_empty() {
                return Err(Failure::new(
                    "IDENTITY_ERROR",
                    "TALA_AGENT is empty",
                    "Unset TALA_AGENT or set it to a complete private handle.",
                ));
            }
            Ok(Some(value))
        }
        (None, Some(path)) => {
            let path = PathBuf::from(path);
            if path.as_os_str().is_empty() {
                return Err(Failure::new(
                    "IDENTITY_ERROR",
                    "TALA_AGENT_FILE is empty",
                    "Unset TALA_AGENT_FILE or set it to a protected selector file path.",
                ));
            }
            client::read_selector_file(&path).map(Some)
        }
        (None, None) => Ok(None),
    }
}

struct StatusSelector {
    source: &'static str,
    state: &'static str,
    error_code: Option<String>,
    credential: Option<Credential>,
}

fn status_selector_source(agent: Option<&str>, agent_file: Option<&Path>) -> &'static str {
    if agent.is_some() {
        "--agent"
    } else if agent_file.is_some() {
        "selector_file"
    } else if std::env::var_os("TALA_AGENT").is_some()
        && std::env::var_os("TALA_AGENT_FILE").is_some()
    {
        "environment"
    } else if std::env::var_os("TALA_AGENT_FILE").is_some() {
        "selector_file"
    } else if std::env::var_os("TALA_AGENT").is_some() {
        "environment"
    } else {
        "none"
    }
}

/// Resolve status identity best-effort so global daemon diagnostics remain
/// available when the selector is absent, malformed, or conflicting.
fn status_selector(agent: Option<String>, agent_file: Option<PathBuf>) -> StatusSelector {
    let source = status_selector_source(agent.as_deref(), agent_file.as_deref());
    let handle = match resolve_selector(agent, agent_file) {
        Ok(Some(handle)) => handle,
        Ok(None) => {
            return StatusSelector {
                source,
                state: "none",
                error_code: None,
                credential: None,
            }
        }
        Err(error) => {
            return StatusSelector {
                source,
                state: "invalid",
                error_code: Some(error.code),
                credential: None,
            }
        }
    };
    match client::credential(Some(&handle)) {
        Ok(credential) => StatusSelector {
            source,
            state: "selected",
            error_code: None,
            credential: Some(credential),
        },
        Err(error) => StatusSelector {
            source,
            state: "invalid",
            error_code: Some(error.code),
            credential: None,
        },
    }
}

fn add_status_identity_context(status: &mut Value, selector: &StatusSelector) {
    let mut context = json!({
        "selector_source": selector.source,
        "selection_state": selector.state,
        "context_source": "local_database_snapshot",
        "agent": Value::Null,
        "project_root": Value::Null,
        "board_latest_id": Value::Null,
        "unavailable_reason": Value::Null,
        "error_code": selector.error_code,
    });
    let Some(credential) = selector.credential.as_ref() else {
        context["unavailable_reason"] = json!(if selector.state == "none" {
            "no_identity"
        } else {
            "invalid_selector"
        });
        status["identity_context"] = context;
        return;
    };

    context["agent"] = json!({"id": credential.agent, "name": Value::Null});
    match store::status_context_snapshot(&store::home().join("tala.sqlite3"), credential) {
        Ok(Some(snapshot)) => {
            context["agent"]["name"] = json!(snapshot.name);
            context["project_root"] = json!(snapshot.project);
            context["board_latest_id"] = json!(snapshot.board_latest_id);
        }
        Ok(None) => {
            context["selection_state"] = json!("invalid");
            context["unavailable_reason"] = json!("identity_not_found");
            context["error_code"] = json!("IDENTITY_ERROR");
        }
        Err(_) => context["unavailable_reason"] = json!("storage_unavailable"),
    }
    status["identity_context"] = context;
}

fn shell_quote_selector_path(path: &str) -> String {
    #[cfg(windows)]
    {
        format!("\"{path}\"")
    }
    #[cfg(not(windows))]
    {
        format!("'{}'", path.replace('\'', "'\\''"))
    }
}

pub async fn run(cli: Cli) -> Result<Outcome> {
    let status_selector = matches!(
        cli.command.as_ref(),
        Some(Commands::Status { message: None })
    )
    .then(|| status_selector(cli.agent.clone(), cli.agent_file.clone()));
    let uses_identity = !matches!(
        cli.command.as_ref(),
        Some(
            Commands::Init { .. }
                | Commands::Register { .. }
                | Commands::Status { message: None }
                | Commands::Stop
                | Commands::Daemon
        )
    );
    let selected = if uses_identity {
        resolve_selector(cli.agent, cli.agent_file)?
    } else {
        None
    };
    let selected = selected.as_deref();
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
            save_handle_to,
            purpose,
            reissue: None,
        } => {
            let save_path_text = save_handle_to
                .as_deref()
                .map(|path| {
                    path.to_str().ok_or_else(|| {
                        usage("--save-handle-to path must be valid UTF-8 so Tala can return a reusable selector")
                    })
                })
                .transpose()?;
            let mut data = client::call(
                Operation::Register {
                    name: name.unwrap_or_default(),
                    tool: tool.unwrap_or_default(),
                    context: context(project.as_deref())?,
                    purpose,
                },
                None,
            )
            .await?;
            let c: Credential = serde_json::from_value(data["credential"].clone())
                .map_err(|e| usage(e.to_string()))?;
            let handle = match client::issue_handle(&c) {
                Ok(handle) => handle,
                Err(handle_error) => match client::save_recovery_credential(&c) {
                    Ok(()) => {
                        return Err(Failure::new(
                            "IO_ERROR",
                            format!(
                                "Registration committed for {}, but saving its private handle failed: {}. A private recovery credential was saved.",
                                c.agent, handle_error.message
                            ),
                            "Recover the same identity with the next command.",
                        )
                        .with_next(format!("tala register --reissue={}", c.agent)));
                    }
                    Err(recovery_error) => {
                        return Err(Failure::new(
                            "IO_ERROR",
                            format!(
                                "Registration committed for {}, but saving its handle failed: {}. Saving the private recovery credential also failed: {}.",
                                c.agent, handle_error.message, recovery_error.message
                            ),
                            "The registration has no locally saved credential. Check Tala home permissions and storage, then register a new identity.",
                        ));
                    }
                },
            };
            data.as_object_mut()
                .expect("register object")
                .remove("credential");
            if let Some(path) = save_handle_to.as_deref() {
                let path_text = save_path_text.expect("save path was checked as UTF-8");
                match client::save_selector_file(path, &handle) {
                    Ok(()) => {
                        let quoted = shell_quote_selector_path(path_text);
                        data["selector_file"] = json!(path_text);
                        data["select"] = json!(format!("--agent-file={quoted}"));
                    }
                    Err(error) => {
                        // Registration and the internal handle are durable. Give
                        // the operator the handle once so a failed external save
                        // never strands the new identity.
                        data["handle"] = json!(handle);
                        data["select"] = json!(format!("--agent={handle}"));
                        data["selector_file_error"] = json!(error.message);
                    }
                }
            } else {
                data["handle"] = json!(handle);
                data["select"] = json!(format!("--agent={handle}"));
            }
            data["next"] = json!("tala guide listening");
            data
        }
        Commands::Guide { topic } => integration::guide(topic.as_deref())?,
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
        Commands::ListProjects {
            stale,
            closed,
            all,
            after,
            limit,
        } => {
            client::call(
                Operation::ListProjects {
                    include_stale: all || stale,
                    include_closed: all || closed,
                    after,
                    limit,
                },
                selected.map(|h| client::credential(Some(h))).transpose()?,
            )
            .await?
        }
        Commands::CloseProject => {
            client::call(Operation::CloseProject, Some(client::credential(selected)?)).await?
        }
        Commands::ReopenProject => {
            client::call(
                Operation::ReopenProject,
                Some(client::credential(selected)?),
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
            } else if let Some(selector) = status_selector.as_ref() {
                add_status_identity_context(&mut status, selector);
            }
            status
        }
        Commands::Stop => client::stop_daemon().await?,
        Commands::Whoami => {
            client::call(Operation::Whoami, Some(client::credential(selected)?)).await?
        }
        Commands::Unregister => {
            client::call(Operation::Unregister, Some(client::credential(selected)?)).await?
        }
        Commands::Rename { name } => {
            client::call(
                Operation::Rename { name },
                Some(client::credential(selected)?),
            )
            .await?
        }
        Commands::UpdatePurpose { purpose, clear } => {
            let purpose = if clear { None } else { purpose };
            client::call(
                Operation::UpdatePurpose { purpose },
                Some(client::credential(selected)?),
            )
            .await?
        }
        Commands::Wait {
            project,
            timeout,
            inbox_after,
            board_after,
        } => {
            let c = client::credential(selected)?;
            let explicit_project = project.is_some();
            let explicit_project_key = project
                .as_deref()
                .map(|path| context(Some(path)).map(|ctx| ctx.project))
                .transpose()?;
            let (_, generation) = client::begin_wait().await?;
            let wait_started = Instant::now();
            let project_key = match explicit_project_key {
                Some(project) => project,
                None => wait_identity_context(&c, generation).await?.project,
            };
            let project_arg = explicit_project.then(|| project_key.clone());
            if human {
                eprintln!(
                    "Waiting for inbox and board activity ({}).",
                    if timeout == 0 {
                        "indefinitely".to_string()
                    } else {
                        format!("timeout: {timeout}s")
                    }
                );
            }
            return wait_events(
                c,
                project_key,
                project_arg,
                timeout,
                inbox_after,
                board_after,
                WaitLifecycle {
                    started: wait_started,
                    generation,
                },
            )
            .await;
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
            supersedes,
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
                    supersedes,
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
                    supersedes: None,
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
                    supersedes: None,
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
        Commands::History {
            thread,
            after,
            cursor,
            limit,
        } => {
            let operation = if let Some(thread) = thread {
                Operation::History {
                    thread,
                    after: after.unwrap_or(0),
                    limit,
                }
            } else {
                Operation::ListThreads { cursor, limit }
            };
            client::call(operation, Some(client::credential(selected)?)).await?
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
                    supersedes: None,
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
fn bidi_formatting_control(c: char) -> bool {
    matches!(
        c as u32,
        0x061c | 0x200e..=0x200f | 0x202a..=0x202e | 0x2066..=0x2069
    )
}
fn escape_purpose_scalar(c: char) -> String {
    if c.is_control() || bidi_formatting_control(c) {
        c.escape_default().to_string()
    } else {
        c.to_string()
    }
}
fn human_purpose(purpose: Option<&str>) -> Option<String> {
    let purpose = purpose?;
    let mut chars = purpose.chars();
    let mut rendered = String::new();
    for _ in 0..80 {
        let Some(c) = chars.next() else {
            return Some(rendered);
        };
        rendered.push_str(&escape_purpose_scalar(c));
    }
    if chars.next().is_some() {
        rendered.push('…');
    }
    Some(rendered)
}
fn render_agent(agent: &Value) {
    println!(
        "Agent: {} ({})",
        agent["name"].as_str().unwrap_or(""),
        agent["id"].as_str().unwrap_or("")
    );
    if let Some(tool) = agent["tool"].as_str() {
        println!("Tool: {tool}");
    }
    if let Some(project) = agent["context"]["project"].as_str() {
        println!("Project: {project}");
    }
    println!(
        "Purpose: {}",
        human_purpose(agent["purpose"].as_str()).unwrap_or_else(|| "(not set)".into())
    );
}
pub fn render(data: &Value, attention: Option<&Value>) {
    if let Some(content) = data["content"].as_str() {
        print!("{content}");
    } else if data["identity_context"].is_object() && data["database"].is_string() {
        render_status(data);
    } else if data["events"].is_array() {
        render_wait(data);
    } else if data["incoming_requests"].is_array() {
        render_summary(data);
    } else if let Some(threads) = data["threads"].as_array() {
        render_thread_list(data, threads);
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
    } else if let Some(projects) = data["projects"].as_array() {
        render_projects(data, projects);
    } else if data["project"].is_string() && data["changed"].is_boolean() {
        let state = if data["closed"] == true {
            "closed"
        } else {
            "open"
        };
        println!(
            "Project {} is {state}{}.",
            data["project"].as_str().unwrap_or(""),
            if data["changed"] == true {
                ""
            } else {
                " (no change)"
            }
        );
    } else if data["agent"].is_object() && data["previous_name"].is_string() {
        let id = data["agent"]["id"].as_str().unwrap_or("");
        let name = data["agent"]["name"].as_str().unwrap_or("");
        if data["changed"] == true {
            println!(
                "Renamed agent {id} from {} to {name}.",
                data["previous_name"].as_str().unwrap_or("")
            );
        } else {
            println!("Agent {id} is already named {name}.");
        }
    } else if let Some(path) = data["selector_file"].as_str() {
        let quoted = shell_quote_selector_path(path);
        println!(
            "Registered {} ({}). Private selector saved to {path}; pass --agent-file={quoted} on tala commands.",
            data["agent"]["id"].as_str().unwrap_or(""),
            data["agent"]["name"].as_str().unwrap_or("")
        );
        if let Some(purpose) = human_purpose(data["agent"]["purpose"].as_str()) {
            println!("Purpose: {purpose}");
        }
        if let Some(next) = data["next"].as_str() {
            println!("Next: {next}");
        }
    } else if let Some(handle) = data["handle"].as_str() {
        if let Some(error) = data["selector_file_error"].as_str() {
            println!(
                "Registered {} ({}), but selector-file save failed: {error}. Use --agent={handle} now and keep this private handle.",
                data["agent"]["id"].as_str().unwrap_or(""),
                data["agent"]["name"].as_str().unwrap_or("")
            );
            return;
        }
        println!(
            "Registered {} ({}). Pass --agent={handle} on every tala command; keep the handle private.",
            data["agent"]["id"].as_str().unwrap_or(""),
            data["agent"]["name"].as_str().unwrap_or("")
        );
        if let Some(purpose) = human_purpose(data["agent"]["purpose"].as_str()) {
            println!("Purpose: {purpose}");
        }
        if let Some(next) = data["next"].as_str() {
            println!("Next: {next}");
        }
    } else if data["agent"].is_object() {
        render_agent(&data["agent"]);
        if let Some(updated_at) = data["agent"]["purpose_updated_at"].as_i64() {
            println!("Purpose updated at: {updated_at}");
        }
    } else if let Some(agents) = data["agents"].as_array() {
        for a in agents {
            let mut row = format!(
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
            if let Some(purpose) = human_purpose(a["purpose"].as_str()) {
                row.push_str("  Purpose: ");
                row.push_str(&purpose);
            }
            println!("{row}");
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
    let recipient_listening = data["recipient_presence"]["listening"]
        .as_bool()
        .or_else(|| data["sent"]["recipient_presence"]["listening"].as_bool());
    if let Some(listening) = recipient_listening {
        if listening {
            println!("Recipient was listening when Tala accepted the message; this is a point-in-time observation.");
        } else {
            println!("Recipient was not listening when Tala accepted the message. The message remains queued.");
        }
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

fn render_thread_list(data: &Value, threads: &[Value]) {
    if threads.is_empty() {
        println!("No visible threads.");
    } else {
        println!("THREAD | LAST ID | MESSAGES | UNREAD | PENDING IN/OUT | STALE IN/OUT | KIND | PROJECT | PARTICIPANTS | HANDOFF ONLY | EXCERPT");
        for thread in threads {
            let people = |field: &str| {
                thread[field]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|person| {
                        format!(
                            "{} ({})",
                            person["name"].as_str().unwrap_or(""),
                            person["id"].as_str().unwrap_or("")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let cells = [
                thread["thread"].as_str().unwrap_or("").to_string(),
                thread["last_message_id"].to_string(),
                thread["message_count"].to_string(),
                thread["unread"].to_string(),
                format!(
                    "{}/{}",
                    thread["pending_incoming"], thread["pending_outgoing"]
                ),
                format!(
                    "{}/{}",
                    thread["stale_pending_incoming"], thread["stale_pending_outgoing"]
                ),
                thread["kind"].as_str().unwrap_or("").to_string(),
                thread["project"].as_str().unwrap_or("").to_string(),
                people("participants"),
                people("handoff_only_recipients"),
                thread["excerpt"].as_str().unwrap_or("").to_string(),
            ];
            println!(
                "{}",
                cells
                    .iter()
                    .map(|cell| escape_status_text(cell).replace('|', "\\|"))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
        }
    }
    println!(
        "Snapshot high-water message ID: {}",
        data["snapshot_high_water_id"]
    );
    if data["has_more"] == true {
        if let Some(cursor) = data["next_cursor"].as_str() {
            println!("More threads: tala history --cursor={cursor}");
        }
    }
}
fn escape_status_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if !character.is_control() {
            escaped.push(character);
            continue;
        }
        match character {
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push_str(&format!("\\u{{{:x}}}", other as u32)),
        }
    }
    escaped
}

fn render_status(data: &Value) {
    let daemon = if data["compatible"] == false {
        "running (incompatible)"
    } else if data["running"] == true {
        "running"
    } else if data["running"].is_null() {
        "unreachable"
    } else {
        "stopped"
    };
    println!("Daemon: {daemon}");
    if data["stale_metadata"] == true {
        println!("Daemon metadata: stale");
    }
    let context = &data["identity_context"];
    println!(
        "Identity selector: {} ({})",
        context["selector_source"].as_str().unwrap_or("none"),
        context["selection_state"].as_str().unwrap_or("unknown")
    );
    if context["context_source"] == "local_database_snapshot"
        && context["unavailable_reason"].is_null()
        && context["selection_state"] == "selected"
    {
        println!("Identity context: available from a read-only local database snapshot");
    }
    if context["agent"].is_object() {
        let id = context["agent"]["id"].as_str().unwrap_or("unknown");
        match context["agent"]["name"].as_str() {
            Some(name) => println!(
                "Identity: {} ({})",
                escape_status_text(name),
                escape_status_text(id)
            ),
            None => println!("Identity ID: {}", escape_status_text(id)),
        }
    } else if context["selection_state"] == "none" {
        println!("Identity: none selected");
    } else {
        println!("Identity: unavailable");
    }
    if let Some(project) = context["project_root"].as_str() {
        println!("Registered project root: {}", escape_status_text(project));
    } else {
        println!("Registered project root: unavailable");
    }
    if let Some(latest) = context["board_latest_id"].as_u64() {
        println!("Board latest ID: {latest} (read-only high-water mark)");
    } else {
        println!("Board latest ID: unavailable");
    }
    if let Some(reason) = context["unavailable_reason"].as_str() {
        println!("Identity context unavailable: {reason}");
    }
    if let Some(error_code) = context["error_code"].as_str() {
        println!("Context error: {error_code}");
    }
    if let Some(home) = data["home"].as_str() {
        println!("Tala home: {}", escape_status_text(home));
    }
    if let Some(database) = data["database"].as_str() {
        println!("Database: {}", escape_status_text(database));
    }
    if let Some(error) = data["error"].as_object() {
        if let Some(code) = error.get("code").and_then(Value::as_str) {
            println!("Daemon diagnostic: {code}");
        }
    }
}
fn render_wait(data: &Value) {
    let events = data["events"].as_array().cloned().unwrap_or_default();
    if events.is_empty() {
        println!("No new events.");
    } else {
        for event in events {
            println!(
                "[{}] {} ({})",
                event["id"],
                event["type"].as_str().unwrap_or("event"),
                event["sources"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    if let Some(inbox) = data["next"]["inbox"].as_str() {
        println!("Fetch inbox: {inbox}");
    }
    if let Some(board) = data["next"]["board"].as_str() {
        println!("Fetch board: {board}");
    }
    if let Some(restart) = data["next"]["listen_again"].as_str() {
        println!("Listen again: {restart}");
    }
}
fn render_projects(data: &Value, projects: &[Value]) {
    if projects.is_empty() {
        println!("No projects match these filters.");
    }
    let now = store::now();
    for project in projects {
        let mut states = vec![if project["closed"] == true {
            "closed"
        } else {
            "open"
        }];
        if project["stale"] == true {
            states.push("stale");
        }
        let age = now.saturating_sub(project["last_activity"].as_i64().unwrap_or(now));
        println!(
            "{}  {}  owner: {} ({})  last activity: {} ago",
            states.join(", "),
            project["project"].as_str().unwrap_or(""),
            project["owner_name"].as_str().unwrap_or(""),
            project["owner_id"].as_str().unwrap_or(""),
            human_age(age),
        );
    }
    let stale = data["hidden"]["stale"].as_u64().unwrap_or(0);
    let closed = data["hidden"]["closed"].as_u64().unwrap_or(0);
    if stale > 0 {
        println!(
            "{stale} stale project(s) hidden. Include with: tala list-projects --stale (or --all)."
        );
    }
    if closed > 0 {
        println!(
            "{closed} closed project(s) hidden. Include with: tala list-projects --closed (or --all)."
        );
    }
    if data["has_more"] == true {
        println!(
            "More projects: repeat with --after='{}'",
            data["next_after"].as_str().unwrap_or("")
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
        if m["expects_reply"] != true || m["superseded_by"].as_u64().is_some() {
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
    if let Some(id) = v["supersedes"].as_u64() {
        println!("  Supersedes message {id}.");
    }
    if let Some(id) = v["superseded_by"].as_u64() {
        println!("  Superseded by message {id}; use that message as the current instruction.");
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
    fn board_baseline_requires_atomic_high_water_mark() {
        let error = board_latest_id(&json!({"next_after": 1})).unwrap_err();
        assert_eq!(error.code, "PROTOCOL_MISMATCH");
        assert_eq!(error.next.as_deref(), Some("tala stop"));
        assert_eq!(board_latest_id(&json!({"latest_id": 7})).unwrap(), 7);
    }
    #[test]
    fn consumed_inbox_wait_batch_survives_a_racing_stop() {
        let batch = json!({"messages":[{"id":7}],"next_after":7});
        assert_eq!(
            inbox_wait_result(batch.clone(), true, false)
                .unwrap()
                .unwrap()
                .data,
            batch
        );
        let error = match inbox_wait_result(batch, false, false) {
            Err(error) => error,
            Ok(_) => panic!("unconsumed inbox data should not outlive a stopped wait"),
        };
        assert_eq!(error.code, "DAEMON_STOPPED");
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
            supersedes: None,
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
    fn supersedes_is_optional_in_canonical_payload_and_keys() {
        let make = |supersedes| match keyed(
            "a",
            Operation::Send {
                to: Some("b".into()),
                project: None,
                thread: None,
                reply_to: None,
                parts: vec![Part::Text { text: "hi".into() }],
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
                supersedes,
            },
        ) {
            Operation::Send { key, .. } => key,
            _ => unreachable!(),
        };
        let ordinary = make(None);
        let amended = make(Some(12));
        let other_amendment = make(Some(13));
        assert_eq!(ordinary, make(None));
        assert_ne!(ordinary, amended);
        assert_ne!(amended, other_amendment);

        let old_material =
            json!(["a", "message", "b", null, null, null, false, [{"type":"text", "text":"hi"}]]);
        let old_digest = Sha256::digest(old_material.to_string().as_bytes());
        let expected_old_key = format!(
            "auto:{}",
            old_digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        assert_eq!(ordinary, expected_old_key);

        let value = serde_json::to_value(Operation::Send {
            to: Some("b".into()),
            project: None,
            thread: None,
            reply_to: None,
            parts: vec![Part::Text { text: "hi".into() }],
            request: false,
            kind: "message".into(),
            key: "k".into(),
            context: Context {
                project: "/p".into(),
                worktree: None,
                working_directory: None,
                branch: None,
                revision: None,
            },
            supersedes: None,
        })
        .unwrap();
        assert!(value.get("supersedes").is_none());
    }
    #[test]
    fn ages_render_in_days_hours_and_minutes() {
        assert_eq!(human_age(30), "<1m");
        assert_eq!(human_age(5400), "1h 30m");
        assert_eq!(human_age(91104 * 60), "63d 6h");
    }
    #[test]
    fn status_text_escapes_controls_without_changing_path_separators() {
        assert_eq!(escape_status_text("C:\\work\\project"), "C:\\work\\project");
        assert_eq!(
            escape_status_text("first\nsecond\t\u{1b}"),
            "first\\nsecond\\t\\u{1b}"
        );
    }
    #[test]
    fn human_purpose_escapes_controls_bidi_and_truncates_by_scalar() {
        assert_eq!(
            human_purpose(Some("team\nreviewer\u{202e}")),
            Some("team\\nreviewer\\u{202e}".into())
        );
        assert_eq!(
            human_purpose(Some(&"é".repeat(81))),
            Some(format!("{}…", "é".repeat(80)))
        );
        assert_eq!(
            human_purpose(Some(&"🧭".repeat(80)))
                .unwrap()
                .chars()
                .count(),
            80
        );
    }
}
