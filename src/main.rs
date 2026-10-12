use clap::Parser;
use models::{Envelope, Failure};
mod api;
mod cli;
mod client;
mod daemon;
mod integration;
mod models;
mod store;
#[tokio::main]
async fn main() {
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let human = std::env::args()
        .take_while(|a| a != "--")
        .any(|a| a == "--human");
    let cli = match cli::Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                let _ = e.print();
                return;
            }
            if human {
                let _ = e.print();
            } else {
                eprintln!(
                    "{}",
                    serde_json::to_string(&Envelope::failure(Failure::new(
                        "USAGE_ERROR",
                        e.to_string(),
                        "Run tala <command> --help."
                    )))
                    .expect("error serializes")
                );
            }
            std::process::exit(2);
        }
    };
    let human = cli.human;
    let is_daemon = matches!(cli.command, Some(cli::Commands::Daemon));
    match cli::run(cli).await {
        Ok(result) => {
            if !is_daemon {
                let attention = client::take_attention();
                if human {
                    cli::render(&result.data, attention.as_ref());
                } else {
                    println!(
                        "{}",
                        serde_json::to_string(&Envelope {
                            attention,
                            ..Envelope::success(result.data)
                        })
                        .expect("output serializes")
                    );
                }
            }
            if result.code != 0 {
                std::process::exit(result.code);
            }
        }
        Err(error) => {
            let code = if error.code == "USAGE_ERROR" {
                2
            } else if error.stored.is_some() {
                4
            } else {
                1
            };
            if human {
                eprintln!("Error: {}\n{}", error.message, error.hint);
                if let Some(next) = &error.next {
                    eprintln!("Next: {next}");
                }
            } else {
                eprintln!(
                    "{}",
                    serde_json::to_string(&Envelope::failure(error)).expect("error serializes")
                );
            }
            std::process::exit(code);
        }
    }
}
