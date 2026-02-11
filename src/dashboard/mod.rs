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
    pub fn load_detail_for_selected(&mut self, db: &ReviewDb) -> Result<(), GitError> {
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

        // Build diff range and get progress from database
        let range = format!("{}..{}", self.base_branch, branch_name);
        let progress = match db.progress(&range) {
            Ok(p) => ReviewProgress {
                reviewed: p.reviewed,
                total: p.total_hunks,
            },
            Err(_) => ReviewProgress {
                reviewed: 0,
                total: 0,
            },
        };

        // Update item with loaded data
        item.detail = Some(detail);
        item.progress = Some(progress);

        Ok(())
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
}
