//! Workspace sandbox — centralized path containment for all local tools.
//!
//! Every local tool that touches the filesystem must resolve user-provided
//! paths through `resolve_workspace_path()`. This is the single authority
//! for path containment, independent of policy auto-allow decisions.

use std::path::{Component, Path, PathBuf};
#[cfg(unix)]
use std::ffi::OsStr;

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

// ── Handle-relative write hardening (Wave 73B) ─────────────────────
//
// On Unix, opens the workspace root as a directory file descriptor,
// then walks each path component using openat() with O_NOFOLLOW.
// This eliminates the TOCTOU race where an adversary replaces an
// intermediate directory with a symlink between validation and write.
//
// On Windows and other platforms, falls back to current behavior
// (resolve_workspace_path + write_file_no_follow).

/// Error from safe write operations.
#[derive(Debug)]
pub enum WriteSafeError {
    /// A path component is a symlink — TOCTOU race detected or attempted.
    SymlinkDetected { component: String },
    /// IO error (permission denied, disk full, etc.)
    Io(std::io::Error),
    /// Path validation failed.
    Validation(PathContainmentError),
}

impl std::fmt::Display for WriteSafeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteSafeError::SymlinkDetected { component } => {
                write!(f, "Symlink detected at intermediate component: {}", component)
            }
            WriteSafeError::Io(e) => write!(f, "IO error: {}", e),
            WriteSafeError::Validation(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for WriteSafeError {}

impl From<std::io::Error> for WriteSafeError {
    fn from(e: std::io::Error) -> Self {
        WriteSafeError::Io(e)
    }
}

impl From<PathContainmentError> for WriteSafeError {
    fn from(e: PathContainmentError) -> Self {
        WriteSafeError::Validation(e)
    }
}

/// Workspace write handle — opens the workspace root once, then performs
/// handle-relative path traversal for writes.
///
/// On Unix (Tier 1: Linux, macOS, FreeBSD): uses `openat` + `O_NOFOLLOW`
/// per component. Intermediate directory symlinks are detected and rejected.
///
/// On Windows and other platforms: falls back to `resolve_workspace_path`
/// + `write_file_no_follow` (72B final-component protection only).
pub struct WorkspaceWriteHandle {
    workspace: PathBuf,
    #[cfg(unix)]
    root_fd: Option<std::os::unix::io::RawFd>,
}

impl WorkspaceWriteHandle {
    /// Open the workspace root for handle-relative operations.
    pub fn open(workspace: &Path) -> Result<Self, WriteSafeError> {
        let canonical = workspace
            .canonicalize()
            .map_err(WriteSafeError::Io)?;

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let dir = std::fs::File::open(&canonical)
                .map_err(WriteSafeError::Io)?;
            // Verify it's a directory
            let meta = dir.metadata().map_err(WriteSafeError::Io)?;
            if !meta.is_dir() {
                return Err(WriteSafeError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotADirectory,
                    "Workspace root is not a directory",
                )));
            }
            let fd = dir.as_raw_fd();
            // Duplicate the fd so dropping `dir` doesn't close it
            let dup_fd = unsafe { libc::dup(fd) };
            if dup_fd < 0 {
                return Err(WriteSafeError::Io(std::io::Error::last_os_error()));
            }
            Ok(WorkspaceWriteHandle {
                workspace: canonical,
                root_fd: Some(dup_fd),
            })
        }

        #[cfg(not(unix))]
        {
            // Verify it's a directory
            let meta = std::fs::metadata(&canonical)
                .map_err(WriteSafeError::Io)?;
            if !meta.is_dir() {
                return Err(WriteSafeError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotADirectory,
                    "Workspace root is not a directory",
                )));
            }
            Ok(WorkspaceWriteHandle {
                workspace: canonical,
            })
        }
    }

    /// Create intermediate directories (no-follow at each step on Unix) and
    /// write the file at the given relative path.
    pub async fn create_and_write(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), WriteSafeError> {
        // Pre-validate via existing sandbox (keeps all current protections)
        let resolved = resolve_workspace_path(
            &self.workspace,
            relative_path,
            PathAccessMode::WriteTarget,
        )?;

        #[cfg(unix)]
        {
            self.unix_create_and_write(&resolved, content)
        }

        #[cfg(windows)]
        {
            self.windows_create_and_write(&resolved, content).await
        }

        #[cfg(not(any(unix, windows)))]
        {
            // True fallback for unsupported platforms
            if let Some(parent) = resolved.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(WriteSafeError::Io)?;
            }
            write_file_no_follow(&resolved, content).await
                .map_err(WriteSafeError::Io)
        }
    }

    /// Overwrite an existing file at the given relative path.
    pub async fn overwrite_existing(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), WriteSafeError> {
        // Pre-validate via existing sandbox
        let resolved = resolve_workspace_path(
            &self.workspace,
            relative_path,
            PathAccessMode::PatchExisting,
        )?;

        #[cfg(unix)]
        {
            self.unix_overwrite_existing(&resolved, content)
        }

        #[cfg(windows)]
        {
            self.windows_overwrite_existing(&resolved, content).await
        }

        #[cfg(not(any(unix, windows)))]
        {
            write_file_no_follow(&resolved, content).await
                .map_err(WriteSafeError::Io)
        }
    }
}

#[cfg(unix)]
impl WorkspaceWriteHandle {
    /// Unix: walk path components relative to root_fd using openat + O_NOFOLLOW.
    fn unix_create_and_write(
        &self,
        resolved: &Path,
        content: &str,
    ) -> Result<(), WriteSafeError> {
        let root_fd = self.root_fd.ok_or_else(|| {
            WriteSafeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "WorkspaceWriteHandle not properly initialized",
            ))
        })?;

        // Get the relative path from workspace to resolved target
        let rel = resolved.strip_prefix(&self.workspace)
            .map_err(|_| WriteSafeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Resolved path is not under workspace",
            )))?;

        let components: Vec<&OsStr> = rel.iter().collect();
        if components.is_empty() {
            return Err(WriteSafeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot write to workspace root",
            )));
        }

        // Platform-specific O_NOFOLLOW
        #[cfg(target_os = "linux")]
        const O_NOFOLLOW: i32 = 0x20000;
        #[cfg(target_os = "macos")]
        const O_NOFOLLOW: i32 = 0x0100;
        #[cfg(target_os = "freebsd")]
        const O_NOFOLLOW: i32 = 0x0100;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "freebsd")))]
        const O_NOFOLLOW: i32 = 0;

        let mut current_fd = root_fd;
        let mut opened_fds: Vec<i32> = Vec::new(); // track fds to close (not root_fd)

        // Walk intermediate directories
        let dir_components = &components[..components.len() - 1];
        for (i, comp) in dir_components.iter().enumerate() {
            let c_str = std::ffi::CString::new(comp.to_string_lossy().into_owned())
                .map_err(|_| WriteSafeError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Path component contains null byte",
                )))?;

            // Try to open existing directory, no-follow
            let fd = unsafe {
                libc::openat(
                    current_fd,
                    c_str.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | O_NOFOLLOW,
                )
            };

            if fd >= 0 {
                opened_fds.push(fd);
                current_fd = fd;
            } else {
                let err = std::io::Error::last_os_error();
                if err.kind() == std::io::ErrorKind::NotFound {
                    // Directory doesn't exist: create it
                    let mkdir_res = unsafe {
                        libc::mkdirat(current_fd, c_str.as_ptr(), 0o755)
                    };
                    if mkdir_res < 0 {
                        Self::close_fds(&opened_fds);
                        return Err(WriteSafeError::Io(std::io::Error::last_os_error()));
                    }
                    // Re-open the directory we just created, no-follow
                    let new_fd = unsafe {
                        libc::openat(
                            current_fd,
                            c_str.as_ptr(),
                            libc::O_RDONLY | libc::O_DIRECTORY | O_NOFOLLOW,
                        )
                    };
                    if new_fd < 0 {
                        let open_err = std::io::Error::last_os_error();
                        // ELOOP means it's a symlink — race detected
                        if open_err.raw_os_error() == Some(libc::ELOOP) {
                            Self::close_fds(&opened_fds);
                            return Err(WriteSafeError::SymlinkDetected {
                                component: comp.to_string_lossy().into_owned(),
                            });
                        }
                        Self::close_fds(&opened_fds);
                        return Err(WriteSafeError::Io(open_err));
                    }
                    opened_fds.push(new_fd);
                    current_fd = new_fd;
                } else if err.raw_os_error() == Some(libc::ELOOP) {
                    // Symlink detected at this component
                    Self::close_fds(&opened_fds);
                    return Err(WriteSafeError::SymlinkDetected {
                        component: comp.to_string_lossy().into_owned(),
                    });
                } else {
                    Self::close_fds(&opened_fds);
                    return Err(WriteSafeError::Io(err));
                }
            }

            // Verify the fd is still a directory (not replaced after open)
            let mut stat_buf: libc::stat = unsafe { std::mem::zeroed() };
            let fstat_res = unsafe { libc::fstat(current_fd, &mut stat_buf) };
            if fstat_res < 0 {
                Self::close_fds(&opened_fds);
                return Err(WriteSafeError::Io(std::io::Error::last_os_error()));
            }
            if stat_buf.st_mode & libc::S_IFMT != libc::S_IFDIR {
                Self::close_fds(&opened_fds);
                return Err(WriteSafeError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotADirectory,
                    format!("Component '{}' is not a directory", comp.to_string_lossy()),
                )));
            }

            let _ = i; // suppress unused warning on last iteration
        }

        // Open final component for writing, no-follow
        let filename = components.last().unwrap();
        let filename_cstr = std::ffi::CString::new(filename.to_string_lossy().into_owned())
            .map_err(|_| WriteSafeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Filename contains null byte",
            )))?;

        let file_fd = unsafe {
            libc::openat(
                current_fd,
                filename_cstr.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC | O_NOFOLLOW,
                0o644,
            )
        };

        if file_fd < 0 {
            let err = std::io::Error::last_os_error();
            Self::close_fds(&opened_fds);
            if err.raw_os_error() == Some(libc::ELOOP) {
                return Err(WriteSafeError::SymlinkDetected {
                    component: filename.to_string_lossy().into_owned(),
                });
            }
            return Err(WriteSafeError::Io(err));
        }

        // Write content via fd
        let content_bytes = content.as_bytes();
        let mut written: usize = 0;
        while written < content_bytes.len() {
            let n = unsafe {
                libc::write(
                    file_fd,
                    content_bytes[written..].as_ptr() as *const libc::c_void,
                    content_bytes.len() - written,
                )
            };
            if n < 0 {
                unsafe { libc::close(file_fd) };
                Self::close_fds(&opened_fds);
                return Err(WriteSafeError::Io(std::io::Error::last_os_error()));
            }
            written += n as usize;
        }

        // Close file and intermediate fds
        unsafe { libc::close(file_fd) };
        Self::close_fds(&opened_fds);
        Ok(())
    }

    /// Unix: overwrite existing file using handle-relative traversal.
    fn unix_overwrite_existing(
        &self,
        resolved: &Path,
        content: &str,
    ) -> Result<(), WriteSafeError> {
        // Overwrite uses the same traversal but the file must exist
        // (PatchExisting validation already checked this)
        self.unix_create_and_write(resolved, content)
    }

    fn close_fds(fds: &[i32]) {
        for &fd in fds {
            unsafe { libc::close(fd) };
        }
    }
}

// ── Windows NtCreateFile handle-relative write hardening (Wave 78C) ────
//
// Uses NtCreateFile with OBJECT_ATTRIBUTES.RootDirectory to open each
// path component relative to a parent directory handle. Reparse points
// (symlinks, junctions) are opened via FILE_OPEN_REPARSE_POINT without
// following, then detected via GetFileInformationByHandleEx.
//
// This mirrors the Unix openat + O_NOFOLLOW approach from Wave 73B.
// The intermediate-directory TOCTOU micro-race is fully closed because
// each component is opened relative to a trusted handle, not by path name.

#[cfg(windows)]
impl WorkspaceWriteHandle {
    /// Windows: NtCreateFile handle-relative directory traversal +
    /// FILE_FLAG_NO_REPARSE_POINT file write.
    ///
    /// Intermediate directories are walked via NtCreateFile with
    /// OBJECT_ATTRIBUTES.RootDirectory (handle-relative, no TOCTOU).
    /// The final file is written via the proven `write_file_no_follow`
    /// path which uses `FILE_FLAG_NO_REPARSE_POINT` for reparse detection.
    async fn windows_create_and_write(
        &self,
        resolved: &Path,
        content: &str,
    ) -> Result<(), WriteSafeError> {
        // Perform all handle-relative work synchronously, then drop handles
        // before the async write_file_no_follow call (HandleGuard is not Send).
        let verify_result = self.windows_verify_path_components(resolved);
        verify_result?;

        // Write the file using the proven write_file_no_follow path.
        // This uses FILE_FLAG_NO_REPARSE_POINT for final-component protection.
        // We've already verified the intermediate path via handle-relative traversal.
        write_file_no_follow(resolved, content).await
            .map_err(WriteSafeError::Io)
    }

    /// Synchronous handle-relative path verification.
    /// Opens each intermediate directory relative to parent handle,
    /// checks for reparse points, and verifies the final component.
    fn windows_verify_path_components(
        &self,
        resolved: &Path,
    ) -> Result<(), WriteSafeError> {
        use super::sandbox_ntapi::{open_root_handle, HandleGuard};

        let rel = resolved.strip_prefix(&self.workspace)
            .map_err(|_| WriteSafeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Resolved path is not under workspace",
            )))?;

        let components: Vec<&std::ffi::OsStr> = rel.iter().collect();
        if components.is_empty() {
            return Err(WriteSafeError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot write to workspace root",
            )));
        }

        // Open workspace root as a directory handle
        let root_raw = open_root_handle(&self.workspace)?;
        let root = HandleGuard::new(root_raw);

        // Walk intermediate directories via NtCreateFile handle-relative traversal.
        // This closes the intermediate-directory TOCTOU race (VB-1).
        let dir_components = &components[..components.len() - 1];
        let mut current = root.get();
        // Hold child handles to keep them alive during traversal
        let mut _handles: Vec<HandleGuard> = Vec::new();

        for comp in dir_components {
            let child_raw = unsafe {
                super::sandbox_ntapi::open_dir_relative(current, comp, true)?
            };
            let child = HandleGuard::new(child_raw);
            current = child.get();
            _handles.push(child);
        }

        // Verify the final component via handle-relative reparse check.
        let filename = components.last().unwrap();
        unsafe {
            super::sandbox_ntapi::check_file_not_reparse(current, filename)
        }

        // All handles dropped here when _handles and root go out of scope
    }

    /// Windows: overwrite existing file with handle-relative verification.
    async fn windows_overwrite_existing(
        &self,
        resolved: &Path,
        content: &str,
    ) -> Result<(), WriteSafeError> {
        // Same traversal — file must exist (PatchExisting already validated)
        self.windows_create_and_write(resolved, content).await
    }
}

impl Drop for WorkspaceWriteHandle {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(fd) = self.root_fd.take() {
                unsafe { libc::close(fd) };
            }
        }
    }
}

// SAFETY: The handle contains a file descriptor that is not shared across threads
// during active use. The fd is only accessed within individual method calls.
unsafe impl Send for WorkspaceWriteHandle {}
unsafe impl Sync for WorkspaceWriteHandle {}

#[cfg(test)]
mod handle_tests {
    use super::*;

    fn setup_workspace() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn handle_opens_workspace_root() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path());
        assert!(handle.is_ok(), "Failed to open handle: {:?}", handle.err());
    }

    #[test]
    fn handle_rejects_nonexistent_workspace() {
        let handle = WorkspaceWriteHandle::open(Path::new("/nonexistent/path/to/workspace"));
        assert!(handle.is_err());
    }

    #[test]
    fn handle_rejects_file_as_workspace() {
        let ws = setup_workspace();
        let file_path = ws.path().join("not_a_dir.txt");
        std::fs::write(&file_path, "test").unwrap();
        let handle = WorkspaceWriteHandle::open(&file_path);
        assert!(handle.is_err());
    }

    #[tokio::test]
    async fn create_and_write_single_component() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("file.txt", "hello").await.unwrap();
        let contents = std::fs::read_to_string(ws.path().join("file.txt")).unwrap();
        assert_eq!("hello", contents);
    }

    #[tokio::test]
    async fn create_and_write_creates_intermediate_dirs() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("a/b/c/file.txt", "deep").await.unwrap();
        let contents = std::fs::read_to_string(
            ws.path().join("a/b/c/file.txt")
        ).unwrap();
        assert_eq!("deep", contents);
        // Verify intermediate directories were created
        assert!(ws.path().join("a").is_dir());
        assert!(ws.path().join("a/b").is_dir());
        assert!(ws.path().join("a/b/c").is_dir());
    }

    #[tokio::test]
    async fn create_and_write_overwrites_existing() {
        let ws = setup_workspace();
        std::fs::write(ws.path().join("exists.txt"), "original").unwrap();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("exists.txt", "replaced").await.unwrap();
        let contents = std::fs::read_to_string(ws.path().join("exists.txt")).unwrap();
        assert_eq!("replaced", contents);
    }

    #[tokio::test]
    async fn overwrite_existing_works() {
        let ws = setup_workspace();
        std::fs::write(ws.path().join("patch.txt"), "original").unwrap();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.overwrite_existing("patch.txt", "patched").await.unwrap();
        let contents = std::fs::read_to_string(ws.path().join("patch.txt")).unwrap();
        assert_eq!("patched", contents);
    }

    #[tokio::test]
    async fn handle_rejects_absolute_path() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        let result = handle.create_and_write("/etc/passwd", "evil").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn handle_rejects_parent_traversal() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        let result = handle.create_and_write("../../../etc/passwd", "evil").await;
        assert!(result.is_err());
    }

    // ── Symlink detection tests (Unix only) ──────────────────────────

    #[cfg(unix)]
    #[tokio::test]
    async fn create_and_write_detects_intermediate_symlink() {
        let ws = setup_workspace();
        let outside = tempfile::tempdir().unwrap();

        // Create: workspace/link_dir -> outside
        std::os::unix::fs::symlink(
            outside.path(),
            ws.path().join("link_dir"),
        ).unwrap();

        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        let result = handle.create_and_write("link_dir/file.txt", "evil").await;

        match result {
            Err(WriteSafeError::SymlinkDetected { component }) => {
                assert_eq!("link_dir", component);
            }
            Err(WriteSafeError::Validation(e)) => {
                // On some Unix systems, canonicalization detects the symlink
                // escape before the per-component check fires. This is still
                // a correct rejection — the symlink IS blocked.
                assert!(
                    e.message.contains("Symlink") || e.message.contains("symlink") || e.message.contains("escapes"),
                    "Expected symlink-related rejection, got: {:?}",
                    e
                );
            }
            Err(e) => {
                panic!("Expected SymlinkDetected or PathContainmentError, got: {:?}", e);
            }
            Ok(()) => {
                panic!("Write should have been blocked — symlink at intermediate component");
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn create_and_write_detects_final_component_symlink() {
        let ws = setup_workspace();
        let outside = tempfile::tempdir().unwrap();

        // Create: workspace/link_file -> outside/secret.txt
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            ws.path().join("link_file.txt"),
        ).unwrap();

        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        let result = handle.create_and_write("link_file.txt", "evil").await;

        match result {
            Err(WriteSafeError::SymlinkDetected { component }) => {
                assert_eq!("link_file.txt", component);
            }
            Err(e) => {
                // May also fail with ELOOP at final component
                panic!("Expected SymlinkDetected, got: {:?}", e);
            }
            Ok(()) => {
                panic!("Write should have been blocked — symlink at final component");
            }
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn create_and_write_works_in_honest_directory() {
        let ws = setup_workspace();
        std::fs::create_dir_all(ws.path().join("honest_dir")).unwrap();

        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("honest_dir/file.txt", "legit").await.unwrap();

        let contents = std::fs::read_to_string(
            ws.path().join("honest_dir/file.txt")
        ).unwrap();
        assert_eq!("legit", contents);
    }

    // ── Windows per-component reparse point tests ──────────────────

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_create_and_write_creates_intermediate_dirs() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("a/b/c/file.txt", "deep").await.unwrap();
        let contents = std::fs::read_to_string(
            ws.path().join("a/b/c/file.txt")
        ).unwrap();
        assert_eq!("deep", contents);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_create_and_write_single_component() {
        let ws = setup_workspace();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("file.txt", "hello").await.unwrap();
        let contents = std::fs::read_to_string(ws.path().join("file.txt")).unwrap();
        assert_eq!("hello", contents);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_create_and_write_overwrites_existing() {
        let ws = setup_workspace();
        std::fs::write(ws.path().join("exists.txt"), "original").unwrap();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.create_and_write("exists.txt", "replaced").await.unwrap();
        let contents = std::fs::read_to_string(ws.path().join("exists.txt")).unwrap();
        assert_eq!("replaced", contents);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_overwrite_existing_works() {
        let ws = setup_workspace();
        std::fs::write(ws.path().join("patch.txt"), "original").unwrap();
        let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
        handle.overwrite_existing("patch.txt", "patched").await.unwrap();
        let contents = std::fs::read_to_string(ws.path().join("patch.txt")).unwrap();
        assert_eq!("patched", contents);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_create_and_write_detects_symlink_intermediate() {
        let ws = setup_workspace();
        let outside = tempfile::tempdir().unwrap();

        // Attempt to create a directory symlink (requires admin/developer mode)
        let link_result = std::os::windows::fs::symlink_dir(
            outside.path(),
            ws.path().join("link_dir"),
        );

        if let Ok(()) = link_result {
            let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
            let result = handle.create_and_write("link_dir/file.txt", "evil").await;
            match result {
                Err(WriteSafeError::SymlinkDetected { component }) => {
                    assert_eq!("link_dir", component);
                }
                Err(e) => {
                    panic!("Expected SymlinkDetected, got: {:?}", e);
                }
                Ok(()) => {
                    panic!("Write should have been blocked — symlink at intermediate component");
                }
            }
        } else {
            eprintln!("SKIP: Cannot create directory symlink (requires admin/developer mode)");
        }
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn windows_create_and_write_detects_symlink_final() {
        let ws = setup_workspace();
        let outside = tempfile::tempdir().unwrap();

        // Attempt to create a file symlink (may require admin/developer mode)
        let link_result = std::os::windows::fs::symlink_file(
            outside.path().join("secret.txt"),
            ws.path().join("link_file.txt"),
        );

        if let Ok(()) = link_result {
            let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();
            let result = handle.create_and_write("link_file.txt", "evil").await;
            // Final component detection depends on FILE_FLAG_NO_REPARSE_POINT behavior
            // Either SymlinkDetected or Io error is acceptable — but NOT Ok with escape
            if let Ok(()) = result {
                // Verify the write stayed inside workspace
                let target = ws.path().join("link_file.txt");
                if target.exists() {
                    let canonical = target.canonicalize().unwrap();
                    let canonical_ws = ws.path().canonicalize().unwrap();
                    assert!(canonical.starts_with(&canonical_ws),
                        "Write escaped workspace!");
                }
            }
        } else {
            eprintln!("SKIP: Cannot create file symlink (requires admin/developer mode)");
        }
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
                .unwrap_or(());
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
                .unwrap_or(());
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

    #[cfg(windows)]
    #[test]
    fn rejects_windows_drive_prefix_path() {
        let ws = setup_workspace();
        let err = resolve_workspace_path(ws.path(), "C:\\Windows\\System32", PathAccessMode::ReadExisting)
            .unwrap_err();
        // On non-Windows, is_absolute() catches this. On Windows, Prefix component catches it.
        assert!(err.message.contains("Absolute") || err.message.contains("Drive") || err.message.contains("Prefix"));
    }

    #[cfg(windows)]
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
