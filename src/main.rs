use anyhow::Result;
use clap::Parser;
use tracing::info;

mod daemon;
mod health;
mod metrics;

#[derive(Parser, Debug)]
#[command(author, version, about = "Himalayas Browser - Agent-Native Operating System", long_about = None)]
struct Args {
    /// Configuration file path
    #[arg(short, long, default_value = "himalayas.toml")]
    config: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    init_logging(args.verbose);

    info!("🏔️ Himalayas Browser - Phase 0 Foundation");
    info!("Starting daemon...");

    let config = daemon::Config::load(&args.config)?;
    let daemon = daemon::Daemon::new(config).await?;

    info!("Daemon initialized successfully");
    info!("Listening on {}", daemon.health_addr());

    daemon.run().await?;

    Ok(())
}

fn init_logging(verbose: bool) {
    let filter_level = if verbose { "debug" } else { "info" };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .or_else(|_| tracing_subscriber::EnvFilter::try_new(filter_level))
                .unwrap(),
        )
        .with_writer(std::io::stderr)
        .init();
}
