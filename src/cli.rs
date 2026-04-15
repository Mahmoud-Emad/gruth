//! Command-line argument definitions.

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "gruth",
    version,
    about = "gruth — Git Repository UTility Helper",
    long_about = "gruth (Git Repository UTility Helper) recursively discovers git repositories \
                  under a root directory and displays a live-updating terminal dashboard showing \
                  branch, status, ahead/behind, and last commit age for each repo."
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

    /// Root directory to scan (opens directory picker if omitted)
    #[arg(short, long)]
    pub path: Option<PathBuf>,

    /// Days without commits before a repo is considered stale
    #[arg(long)]
    pub stale_days: Option<u64>,
}
