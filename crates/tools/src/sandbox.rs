//! Workspace sandbox — centralized path containment for all local tools.
//!
//! Every local tool that touches the filesystem must resolve user-provided
//! paths through `resolve_workspace_path()`. This is the single authority
//! for path containment, independent of policy auto-allow decisions.

use std::path::{Component, Path, PathBuf};

/// How the resolved path will be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathAccessMode {
    /// Read an existing file.
    ReadExisting,
    /// List an existing directory.
    ListExisting,
    /// Search under an existing directory.
    SearchExisting,
    /// Write to a target (may not exist yet).
    WriteTarget,
    /// Patch an existing file.
    PatchExisting,
}

/// Path containment error. Messages are safe for user display —
/// they do not leak external canonical paths (Patch 7).
#[derive(Debug, Clone)]
pub struct PathContainmentError {
    pub message: String,
}

impl std::fmt::Display for PathContainmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PathContainmentError {}

fn containment_err(msg: impl Into<String>) -> PathContainmentError {
    PathContainmentError { message: msg.into() }
}

/// Resolve a user-provided path against the canonical workspace root.
///
/// Enforces (Patches 1, 2, 7):
/// - Rejects absolute paths (including Windows drive/UNC prefixes)
/// - Rejects `..` parent traversal components
/// - Resolves against canonical workspace root
/// - Verifies final canonical target or parent remains under workspace
/// - Rejects symlink escapes
/// - For WriteTarget: canonicalizes parent (target may not exist)
/// - For PatchExisting/ReadExisting/ListExisting/SearchExisting: canonicalizes target
pub fn resolve_workspace_path(
    workspace: &Path,
    user_path: &str,
    mode: PathAccessMode,
) -> Result<PathBuf, PathContainmentError> {
    let user = Path::new(user_path);

    // Reject absolute paths (Patch 8: handles Windows drive/UNC too)
    // On Windows, /foo is NOT absolute — it's relative to current drive root.
    // We must also check for leading '/' on Windows.
    if user.is_absolute() {
        return Err(containment_err("Absolute paths are not allowed"));
    }

    // On Windows, also reject paths starting with '/' (drive-root relative)
    #[cfg(windows)]
    if user_path.starts_with('/') || user_path.starts_with('\\') {
        return Err(containment_err("Absolute paths are not allowed"));
    }

    // Reject Windows UNC and drive-prefix components
    for component in user.components() {
        match component {
            Component::Prefix(_) => {
                return Err(containment_err(
                    "Drive/UNC prefix paths are not allowed",
                ));
            }
            Component::ParentDir => {
                return Err(containment_err(
                    "Parent traversal (..) is not allowed",
                ));
            }
            _ => {}
        }
    }

    // Canonicalize workspace root
    let canonical_workspace = workspace
        .canonicalize()
        .map_err(|_| containment_err("Cannot canonicalize workspace root"))?;

    // Join user path with workspace
    let joined = canonical_workspace.join(user);

    match mode {
        PathAccessMode::WriteTarget => {
            // Target may not exist: canonicalize parent, verify parent is inside workspace
            if let Some(parent) = joined.parent() {
                if parent.exists() {
                    let canonical_parent = parent
                        .canonicalize()
                        .map_err(|_| containment_err("Cannot canonicalize parent directory"))?;
                    if !canonical_parent.starts_with(&canonical_workspace) {
                        return Err(containment_err(
                            "Symlink target escapes the authorized workspace",
                        ));
                    }
                } else {
                    // Parent doesn't exist either — verify the whole chain
                    // Walk up until we find an existing ancestor
                    let mut check = parent.to_path_buf();
                    loop {
                        if check.exists() {
                            let canonical = check
                                .canonicalize()
                                .map_err(|_| containment_err("Cannot canonicalize ancestor directory"))?;
                            if !canonical.starts_with(&canonical_workspace) {
                                return Err(containment_err(
                                    "Path escapes the authorized workspace",
                                ));
                            }
                            break;
                        }
                        match check.parent() {
                            Some(p) => check = p.to_path_buf(),
                            None => break,
                        }
                    }
                }
            }
            // Check if target itself is a symlink that escapes
            if joined.is_symlink() {
                if let Ok(canonical_target) = joined.canonicalize() {
                    if !canonical_target.starts_with(&canonical_workspace) {
                        return Err(containment_err(
                            "Symlink target escapes the authorized workspace",
                        ));
                    }
                }
            }
            Ok(joined)
        }
        PathAccessMode::ReadExisting
        | PathAccessMode::ListExisting
        | PathAccessMode::SearchExisting
        | PathAccessMode::PatchExisting => {
            // Target should exist: canonicalize it and verify containment
            if joined.exists() {
                let canonical_target = joined
                    .canonicalize()
                    .map_err(|_| containment_err("Cannot canonicalize target path"))?;
                if !canonical_target.starts_with(&canonical_workspace) {
                    return Err(containment_err(
                        "Path escapes the authorized workspace",
                    ));
                }
                Ok(canonical_target)
            } else if mode == PathAccessMode::SearchExisting || mode == PathAccessMode::ListExisting {
                // For search/list, the directory might not exist — that's a normal error, not a containment error
                // Return the joined path so the handler can produce a "not found" error
                Ok(joined)
            } else {
                // ReadExisting/PatchExisting: file must exist
                Ok(joined)
            }
        }
    }
}

/// Check if a resolved path is contained within the canonical workspace.
/// Used by search to verify each traversed path during recursive walk.
pub fn is_path_in_workspace(resolved: &Path, canonical_workspace: &Path) -> bool {
    if let Ok(canonical) = resolved.canonicalize() {
        canonical.starts_with(canonical_workspace)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_workspace() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── Absolute path rejection ────────────────────────────────────────

    #[test]
    fn rejects_absolute_path_on_read() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "/etc/passwd", PathAccessMode::ReadExisting)
            .unwrap_err();
        assert!(err.message.contains("Absolute paths"));
    }

    #[test]
    fn rejects_absolute_path_on_list() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "/etc", PathAccessMode::ListExisting)
            .unwrap_err();
        assert!(err.message.contains("Absolute paths"));
    }

    #[test]
    fn rejects_absolute_path_on_search() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "/etc", PathAccessMode::SearchExisting)
            .unwrap_err();
        assert!(err.message.contains("Absolute paths"));
    }

    #[test]
    fn rejects_absolute_path_on_write() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "/tmp/evil", PathAccessMode::WriteTarget)
            .unwrap_err();
        assert!(err.message.contains("Absolute paths"));
    }

    #[test]
    fn rejects_absolute_path_on_patch() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "/etc/hosts", PathAccessMode::PatchExisting)
            .unwrap_err();
        assert!(err.message.contains("Absolute paths"));
    }

    // ── Parent traversal rejection ─────────────────────────────────────

    #[test]
    fn rejects_parent_traversal_on_read() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "../../../etc/passwd", PathAccessMode::ReadExisting)
            .unwrap_err();
        assert!(err.message.contains("Parent traversal"));
    }

    #[test]
    fn rejects_parent_traversal_on_list() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "../../..", PathAccessMode::ListExisting)
            .unwrap_err();
        assert!(err.message.contains("Parent traversal"));
    }

    #[test]
    fn rejects_parent_traversal_on_search() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "../secrets", PathAccessMode::SearchExisting)
            .unwrap_err();
        assert!(err.message.contains("Parent traversal"));
    }

    #[test]
    fn rejects_parent_traversal_on_write() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "../outside.txt", PathAccessMode::WriteTarget)
            .unwrap_err();
        assert!(err.message.contains("Parent traversal"));
    }

    #[test]
    fn rejects_parent_traversal_on_patch() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "../hosts", PathAccessMode::PatchExisting)
            .unwrap_err();
        assert!(err.message.contains("Parent traversal"));
    }

    // ── Valid paths are resolved correctly ─────────────────────────────

    #[test]
    fn resolves_valid_relative_read() {
        let ws = setup_workspace();
        fs::write(ws.path().join("test.txt"), "hello").unwrap();
        let resolved = resolve_workspace_path(ws.path(), "test.txt", PathAccessMode::ReadExisting)
            .unwrap();
        let canonical_ws = ws.path().canonicalize().unwrap();
        assert!(resolved.starts_with(&canonical_ws));
        assert!(resolved.exists());
    }

    #[test]
    fn resolves_valid_subdirectory_read() {
        let ws = setup_workspace();
        fs::create_dir_all(ws.path().join("src")).unwrap();
        fs::write(ws.path().join("src/main.rs"), "fn main() {}").unwrap();
        let resolved = resolve_workspace_path(ws.path(), "src/main.rs", PathAccessMode::ReadExisting)
            .unwrap();
        let canonical_ws = ws.path().canonicalize().unwrap();
        assert!(resolved.starts_with(&canonical_ws));
    }

    #[test]
    fn resolves_valid_write_target() {
        let ws = setup_workspace();
        let resolved = resolve_workspace_path(ws.path(), "new_file.txt", PathAccessMode::WriteTarget)
            .unwrap();
        let canonical_ws = ws.path().canonicalize().unwrap();
        // WriteTarget returns joined path (not canonicalized since target doesn't exist)
        assert!(resolved.starts_with(&canonical_ws));
    }

    #[test]
    fn resolves_nested_write_target() {
        let ws = setup_workspace();
        fs::create_dir_all(ws.path().join("src")).unwrap();
        let resolved = resolve_workspace_path(ws.path(), "src/new.rs", PathAccessMode::WriteTarget)
            .unwrap();
        let canonical_ws = ws.path().canonicalize().unwrap();
        assert!(resolved.starts_with(&canonical_ws));
    }

    // ── Symlink escape rejection ───────────────────────────────────────

    #[test]
    fn rejects_symlink_target_outside_workspace_on_read() {
        let ws = setup_workspace();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "secret").unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.path().join("secret.txt"), ws.path().join("link.txt"))
                .unwrap();
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(outside.path().join("secret.txt"), ws.path().join("link.txt"))
                .unwrap_or_else(|_| {
                    // Symlink creation may require admin on Windows; skip gracefully
                    return;
                });
        }

        if ws.path().join("link.txt").exists() {
            let err = resolve_workspace_path(ws.path(), "link.txt", PathAccessMode::ReadExisting)
                .unwrap_err();
            assert!(err.message.contains("escapes") || err.message.contains("workspace"));
        }
    }

    #[test]
    fn rejects_symlink_target_outside_workspace_on_write() {
        let ws = setup_workspace();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir_all(outside.path()).unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside.path(), ws.path().join("escape_dir"))
                .unwrap();
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(outside.path(), ws.path().join("escape_dir"))
                .unwrap_or_else(|_| return);
        }

        if ws.path().join("escape_dir").exists() {
            let err = resolve_workspace_path(ws.path(), "escape_dir/evil.txt", PathAccessMode::WriteTarget)
                .unwrap_err();
            assert!(err.message.contains("escapes") || err.message.contains("Symlink"));
        }
    }

    // ── Error message safety (Patch 7) ─────────────────────────────────

    #[test]
    fn error_messages_do_not_leak_external_paths() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "/etc/shadow", PathAccessMode::ReadExisting)
            .unwrap_err();
        // Should NOT contain the actual /etc/shadow canonical path
        assert!(!err.message.contains("/etc/shadow"));
        assert!(!err.message.contains("/private/etc"));
    }

    #[test]
    fn error_messages_do_not_leak_canonical_workspace_path() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "../../etc/passwd", PathAccessMode::ReadExisting)
            .unwrap_err();
        // Should not contain the full canonical workspace path
        let ws_canonical = ws.path().canonicalize().unwrap();
        let ws_str = ws_canonical.to_string_lossy();
        assert!(!err.message.contains(&*ws_str));
    }

    // ── Windows prefix rejection (Patch 8) ─────────────────────────────

    #[test]
    fn rejects_windows_drive_prefix_path() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "C:\\Windows\\System32", PathAccessMode::ReadExisting)
            .unwrap_err();
        // On non-Windows, is_absolute() catches this. On Windows, Prefix component catches it.
        assert!(err.message.contains("Absolute") || err.message.contains("Drive") || err.message.contains("Prefix"));
    }

    #[test]
    fn rejects_unc_path() {
        let ws = setup_workspace();
        // UNC paths start with \\ which is absolute on Windows
        let err = resolve_workspace_path(ws.path(), "\\\\server\\share", PathAccessMode::ReadExisting)
            .unwrap_err();
        assert!(err.message.contains("Absolute") || err.message.contains("UNC") || err.message.contains("Prefix"));
    }

    // ── PathAccessMode semantics ───────────────────────────────────────

    #[test]
    fn read_existing_resolves_canonical() {
        let ws = setup_workspace();
        fs::write(ws.path().join("real.txt"), "data").unwrap();
        let resolved = resolve_workspace_path(ws.path(), "real.txt", PathAccessMode::ReadExisting)
            .unwrap();
        // Should be canonical (no relative components)
        assert!(resolved.is_absolute());
        assert!(resolved.exists());
    }

    #[test]
    fn write_target_allows_nonexistent() {
        let ws = setup_workspace();
        let resolved = resolve_workspace_path(ws.path(), "does_not_exist_yet.txt", PathAccessMode::WriteTarget)
            .unwrap();
        let canonical_ws = ws.path().canonicalize().unwrap();
        assert!(resolved.starts_with(&canonical_ws));
        assert!(!resolved.exists());
    }

    #[test]
    fn search_existing_returns_joined_for_missing_dir() {
        let ws = setup_workspace();
        let resolved = resolve_workspace_path(ws.path(), "nonexistent_dir", PathAccessMode::SearchExisting)
            .unwrap();
        let canonical_ws = ws.path().canonicalize().unwrap();
        // Returns joined path so handler can produce "not found" error
        assert!(resolved.starts_with(&canonical_ws));
    }
}
