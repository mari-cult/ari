use clap::Parser;
use std::path::PathBuf;

/// A re-creation of Discord's discontinued Clyde AI experiment
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Config .toml file
    #[arg(short, long, default_value = "./Clyde.toml")]
    pub config: PathBuf,
}

pub fn parse_cli() -> Cli {
    Cli::parse()
}
