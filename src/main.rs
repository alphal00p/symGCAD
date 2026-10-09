use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Exact generic cylindrical algebraic decomposition using Symbolica")]
struct Cli { #[command(subcommand)] command: Command }
#[derive(Subcommand)]
enum Command {
    Solve { input: PathBuf },
    Project { input: PathBuf },
    Order { input: PathBuf },
    Verify { input: PathBuf },
    Bench { input: PathBuf },
}
fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    anyhow::bail!("solver implementation is being assembled; no result was computed")
}
