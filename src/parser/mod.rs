use crate::{DiffFile, DiffHunk, HunkStatus};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Parse raw `git diff` output into structured `DiffFile` entries.
///
/// Parses unified diff format, extracting file paths, hunk headers, and content.
/// Each hunk is assigned a SHA-256 hash of its content and starts with status `Unreviewed`.
/// Binary files are skipped. Handles new files, deleted files, and renames.
pub fn parse_diff(input: &str) -> Vec<DiffFile> {
    let mut files = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Look for file headers: "diff --git a/path b/path"
        if line.starts_with("diff --git ") {
            // Extract file path from the next lines
            let mut path: Option<PathBuf> = None;
            let mut hunks = Vec::new();
            let mut is_binary = false;
            i += 1;

            // Skip until we find +++ line (or detect binary)
            while i < lines.len() {
                let current = lines[i];

                // Check for binary file marker
                if current.starts_with("Binary files ") {
                    is_binary = true;
                    i += 1;
                    break;
                }

                // Extract path from +++ line
                if current.starts_with("+++ ") {
                    let path_str = current.strip_prefix("+++ ").unwrap_or("");
                    // Handle new files (--- /dev/null)
                    if path_str != "/dev/null" {
                        // Remove "b/" prefix if present
                        let clean_path = path_str.strip_prefix("b/").unwrap_or(path_str);
                        path = Some(PathBuf::from(clean_path));
                    } else {
                        // Deleted file - get path from --- line
                        if i > 0 && lines[i - 1].starts_with("--- ") {
                            let prev_path = lines[i - 1].strip_prefix("--- ").unwrap_or("");
                            let clean_path = prev_path.strip_prefix("a/").unwrap_or(prev_path);
                            if clean_path != "/dev/null" {
                                path = Some(PathBuf::from(clean_path));
                            }
                        }
                    }
                    i += 1;
                    break;
                }

                i += 1;
            }

            // Skip binary files
            if is_binary {
                continue;
            }

            // Parse hunks for this file
            while i < lines.len() {
                let current = lines[i];

                // Stop if we hit the next file
                if current.starts_with("diff --git ") {
                    break;
                }

                // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
                if current.starts_with("@@ ") {
                    if let Some(hunk) = parse_hunk(&lines, &mut i) {
                        hunks.push(hunk);
                    } else {
                        // parse_hunk failed without advancing i — skip this line
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }

            // Add file if we have a path and hunks
            if let Some(p) = path
                && !hunks.is_empty()
            {
                files.push(DiffFile { path: p, hunks });
            }
        } else {
            i += 1;
        }
    }

    files
}

/// Parse a single hunk starting at the @@ line.
fn parse_hunk(lines: &[&str], i: &mut usize) -> Option<DiffHunk> {
    let line = lines[*i];

    // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@ [context]
    let header = line.strip_prefix("@@ ")?;
    // Find the closing @@ — everything after it is optional context
    let header = match header.find(" @@") {
        Some(pos) => &header[..pos],
        None => return None,
    };
    let parts: Vec<&str> = header.split(' ').collect();
    if parts.len() < 2 {
        return None;
    }

    // Parse old range: -start,count or -start
    let old_part = parts[0].strip_prefix('-')?;
    let (old_start, old_count) = parse_range(old_part);

    // Parse new range: +start,count or +start
    let new_part = parts[1].strip_prefix('+')?;
    let (new_start, new_count) = parse_range(new_part);

    // Collect hunk content (lines starting with +, -, or space)
    let mut content_lines = Vec::new();
    *i += 1;

    while *i < lines.len() {
        let current = lines[*i];

        // Stop at next hunk or file
        if current.starts_with("@@") || current.starts_with("diff --git ") {
            break;
        }

        // Include lines that are part of the hunk content
        if current.starts_with('+')
            || current.starts_with('-')
            || current.starts_with(' ')
            || current.starts_with('\\')
        {
            content_lines.push(current);
            *i += 1;
        } else {
            break;
        }
    }

    let content = content_lines.join("\n");
    let content_hash = compute_hash(&content);

    Some(DiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        content,
        content_hash,
        status: HunkStatus::Unreviewed,
    })
}

/// Parse a range like "start,count" or "start" (count defaults to 1).
fn parse_range(s: &str) -> (u32, u32) {
    if let Some(comma_pos) = s.find(',') {
        let start = s[..comma_pos].parse().unwrap_or(0);
        let count = s[comma_pos + 1..].parse().unwrap_or(0);
        (start, count)
    } else {
        let start = s.parse().unwrap_or(0);
        (start, 1)
    }
}

/// Compute SHA-256 hash of content.
fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_diff_returns_empty() {
        assert!(parse_diff("").is_empty());
    }

    #[test]
    fn parse_single_file_single_hunk() {
        let diff = r#"diff --git a/file.txt b/file.txt
index 1234567..abcdefg 100644
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line1
-line2
+line2_modified
 line3
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("file.txt"));
        assert_eq!(files[0].hunks.len(), 1);

        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 3);
        assert_eq!(hunk.status, HunkStatus::Unreviewed);
        assert!(!hunk.content_hash.is_empty());
    }

    #[test]
    fn parse_single_file_multiple_hunks() {
        let diff = r#"diff --git a/file.txt b/file.txt
index 1234567..abcdefg 100644
--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line1
-line2
+line2_modified
 line3
@@ -10,2 +10,3 @@
 line10
+new_line
 line11
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].hunks.len(), 2);
    }

    #[test]
    fn parse_multiple_files() {
        let diff = r#"diff --git a/file1.txt b/file1.txt
index 1234567..abcdefg 100644
--- a/file1.txt
+++ b/file1.txt
@@ -1,2 +1,2 @@
-old
+new
diff --git a/file2.txt b/file2.txt
index 1234567..abcdefg 100644
--- a/file2.txt
+++ b/file2.txt
@@ -1,2 +1,2 @@
-old2
+new2
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, PathBuf::from("file1.txt"));
        assert_eq!(files[1].path, PathBuf::from("file2.txt"));
    }

    #[test]
    fn parse_binary_file_skipped() {
        let diff = r#"diff --git a/image.png b/image.png
index 1234567..abcdefg 100644
Binary files a/image.png and b/image.png differ
diff --git a/file.txt b/file.txt
index 1234567..abcdefg 100644
--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,2 @@
-old
+new
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("file.txt"));
    }

    #[test]
    fn parse_new_file() {
        let diff = r#"diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000..abcdefg
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+line1
+line2
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("new.txt"));
        assert_eq!(files[0].hunks.len(), 1);
    }

    #[test]
    fn parse_deleted_file() {
        let diff = r#"diff --git a/deleted.txt b/deleted.txt
deleted file mode 100644
index abcdefg..0000000
--- a/deleted.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-line1
-line2
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, PathBuf::from("deleted.txt"));
        assert_eq!(files[0].hunks.len(), 1);
    }

    #[test]
    fn hash_is_deterministic() {
        let diff = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,2 @@
-old
+new
"#;
        let files1 = parse_diff(diff);
        let files2 = parse_diff(diff);
        assert_eq!(
            files1[0].hunks[0].content_hash,
            files2[0].hunks[0].content_hash
        );
    }

    #[test]
    fn hunk_header_edge_cases() {
        // Omitted count (defaults to 1)
        let diff = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -5 +5 @@
-old
+new
"#;
        let files = parse_diff(diff);
        assert_eq!(files.len(), 1);
        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.old_start, 5);
        assert_eq!(hunk.old_count, 1);
        assert_eq!(hunk.new_start, 5);
        assert_eq!(hunk.new_count, 1);

        // Count = 0
        let diff2 = r#"diff --git a/file.txt b/file.txt
--- a/file.txt
+++ b/file.txt
@@ -0,0 +1,2 @@
+line1
+line2
"#;
        let files2 = parse_diff(diff2);
        assert_eq!(files2.len(), 1);
        let hunk2 = &files2[0].hunks[0];
        assert_eq!(hunk2.old_start, 0);
        assert_eq!(hunk2.old_count, 0);
        assert_eq!(hunk2.new_start, 1);
        assert_eq!(hunk2.new_count, 2);
    }
}
