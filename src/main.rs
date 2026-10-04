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
    let json = std::env::args()
        .take_while(|a| a != "--")
        .any(|a| a == "--json" || a == "-j");
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
            if json {
                eprintln!(
                    "{}",
                    serde_json::to_string(&Envelope::failure(Failure::new(
                        "USAGE_ERROR",
                        e.to_string(),
                        "Run tala <command> --help."
                    )))
                    .expect("error serializes")
                );
            } else {
                let _ = e.print();
            }
            std::process::exit(2);
        }
    };
    let json = cli.json;
    let is_daemon = matches!(cli.command, cli::Commands::Daemon);
    match cli::run(cli).await {
        Ok(result) => {
            if !is_daemon {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string(&Envelope::success(result.data))
                            .expect("output serializes")
                    );
                } else {
                    cli::render(&result.data);
                }
            }
            if result.code != 0 {
                std::process::exit(result.code);
            }
        }
        Err(error) => {
            let code = if error.code == "USAGE_ERROR" { 2 } else { 1 };
            if json {
                eprintln!(
                    "{}",
                    serde_json::to_string(&Envelope::failure(error)).expect("error serializes")
                );
            } else {
                eprintln!("Error: {}\n{}", error.message, error.hint);
            }
            std::process::exit(code);
        }
    }
}
