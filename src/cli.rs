use crate::{client, daemon, integration, models::*, store};
use clap::{Args, Parser, Subcommand};
use serde_json::{json, Value};
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
    about = "Local messaging for distinct coding agents",
    after_help = "Start: tala register --name=codex --tool=codex\nSelect your returned ID with TALA_AGENT or --agent.\nThen: tala agents; tala send --to=<id> --request 'Review this'; tala inbox; tala reply <id> 'Done'\nUse inbox --wait to receive, board for shared updates, history for context.\nExit codes: 0 success, 1 operational error, 2 usage error, 3 wait timeout."
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        short = 'j',
        help = "Emit one JSON envelope; errors go to stderr"
    )]
    pub json: bool,
    #[arg(
        long,
        global = true,
        env = "TALA_AGENT",
        help = "Your complete registered agent ID (independent of CWD)"
    )]
    pub agent: Option<String>,
    #[command(subcommand)]
    pub command: Commands,
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
    /// Register a distinct agent instance; keep the returned ID for this run
    Register {
        #[arg(long)]
        name: String,
        #[arg(long)]
        tool: String,
        #[arg(long, help = "Checkout path; defaults to the current project root")]
        project: Option<PathBuf>,
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
    let project =
        git(&["rev-parse", "--show-toplevel"]).unwrap_or_else(|| dir.display().to_string());
    Ok(Context {
        project,
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
    context(Some(Path::new(&registered.context.project)))
}
async fn wait_inbox(
    credential: Credential,
    peek: bool,
    after: u64,
    limit: usize,
    reply_to: Option<u64>,
    timeout: u64,
) -> Result<Outcome> {
    let start = Instant::now();
    let expired = || timeout != 0 && start.elapsed() >= Duration::from_secs(timeout);
    let timeout_result = || Outcome {
        data: json!({"messages":[],"next_after":after,"has_more":false,"timed_out":true}),
        code: 3,
    };
    loop {
        if expired() {
            return Ok(timeout_result());
        }
        // Poll without consumption, so cancelling a network call at the deadline
        // cannot acknowledge a batch that we never return to the caller.
        let poll = client::call(
            Operation::Inbox {
                peek: true,
                after,
                limit,
                reply_to,
                listening: true,
            },
            Some(credential.clone()),
        );
        let data = if timeout == 0 {
            poll.await?
        } else {
            let remaining = Duration::from_secs(timeout).saturating_sub(start.elapsed());
            match tokio::time::timeout(remaining, poll).await {
                Ok(result) => result?,
                Err(_) => return Ok(timeout_result()),
            }
        };
        if data["messages"].as_array().is_some_and(|m| !m.is_empty()) {
            if peek {
                return Ok(done(data));
            }
            let consumed = client::call(
                Operation::Inbox {
                    peek: false,
                    after,
                    limit,
                    reply_to,
                    listening: false,
                },
                Some(credential.clone()),
            )
            .await?;
            if consumed["messages"]
                .as_array()
                .is_some_and(|m| !m.is_empty())
            {
                return Ok(done(consumed));
            }
        }
        if expired() {
            return Ok(timeout_result());
        }
        let delay = if timeout == 0 {
            Duration::from_millis(200)
        } else {
            Duration::from_millis(200)
                .min(Duration::from_secs(timeout).saturating_sub(start.elapsed()))
        };
        tokio::time::sleep(delay).await;
    }
}

pub async fn run(cli: Cli) -> Result<Outcome> {
    let selected = cli.agent.as_deref();
    let json_output = cli.json;
    let data = match cli.command {
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
            name,
            tool,
            project,
        } => {
            let mut data = client::call(
                Operation::Register {
                    name,
                    tool,
                    context: context(project.as_deref())?,
                },
                None,
            )
            .await?;
            let c: Credential = serde_json::from_value(data["credential"].clone())
                .map_err(|e| usage(e.to_string()))?;
            let directory = store::home().join("identities");
            store::private_dir(&directory)?;
            store::private_write(
                &directory.join(format!("{}.json", c.agent)),
                &serde_json::to_vec(&c).map_err(|e| usage(e.to_string()))?,
            )?;
            data.as_object_mut()
                .expect("register object")
                .remove("credential");
            data["select"] = json!(format!("export TALA_AGENT={}", c.agent));
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
                None,
            )
            .await?
        }
        Commands::Status { message } => {
            let mut status = client::existing(Operation::Status)
                .await?
                .unwrap_or_else(|| json!({"running":false,"protocol_version":PROTOCOL_VERSION}));
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
            timeout,
            page,
        } => {
            let c = client::credential(selected)?;
            if wait {
                if !json_output {
                    eprintln!(
                        "Waiting for messages addressed to {} (timeout: {}s; 0 = indefinite)",
                        c.agent, timeout
                    );
                }
                return wait_inbox(c, peek, page.after, page.limit, None, timeout).await;
            }
            client::call(
                Operation::Inbox {
                    peek,
                    after: page.after,
                    limit: page.limit,
                    reply_to: None,
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
            let data = client::call(
                Operation::Send {
                    to: Some(to),
                    project: None,
                    thread,
                    reply_to: None,
                    parts,
                    request: request || wait,
                    kind: "message".into(),
                    key: key.unwrap_or_else(|| Uuid::new_v4().to_string()),
                    context: identity_context(&c).await?,
                },
                Some(c.clone()),
            )
            .await?;
            if wait {
                let id = data["message"]["id"].as_u64().expect("stored message ID");
                if !json_output {
                    eprintln!("Stored message {id}; waiting for its correlated reply.");
                }
                let mut result = wait_inbox(c, false, 0, 50, Some(id), timeout).await?;
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
            client::call(
                Operation::Send {
                    to: None,
                    project: None,
                    thread: None,
                    reply_to: Some(message),
                    parts,
                    request,
                    kind: "reply".into(),
                    key: key.unwrap_or_else(|| Uuid::new_v4().to_string()),
                    context: identity_context(&c).await?,
                },
                Some(c),
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
            client::call(
                Operation::Send {
                    to: None,
                    project: Some(p),
                    thread: None,
                    reply_to: None,
                    parts,
                    request: false,
                    kind: "post".into(),
                    key: key.unwrap_or_else(|| Uuid::new_v4().to_string()),
                    context: ctx,
                },
                Some(c),
            )
            .await?
        }
        Commands::Board { project, page } => {
            let c = client::credential(selected)?;
            let p = project
                .as_deref()
                .map(|p| context(Some(p)).map(|c| c.project))
                .transpose()?
                .unwrap_or(identity_context(&c).await?.project);
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
        Commands::Pending { direction, page } => {
            client::call(
                Operation::Pending {
                    direction,
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
            client::call(
                Operation::Send {
                    to: Some(to),
                    project: None,
                    thread: Some(thread),
                    reply_to: None,
                    parts,
                    request: true,
                    kind: "handoff".into(),
                    key: key.unwrap_or_else(|| Uuid::new_v4().to_string()),
                    context: identity_context(&c).await?,
                },
                Some(c),
            )
            .await?
        }
    };
    Ok(done(data))
}
pub fn render(data: &Value) {
    if let Some(messages) = data["messages"].as_array() {
        if messages.is_empty() {
            println!("No messages.");
        }
        for message in messages {
            render_message(message);
        }
        if data["has_more"] == true {
            println!("More messages: repeat with --after={}", data["next_after"]);
        }
    } else if data["message"].is_object() {
        render_message(&data["message"]);
        if data["duplicate"] == true {
            println!("Duplicate suppressed.");
        }
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
        println!("Wait timed out; stored messages remain available.");
    }
}
fn render_message(v: &Value) {
    println!(
        "[{}] {} → {}  {}{}  thread {}",
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
}
