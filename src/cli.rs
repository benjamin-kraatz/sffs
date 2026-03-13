use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about = "Super Fast File Size (sffs)", long_about = None)]
pub struct Args {
    /// Path(s) to check size for. If omitted, checks the current directory.
    #[arg()]
    pub paths: Vec<PathBuf>,

    /// Follow symbolic links
    #[arg(short = 'L', long)]
    pub follow_links: bool,

    /// Respect .gitignore files
    #[arg(short = 'g', long)]
    pub git_ignore: bool,

    /// Respect .ignore files
    #[arg(short = 'i', long)]
    pub ignore_files: bool,

    /// Ignore hidden files
    #[arg(short = 'H', long)]
    pub ignore_hidden: bool,

    /// Maximum depth to recurse
    #[arg(short = 'd', long)]
    pub max_depth: Option<usize>,

    /// Use the provided number of threads
    #[arg(short = 't', long)]
    pub threads: Option<usize>,

    /// Show size in raw bytes
    #[arg(short = 'b', long)]
    pub bytes: bool,

    /// Use SI units (1000 bytes = 1 KB) instead of 1024
    #[arg(long)]
    pub si: bool,

    /// Don't cross filesystem boundaries
    #[arg(short = 'x', long)]
    pub one_file_system: bool,

    /// Show top N largest files
    #[arg(long, value_name = "N")]
    pub top: Option<usize>,

    /// Suppress headers and footer
    #[arg(short = 's', long)]
    pub silent: bool,
}
