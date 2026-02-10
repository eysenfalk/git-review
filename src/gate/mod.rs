use crate::state::ReviewDb;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const HOOK_MARKER: &str = "# Installed by git-review";
const HOOK_CONTENT: &str = "#!/bin/sh
# Installed by git-review
exec git-review gate check
";

/// Check whether all hunks have been reviewed (gate passes).
///
/// Returns `true` if all hunks for the given base ref are reviewed.
/// Returns `false` if any hunks are unreviewed or stale.
pub fn check_gate(db: &ReviewDb, base_ref: &str) -> Result<bool> {
    let progress = db.progress(base_ref)?;

    // Gate passes only if all hunks are reviewed (no unreviewed or stale hunks)
    Ok(progress.unreviewed == 0 && progress.stale == 0)
}

/// Install the pre-commit hook that enforces review gating.
///
/// If a pre-commit hook already exists, it is backed up to `.git/hooks/pre-commit.backup`.
/// The new hook will execute `git-review gate check` to enforce the review gate.
pub fn enable_gate(repo_root: &Path) -> Result<()> {
    let hooks_dir = repo_root.join(".git/hooks");
    let hook_path = hooks_dir.join("pre-commit");
    let backup_path = hooks_dir.join("pre-commit.backup");

    // Ensure hooks directory exists
    fs::create_dir_all(&hooks_dir).context("Failed to create .git/hooks directory")?;

    // Backup existing hook if present
    if hook_path.exists() {
        fs::copy(&hook_path, &backup_path).context("Failed to backup existing pre-commit hook")?;
    }

    // Write the new hook
    fs::write(&hook_path, HOOK_CONTENT).context("Failed to write pre-commit hook")?;

    // Make the hook executable (Unix only)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&hook_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&hook_path, perms).context("Failed to make hook executable")?;
    }

    Ok(())
}

/// Remove the pre-commit hook.
///
/// Only removes the hook if it contains the git-review marker comment.
/// This prevents accidentally removing user-created hooks.
pub fn disable_gate(repo_root: &Path) -> Result<()> {
    let hook_path = repo_root.join(".git/hooks/pre-commit");

    // Check if hook exists
    if !hook_path.exists() {
        return Ok(()); // Nothing to do
    }

    // Read hook content
    let content = fs::read_to_string(&hook_path).context("Failed to read pre-commit hook")?;

    // Only remove if it has our marker
    if content.contains(HOOK_MARKER) {
        fs::remove_file(&hook_path).context("Failed to remove pre-commit hook")?;
    }

    Ok(())
}
