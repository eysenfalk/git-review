use anyhow::{Context, Result, bail};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use git_review::cli::{self, Commands, GateAction};
use git_review::gate::{check_gate, disable_gate, enable_gate};
use git_review::parser::parse_diff;
use git_review::state::ReviewDb;
use git_review::tui::run_tui;

fn main() -> Result<()> {
    let args = cli::parse_args();

    match args.command {
        None => {
            // Top-level usage: git-review [range] [--status]
            let diff_range = args.diff_range.unwrap_or_else(|| "HEAD".to_string());
            handle_review(&diff_range, args.status)?;
        }
        Some(Commands::Review(review_args)) => {
            let diff_range = review_args.diff_range.unwrap_or_else(|| "HEAD".to_string());
            handle_review(&diff_range, review_args.status)?;
        }
        Some(Commands::Status(status_args)) => {
            let diff_range = status_args.diff_range.unwrap_or_else(|| "HEAD".to_string());
            handle_review(&diff_range, true)?;
        }
        Some(Commands::Gate { action }) => match action {
            GateAction::Check => {
                handle_gate_check()?;
            }
            GateAction::Enable => {
                let repo_root = find_repo_root()?;
                enable_gate(&repo_root)?;
                println!("✓ Review gate enabled (pre-commit hook installed)");
            }
            GateAction::Disable => {
                let repo_root = find_repo_root()?;
                disable_gate(&repo_root)?;
                println!("✓ Review gate disabled");
            }
        },
        Some(Commands::Commit { git_args }) => {
            handle_commit(&git_args)?;
        }
        Some(Commands::Reset(reset_args)) => {
            let diff_range = reset_args.diff_range.unwrap_or_else(|| "HEAD".to_string());
            handle_reset(&diff_range)?;
        }
    }

    Ok(())
}

/// Handle the review command - either launch TUI or show status.
fn handle_review(diff_range: &str, status_only: bool) -> Result<()> {
    let repo_root = find_repo_root()?;
    let base_ref = normalize_diff_range(diff_range);

    // Get the diff
    let diff_output = get_git_diff(diff_range)?;

    // Parse the diff
    let files = parse_diff(&diff_output);

    if files.is_empty() {
        println!("No changes to review");
        return Ok(());
    }

    // Open database
    let db_path = repo_root.join(".git/review-state");
    std::fs::create_dir_all(&db_path)?;
    let db_file = db_path.join("review.db");
    let mut db = ReviewDb::open(&db_file)?;

    // Sync with current diff
    db.sync_with_diff(&base_ref, &files)?;

    if status_only {
        // Show progress summary
        let progress = db.progress(&base_ref)?;
        println!("Review Progress for {}", diff_range);
        println!("─────────────────────────────────────");
        println!(
            "  Reviewed:   {}/{} hunks ({:.0}%)",
            progress.reviewed,
            progress.total_hunks,
            if progress.total_hunks > 0 {
                (progress.reviewed as f64 / progress.total_hunks as f64) * 100.0
            } else {
                0.0
            }
        );
        println!("  Unreviewed: {}", progress.unreviewed);
        println!("  Stale:      {}", progress.stale);
        println!(
            "  Files:      {}/{} remaining",
            progress.files_remaining, progress.total_files
        );

        if progress.unreviewed == 0 && progress.stale == 0 {
            println!("\n✓ All hunks reviewed!");
        } else if progress.stale > 0 {
            println!("\n⚠ Some hunks have become stale (code changed since review)");
        }
    } else {
        // Launch TUI
        run_tui(files, db, base_ref)?;
    }

    Ok(())
}

/// Handle gate check - check if all hunks are reviewed and exit with appropriate code.
fn handle_gate_check() -> Result<()> {
    let repo_root = find_repo_root()?;
    let base_ref = "HEAD".to_string(); // Gate check uses staged changes

    // Get the diff
    let diff_output = get_git_diff(&base_ref)?;
    let files = parse_diff(&diff_output);

    if files.is_empty() {
        // No changes - gate passes
        std::process::exit(0);
    }

    // Open database
    let db_path = repo_root.join(".git/review-state/review.db");
    if !db_path.exists() {
        eprintln!("✗ Review gate: No review state found");
        eprintln!("  Run 'git-review' to review your changes");
        std::process::exit(1);
    }

    let db = ReviewDb::open(&db_path)?;

    // Check gate
    if check_gate(&db, &base_ref)? {
        println!("✓ Review gate passed");
        std::process::exit(0);
    } else {
        let progress = db.progress(&base_ref)?;
        eprintln!("✗ Review gate: Not all hunks reviewed");
        eprintln!(
            "  {}/{} hunks reviewed, {} unreviewed, {} stale",
            progress.reviewed, progress.total_hunks, progress.unreviewed, progress.stale
        );
        eprintln!("  Run 'git-review' to complete your review");
        std::process::exit(1);
    }
}

/// Handle commit command - check gate then execute git commit.
fn handle_commit(git_args: &[String]) -> Result<()> {
    let repo_root = find_repo_root()?;
    let base_ref = "HEAD".to_string();

    // Get the diff
    let diff_output = get_git_diff(&base_ref)?;
    let files = parse_diff(&diff_output);

    if files.is_empty() {
        bail!("No changes to commit");
    }

    // Check gate
    let db_path = repo_root.join(".git/review-state/review.db");
    if !db_path.exists() {
        bail!("No review state found. Run 'git-review' first to review your changes");
    }

    let db = ReviewDb::open(&db_path)?;

    if !check_gate(&db, &base_ref)? {
        let progress = db.progress(&base_ref)?;
        bail!(
            "Review gate failed: {}/{} hunks reviewed, {} unreviewed, {} stale. Run 'git-review' to complete your review",
            progress.reviewed,
            progress.total_hunks,
            progress.unreviewed,
            progress.stale
        );
    }

    // Gate passed - execute git commit
    println!("✓ Review gate passed, proceeding with commit");

    let status = Command::new("git")
        .arg("commit")
        .args(git_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to execute git commit")?;

    if !status.success() {
        bail!("git commit failed");
    }

    Ok(())
}

/// Handle reset command - clear review state for a diff range.
fn handle_reset(diff_range: &str) -> Result<()> {
    let repo_root = find_repo_root()?;
    let base_ref = normalize_diff_range(diff_range);

    let db_path = repo_root.join(".git/review-state/review.db");
    if !db_path.exists() {
        println!("No review state to reset");
        return Ok(());
    }

    let mut db = ReviewDb::open(&db_path)?;
    db.reset(&base_ref)?;

    println!("✓ Review state reset for {}", diff_range);
    Ok(())
}

/// Get git diff output for a given range.
///
/// Validates the range format to prevent shell injection.
fn get_git_diff(range: &str) -> Result<String> {
    // Validate range format to prevent shell injection
    validate_git_ref(range)?;

    let output = Command::new("git")
        .arg("diff")
        .arg(range)
        .output()
        .context("Failed to execute git diff")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git diff failed: {}", stderr);
    }

    String::from_utf8(output.stdout).context("git diff output is not valid UTF-8")
}

/// Find the root of the git repository.
fn find_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .context("Failed to execute git rev-parse")?;

    if !output.status.success() {
        bail!("Not in a git repository");
    }

    let path = String::from_utf8(output.stdout)
        .context("git rev-parse output is not valid UTF-8")?
        .trim()
        .to_string();

    Ok(PathBuf::from(path))
}

/// Validate a git ref to prevent shell injection.
///
/// Allows: alphanumeric, dash, underscore, slash, dot, tilde, caret, @
fn validate_git_ref(ref_str: &str) -> Result<()> {
    if ref_str.is_empty() {
        bail!("Empty git ref");
    }

    // Check for shell metacharacters
    for ch in ref_str.chars() {
        if !ch.is_alphanumeric() && !matches!(ch, '-' | '_' | '/' | '.' | '~' | '^' | '@' | ':') {
            bail!("Invalid character in git ref: '{}'", ch);
        }
    }

    Ok(())
}

/// Normalize a diff range to a consistent base ref format.
///
/// Converts "main..HEAD" to "main..HEAD" (as-is) and "HEAD" to "HEAD".
fn normalize_diff_range(range: &str) -> String {
    range.to_string()
}
