use crate::{DiffFile, HunkStatus, ReviewProgress};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during state operations.
#[derive(Debug, Error)]
pub enum StateError {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid hunk status: {0}")]
    InvalidStatus(String),
}

pub type Result<T> = std::result::Result<T, StateError>;

/// SQLite-backed review state database.
///
/// Stores review status per hunk (keyed by SHA-256 content hash).
/// Detects stale hunks when diff content changes.
pub struct ReviewDb {
    conn: Connection,
}

impl ReviewDb {
    /// Open or create the review database at the given path.
    ///
    /// Creates the necessary tables if they don't exist.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS hunks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                base_ref TEXT NOT NULL,
                file_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'unreviewed',
                reviewed_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(base_ref, file_path, content_hash)
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    /// Get the review status for a specific hunk.
    ///
    /// Returns `HunkStatus::Unreviewed` if the hunk is not found in the database.
    pub fn get_status(
        &self,
        base_ref: &str,
        file_path: &str,
        content_hash: &str,
    ) -> Result<HunkStatus> {
        let mut stmt = self.conn.prepare(
            "SELECT status FROM hunks WHERE base_ref = ?1 AND file_path = ?2 AND content_hash = ?3",
        )?;

        let status: Option<String> = stmt
            .query_row(params![base_ref, file_path, content_hash], |row| row.get(0))
            .optional()?;

        match status.as_deref() {
            Some("reviewed") => Ok(HunkStatus::Reviewed),
            Some("stale") => Ok(HunkStatus::Stale),
            Some("unreviewed") | None => Ok(HunkStatus::Unreviewed),
            Some(other) => Err(StateError::InvalidStatus(other.to_owned())),
        }
    }

    /// Set the review status for a specific hunk.
    pub fn set_status(
        &mut self,
        base_ref: &str,
        file_path: &str,
        content_hash: &str,
        status: HunkStatus,
    ) -> Result<()> {
        let status_str = status_to_string(status);

        if status == HunkStatus::Reviewed {
            self.conn.execute(
                "INSERT INTO hunks (base_ref, file_path, content_hash, status, reviewed_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))
                 ON CONFLICT(base_ref, file_path, content_hash)
                 DO UPDATE SET status = ?4, reviewed_at = datetime('now')",
                params![base_ref, file_path, content_hash, status_str],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO hunks (base_ref, file_path, content_hash, status, reviewed_at)
                 VALUES (?1, ?2, ?3, ?4, NULL)
                 ON CONFLICT(base_ref, file_path, content_hash)
                 DO UPDATE SET status = ?4, reviewed_at = NULL",
                params![base_ref, file_path, content_hash, status_str],
            )?;
        }

        Ok(())
    }

    /// Synchronize the database with the current diff output.
    ///
    /// - New hunks (not in DB) are marked as `Unreviewed`
    /// - Hunks that no longer exist in the diff are marked as `Stale`
    /// - Hunks with `Reviewed` status and matching hash are preserved
    pub fn sync_with_diff(&mut self, base_ref: &str, files: &[DiffFile]) -> Result<()> {
        // Collect all current hunk hashes from the diff
        let mut current_hunks = std::collections::HashSet::new();
        for file in files {
            let file_path = file.path.to_string_lossy();
            for hunk in &file.hunks {
                current_hunks.insert((file_path.to_string(), hunk.content_hash.clone()));

                // Insert new hunks as Unreviewed (or keep existing status)
                let existing_status = self.get_status(base_ref, &file_path, &hunk.content_hash)?;
                if existing_status == HunkStatus::Unreviewed {
                    // Only insert if it doesn't exist yet
                    self.conn.execute(
                        "INSERT OR IGNORE INTO hunks (base_ref, file_path, content_hash, status)
                         VALUES (?1, ?2, ?3, 'unreviewed')",
                        params![base_ref, file_path, hunk.content_hash],
                    )?;
                }
            }
        }

        // Mark hunks in DB that are not in current diff as Stale
        // Collect hunks to mark as stale first to avoid borrow checker issues
        let db_hunks: Vec<(String, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT file_path, content_hash FROM hunks WHERE base_ref = ?1 AND status != 'stale'",
            )?;
            stmt.query_map(params![base_ref], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?
        };

        for (file_path, content_hash) in db_hunks {
            if !current_hunks.contains(&(file_path.clone(), content_hash.clone())) {
                self.set_status(base_ref, &file_path, &content_hash, HunkStatus::Stale)?;
            }
        }

        Ok(())
    }

    /// Get review progress summary for a given base ref.
    pub fn progress(&self, base_ref: &str) -> Result<ReviewProgress> {
        let mut stmt = self
            .conn
            .prepare("SELECT status, COUNT(*) FROM hunks WHERE base_ref = ?1 GROUP BY status")?;

        let mut reviewed = 0;
        let mut unreviewed = 0;
        let mut stale = 0;

        let rows = stmt.query_map(params![base_ref], |row| {
            let status: String = row.get(0)?;
            let count: usize = row.get(1)?;
            Ok((status, count))
        })?;

        for row in rows {
            let (status, count) = row?;
            match status.as_str() {
                "reviewed" => reviewed = count,
                "unreviewed" => unreviewed = count,
                "stale" => stale = count,
                _ => {}
            }
        }

        // Count files with remaining hunks
        let mut file_stmt = self.conn.prepare(
            "SELECT DISTINCT file_path FROM hunks WHERE base_ref = ?1 AND status != 'reviewed'",
        )?;
        let files_remaining = file_stmt
            .query_map(params![base_ref], |_row| Ok(()))?
            .count();

        // Count total files
        let mut total_files_stmt = self
            .conn
            .prepare("SELECT DISTINCT file_path FROM hunks WHERE base_ref = ?1")?;
        let total_files = total_files_stmt
            .query_map(params![base_ref], |_row| Ok(()))?
            .count();

        let total_hunks = reviewed + unreviewed + stale;

        Ok(ReviewProgress {
            total_hunks,
            reviewed,
            unreviewed,
            stale,
            files_remaining,
            total_files,
        })
    }

    /// Reset all review state for a given base ref.
    ///
    /// Deletes all hunks associated with the base ref.
    pub fn reset(&mut self, base_ref: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM hunks WHERE base_ref = ?1", params![base_ref])?;
        Ok(())
    }

    /// Approve all hunks for a given base ref (mark all as Reviewed).
    ///
    /// Returns the count of hunks that were updated.
    pub fn approve_all(&mut self, base_ref: &str) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE hunks SET status = 'reviewed', reviewed_at = datetime('now')
             WHERE base_ref = ?1 AND status != 'reviewed'",
            params![base_ref],
        )?;
        Ok(count)
    }

    /// Approve all hunks for a specific file within a base ref.
    ///
    /// Returns the count of hunks that were updated.
    pub fn approve_file(&mut self, base_ref: &str, file_path: &str) -> Result<usize> {
        let count = self.conn.execute(
            "UPDATE hunks SET status = 'reviewed', reviewed_at = datetime('now')
             WHERE base_ref = ?1 AND file_path = ?2 AND status != 'reviewed'",
            params![base_ref, file_path],
        )?;
        Ok(count)
    }

    /// List all distinct base refs in the database (for dashboard).
    ///
    /// Returns base refs sorted alphabetically.
    pub fn list_base_refs(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT base_ref FROM hunks ORDER BY base_ref")?;

        let refs = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;

        Ok(refs)
    }
}

/// Convert HunkStatus to string representation for database storage.
fn status_to_string(status: HunkStatus) -> &'static str {
    match status {
        HunkStatus::Unreviewed => "unreviewed",
        HunkStatus::Reviewed => "reviewed",
        HunkStatus::Stale => "stale",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DiffHunk;
    use std::path::PathBuf;

    #[test]
    fn open_creates_db() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let _db = ReviewDb::open(&db_path).unwrap();
        assert!(db_path.exists());
    }

    #[test]
    fn open_creates_tables() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let db = ReviewDb::open(&db_path).unwrap();

        // Verify table exists by querying it
        let count: i64 = db
            .conn
            .query_row("SELECT COUNT(*) FROM hunks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn save_and_retrieve_status() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        db.set_status("main", "file.txt", "hash123", HunkStatus::Reviewed)
            .unwrap();

        let status = db.get_status("main", "file.txt", "hash123").unwrap();
        assert_eq!(status, HunkStatus::Reviewed);
    }

    #[test]
    fn toggle_unreviewed_reviewed() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Start as unreviewed
        db.set_status("main", "file.txt", "hash123", HunkStatus::Unreviewed)
            .unwrap();
        let status = db.get_status("main", "file.txt", "hash123").unwrap();
        assert_eq!(status, HunkStatus::Unreviewed);

        // Toggle to reviewed
        db.set_status("main", "file.txt", "hash123", HunkStatus::Reviewed)
            .unwrap();
        let status = db.get_status("main", "file.txt", "hash123").unwrap();
        assert_eq!(status, HunkStatus::Reviewed);

        // Toggle back to unreviewed
        db.set_status("main", "file.txt", "hash123", HunkStatus::Unreviewed)
            .unwrap();
        let status = db.get_status("main", "file.txt", "hash123").unwrap();
        assert_eq!(status, HunkStatus::Unreviewed);
    }

    #[test]
    fn sync_marks_new_hunks_unreviewed() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        let files = vec![DiffFile {
            path: PathBuf::from("file.txt"),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                content: "test".to_string(),
                content_hash: "hash1".to_string(),
                status: HunkStatus::Unreviewed,
            }],
        }];

        db.sync_with_diff("main", &files).unwrap();

        let status = db.get_status("main", "file.txt", "hash1").unwrap();
        assert_eq!(status, HunkStatus::Unreviewed);
    }

    #[test]
    fn sync_marks_changed_hunks_stale() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Mark a hunk as reviewed
        db.set_status("main", "file.txt", "old_hash", HunkStatus::Reviewed)
            .unwrap();

        // Sync with a different hash (simulating changed content)
        let files = vec![DiffFile {
            path: PathBuf::from("file.txt"),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                content: "new_content".to_string(),
                content_hash: "new_hash".to_string(),
                status: HunkStatus::Unreviewed,
            }],
        }];

        db.sync_with_diff("main", &files).unwrap();

        // Old hash should be stale
        let old_status = db.get_status("main", "file.txt", "old_hash").unwrap();
        assert_eq!(old_status, HunkStatus::Stale);

        // New hash should be unreviewed
        let new_status = db.get_status("main", "file.txt", "new_hash").unwrap();
        assert_eq!(new_status, HunkStatus::Unreviewed);
    }

    #[test]
    fn sync_preserves_reviewed_with_same_hash() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Mark a hunk as reviewed
        db.set_status("main", "file.txt", "hash1", HunkStatus::Reviewed)
            .unwrap();

        // Sync with the same hash
        let files = vec![DiffFile {
            path: PathBuf::from("file.txt"),
            hunks: vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                content: "test".to_string(),
                content_hash: "hash1".to_string(),
                status: HunkStatus::Unreviewed,
            }],
        }];

        db.sync_with_diff("main", &files).unwrap();

        // Should still be reviewed
        let status = db.get_status("main", "file.txt", "hash1").unwrap();
        assert_eq!(status, HunkStatus::Reviewed);
    }

    #[test]
    fn progress_counts_accurate() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Create some hunks with different statuses
        db.set_status("main", "file1.txt", "hash1", HunkStatus::Reviewed)
            .unwrap();
        db.set_status("main", "file1.txt", "hash2", HunkStatus::Unreviewed)
            .unwrap();
        db.set_status("main", "file2.txt", "hash3", HunkStatus::Stale)
            .unwrap();

        let progress = db.progress("main").unwrap();
        assert_eq!(progress.total_hunks, 3);
        assert_eq!(progress.reviewed, 1);
        assert_eq!(progress.unreviewed, 1);
        assert_eq!(progress.stale, 1);
        assert_eq!(progress.total_files, 2);
        assert_eq!(progress.files_remaining, 2); // file1 has unreviewed, file2 has stale
    }

    #[test]
    fn reset_clears_state() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let mut db = ReviewDb::open(&db_path).unwrap();

        // Add some hunks
        db.set_status("main", "file.txt", "hash1", HunkStatus::Reviewed)
            .unwrap();
        db.set_status("main", "file.txt", "hash2", HunkStatus::Unreviewed)
            .unwrap();

        // Verify they exist
        let progress = db.progress("main").unwrap();
        assert_eq!(progress.total_hunks, 2);

        // Reset
        db.reset("main").unwrap();

        // Verify they're gone
        let progress = db.progress("main").unwrap();
        assert_eq!(progress.total_hunks, 0);
    }

    #[test]
    fn get_status_returns_unreviewed_for_missing_hunk() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("review.db");
        let db = ReviewDb::open(&db_path).unwrap();

        let status = db.get_status("main", "nonexistent.txt", "no_hash").unwrap();
        assert_eq!(status, HunkStatus::Unreviewed);
    }
}
