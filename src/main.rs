use clap::{error::ErrorKind, Parser};
use tracing_subscriber::EnvFilter;

mod api;
mod cli;
mod daemon;
mod models;
mod store;

/// Rust ignores SIGPIPE by default, so printing to a closed pipe panics with
/// "failed printing to stdout: Broken pipe" (B038). For an agent CLI that is
/// routinely piped into `head`/`grep -m1`, restore the Unix default: die
/// quietly on SIGPIPE like every other command-line tool.
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(unix)]
    reset_sigpipe();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("tala=info".parse().unwrap())
                .from_env_lossy(),
        )
        .with_target(false)
        .init();

    let cli = match cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            if error.kind() == ErrorKind::InvalidSubcommand {
                if let Some(warning) = cli::unknown_command_integration_hint() {
                    eprintln!("{}", warning);
                    eprintln!(
                        "hint: run `tala --help` to inspect the installed command surface, or `tala init --refresh` to update this project's integration"
                    );
                }
            }
            error.exit();
        }
    };
    cli::run(cli).await
}
