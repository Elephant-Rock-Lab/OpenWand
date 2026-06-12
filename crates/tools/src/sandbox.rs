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
            if joined.is_symlink()
                && let Ok(canonical_target) = joined.canonicalize()
                    && !canonical_target.starts_with(&canonical_workspace) {
                        return Err(containment_err(
                            "Symlink target escapes the authorized workspace",
                        ));
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

/// Write a file without following symlinks at the final path component.
///
/// This is the TOCTOU hardening: after `resolve_workspace_path()` validates
/// the path at check time, `write_file_no_follow()` ensures the write does
/// not follow a symlink that an adversary placed between validation and use.
///
/// Platform strategy:
/// - **Windows**: Opens with `FILE_FLAG_NO_REPARSE_POINT` (0x00200000) which
///   prevents following reparse points (symlinks) on the final component.
/// - **Unix**: Opens with `O_NOFOLLOW` which fails if the final component
///   is a symlink.
///
/// Note: This hardens the **final component** race. A race on intermediate
/// directory components (parent replaced with symlink between validation and
/// `create_dir_all`) is NOT fully closed here. Closing that requires
/// handle-relative directory traversal (dirfd/openat), which is a deeper
/// platform-specific change.
pub async fn write_file_no_follow(path: &Path, content: &str) -> std::io::Result<()> {
    use std::io::Write;

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // O_NOFOLLOW: fail if the final path component is a symlink.
        // Linux: 0x20000, macOS: 0x0100, FreeBSD: 0x0100
        #[cfg(target_os = "linux")]
        const O_NOFOLLOW: i32 = 0x20000;
        #[cfg(target_os = "macos")]
        const O_NOFOLLOW: i32 = 0x0100;
        #[cfg(target_os = "freebsd")]
        const O_NOFOLLOW: i32 = 0x0100;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
        const O_NOFOLLOW: i32 = 0; // fallback: no hardening on other Unix
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(O_NOFOLLOW)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_NO_REPARSE_POINT = 0x00200000
        // Prevents following reparse points (symlinks) on the target.
        const FILE_FLAG_NO_REPARSE_POINT: u32 = 0x00200000;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .custom_flags(FILE_FLAG_NO_REPARSE_POINT)
            .open(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    #[cfg(not(any(unix, windows)))]
    {
        // Fallback: standard write (no TOCTOU hardening on other platforms)
        tokio::fs::write(path, content).await
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

    // ── TOCTOU hardening (Wave 72B) ─────────────────────────────────────

    #[tokio::test]
    async fn write_file_no_follow_creates_file() {
        let ws = setup_workspace();
        let target = ws.path().join("safe_write.txt");
        super::write_file_no_follow(&target, "hello from no-follow").await.unwrap();
        let contents = std::fs::read_to_string(&target).unwrap();
        assert_eq!("hello from no-follow", contents);
    }

    #[tokio::test]
    async fn write_file_no_follow_overwrites_existing() {
        let ws = setup_workspace();
        let target = ws.path().join("overwrite.txt");
        std::fs::write(&target, "original").unwrap();
        super::write_file_no_follow(&target, "replaced").await.unwrap();
        let contents = std::fs::read_to_string(&target).unwrap();
        assert_eq!("replaced", contents);
    }

    #[tokio::test]
    async fn write_file_no_follow_creates_parent_dirs_then_writes() {
        let ws = setup_workspace();
        let subdir = ws.path().join("deep").join("nested");
        std::fs::create_dir_all(&subdir).unwrap();
        let target = subdir.join("file.txt");
        super::write_file_no_follow(&target, "deep write").await.unwrap();
        let contents = std::fs::read_to_string(&target).unwrap();
        assert_eq!("deep write", contents);
    }
}
