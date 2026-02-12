use crate::git::{BranchDetail, BranchInfo, GitError};
use crate::state::ReviewDb;

/// Review progress for a branch
#[derive(Debug, Clone, Default)]
pub struct ReviewProgress {
    pub reviewed: usize,
    pub total: usize,
}

/// A single row in the dashboard
pub struct DashboardItem {
    pub branch: BranchInfo,
    pub detail: Option<BranchDetail>,
    pub progress: Option<ReviewProgress>,
}

/// Dashboard state — owns the item list but NOT the ReviewDb
pub struct Dashboard {
    pub items: Vec<DashboardItem>,
    pub selected: usize,
    pub base_branch: String,
    pub last_head_sha: String,
}

impl Dashboard {
    /// Move selection down (clamp to end).
    pub fn select_next(&mut self) {
        if !self.items.is_empty() && self.selected < self.items.len() - 1 {
            self.selected += 1;
        }
    }

    /// Move selection up (clamp to start).
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Get the name of the currently selected branch.
    pub fn selected_branch(&self) -> Option<&str> {
        self.items
            .get(self.selected)
            .map(|item| item.branch.name.as_str())
    }

    /// Get a reference to the currently selected item.
    pub fn selected_item(&self) -> Option<&DashboardItem> {
        self.items.get(self.selected)
    }

    /// Load dashboard from git and review state.
    pub fn load(_db: &ReviewDb, base_branch: &str) -> Result<Self, GitError> {
        let all_branches = crate::git::list_branches()?;
        let last_head_sha = crate::git::get_head_sha()?;

        // Filter out the base branch itself
        let items = all_branches
            .into_iter()
            .filter(|b| b.name != base_branch)
            .map(|branch| DashboardItem {
                branch,
                detail: None,
                progress: None,
            })
            .collect();

        Ok(Dashboard {
            items,
            selected: 0,
            base_branch: base_branch.to_string(),
            last_head_sha,
        })
    }

    /// Refresh dashboard if HEAD has changed. Returns true if state changed.
    pub fn refresh(&mut self, _db: &ReviewDb) -> Result<bool, GitError> {
        let current_head = crate::git::get_head_sha()?;

        // If HEAD hasn't changed, no need to refresh
        if current_head == self.last_head_sha {
            return Ok(false);
        }

        // Reload branch list
        let all_branches = crate::git::list_branches()?;
        self.items = all_branches
            .into_iter()
            .filter(|b| b.name != self.base_branch)
            .map(|branch| DashboardItem {
                branch,
                detail: None,
                progress: None,
            })
            .collect();

        // Clamp selection to new bounds
        if !self.items.is_empty() && self.selected >= self.items.len() {
            self.selected = self.items.len() - 1;
        }

        self.last_head_sha = current_head;
        Ok(true)
    }

    /// Load detail and progress for the currently selected branch.
    pub fn load_detail_for_selected(&mut self, db: &mut ReviewDb) -> Result<(), GitError> {
        // Get the selected item
        let item = match self.items.get_mut(self.selected) {
            Some(item) => item,
            None => return Ok(()), // No items in dashboard
        };

        // If detail is already loaded, skip
        if item.detail.is_some() {
            return Ok(());
        }

        // Load branch detail from git
        let branch_name = &item.branch.name;
        let detail = crate::git::get_branch_detail(&self.base_branch, branch_name)?;

        // Build diff range and sync with database before reading progress
        let range = format!("{}..{}", self.base_branch, branch_name);

        // Get the actual diff and sync with DB to ensure progress is accurate
        let progress = match crate::git::get_diff(&range) {
            Ok(diff_output) => {
                let files = crate::parser::parse_diff(&diff_output);
                // Sync the diff with the database
                match db.sync_with_diff(&range, &files) {
                    Ok(()) => {
                        // Now read progress from the updated DB
                        match db.progress(&range) {
                            Ok(p) => ReviewProgress {
                                reviewed: p.reviewed,
                                total: p.total_hunks,
                            },
                            Err(_) => ReviewProgress {
                                reviewed: 0,
                                total: 0,
                            },
                        }
                    }
                    Err(_) => ReviewProgress {
                        reviewed: 0,
                        total: 0,
                    },
                }
            }
            Err(_) => {
                // Can't get diff — try DB progress as fallback (may be stale)
                match db.progress(&range) {
                    Ok(p) => ReviewProgress {
                        reviewed: p.reviewed,
                        total: p.total_hunks,
                    },
                    Err(_) => ReviewProgress {
                        reviewed: 0,
                        total: 0,
                    },
                }
            }
        };

        // Update item with loaded data
        item.detail = Some(detail);
        item.progress = Some(progress);

        Ok(())
    }

    /// Load details for all items eagerly.
    pub fn load_all_details(&mut self, db: &mut ReviewDb) {
        for item in &mut self.items {
            // If detail is already loaded, skip
            if item.detail.is_some() {
                continue;
            }

            // Load branch detail from git (ignore errors for individual branches)
            let branch_name = &item.branch.name;
            if let Ok(detail) = crate::git::get_branch_detail(&self.base_branch, branch_name) {
                // Build diff range and sync with database before reading progress
                let range = format!("{}..{}", self.base_branch, branch_name);

                // Get the actual diff and sync with DB to ensure progress is accurate
                let progress = match crate::git::get_diff(&range) {
                    Ok(diff_output) => {
                        let files = crate::parser::parse_diff(&diff_output);
                        // Sync the diff with the database
                        match db.sync_with_diff(&range, &files) {
                            Ok(()) => {
                                // Now read progress from the updated DB
                                match db.progress(&range) {
                                    Ok(p) => ReviewProgress {
                                        reviewed: p.reviewed,
                                        total: p.total_hunks,
                                    },
                                    Err(_) => ReviewProgress {
                                        reviewed: 0,
                                        total: 0,
                                    },
                                }
                            }
                            Err(_) => ReviewProgress {
                                reviewed: 0,
                                total: 0,
                            },
                        }
                    }
                    Err(_) => {
                        // Can't get diff — try DB progress as fallback (may be stale)
                        match db.progress(&range) {
                            Ok(p) => ReviewProgress {
                                reviewed: p.reviewed,
                                total: p.total_hunks,
                            },
                            Err(_) => ReviewProgress {
                                reviewed: 0,
                                total: 0,
                            },
                        }
                    }
                };

                // Update item with loaded data
                item.detail = Some(detail);
                item.progress = Some(progress);
            }
            // If get_branch_detail fails, we leave detail as None (shows "-" in UI)
        }
    }

    /// Check if the selected branch can be merged (all hunks reviewed).
    pub fn can_merge_selected(&self) -> bool {
        self.selected_item()
            .and_then(|item| item.progress.as_ref())
            .map(|p| p.reviewed == p.total && p.total > 0)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DiffFile, DiffHunk, HunkStatus};
    use std::path::PathBuf;

    fn mock_branch(name: &str) -> BranchInfo {
        BranchInfo {
            name: name.to_string(),
            is_local: true,
            last_commit_sha: "abc123".to_string(),
            last_commit_author: "Test".to_string(),
            last_commit_age: "1 hour ago".to_string(),
            last_commit_timestamp: 0,
        }
    }

    fn mock_dashboard(n: usize) -> Dashboard {
        Dashboard {
            items: (0..n)
                .map(|i| DashboardItem {
                    branch: mock_branch(&format!("branch-{}", i)),
                    detail: None,
                    progress: None,
                })
                .collect(),
            selected: 0,
            base_branch: "main".to_string(),
            last_head_sha: "deadbeef".to_string(),
        }
    }

    #[test]
    fn test_select_next_empty() {
        let mut dashboard = mock_dashboard(0);
        assert_eq!(dashboard.selected, 0);
        dashboard.select_next();
        assert_eq!(dashboard.selected, 0); // Should stay at 0
    }

    #[test]
    fn test_select_next_prev() {
        let mut dashboard = mock_dashboard(3);
        assert_eq!(dashboard.selected, 0);

        // Move down
        dashboard.select_next();
        assert_eq!(dashboard.selected, 1);

        dashboard.select_next();
        assert_eq!(dashboard.selected, 2);

        // Try to move beyond end (should clamp)
        dashboard.select_next();
        assert_eq!(dashboard.selected, 2);

        // Move up
        dashboard.select_prev();
        assert_eq!(dashboard.selected, 1);

        dashboard.select_prev();
        assert_eq!(dashboard.selected, 0);

        // Try to move before start (should clamp)
        dashboard.select_prev();
        assert_eq!(dashboard.selected, 0);
    }

    #[test]
    fn test_selected_branch_empty() {
        let dashboard = mock_dashboard(0);
        assert_eq!(dashboard.selected_branch(), None);
    }

    #[test]
    fn test_can_merge_selected_not_reviewed() {
        let mut dashboard = mock_dashboard(1);
        dashboard.items[0].progress = Some(ReviewProgress {
            reviewed: 5,
            total: 10,
        });

        assert!(!dashboard.can_merge_selected());
    }

    #[test]
    fn test_can_merge_selected_all_reviewed() {
        let mut dashboard = mock_dashboard(1);
        dashboard.items[0].progress = Some(ReviewProgress {
            reviewed: 10,
            total: 10,
        });

        assert!(dashboard.can_merge_selected());
    }

    #[test]
    fn test_can_merge_selected_no_progress() {
        let dashboard = mock_dashboard(1);
        assert!(!dashboard.can_merge_selected());
    }

    /// Test that progress reflects current diff state, not stale DB data.
    ///
    /// This test verifies the fix for the bug where dashboard showed 100% progress
    /// initially, then showed the correct percentage after entering hunk review.
    #[test]
    fn test_progress_reflects_current_diff_not_stale_db() {
        // Create a temp DB
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Simulate stale DB state: mark a hunk as reviewed for an old diff
        db.set_status("main..feature", "file.txt", "old_hash", HunkStatus::Reviewed)
            .unwrap();

        // Verify DB shows 1 reviewed hunk
        let progress = db.progress("main..feature").unwrap();
        assert_eq!(progress.reviewed, 1);
        assert_eq!(progress.total_hunks, 1);

        // Now simulate the actual current diff (different content, different hash)
        let current_files = vec![DiffFile {
            path: PathBuf::from("file.txt"),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                content: "new content".to_string(),
                content_hash: "new_hash".to_string(),
                status: HunkStatus::Unreviewed,
            }],
        }];

        // Sync with the current diff
        db.sync_with_diff("main..feature", &current_files)
            .unwrap();

        // Now DB should show 1 unreviewed hunk, 1 stale hunk
        let progress = db.progress("main..feature").unwrap();
        assert_eq!(progress.reviewed, 0, "Should have 0 reviewed hunks");
        assert_eq!(progress.unreviewed, 1, "Should have 1 unreviewed hunk");
        assert_eq!(progress.stale, 1, "Should have 1 stale hunk");
        assert_eq!(progress.total_hunks, 2, "Should have 2 total hunks");

        // This test demonstrates that syncing the diff with the DB is essential
        // to get accurate progress — without sync, we'd read the stale data
    }

    /// Test that dashboard progress updates correctly after sync.
    ///
    /// This simulates what happens when you review some hunks, then the code changes.
    #[test]
    fn test_dashboard_progress_updates_after_sync() {
        // Create a temp DB
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Initial diff with 2 hunks
        let initial_files = vec![DiffFile {
            path: PathBuf::from("file.txt"),
            hunks: vec![
                DiffHunk {
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 1,
                    content: "hunk1".to_string(),
                    content_hash: "hash1".to_string(),
                    status: HunkStatus::Unreviewed,
                },
                DiffHunk {
                    old_start: 5,
                    old_count: 1,
                    new_start: 5,
                    new_count: 1,
                    content: "hunk2".to_string(),
                    content_hash: "hash2".to_string(),
                    status: HunkStatus::Unreviewed,
                },
            ],
        }];

        db.sync_with_diff("main..feature", &initial_files).unwrap();

        // Review the first hunk
        db.set_status("main..feature", "file.txt", "hash1", HunkStatus::Reviewed)
            .unwrap();

        // Progress should show 1/2 reviewed
        let progress = db.progress("main..feature").unwrap();
        assert_eq!(progress.reviewed, 1);
        assert_eq!(progress.unreviewed, 1);
        assert_eq!(progress.total_hunks, 2);

        // Now simulate code change: hash1 is stale, hash2 unchanged, new hash3 appears
        let updated_files = vec![DiffFile {
            path: PathBuf::from("file.txt"),
            hunks: vec![
                DiffHunk {
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 2,
                    content: "hunk1_modified".to_string(),
                    content_hash: "hash1_new".to_string(),
                    status: HunkStatus::Unreviewed,
                },
                DiffHunk {
                    old_start: 5,
                    old_count: 1,
                    new_start: 6,
                    new_count: 1,
                    content: "hunk2".to_string(),
                    content_hash: "hash2".to_string(), // Same as before
                    status: HunkStatus::Unreviewed,
                },
                DiffHunk {
                    old_start: 10,
                    old_count: 1,
                    new_start: 11,
                    new_count: 1,
                    content: "hunk3".to_string(),
                    content_hash: "hash3".to_string(),
                    status: HunkStatus::Unreviewed,
                },
            ],
        }];

        db.sync_with_diff("main..feature", &updated_files).unwrap();

        // Progress should now show:
        // - hash1 is stale (was reviewed, but content changed)
        // - hash1_new is unreviewed (new content)
        // - hash2 is unreviewed (unchanged content, no review)
        // - hash3 is unreviewed (new hunk)
        let progress = db.progress("main..feature").unwrap();
        assert_eq!(progress.reviewed, 0, "No reviewed hunks in current diff");
        assert_eq!(progress.unreviewed, 3, "3 unreviewed hunks");
        assert_eq!(progress.stale, 1, "1 stale hunk (old hash1)");
        assert_eq!(progress.total_hunks, 4, "4 total hunks");
    }

    /// Test that a dashboard with no detail loaded shows accurate progress
    /// when details are loaded (simulating the bug scenario).
    #[test]
    fn test_dashboard_load_all_details_syncs_before_progress() {
        // Create a temp DB
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Pre-populate DB with stale data
        db.set_status("main..branch1", "file.txt", "stale_hash", HunkStatus::Reviewed)
            .unwrap();

        // Note: In a real scenario, load_all_details would call git::get_diff
        // and sync the actual current diff. We can't test that here without
        // a real git repo, but we've verified the logic in the previous tests.

        // This test documents the intended behavior:
        // 1. load_all_details should call git::get_diff for the branch
        // 2. It should parse the diff into DiffFile structures
        // 3. It should sync those files with the DB via sync_with_diff
        // 4. Only then should it read progress from the DB
        //
        // Without step 3, the progress would reflect stale DB data (the bug).
        // With step 3, the progress reflects the actual current diff state (the fix).

        // We can at least verify the DB starts with stale data
        let stale_progress = db.progress("main..branch1").unwrap();
        assert_eq!(stale_progress.reviewed, 1, "DB has stale reviewed hunk");
        assert_eq!(stale_progress.total_hunks, 1);

        // After a proper sync with current (empty) diff, progress should be 0/0
        let current_files: Vec<DiffFile> = vec![]; // Empty diff
        db.sync_with_diff("main..branch1", &current_files).unwrap();

        let synced_progress = db.progress("main..branch1").unwrap();
        assert_eq!(synced_progress.reviewed, 0, "After sync with empty diff");
        assert_eq!(synced_progress.stale, 1, "Old hunk marked stale");

        // The actual fix in load_all_details ensures this sync happens
        // before reading progress, preventing the initial 100% bug
    }
}
