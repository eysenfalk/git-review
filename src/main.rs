use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

use git_review::cli::{self, Commands, GateAction};
use git_review::gate::{check_gate, disable_gate, enable_gate};
use git_review::parser::parse_diff;
use git_review::state::ReviewDb;
use git_review::tui::{App, run_tui};

fn main() -> Result<()> {
    let args = cli::parse_args();

    match args.command {
        None => {
            match (args.diff_range, args.status) {
                (Some(range), status) => {
                    // Explicit range provided — always hunk review
                    handle_review(&range, status)?;
                }
                (None, true) => {
                    // --status with no range — status for HEAD
                    handle_review("HEAD", true)?;
                }
                (None, false) => {
                    // No args, no subcommand — auto-detect mode
                    let current = git_review::git::get_current_branch();
                    let default_branch = git_review::git::detect_default_branch();

                    match (current, default_branch) {
                        (Ok(Some(ref branch)), Ok(ref default)) if branch == default => {
                            handle_dashboard()?;
                        }
                        (Ok(Some(_)), Ok(default)) => {
                            let range = format!("{}..HEAD", default);
                            handle_review(&range, false)?;
                        }
                        _ => {
                            // Detached HEAD or can't detect branches — fall back
                            handle_review("HEAD", false)?;
                        }
                    }
                }
            }
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
                let repo_root =
                    git_review::git::find_repo_root().context("Not in a git repository")?;
                enable_gate(&repo_root)?;
                println!("✓ Review gate enabled (pre-commit hook installed)");
            }
            GateAction::Disable => {
                let repo_root =
                    git_review::git::find_repo_root().context("Not in a git repository")?;
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
        Some(Commands::Approve(args)) => {
            handle_approve(&args.diff_range, args.file.as_deref())?;
        }
        Some(Commands::Watch(args)) => {
            handle_watch(args.interval)?;
        }
        Some(Commands::Dashboard) => {
            handle_dashboard()?;
        }
    }

    Ok(())
}

/// Handle the dashboard mode — show branch overview.
fn handle_dashboard() -> Result<()> {
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
    let default_branch =
        git_review::git::detect_default_branch().context("Could not detect default branch")?;

    let db_path = repo_root.join(".git/review-state");
    std::fs::create_dir_all(&db_path)?;
    let db_file = db_path.join("review.db");
    let db = ReviewDb::open(&db_file)?;

    let app = App::new_dashboard(db, default_branch)?;
    run_tui(app)?;

    Ok(())
}

/// Handle the review command - either launch TUI or show status.
fn handle_review(diff_range: &str, status_only: bool) -> Result<()> {
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
    let base_ref = normalize_diff_range(diff_range);

    // Get the diff
    let diff_output = git_review::git::get_diff(diff_range).context("Failed to get git diff")?;

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

    if status_only {
        let mut db = ReviewDb::open(&db_file)?;
        db.sync_with_diff(&base_ref, &files)?;

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
        // Launch TUI — App::new_hunk_review handles DB sync internally
        let db = ReviewDb::open(&db_file)?;
        let app = App::new_hunk_review(files, db, base_ref)?;
        run_tui(app)?;
    }

    Ok(())
}

/// Handle gate check - check if all hunks are reviewed and exit with appropriate code.
fn handle_gate_check() -> Result<()> {
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
    let base_ref = "HEAD".to_string(); // Gate check uses staged changes

    // Get the diff
    let diff_output = git_review::git::get_diff(&base_ref).context("Failed to get git diff")?;
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
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
    let base_ref = "HEAD".to_string();

    // Get the diff
    let diff_output = git_review::git::get_diff(&base_ref).context("Failed to get git diff")?;
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
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
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

/// Normalize a diff range to a consistent base ref format.
fn normalize_diff_range(range: &str) -> String {
    range.to_string()
}

/// Handle approve command - bulk approve hunks.
fn handle_approve(diff_range: &str, file_filter: Option<&str>) -> Result<()> {
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
    let base_ref = normalize_diff_range(diff_range);
    let diff_output = git_review::git::get_diff(diff_range).context("Failed to get git diff")?;
    let files = parse_diff(&diff_output);

    if files.is_empty() {
        println!("No changes to approve");
        return Ok(());
    }

    let db_path = repo_root.join(".git/review-state");
    std::fs::create_dir_all(&db_path)?;
    let db_file = db_path.join("review.db");
    let mut db = ReviewDb::open(&db_file)?;
    db.sync_with_diff(&base_ref, &files)?;

    let count = if let Some(file_path) = file_filter {
        db.approve_file(&base_ref, file_path)?
    } else {
        db.approve_all(&base_ref)?
    };

    println!("✓ Approved {} hunks for {}", count, diff_range);
    Ok(())
}

/// Handle watch command - continuously monitor branches.
fn handle_watch(interval: u64) -> Result<()> {
    let repo_root = git_review::git::find_repo_root().context("Not in a git repository")?;
    println!("Watching for branches needing review (Ctrl+C to stop)...\n");

    loop {
        // Get list of local branches
        let output = Command::new("git")
            .args(["branch", "--format", "%(refname:short)"])
            .output()
            .context("Failed to list branches")?;
        let branches = String::from_utf8_lossy(&output.stdout);

        // Check each non-main branch
        for branch in branches.lines() {
            let branch = branch.trim();
            if branch == "main" || branch == "master" || branch.is_empty() {
                continue;
            }
            let diff_range = format!("main..{}", branch);
            if let Ok(diff_output) = git_review::git::get_diff(&diff_range) {
                let files = parse_diff(&diff_output);
                if files.is_empty() {
                    continue;
                }

                let db_path = repo_root.join(".git/review-state");
                std::fs::create_dir_all(&db_path).ok();
                let db_file = db_path.join("review.db");
                if let Ok(mut db) = ReviewDb::open(&db_file) {
                    db.sync_with_diff(&diff_range, &files).ok();
                    if let Ok(progress) = db.progress(&diff_range) {
                        let pct = if progress.total_hunks > 0 {
                            (progress.reviewed as f64 / progress.total_hunks as f64) * 100.0
                        } else {
                            0.0
                        };
                        let status = if progress.unreviewed == 0 && progress.stale == 0 {
                            "✓"
                        } else {
                            "○"
                        };
                        println!(
                            "{} {:40} {}/{} ({:.0}%)",
                            status, branch, progress.reviewed, progress.total_hunks, pct
                        );
                    }
                }
            }
        }
        println!("─── refreshing in {}s ───\n", interval);
        std::thread::sleep(std::time::Duration::from_secs(interval));
    }
}
