use clap::Parser;
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

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
