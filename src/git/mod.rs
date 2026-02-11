use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("not in a git repository")]
    NotARepo,
    #[error("git command failed: {0}")]
    CommandFailed(String),
    #[error("invalid git ref: {0}")]
    InvalidRef(String),
    #[error("merge failed: {0}")]
    MergeFailed(String),
    #[error("utf-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GitError>;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_local: bool,
    pub last_commit_sha: String,
    pub last_commit_author: String,
    pub last_commit_age: String,
    pub last_commit_timestamp: i64,
}

/// Loaded lazily — only for visible/selected branches
#[derive(Debug, Clone, Default)]
pub struct BranchDetail {
    pub ahead: u32,
    pub behind: u32,
    pub diff_stats: DiffStats,
}

#[derive(Debug, Clone, Default)]
pub struct DiffStats {
    pub file_count: usize,
    pub insertions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct MergeOptions {
    pub branch: String,
    pub delete_after: bool,
}

#[derive(Debug, Clone)]
pub enum WorktreeStatus {
    Clean,
    Dirty { modified: usize, untracked: usize },
}

/// Result of a merge conflict pre-check
pub enum MergeCheck {
    Clean,
    Conflicts,
    Error(String),
}

/// Find the root of the git repository.
pub fn find_repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()?;

    if !output.status.success() {
        return Err(GitError::NotARepo);
    }

    let path = String::from_utf8(output.stdout)?.trim().to_string();

    Ok(PathBuf::from(path))
}

/// Validate a git ref to prevent shell injection (only for user-supplied refs).
pub fn validate_git_ref(ref_str: &str) -> Result<()> {
    if ref_str.is_empty() {
        return Err(GitError::InvalidRef("Empty git ref".to_string()));
    }

    // Check for shell metacharacters
    for ch in ref_str.chars() {
        if !ch.is_alphanumeric()
            && !matches!(
                ch,
                '-' | '_' | '/' | '.' | '~' | '^' | '@' | ':' | '{' | '}'
            )
        {
            return Err(GitError::InvalidRef(format!(
                "Invalid character in git ref: '{}'",
                ch
            )));
        }
    }

    Ok(())
}

/// Detect the default branch (origin/HEAD -> main -> master fallback).
pub fn detect_default_branch() -> Result<String> {
    // Try to get origin/HEAD symbolic ref
    let output = Command::new("git")
        .arg("symbolic-ref")
        .arg("refs/remotes/origin/HEAD")
        .output()?;

    if output.status.success() {
        let symbolic = String::from_utf8(output.stdout)?;
        let trimmed = symbolic.trim();
        if let Some(branch) = trimmed.strip_prefix("refs/remotes/origin/") {
            return Ok(branch.to_string());
        }
    }

    // Fallback: try main
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("main")
        .output()?;

    if output.status.success() {
        return Ok("main".to_string());
    }

    // Fallback: try master
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg("master")
        .output()?;

    if output.status.success() {
        return Ok("master".to_string());
    }

    Err(GitError::CommandFailed(
        "could not detect default branch".to_string(),
    ))
}

/// Get git diff output for a given range.
pub fn get_diff(range: &str) -> Result<String> {
    validate_git_ref(range)?;

    let output = Command::new("git").arg("diff").arg(range).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git diff failed: {}",
            stderr
        )));
    }

    String::from_utf8(output.stdout).map_err(GitError::from)
}

/// List all local branches via a single git for-each-ref call.
pub fn list_branches() -> Result<Vec<BranchInfo>> {
    let output = Command::new("git")
        .arg("for-each-ref")
        .arg("--format=%(refname:short)|%(objectname:short)|%(authorname)|%(committerdate:relative)|%(committerdate:unix)")
        .arg("--sort=-committerdate")
        .arg("refs/heads/")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git for-each-ref failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut branches = Vec::new();

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() >= 5 {
            branches.push(BranchInfo {
                name: fields[0].to_string(),
                is_local: true,
                last_commit_sha: fields[1].to_string(),
                last_commit_author: fields[2].to_string(),
                last_commit_age: fields[3].to_string(),
                last_commit_timestamp: fields[4].parse::<i64>().unwrap_or(0),
            });
        }
    }

    Ok(branches)
}

/// Get ahead/behind counts and diff stats for a branch (lazy, per-branch).
pub fn get_branch_detail(base: &str, branch: &str) -> Result<BranchDetail> {
    // Get ahead/behind counts
    let output = Command::new("git")
        .arg("rev-list")
        .arg("--count")
        .arg("--left-right")
        .arg(format!("{}...{}", base, branch))
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git rev-list failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let parts: Vec<&str> = stdout.trim().split('\t').collect();
    let (behind, ahead) = if parts.len() >= 2 {
        (
            parts[0].parse::<u32>().unwrap_or(0),
            parts[1].parse::<u32>().unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    // Get diff stats
    let output = Command::new("git")
        .arg("diff")
        .arg("--numstat")
        .arg(format!("{}..{}", base, branch))
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git diff --numstat failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8(output.stdout)?;
    let mut file_count = 0;
    let mut insertions = 0;
    let mut deletions = 0;

    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 {
            // Skip binary files (marked with "-")
            if parts[0] == "-" || parts[1] == "-" {
                continue;
            }

            file_count += 1;
            insertions += parts[0].parse::<usize>().unwrap_or(0);
            deletions += parts[1].parse::<usize>().unwrap_or(0);
        }
    }

    Ok(BranchDetail {
        ahead,
        behind,
        diff_stats: DiffStats {
            file_count,
            insertions,
            deletions,
        },
    })
}

/// Get current HEAD SHA (lightweight staleness check).
pub fn get_head_sha() -> Result<String> {
    let output = Command::new("git").arg("rev-parse").arg("HEAD").output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git rev-parse HEAD failed: {}",
            stderr
        )));
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

/// Check if the worktree has uncommitted changes.
pub fn check_worktree_status() -> Result<WorktreeStatus> {
    let output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git status --porcelain failed: {}",
            stderr
        )));
    }

    let stdout = String::from_utf8(output.stdout)?;
    if stdout.trim().is_empty() {
        return Ok(WorktreeStatus::Clean);
    }

    let mut modified = 0;
    let mut untracked = 0;

    for line in stdout.lines() {
        if line.starts_with("?? ") {
            untracked += 1;
        } else {
            modified += 1;
        }
    }

    Ok(WorktreeStatus::Dirty {
        modified,
        untracked,
    })
}

/// Pre-check for merge conflicts using git merge-tree.
pub fn check_merge_conflicts(base: &str, branch: &str) -> Result<MergeCheck> {
    // Try modern git merge-tree --write-tree first
    let output = Command::new("git")
        .arg("merge-tree")
        .arg("--write-tree")
        .arg(base)
        .arg(branch)
        .output()?;

    match output.status.code() {
        Some(0) => Ok(MergeCheck::Clean),
        Some(1) => Ok(MergeCheck::Conflicts),
        _ => {
            // Older git version or other error
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("unknown option") || stderr.contains("usage:") {
                // Fall back to 3-arg form for older git versions
                let merge_base_output = Command::new("git")
                    .arg("merge-base")
                    .arg(base)
                    .arg(branch)
                    .output()?;

                if !merge_base_output.status.success() {
                    return Ok(MergeCheck::Error("Could not find merge base".to_string()));
                }

                let merge_base = String::from_utf8(merge_base_output.stdout)?
                    .trim()
                    .to_string();

                let fallback_output = Command::new("git")
                    .arg("merge-tree")
                    .arg(&merge_base)
                    .arg(base)
                    .arg(branch)
                    .output()?;

                if !fallback_output.status.success() {
                    return Ok(MergeCheck::Error(
                        String::from_utf8_lossy(&fallback_output.stderr).to_string(),
                    ));
                }

                let stdout = String::from_utf8(fallback_output.stdout)?;
                if stdout.contains("<<<<<<<") {
                    Ok(MergeCheck::Conflicts)
                } else {
                    Ok(MergeCheck::Clean)
                }
            } else {
                Ok(MergeCheck::Error(stderr.to_string()))
            }
        }
    }
}

/// Execute git merge --no-ff. Auto-aborts on failure.
pub fn merge_branch(options: &MergeOptions) -> Result<()> {
    let output = Command::new("git")
        .arg("merge")
        .arg("--no-ff")
        .arg(&options.branch)
        .output()?;

    if !output.status.success() {
        // Abort the merge
        let _ = Command::new("git").arg("merge").arg("--abort").output();

        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::MergeFailed(stderr.to_string()));
    }

    if options.delete_after {
        delete_branch(&options.branch)?;
    }

    Ok(())
}

/// Delete a branch (safe delete, not force).
pub fn delete_branch(name: &str) -> Result<()> {
    let output = Command::new("git")
        .arg("branch")
        .arg("-d")
        .arg(name)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git branch -d failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Get the current branch name (None for detached HEAD).
pub fn get_current_branch() -> Result<Option<String>> {
    let output = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitError::CommandFailed(format!(
            "git branch --show-current failed: {}",
            stderr
        )));
    }

    let branch = String::from_utf8(output.stdout)?.trim().to_string();
    if branch.is_empty() {
        Ok(None)
    } else {
        Ok(Some(branch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_git_ref_valid() {
        assert!(validate_git_ref("main").is_ok());
        assert!(validate_git_ref("feature/foo").is_ok());
        assert!(validate_git_ref("HEAD~1").is_ok());
        assert!(validate_git_ref("main..HEAD").is_ok());
        assert!(validate_git_ref("v1.2.3").is_ok());
        assert!(validate_git_ref("origin/main").is_ok());
        assert!(validate_git_ref("HEAD^").is_ok());
        assert!(validate_git_ref("@{-1}").is_ok());
    }

    #[test]
    fn test_validate_git_ref_invalid() {
        assert!(validate_git_ref(";rm -rf").is_err());
        assert!(validate_git_ref("$(cmd)").is_err());
        assert!(validate_git_ref("|pipe").is_err());
        assert!(validate_git_ref("&bg").is_err());
        assert!(validate_git_ref("foo bar").is_err());
        assert!(validate_git_ref("foo\nbar").is_err());
    }

    #[test]
    fn test_validate_git_ref_empty() {
        assert!(validate_git_ref("").is_err());
    }

    #[test]
    fn test_get_head_sha() {
        let result = get_head_sha();
        assert!(result.is_ok());
        let sha = result.unwrap();
        assert!(!sha.is_empty());
        // SHA should be hexadecimal
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_get_current_branch() {
        let result = get_current_branch();
        assert!(result.is_ok());
        let branch = result.unwrap();
        // We should be on a branch (not detached HEAD) in this test
        assert!(branch.is_some());
    }

    #[test]
    fn test_find_repo_root() {
        let result = find_repo_root();
        assert!(result.is_ok());
        let path = result.unwrap();
        // The path should contain .git directory
        let git_dir = path.join(".git");
        assert!(
            git_dir.exists(),
            "Expected .git directory to exist at {:?}",
            git_dir
        );
    }
}
