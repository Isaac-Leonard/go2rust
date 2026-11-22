use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version,about,long_about=None)]
pub struct Cli {
    #[arg(long)]
    pub input: PathBuf,
    #[arg(long)]
    pub output: Option<PathBuf>,
}
