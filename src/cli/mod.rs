use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "git-review", about = "Per-hunk review tracking for git diffs")]
pub struct Cli {
    /// Diff range to review (e.g., "main..HEAD"). Shorthand for `review <range>`.
    pub diff_range: Option<String>,

    /// Show progress summary instead of launching TUI.
    #[arg(short, long)]
    pub status: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Open the interactive review TUI (default) or show status.
    Review(ReviewArgs),
    /// Print review progress summary.
    Status(StatusArgs),
    /// Manage the pre-commit review gate.
    Gate {
        #[command(subcommand)]
        action: GateAction,
    },
    /// Commit changes after passing review gate.
    Commit {
        /// Additional arguments to pass to git commit (after --).
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        git_args: Vec<String>,
    },
    /// Reset review state for the current diff.
    Reset(ResetArgs),
    /// Approve all hunks (or specific file) without individual review.
    Approve(ApproveArgs),
    /// Watch branches for review status changes.
    Watch(WatchArgs),
}

#[derive(Args, Debug)]
pub struct ReviewArgs {
    /// Diff range to review (e.g., "main..HEAD" or "HEAD~3..HEAD").
    /// If not specified, defaults to "HEAD" (staged changes).
    pub diff_range: Option<String>,

    /// Show progress summary instead of launching TUI.
    #[arg(short, long)]
    pub status: bool,
}

#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Diff range to check status for (e.g., "main..HEAD").
    /// If not specified, defaults to "HEAD" (staged changes).
    pub diff_range: Option<String>,
}

#[derive(Args, Debug)]
pub struct ResetArgs {
    /// Diff range to reset review state for (e.g., "main..HEAD").
    /// If not specified, defaults to "HEAD" (staged changes).
    pub diff_range: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum GateAction {
    /// Check if all hunks are reviewed.
    Check,
    /// Install the pre-commit hook.
    Enable,
    /// Remove the pre-commit hook.
    Disable,
}

#[derive(Args, Debug)]
pub struct ApproveArgs {
    /// Diff range to approve (e.g., "main..HEAD").
    pub diff_range: String,
    /// Approve only hunks in this file path.
    #[arg(short, long)]
    pub file: Option<String>,
}

#[derive(Args, Debug)]
pub struct WatchArgs {
    /// Refresh interval in seconds (default: 5).
    #[arg(short, long, default_value = "5")]
    pub interval: u64,
}

/// Parse CLI arguments.
pub fn parse_args() -> Cli {
    Cli::parse()
}
