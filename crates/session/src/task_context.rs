//! Task context — captures git pre/post state around a governed task invocation.
//!
//! TaskContext observes the working directory before and after a task run,
//! producing a TaskSummary with changed files, diff stats, and completion status.
//! Uses the same governed git observation tools available to the agent loop.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Summary of what changed during a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    /// Files changed (added, modified, deleted) between pre and post observation.
    pub changed_files: Vec<String>,
    /// Diff stat summary (insertions/deletions).
    pub diff_stat: Option<String>,
    /// Whether the task completed (post observation was captured).
    pub completed: bool,
    /// Test result output, if a test command was captured.
    pub test_output: Option<String>,
}

/// Captures working directory state before a task, used to compute summary after.
#[derive(Debug, Clone)]
pub struct TaskContext {
    pre_git_status: Option<String>,
    pre_git_diff_stat: Option<String>,
    working_dir: String,
}

impl TaskContext {
    /// Capture the current state of the working directory before a task starts.
    ///
    /// This is best-effort: if git is not available or the directory is not a git repo,
    /// captures will be None and the summary will reflect that.
    pub async fn before_task(working_dir: &Path) -> Self {
        let working_dir_str = working_dir.to_string_lossy().to_string();

        // Best-effort git status capture
        let pre_git_status = capture_git_status(working_dir).await;
        let pre_git_diff_stat = capture_git_diff_stat(working_dir).await;

        Self {
            pre_git_status,
            pre_git_diff_stat,
            working_dir: working_dir_str,
        }
    }

    /// Compute a TaskSummary by comparing current state to the pre-task capture.
    pub async fn after_task(&self) -> TaskSummary {
        let post_git_status = capture_git_status(Path::new(&self.working_dir)).await;
        let diff_stat = compute_diff_stat(
            &self.pre_git_status,
            &post_git_status,
            &self.pre_git_diff_stat,
            Path::new(&self.working_dir),
        )
        .await;

        let changed_files = compute_changed_files(&self.pre_git_status, &post_git_status);

        TaskSummary {
            changed_files,
            diff_stat,
            completed: post_git_status.is_some(),
            test_output: None,
        }
    }

    /// Create a TaskSummary with explicit test output (for test capture).
    pub fn with_test_output(mut summary: TaskSummary, test_output: String) -> TaskSummary {
        summary.test_output = Some(test_output);
        summary
    }

    /// Create a stub context for testing (no real git operations).
    pub fn stub(working_dir: &str, pre_status: Option<String>, pre_diff: Option<String>) -> Self {
        Self {
            pre_git_status: pre_status,
            pre_git_diff_stat: pre_diff,
            working_dir: working_dir.to_string(),
        }
    }
}

/// Best-effort capture of `git status --porcelain` output.
async fn capture_git_status(working_dir: &Path) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(working_dir)
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    }
}

/// Best-effort capture of `git diff --stat` output.
async fn capture_git_diff_stat(working_dir: &Path) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .args(["diff", "--stat"])
        .current_dir(working_dir)
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).to_string();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    }
}

/// Compute diff stat by running git diff --stat again (post-task).
async fn compute_diff_stat(
    _pre_status: &Option<String>,
    _post_status: &Option<String>,
    _pre_diff_stat: &Option<String>,
    working_dir: &Path,
) -> Option<String> {
    capture_git_diff_stat(working_dir).await
}

/// Compare pre and post git status to find changed files.
fn compute_changed_files(
    pre_status: &Option<String>,
    post_status: &Option<String>,
) -> Vec<String> {
    match (pre_status, post_status) {
        (Some(pre), Some(post)) => {
            let pre_files: std::collections::HashSet<String> = parse_status_files(pre);
            let post_files: std::collections::HashSet<String> = parse_status_files(post);

            // Files that are new or changed in post but not in pre
            let mut changed: Vec<String> = post_files
                .difference(&pre_files)
                .cloned()
                .chain(pre_files.difference(&post_files).cloned())
                .collect();
            changed.sort();
            changed
        }
        (None, Some(post)) => parse_status_files(post).into_iter().collect(),
        _ => vec![],
    }
}

/// Parse `git status --porcelain` output into a set of file paths.
fn parse_status_files(status: &str) -> std::collections::HashSet<String> {
    status
        .lines()
        .filter_map(|line| {
            // Format: "XY filename" — X is index status, Y is worktree status
            // Examples: "M  file", "A  file", " D file", "?? file"
            // First 2 chars are status, then separator, then path
            // Find path after the status prefix
            if line.len() > 3 {
                Some(line[3..].trim().to_string())
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_files_extracts_paths() {
        let status = "M  src/lib.rs\nA  new_file.rs\n D deleted.rs\n?? untracked.txt\n";
        let files = parse_status_files(status);
        assert!(files.contains("src/lib.rs"), "missing src/lib.rs, got: {:?}", files);
        assert!(files.contains("new_file.rs"), "missing new_file.rs, got: {:?}", files);
        assert!(files.contains("deleted.rs"), "missing deleted.rs, got: {:?}", files);
        assert!(files.contains("untracked.txt"), "missing untracked.txt, got: {:?}", files);
        assert_eq!(4, files.len());
    }

    #[test]
    fn compute_changed_files_detects_new_files() {
        let pre = Some("M  existing.rs\n".to_string());
        let post = Some("M  existing.rs\nA  new_file.rs\n".to_string());
        let changed = compute_changed_files(&pre, &post);
        assert!(changed.contains(&"new_file.rs".to_string()));
    }

    #[test]
    fn compute_changed_files_detects_removals() {
        let pre = Some("M  keep.rs\nA  remove_me.rs\n".to_string());
        let post = Some("M  keep.rs\n".to_string());
        let changed = compute_changed_files(&pre, &post);
        assert!(changed.contains(&"remove_me.rs".to_string()));
    }

    #[test]
    fn compute_changed_files_no_changes() {
        let pre = Some("M  same.rs\n".to_string());
        let post = Some("M  same.rs\n".to_string());
        let changed = compute_changed_files(&pre, &post);
        assert!(changed.is_empty());
    }

    #[test]
    fn task_summary_serializes_round_trip() {
        let summary = TaskSummary {
            changed_files: vec!["src/lib.rs".into()],
            diff_stat: Some("1 file changed, 5 insertions(+), 2 deletions(-)".into()),
            completed: true,
            test_output: Some("3 passed".into()),
        };
        let json = serde_json::to_string(&summary).unwrap();
        let back: TaskSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary.changed_files, back.changed_files);
        assert_eq!(summary.diff_stat, back.diff_stat);
        assert!(back.completed);
    }

    #[test]
    fn stub_context_works_without_git() {
        let ctx = TaskContext::stub(
            "/fake/dir",
            Some("M  a.rs\n".to_string()),
            None,
        );
        assert_eq!(ctx.pre_git_status, Some("M  a.rs\n".to_string()));
        assert!(ctx.pre_git_diff_stat.is_none());
    }

    #[tokio::test]
    async fn before_task_on_non_git_dir_produces_none() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = TaskContext::before_task(dir.path()).await;
        // Not a git repo, so status should be None
        assert!(ctx.pre_git_status.is_none());
    }

    #[tokio::test]
    async fn after_task_on_non_git_dir_marks_incomplete() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = TaskContext::stub(&dir.path().to_string_lossy(), None, None);
        let summary = ctx.after_task().await;
        assert!(!summary.completed);
        assert!(summary.changed_files.is_empty());
    }

    #[test]
    fn with_test_output_adds_test_result() {
        let summary = TaskSummary {
            changed_files: vec![],
            diff_stat: None,
            completed: true,
            test_output: None,
        };
        let with_test = TaskContext::with_test_output(summary, "5 passed".into());
        assert_eq!(Some("5 passed".to_string()), with_test.test_output);
    }
}
