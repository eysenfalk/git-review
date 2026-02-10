use git_review::gate::{check_gate, disable_gate, enable_gate};
use git_review::state::ReviewDb;
use git_review::{DiffFile, DiffHunk, HunkStatus};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a git repository structure for testing
fn setup_test_repo() -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    let git_dir = temp.path().join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::create_dir(git_dir.join("hooks")).unwrap();
    temp
}

/// Helper to create a test database with some hunks
fn create_test_db(path: &std::path::Path, base_ref: &str, all_reviewed: bool) -> ReviewDb {
    let mut db = ReviewDb::open(path).unwrap();

    let files = vec![DiffFile {
        path: PathBuf::from("test.txt"),
        hunks: vec![
            DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1,
                content: "test1".to_string(),
                content_hash: "hash1".to_string(),
                status: HunkStatus::Unreviewed,
            },
            DiffHunk {
                old_start: 5,
                old_count: 1,
                new_start: 5,
                new_count: 1,
                content: "test2".to_string(),
                content_hash: "hash2".to_string(),
                status: HunkStatus::Unreviewed,
            },
        ],
    }];

    db.sync_with_diff(base_ref, &files).unwrap();

    if all_reviewed {
        db.set_status(base_ref, "test.txt", "hash1", HunkStatus::Reviewed)
            .unwrap();
        db.set_status(base_ref, "test.txt", "hash2", HunkStatus::Reviewed)
            .unwrap();
    }

    db
}

#[test]
fn enable_gate_creates_hook() {
    let temp_repo = setup_test_repo();
    let repo_root = temp_repo.path();

    enable_gate(repo_root).unwrap();

    let hook_path = repo_root.join(".git/hooks/pre-commit");
    assert!(hook_path.exists(), "Hook file should be created");

    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(content.contains("#!/bin/sh"), "Hook should have shebang");
    assert!(
        content.contains("Installed by git-review"),
        "Hook should have marker comment"
    );
    assert!(
        content.contains("git-review gate check"),
        "Hook should execute gate check"
    );

    // Check that hook is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(&hook_path).unwrap();
        let permissions = metadata.permissions();
        assert!(permissions.mode() & 0o111 != 0, "Hook should be executable");
    }
}

#[test]
fn enable_gate_backs_up_existing_hook() {
    let temp_repo = setup_test_repo();
    let repo_root = temp_repo.path();
    let hook_path = repo_root.join(".git/hooks/pre-commit");

    // Create an existing hook
    fs::write(&hook_path, "#!/bin/sh\necho 'existing hook'").unwrap();

    enable_gate(repo_root).unwrap();

    // Backup should exist
    let backup_path = repo_root.join(".git/hooks/pre-commit.backup");
    assert!(backup_path.exists(), "Backup should be created");

    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert!(
        backup_content.contains("existing hook"),
        "Backup should contain original hook"
    );
}

#[test]
fn disable_gate_removes_hook() {
    let temp_repo = setup_test_repo();
    let repo_root = temp_repo.path();

    // Enable the gate first
    enable_gate(repo_root).unwrap();

    let hook_path = repo_root.join(".git/hooks/pre-commit");
    assert!(hook_path.exists(), "Hook should exist before disable");

    // Disable the gate
    disable_gate(repo_root).unwrap();

    assert!(!hook_path.exists(), "Hook should be removed after disable");
}

#[test]
fn disable_gate_ignores_non_git_review_hooks() {
    let temp_repo = setup_test_repo();
    let repo_root = temp_repo.path();
    let hook_path = repo_root.join(".git/hooks/pre-commit");

    // Create a hook without the git-review marker
    fs::write(&hook_path, "#!/bin/sh\necho 'user hook'").unwrap();

    // Try to disable
    disable_gate(repo_root).unwrap();

    // Hook should still exist
    assert!(
        hook_path.exists(),
        "Non-git-review hook should not be removed"
    );

    let content = fs::read_to_string(&hook_path).unwrap();
    assert!(
        content.contains("user hook"),
        "Original hook content should be preserved"
    );
}

#[test]
fn check_gate_returns_true_when_all_reviewed() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("review.db");
    let db = create_test_db(&db_path, "main", true);

    let result = check_gate(&db, "main").unwrap();
    assert!(result, "Gate should pass when all hunks are reviewed");
}

#[test]
fn check_gate_returns_false_when_unreviewed() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("review.db");
    let db = create_test_db(&db_path, "main", false);

    let result = check_gate(&db, "main").unwrap();
    assert!(!result, "Gate should fail when hunks are unreviewed");
}

#[test]
fn check_gate_returns_false_when_stale() {
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().join("review.db");
    let mut db = create_test_db(&db_path, "main", true);

    // Mark one hunk as stale
    db.set_status("main", "test.txt", "hash1", HunkStatus::Stale)
        .unwrap();

    let result = check_gate(&db, "main").unwrap();
    assert!(!result, "Gate should fail when hunks are stale");
}
