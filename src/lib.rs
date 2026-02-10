pub mod cli;
pub mod gate;
pub mod parser;
pub mod state;
pub mod tui;

use std::path::PathBuf;

/// Status of a diff hunk in the review process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HunkStatus {
    Unreviewed,
    Reviewed,
    Stale,
}

/// A single diff hunk.
#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub content: String,
    pub content_hash: String,
    pub status: HunkStatus,
}

/// A file containing diff hunks.
#[derive(Debug, Clone)]
pub struct DiffFile {
    pub path: PathBuf,
    pub hunks: Vec<DiffHunk>,
}

/// Review progress summary.
#[derive(Debug, Clone)]
pub struct ReviewProgress {
    pub total_hunks: usize,
    pub reviewed: usize,
    pub unreviewed: usize,
    pub stale: usize,
    pub files_remaining: usize,
    pub total_files: usize,
}
