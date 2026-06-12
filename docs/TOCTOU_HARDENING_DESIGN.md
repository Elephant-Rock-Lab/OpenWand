# Wave 73A — Intermediate-Directory TOCTOU Hardening Design

**Date:** 2026-06-12
**Status:** Design + feasibility assessment
**Lock condition:** This wave produces a precise platform-specific design for closing the
remaining intermediate-directory TOCTOU write race. It does not claim the race is closed
unless implementation and tests prove it.

---

## 1. Threat Model (Precise)

### Current protections (what is already closed)

| Attack | Protection | Since |
|--------|-----------|-------|
| Direct path traversal (`../../etc/passwd`) | `..` component rejection | 69A |
| Static symlink escape (file → outside) | Canonicalize + prefix check | 69A |
| Windows drive/UNC prefix | Prefix component rejection | 69A Patch 8 |
| Final-component symlink race (file write) | `write_file_no_follow()` (no-follow flags) | 72B |

### Remaining race (what is NOT closed)

**Scenario:** An adversary with concurrent local filesystem access replaces an
intermediate directory between path validation and file write.

```
Timeline:

T1  resolve_workspace_path("src/deep/file.txt", WriteTarget)
    → canonicalize workspace root: /workspace/
    → join: /workspace/src/deep/file.txt
    → canonicalize parent /workspace/src/deep/ → exists, starts_with(/workspace/)
    → return /workspace/src/deep/file.txt  ✅ PASS

T2  Adversary: mv /workspace/src/deep /tmp/stolen && ln -s /etc /workspace/src/deep

T3  file_write_handler:
    → create_dir_all(/workspace/src/deep/)  ← follows symlink → creates /etc/
    → write_file_no_follow(/workspace/src/deep/file.txt, ...)
       ← FILE_FLAG_NO_REPARSE_POINT / O_NOFOLLOW
       ← checks FINAL component "file.txt" — not a symlink → WRITES TO /etc/file.txt
```

**Key insight:** `write_file_no_follow()` only protects the *final component*. If an
intermediate directory was replaced with a symlink, the no-follow write follows the
symlinked directory path and writes inside it. The final component ("file.txt") is
honestly a new file, so no-follow passes.

### Attack requirements

1. Local concurrent filesystem access (same machine, same user or same FS).
2. Timing precision to win the race window between T1 and T3.
3. Write permission on the intermediate directory being replaced.
4. The target path must have at least 2 directory components (workspace/child/grandchild/file).

### Attack window

The race window is between `resolve_workspace_path()` returning and the write syscall
completing. In practice this is microseconds to milliseconds — tight but exploitable by
a dedicated adversary using inotify/FSEvents or polling.

---

## 2. Write Path Audit

Three write sites in the codebase:

| Site | File | Current behavior | TOCTOU gap |
|------|------|-----------------|------------|
| `file_write_handler` | `local.rs:610` | `create_dir_all(parent)` then `write_file_no_follow()` | `create_dir_all` follows symlinks |
| `file_patch_apply` | `file_patch.rs:158` | `tokio::fs::write(abs_path)` | No no-follow; also uses `create_dir_all` for rollback dir |
| Rollback writes | `local.rs:625`, `file_patch.rs:130` | `tokio::fs::write` / `create_dir_all` | Same gaps, but writes to `.openwand/rollback/` inside workspace |

### Read path

`file_read_handler` reads via `tokio::fs::read_to_string` after `resolve_workspace_path()`
with `ReadExisting` mode (canonicalizes the target). The read path is less critical (reads
can't corrupt data), but a race could leak information if a symlink is placed after
canonicalization. This is a lower-severity variant of the same problem.

---

## 3. Platform-Specific Strategy

### 3.1 Unix (Linux, macOS, FreeBSD)

**Primitive:** `openat()` / `dirfd` — open directories by file descriptor, then open
children relative to that descriptor. Each step uses `O_NOFOLLOW` to refuse symlinks.

**Algorithm:**

```
fn safe_create_and_write(workspace_fd, relative_path, content):
    components = split(relative_path, '/')
    current_fd = workspace_fd

    // Walk all intermediate directories
    for i in 0..components.len()-1:
        dir_name = components[i]
        // Try to open existing directory, no-follow
        match openat(current_fd, dir_name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW):
            Ok(fd) => current_fd = fd
            Err(ENOENT) =>
                // Directory doesn't exist: create it, no-follow
                mkdirat(current_fd, dir_name, 0o755)?
                current_fd = openat(current_fd, dir_name, O_RDONLY | O_DIRECTORY | O_NOFOLLOW)?
            Err(ELOOP) => panic("symlink detected at intermediate component")
            Err(e) => return Err(e)

    // Open final component for writing, no-follow
    filename = components.last()
    openat(current_fd, filename, O_WRONLY | O_CREAT | O_TRUNC | O_NOFOLLOW, 0o644)
    write(content)
```

**Key properties:**
- Each step operates relative to the parent directory's file descriptor.
- `O_NOFOLLOW` refuses symlinks at every component.
- `mkdirat` + `openat(O_NOFOLLOW)` creates a new directory and immediately re-opens it
  without following — if an adversary replaces it between mkdir and open, the open fails
  with `ELOOP`.
- The workspace root fd is obtained once via `open(workspace, O_RDONLY | O_DIRECTORY)`.
  Its validity is the trust anchor.

**Rust APIs:**
- `std::os::unix::io::AsRawFd` / `FromRawFd` for fd manipulation.
- `libc::openat`, `libc::mkdirat`, `libc::fstat` for syscalls.
- No external crate needed — direct `libc` calls (already transitive dependency).

**Error on symlink detection:** `ELOOP` from `openat(..., O_NOFOLLOW)` when the
component is a symlink. This is the positive detection signal.

### 3.2 Windows

**Primitive:** `CreateFileW` with `FILE_FLAG_NO_REPARSE_POINT` extended to directory
traversal, or handle-relative operations via `NtCreateFile` / `CreateFileW` with
relative path syntax.

**Algorithm:**

```
fn safe_create_and_write(workspace_handle, relative_path, content):
    components = split(relative_path, '\')
    current_handle = workspace_handle

    for i in 0..components.len()-1:
        dir_name = components[i]
        // Open directory relative to parent handle, no-reparse
        match CreateFileW(
            current_handle, dir_name,
            GENERIC_READ, FILE_SHARE_READ,
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_NO_REPARSE_POINT  // no-follow on dirs
        ):
            Ok(handle) => current_handle = handle
            Err(ERROR_PATH_NOT_FOUND) =>
                // Create directory
                CreateDirectoryW(current_handle, dir_name, ...)
                current_handle = CreateFileW(
                    current_handle, dir_name,
                    GENERIC_READ, FILE_SHARE_READ,
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_NO_REPARSE_POINT
                )
            Err(...) => return Err

    // Write final component
    filename = components.last()
    CreateFileW(
        current_handle, filename,
        GENERIC_WRITE, 0,
        CREATE_ALWAYS,
        FILE_FLAG_NO_REPARSE_POINT  // already works for files
    )
    WriteFile(handle, content)
```

**Key properties:**
- `FILE_FLAG_NO_REPARSE_POINT` on directory opens prevents following reparse points.
- `FILE_FLAG_BACKUP_SEMANTICS` is required to open directories via `CreateFileW`.
- Handle-relative paths: Windows supports `\\?\` prefix for extended-length paths, and
  `CreateFileW` can accept relative paths when the calling thread's working directory is
  set — but this is not safe for concurrent operation. Instead, we use `NtCreateFile`
  with `RootDirectory` handle, or compose paths as `\\?\GLOBALROOT\Handle\0xNNNN\child`.

**Complication:** Windows does not have a direct equivalent of `openat()` — there's no
standard "open relative to directory handle" in the Win32 API. Options:

| Approach | Pros | Cons |
|----------|------|------|
| **A. `NtCreateFile` with `RootDirectory`** | Direct handle-relative opens | Undocumented/NT API, requires `ntapi` or raw bindings |
| **B. Compose `\\?\` path with handle** | Pure Win32 | Path composition is fragile, no standard "reparse-point-on-intermediate" flag |
| **C. Re-canonicalize each component** | Simple | Still has micro-race per component (but much smaller window) |
| **D. `CreateFileW` + `FILE_FLAG_NO_REPARSE_POINT` on each dir** | Best Win32 approach | Need to use `SetCurrentDirectory` per-handle or compose extended paths |

**Recommended approach for Windows:** Use `CreateFileW` with extended-length path syntax
(`\\?\` prefix) and `FILE_FLAG_NO_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS` to open
each intermediate directory. For handle-relative composition, use the
`\\?\GLOBALROOT\Handle\0xNNNN\child` syntax which allows opening a child path relative
to a kernel object handle.

**Alternative (simpler Windows approach):** Walk the path component-by-component, using
`CreateFileW` with full absolute path but verifying each intermediate is not a reparse
point via `GetFileAttributesW` + `FILE_ATTRIBUTE_REPARSE_POINT` check before opening the
next level. This re-validates at each step but doesn't require NT API.

---

## 4. Proposed API

```rust
/// Workspace write handle — obtained once per write operation, then used for
/// all path resolution within that write. Eliminates TOCTOU by operating
/// relative to verified directory handles.
pub struct WorkspaceWriteHandle {
    /// On Unix: file descriptor of the workspace root.
    #[cfg(unix)]
    root_fd: RawFd,
    /// On Windows: handle to the workspace root directory.
    #[cfg(windows)]
    root_handle: windows::Win32::Foundation::HANDLE,
}

impl WorkspaceWriteHandle {
    /// Open the workspace root for handle-relative operations.
    /// Validates the root once, then all operations are relative.
    pub fn open(workspace: &Path) -> Result<Self, PathContainmentError>;

    /// Create intermediate directories (no-follow at each step) and write
    /// the file at the given relative path. Content is written atomically.
    pub fn create_and_write(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), WriteSafeError>;

    /// Write to an existing file (for patch operations).
    /// Re-validates the file is within workspace via handle-relative traversal.
    pub fn overwrite_existing(
        &self,
        relative_path: &str,
        content: &str,
    ) -> Result<(), WriteSafeError>;
}

/// Error type for safe write operations.
pub enum WriteSafeError {
    /// A path component is a symlink — TOCTOU race detected or symlink escape attempted.
    SymlinkDetected { component: String },
    /// IO error (permission denied, disk full, etc.)
    Io(std::io::Error),
    /// Path validation failed (before handle-relative ops).
    Validation(PathContainmentError),
}
```

---

## 5. Integration Plan

### Phase 1: `WorkspaceWriteHandle` in `sandbox.rs`

Add the new struct and `create_and_write()` method to `sandbox.rs`. Existing
`resolve_workspace_path()` remains for reads and validation. Write path changes
to use the new handle.

### Phase 2: Update `file_write_handler`

Replace the current `create_dir_all` + `write_file_no_follow` sequence with:

```rust
let handle = WorkspaceWriteHandle::open(workspace)?;
handle.create_and_write(relative_path, content)?;
```

### Phase 3: Update `file_patch_apply`

Replace `tokio::fs::write(abs_path, ...)` with `handle.overwrite_existing(...)`.

### Phase 4: Update rollback writes

Rollback writes to `.openwand/rollback/` use the same handle since they're within
the workspace.

### Phase 5: Synchronous API

The handle-relative operations use synchronous I/O (`openat` / `CreateFileW`). This
is intentional — the operations are fast (single syscall per component) and the
current async write path wraps sync I/O anyway. Wrap in `tokio::task::spawn_blocking`
if needed to avoid blocking the tokio runtime.

---

## 6. Dependency Impact

| Dependency | Needed? | Notes |
|------------|---------|-------|
| `libc` | Unix only | Already transitive; direct for `openat`, `mkdirat` |
| `windows-sys` or `windows` | Windows only | Already transitive via tokio; direct for `CreateFileW` |
| `cap-std` or `openat` | **No** | Could use, but direct syscalls give more control and avoid new deps |
| `nix` | **No** | Same rationale — `libc` is sufficient |

**Net new dependencies:** None. All required primitives are already transitive.

---

## 7. Fallback Behavior

### Platforms without handle-relative support

If `openat`/`NtCreateFile` is unavailable (WASI, embedded), fall back to the current
behavior: `resolve_workspace_path()` + `write_file_no_follow()`. This is the same
protection level as 72B.

```rust
#[cfg(not(any(unix, windows)))]
impl WorkspaceWriteHandle {
    pub fn create_and_write(&self, path: &str, content: &str) -> Result<(), WriteSafeError> {
        // Fallback: current behavior (final-component protection only)
        let resolved = resolve_workspace_path(&self.workspace, path, PathAccessMode::WriteTarget)?;
        write_file_no_follow(&resolved, content).await
    }
}
```

### Platform support tiers

| Tier | Platform | Strategy |
|------|----------|----------|
| Tier 1 | Linux (x86_64, aarch64) | `openat` + `O_NOFOLLOW` — full hardening |
| Tier 1 | macOS (aarch64, x86_64) | `openat` + `O_NOFOLLOW` — full hardening |
| Tier 1 | Windows (x86_64) | `CreateFileW` + `FILE_FLAG_NO_REPARSE_POINT` — full hardening |
| Tier 2 | FreeBSD, other Unix | `openat` + `O_NOFOLLOW` — likely works, needs testing |
| Tier 3 | WASI, embedded | Fallback to current behavior |

---

## 8. Test Strategy

### Unit tests (no filesystem adversary)

| Test | What it proves |
|------|----------------|
| `handle_opens_workspace_root` | `WorkspaceWriteHandle::open()` succeeds on valid workspace |
| `create_and_write_creates_file` | Single-component path: writes correctly |
| `create_and_write_creates_intermediate_dirs` | Multi-component path: directories created, file written |
| `create_and_write_refuses_symlink_at_intermediate` | Symlink as directory component → `SymlinkDetected` error |
| `overwrite_existing_writes_to_file` | Patch-style overwrite works |
| `overwrite_existing_refuses_symlink_at_intermediate` | Symlink in path → error |
| `handle_rejects_absolute_path` | Same validation as current `resolve_workspace_path` |
| `handle_rejects_parent_traversal` | Same validation as current |
| `handle_rejects_escaping_symlink` | Symlink target outside workspace → error |

### TOCTOU race test (with simulated adversary)

This is the critical test. It requires a concurrent thread that replaces a directory
with a symlink between validation and write.

```rust
#[test]
fn toctou_race_on_intermediate_dir_is_blocked() {
    let ws = tempfile::tempdir().unwrap();
    fs::create_dir_all(ws.path().join("src/deep")).unwrap();

    let handle = WorkspaceWriteHandle::open(ws.path()).unwrap();

    // Spawn adversary thread that races to replace "src/deep" with a symlink
    let adversary_path = ws.path().join("src/deep");
    let outside = tempfile::tempdir().unwrap();
    let adversary = thread::spawn(move || {
        for _ in 0..1000 {
            let _ = fs::remove_dir_all(&adversary_path);
            let _ = symlink(&outside.path(), &adversary_path);
            let _ = fs::remove_dir(&adversary_path);
            let _ = fs::create_dir_all(&adversary_path);
        }
    });

    // Attempt write — should either succeed (race not won) or fail with SymlinkDetected
    let result = handle.create_and_write("src/deep/target.txt", "safe");
    match result {
        Ok(()) => {
            // Race not won by adversary — verify file is actually inside workspace
            let canonical = ws.path().join("src/deep/target.txt").canonicalize().unwrap();
            let canonical_ws = ws.path().canonicalize().unwrap();
            assert!(canonical.starts_with(&canonical_ws), "File escaped workspace!");
        }
        Err(WriteSafeError::SymlinkDetected { .. }) => {
            // Race detected and blocked — correct behavior
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    adversary.join().unwrap();
}
```

### Integration tests

- Production-path approval E2E test updated to verify new write path.
- Real-provider validation test remains unchanged (exercises sandbox).

---

## 9. Release Implications

### If implemented (73B)

- DEFERRED-008 status changes from "partially closed" to "closed".
- Intermediate-directory TOCTOU race is fully mitigated on Tier 1 platforms.
- Fallback behavior documented for Tier 3 platforms.
- Binary size impact: negligible (< 10 KB for the new code).
- No new external dependencies.

### If not implemented

- DEFERRED-008 remains as accepted residual.
- Documentation explicitly states: "OpenWand assumes a non-adversarial local filesystem
  for intermediate directory components. Final-component protection is provided."
- Final release proceeds with this accepted limitation.

### Honest scope statement

```
Intermediate-directory TOCTOU hardening eliminates the race where a local
concurrent adversary replaces an intermediate directory with a symlink between
path validation and file write. It operates on Tier 1 platforms (Linux, macOS,
Windows) using handle-relative directory traversal with no-follow semantics at
every path component. It does not protect against kernel-level compromise,
adversarial filesystem drivers, or platforms without openat/CreateFileW support.
```

---

## 10. Implementation Cost Estimate

| Phase | Effort | Risk |
|-------|--------|------|
| Unix `WorkspaceWriteHandle` (openat-based) | Medium | Low — well-understood API |
| Windows `WorkspaceWriteHandle` (CreateFileW-based) | Medium-High | Medium — handle-relative path composition |
| Integration into `file_write_handler` | Low | Low — drop-in replacement |
| Integration into `file_patch_apply` | Low | Low |
| Unit tests | Medium | Low |
| Race test (adversary simulation) | Medium | Medium — timing-dependent, may be flaky |
| Fallback for unsupported platforms | Low | Low |

**Total estimate:** 1-2 waves (73B + possibly 73C for Windows polish).

---

## 11. Decision Matrix

| Factor | Implement (73B) | Defer to post-release |
|--------|-----------------|----------------------|
| Security improvement | Full TOCTOU closure | Residual remains |
| Implementation risk | Medium (Windows complexity) | None |
| Time to final release | +1-2 waves | Immediate |
| Dependency cost | Zero new deps | N/A |
| Test complexity | Race test may be flaky | N/A |

**Recommendation:** Implement. The Unix path is straightforward (openat is well-tested in
production systems). The Windows path requires more care but uses standard Win32 APIs.
The race test is the highest-risk item — if it proves unstable, accept it as a manual
verification test rather than CI.

---

*This document is a design and feasibility assessment. It does not claim the race is
closed. Implementation and tests are required before that claim can be made. No new
feature behavior, authority, policy change, prompt change, or final-release claim.*
