use std::error::Error;
use std::path::PathBuf;

use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};
use transaction::io;

/// Struct to register all CLI args.
#[derive(Debug, Parser)]
#[command(about = "A simple toy payments engine!")]
struct Cli {
    /// The payment inputs as a path to a valid CSV file
    input_file_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    // Install logger
    fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Parse CLI args
    let cli = Cli::parse();

    tracing::info!("Processing payments from input file: `{}`", cli.input_file_path.display());

    let file = tokio::fs::File::open(cli.input_file_path).await?;

    let reader = io::reader(file)?;
    let writer = io::writer(tokio::io::stdout())?;

    io::process(reader, writer).await?;

    Ok(())
}
