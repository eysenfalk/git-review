use git_review::HunkStatus;
use git_review::state::ReviewDb;

#[test]
fn approve_all_marks_all_unreviewed_as_reviewed() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // Set up some unreviewed hunks
    db.set_status("main", "file1.txt", "hash1", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file1.txt", "hash2", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file2.txt", "hash3", HunkStatus::Unreviewed)
        .unwrap();

    // Approve all
    let count = db.approve_all("main").unwrap();
    assert_eq!(count, 3);

    // Verify all are reviewed
    assert_eq!(
        db.get_status("main", "file1.txt", "hash1").unwrap(),
        HunkStatus::Reviewed
    );
    assert_eq!(
        db.get_status("main", "file1.txt", "hash2").unwrap(),
        HunkStatus::Reviewed
    );
    assert_eq!(
        db.get_status("main", "file2.txt", "hash3").unwrap(),
        HunkStatus::Reviewed
    );
}

#[test]
fn approve_all_returns_count_of_affected_rows() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // Set up mixed statuses
    db.set_status("main", "file1.txt", "hash1", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file1.txt", "hash2", HunkStatus::Reviewed)
        .unwrap();
    db.set_status("main", "file2.txt", "hash3", HunkStatus::Unreviewed)
        .unwrap();

    // Approve all should only affect unreviewed hunks
    let count = db.approve_all("main").unwrap();
    assert_eq!(count, 2); // Only hash1 and hash3 were unreviewed
}

#[test]
fn approve_all_does_not_double_count_already_reviewed() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // All hunks already reviewed
    db.set_status("main", "file1.txt", "hash1", HunkStatus::Reviewed)
        .unwrap();
    db.set_status("main", "file1.txt", "hash2", HunkStatus::Reviewed)
        .unwrap();

    // Approve all should affect 0 hunks
    let count = db.approve_all("main").unwrap();
    assert_eq!(count, 0);
}

#[test]
fn approve_file_only_affects_specified_file() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // Set up hunks in different files
    db.set_status("main", "file1.txt", "hash1", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file1.txt", "hash2", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file2.txt", "hash3", HunkStatus::Unreviewed)
        .unwrap();

    // Approve only file1.txt
    let count = db.approve_file("main", "file1.txt").unwrap();
    assert_eq!(count, 2);

    // Verify file1.txt hunks are reviewed
    assert_eq!(
        db.get_status("main", "file1.txt", "hash1").unwrap(),
        HunkStatus::Reviewed
    );
    assert_eq!(
        db.get_status("main", "file1.txt", "hash2").unwrap(),
        HunkStatus::Reviewed
    );

    // Verify file2.txt hunk is still unreviewed
    assert_eq!(
        db.get_status("main", "file2.txt", "hash3").unwrap(),
        HunkStatus::Unreviewed
    );
}

#[test]
fn approve_file_leaves_other_files_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // Set up multiple files
    db.set_status("main", "file1.txt", "hash1", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file2.txt", "hash2", HunkStatus::Reviewed)
        .unwrap();
    db.set_status("main", "file3.txt", "hash3", HunkStatus::Unreviewed)
        .unwrap();

    // Approve only file1.txt
    db.approve_file("main", "file1.txt").unwrap();

    // Verify file1.txt is reviewed
    assert_eq!(
        db.get_status("main", "file1.txt", "hash1").unwrap(),
        HunkStatus::Reviewed
    );

    // Verify file2.txt is still reviewed (unchanged)
    assert_eq!(
        db.get_status("main", "file2.txt", "hash2").unwrap(),
        HunkStatus::Reviewed
    );

    // Verify file3.txt is still unreviewed (unchanged)
    assert_eq!(
        db.get_status("main", "file3.txt", "hash3").unwrap(),
        HunkStatus::Unreviewed
    );
}

#[test]
fn list_base_refs_returns_distinct_refs() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // Add hunks for multiple base refs
    db.set_status("main", "file1.txt", "hash1", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("main", "file2.txt", "hash2", HunkStatus::Reviewed)
        .unwrap();
    db.set_status(
        "feature-branch",
        "file3.txt",
        "hash3",
        HunkStatus::Unreviewed,
    )
    .unwrap();
    db.set_status("feature-branch", "file4.txt", "hash4", HunkStatus::Reviewed)
        .unwrap();
    db.set_status(
        "another-branch",
        "file5.txt",
        "hash5",
        HunkStatus::Unreviewed,
    )
    .unwrap();

    // Get distinct base refs
    let refs = db.list_base_refs().unwrap();
    assert_eq!(refs.len(), 3);
    assert!(refs.contains(&"main".to_string()));
    assert!(refs.contains(&"feature-branch".to_string()));
    assert!(refs.contains(&"another-branch".to_string()));
}

#[test]
fn list_base_refs_returns_empty_for_empty_db() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let db = ReviewDb::open(&db_path).unwrap();

    let refs = db.list_base_refs().unwrap();
    assert_eq!(refs.len(), 0);
}

#[test]
fn list_base_refs_returns_sorted_refs() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("review.db");
    let mut db = ReviewDb::open(&db_path).unwrap();

    // Add refs in non-alphabetical order
    db.set_status("zebra", "file1.txt", "hash1", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("alpha", "file2.txt", "hash2", HunkStatus::Unreviewed)
        .unwrap();
    db.set_status("beta", "file3.txt", "hash3", HunkStatus::Unreviewed)
        .unwrap();

    let refs = db.list_base_refs().unwrap();
    assert_eq!(refs, vec!["alpha", "beta", "zebra"]);
}
