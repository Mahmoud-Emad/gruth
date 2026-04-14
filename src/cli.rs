use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gruth",
    version,
    about = "A TUI dashboard for monitoring and syncing multiple git repositories",
    long_about = "gruth recursively discovers git repositories under a root directory and \
                  displays a live-updating terminal dashboard showing branch, status, \
                  ahead/behind, and last commit age for each repo."
)]
pub struct Args {
    /// Fetch and pull all clean repos, then exit
    #[arg(long)]
    pub sync: bool,

    /// Max recursion depth for repo discovery
    #[arg(short, long)]
    pub depth: Option<usize>,

    /// Refresh interval in seconds
    #[arg(short, long)]
    pub interval: Option<u64>,

    /// Root directory to scan
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,

    /// Days without commits before a repo is considered stale
    #[arg(long)]
    pub stale_days: Option<u64>,
}
