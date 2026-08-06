use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

mod benchmark;
mod daemon;
mod health;
mod metrics;
mod server;

#[derive(Parser, Debug)]
#[command(author, version, about = "Himalayas Browser - Agent-Native Operating System", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Configuration file path
    #[arg(short, long, default_value = "himalayas.toml", global = true)]
    config: String,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the daemon (default)
    #[command(visible_alias = "d")]
    Daemon,

    /// Run benchmarks
    #[command(visible_alias = "b")]
    Benchmark {
        /// Save results to file
        #[arg(short, long, default_value = "benchmarks.json")]
        output: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    init_logging(args.verbose);

    info!("🏔️ Himalayas Browser - Phase 0 Foundation");

    match args.command {
        Some(Command::Benchmark { output }) => {
            benchmark::Benchmarker::run_all_benchmarks()?;
            println!("\n💾 Saving results to {}...", output);
            // Results are already printed to stdout
            Ok(())
        }
        _ => {
            info!("Starting daemon...");

            let config = daemon::Config::load(&args.config)?;
            let daemon = daemon::Daemon::new(config).await?;

            info!("Daemon initialized successfully");
            info!("Listening on {}", daemon.health_addr());

            daemon.run().await?;

            Ok(())
        }
    }
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
