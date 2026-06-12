# Windows NtCreateFile Implementation — API Surface Audit

**Date:** 2026-06-13
**Wave:** 78B (partial)
**Status:** API audit complete, implementation not yet merged

---

## API Surface Findings

### Dependency

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Wdk",
    "Wdk_Storage_FileSystem",
    "Wdk_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_Foundation",
    "Win32_System_IO",
    "Win32_System_SystemServices",
    "Win32_System_WindowsProgramming",
    "Win32_Security",  # Required for OBJECT_ATTRIBUTES::default()
] }
```

The `windows` 0.61.3 crate is already a transitive dependency. Adding it as a
direct dep to `openwand-tools` with the above features compiles successfully
(tested: `cargo check -p openwand-tools` passes with the feature flags).

### Type Mapping Issues

The `windows` crate uses **newtype wrappers**, not bitflags. Key findings:

| Windows API Concept | Rust Type | Notes |
|---------------------|-----------|-------|
| `HANDLE` | `windows::Win32::Foundation::HANDLE` | Wrapper struct, `.0` is `isize` |
| `NTSTATUS` | `windows::Win32::Foundation::NTSTATUS` | `.is_success()` method available |
| `OBJECT_ATTRIBUTES` | `windows::Wdk::Foundation::OBJECT_ATTRIBUTES` | `Default` impl requires `Win32_Security` feature |
| `UNICODE_STRING` | `windows::Win32::Foundation::UNICODE_STRING` | Raw struct, init via `RtlInitUnicodeString` |
| `IO_STATUS_BLOCK` | `windows::Win32::System::IO::IO_STATUS_BLOCK` | Zeroed with `mem::zeroed()` |
| `FILE_ACCESS_RIGHTS` | Newtype `struct(u32)` | Not bitflags — use bitwise OR on `.0` |
| `FILE_SHARE_MODE` | Newtype `struct(u32)` | Same |
| `FILE_FLAGS_AND_ATTRIBUTES` | Newtype `struct(u32)` | Constants like `FILE_ATTRIBUTE_REPARSE_POINT` are this type |
| `NTCREATEFILE_CREATE_DISPOSITION` | Newtype `struct(u32)` | `FILE_OPEN=1`, `FILE_CREATE=2`, `FILE_OPEN_IF=3` |
| `NTCREATEFILE_CREATE_OPTIONS` | Newtype `struct(u32)` | `FILE_DIRECTORY_FILE`, `FILE_OPEN_REPARSE_POINT` etc. |

### OBJECT_ATTRIBUTES Initialization

```rust
// Requires Win32_Security feature for Default impl
let mut obj_attr = OBJECT_ATTRIBUTES::default();
obj_attr.Length = std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32;
obj_attr.RootDirectory = parent_handle;
// ObjectName is *const UNICODE_STRING (raw pointer, not ManuallyDrop)
obj_attr.ObjectName = &obj_name;
```

### Reparse Point Detection

```rust
// FILE_ATTRIBUTE_REPARSE_POINT is FILE_FLAGS_AND_ATTRIBUTES(1024u32)
// Not a bitflags type — use bitwise AND:
let is_reparse = file_info.FileAttributes.0 & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0;
```

### CreateFileW for Root Handle

```rust
// CreateFileW returns Result<HANDLE> in the windows crate (not HANDLE directly)
let root_handle = unsafe {
    CreateFileW(
        &HSTRING::from(path.as_os_str()),
        FILE_ACCESS_RIGHTS(FILE_GENERIC_READ.0),
        FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0),
        None,
        OPEN_EXISTING,
        FILE_FLAGS_AND_ATTRIBUTES(FILE_FLAG_BACKUP_SEMANTICS.0),
        None,
    )
}.map_err(|e| ...)?;
```

### WriteFile

```rust
// WriteFile returns Result<()> — bytes written via OVERLAPPED
let result = unsafe {
    WriteFile(handle, Some(buffer), Some(&mut bytes_written), None)
};
```

---

## Implementation Complexity Assessment

| Aspect | Difficulty | Notes |
|--------|:---------:|-------|
| Feature flag setup | Low | Tested, compiles |
| OBJECT_ATTRIBUTES init | Medium | Needs `Win32_Security`, raw pointer handling |
| NtCreateFile call | Medium | 12 parameters, newtype wrapping |
| Reparse point detection | Low | `GetFileInformationByHandleEx` with `FileAttributeTagInfo` |
| Handle lifecycle | Medium | Must close all handles, including on error paths |
| WriteFile via handle | Low | Standard Win32 write |
| Error mapping | Medium | NTSTATUS → io::Error mapping needed |
| Directory creation | Medium | NtCreateFile with FILE_CREATE + re-verify |

**Estimated implementation:** 300–400 lines of `unsafe` Rust, replacing ~80 lines of current 73C code.

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|:--------:|------------|
| NT API instability | Very Low | NtCreateFile stable since NT 4.0 |
| windows crate type confusion | Medium | Careful newtype handling, compile-time checks |
| Handle leaks on error paths | Medium | RAII wrapper or explicit close in all paths |
| Feature flag proliferation | Low | Single `windows` dep with feature list |
| Test coverage | Medium | Need symlink creation tests (may require admin) |

---

## Recommended Implementation Order

1. Add `windows` dependency with all features to `openwand-tools`
2. Create helper module `sandbox_ntapi.rs` with:
   - `nt_open_dir_relative()` — open directory relative to parent handle
   - `nt_open_file_for_write()` — open file for write relative to parent handle
   - `write_to_handle()` — write content to handle
3. Replace `windows_create_and_write()` to use the new helpers
4. Keep `windows_overwrite_existing()` as delegation
5. Add tests for:
   - Basic file write (existing test should pass)
   - Intermediate directory creation
   - Reparse point rejection (requires symlink creation, may be admin-only)
6. Verify all 111 existing tests still pass

---

## Outcome for Wave 78B

This wave produced a **detailed API audit** confirming NtCreateFile is viable
and documenting the exact type mapping required. The implementation was not
completed because the `windows` crate's type system (newtypes, raw pointers,
feature-gated Default impls) requires careful handling that should not be
rushed in a security-critical path.

The correct next step is a dedicated implementation wave with the type
mapping above as reference.
