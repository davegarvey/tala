use clap::Parser;
use tracing_subscriber::EnvFilter;

mod api;
mod cli;
mod daemon;
mod models;
mod store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive("chit=info".parse().unwrap())
                .from_env_lossy(),
        )
        .with_target(false)
        .init();

    let cli = cli::Cli::parse();
    cli::run(cli).await
}
