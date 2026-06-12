# Wave 76B — Windows TOCTOU Residual Hardening Feasibility

**Date:** 2026-06-12
**Status:** Feasibility assessment
**Residual:** DEFERRED-008 (Windows per-component micro-race)

---

## 1. Current State

The Windows `WorkspaceWriteHandle.windows_create_and_write()` (73C) walks each
intermediate path component and checks `symlink_metadata()` for reparse point
status. Directories are created with `create_dir()` and re-verified.

**Remaining gap:** Between `symlink_metadata()` and the subsequent `create_dir()`
or `write_file_no_follow()` call, an adversary could replace a component with a
symlink. This is a per-component TOCTOU micro-race.

---

## 2. API Investigation

### Option A: NtCreateFile with RootDirectory (RECOMMENDED)

**API:** `NtCreateFile` from `ntdll.dll` (undocumented/NT API, but stable since
Windows 2000 and used by Chrome, Firefox, Rust's own `std::fs`, and most security
tools).

**Key features:**
- `OBJECT_ATTRIBUTES.RootDirectory`: handle to a directory. `ObjectName` is then
  interpreted **relative to that handle**, not relative to the path string.
- `FILE_OPEN_REPARSE_POINT` in `CreateOptions`: bypasses normal reparse point
  processing — opens the reparse point itself rather than following it.
- `FILE_DIRECTORY_FILE`: opens only directories (fails on files).
- Can create directories via `FILE_CREATE` disposition.

**How it closes the race:**
1. Open workspace root as a directory handle via `CreateFileW(FILE_FLAG_BACKUP_SEMANTICS)`.
2. For each intermediate component: `NtCreateFile(RootDirectory=parent_handle, ObjectName="child", CreateOptions=FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT)`.
3. If `FILE_OPEN_REPARSE_POINT` succeeds but the handle refers to a reparse point,
   detect via `GetFileAttributesByHandle` or `FILE_ATTRIBUTE_REPARSE_POINT` on the
   returned handle.
4. If directory doesn't exist: `NtCreateFile(..., FILE_CREATE, FILE_DIRECTORY_FILE)`,
   then immediately re-open with `FILE_OPEN_REPARSE_POINT` to verify it wasn't
   replaced.
5. Open final component with `NtCreateFile(RootDirectory=parent_handle, ...)`
   using `FILE_OPEN_REPARSE_POINT` or `FILE_FLAG_NO_REPARSE_POINT` equivalent.

**Dependency:** Requires `windows-sys` crate with `Wdk_Storage_FileSystem` or
manual `ntdll.dll` binding via `GetProcAddress`. The `windows-sys` 0.61.2 crate
(already transitive) has `NtCreateFile` under the `Wdk_Storage_FileSystem` feature.

**Unsafe code:** Required. `NtCreateFile` is an FFI call with raw pointers.
But the current 73B Unix code already uses `unsafe` for `libc::openat` etc.
Same safety model: encapsulated in `WorkspaceWriteHandle` methods, validated by
tests.

### Option B: CreateFileW with FILE_FLAG_NO_REPARSE_POINT on directories

**API:** Standard Win32 `CreateFileW` with `FILE_FLAG_NO_REPARSE_POINT` and
`FILE_FLAG_BACKUP_SEMANTICS`.

**Problem:** `FILE_FLAG_NO_REPARSE_POINT` only affects the **final component** of
the path. If intermediate components are symlinks, `CreateFileW` follows them
before applying the flag to the last component. This is the same limitation we
already have with the current 72B approach.

**Verdict:** Does NOT close the intermediate-directory race. Not viable for this
purpose.

### Option C: OpenFileById

**API:** `OpenFileById` opens a file by its file ID (inode equivalent).

**Problem:** Requires knowing the file ID in advance. We'd need to enumerate
directory entries to get the file ID for each component, which still requires
opening the directory first — creating a chicken-and-egg problem for new
directories.

**Verdict:** Not viable for directory creation.

### Option D: Per-component identity verification via GetFileInformationByHandle

**API:** After opening each directory, call `GetFileInformationByHandle` to get
the file index / volume serial number. On subsequent opens, verify the identity
matches.

**Problem:** This is a detection strategy, not a prevention strategy. The race
still exists between open and verify.

**Verdict:** Strengthens 73C but does NOT fully close the race.

---

## 3. Recommendation

**Option A (NtCreateFile with RootDirectory)** is the only approach that fully
closes the Windows intermediate-directory TOCTOU race.

### Implementation plan

| Phase | Description | Effort |
|-------|-------------|--------|
| 1 | Add `windows-sys` feature flag for `Wdk_Storage_FileSystem` to `openwand-tools` | Low |
| 2 | Implement `windows_create_and_write_ntapi()` using `NtCreateFile` with `RootDirectory` | Medium |
| 3 | Handle directory creation via `NtCreateFile` with `FILE_CREATE` disposition | Medium |
| 4 | Add reparse point detection on opened handles | Low |
| 5 | Fallback to 73C behavior if `NtCreateFile` unavailable (unlikely) | Low |
| 6 | Add adversarial symlink race test (Windows) | Medium |
| 7 | Update DEFERRED-008 status | Low |

**Total estimate:** 1 wave (76C).

### Risk assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| `NtCreateFile` is undocumented | Medium | Used by Chrome, Firefox, Rust std. Stable since Win2K. |
| `windows-sys` feature adds compile time | Low | Feature-gated behind `cfg(windows)` |
| FFI unsafe code | Medium | Same model as Unix `libc` calls. Encapsulated + tested. |
| `ntdll.dll` not available | Very Low | Present on all Windows since NT 4.0. |
| Test requires symlink creation (admin) | Medium | Skip gracefully like 73C tests. |

---

## 4. Alternative: Accept as Documented Residual

If the implementation cost is deemed too high for alpha/beta:

- Keep 73C per-component hardening (substantially hardened).
- Document that Windows full closure requires NT API (`NtCreateFile`).
- Target for v0.2.0 or post-beta.

This is acceptable because:
- The micro-race requires local concurrent filesystem access.
- The 73C hardening already shrinks the race window by orders of magnitude.
- No model-driven or network-accessible attack vector exploits this.

---

## 5. Decision

| Option | Description | Cost | Risk closure |
|--------|-------------|------|-------------|
| **Implement (76C)** | `NtCreateFile` + `RootDirectory` + `FILE_OPEN_REPARSE_POINT` | 1 wave | Full closure on Windows |
| **Accept residual** | Keep 73C hardening, document NT API path | 0 waves | Reduced residual |

**Recommendation:** Accept as documented residual for v0.1.0-beta. Schedule
`NtCreateFile` implementation for v0.2.0 cycle. The 73C hardening provides
strong practical protection, and the NT API work requires more testing and
review than a post-alpha stabilization arc should invest.

---

*This document records the feasibility of closing the Windows TOCTOU micro-race.
It does not claim the race is closed. Implementation requires a separate wave
with tests proving closure.*
