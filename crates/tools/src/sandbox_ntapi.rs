//! Windows NtCreateFile handle-relative file write helpers.
//!
//! Implements the Windows analogue of Unix `openat` + `O_NOFOLLOW` using
//! `NtCreateFile` with `OBJECT_ATTRIBUTES.RootDirectory` for handle-relative
//! path traversal and `FILE_OPEN_REPARSE_POINT` to open reparse points
//! without following them.
//!
//! # Safety
//!
//! All functions in this module are `unsafe` because they call Windows NT API
//! FFI functions. The following invariants must be maintained:
//!
//! - All HANDLEs returned must be closed via `CloseHandle` on all code paths,
//!   including error paths.
//! - `OBJECT_ATTRIBUTES.RootDirectory` must be a valid directory handle.
//! - `OBJECT_ATTRIBUTES.ObjectName` must point to a valid `UNICODE_STRING`
//!   whose buffer outlives the `NtCreateFile` call.
//! - Caller must not use a HANDLE after closing it.

// This module is entirely unsafe NT API internals. Allow unsafe operations
// inside unsafe functions without requiring extra `unsafe {}` blocks.
#![allow(unsafe_op_in_unsafe_fn)]

#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, HANDLE};

/// RAII guard for a Windows HANDLE. Calls `CloseHandle` on drop.
#[cfg(windows)]
pub(crate) struct HandleGuard {
    handle: HANDLE,
    owned: bool,
}

#[cfg(windows)]
impl HandleGuard {
    /// Create a guard that will close the handle on drop.
    pub fn new(handle: HANDLE) -> Self {
        Self { handle, owned: true }
    }

    /// Create a guard that does NOT close the handle on drop (for borrowed handles).
    pub fn borrowed(handle: HANDLE) -> Self {
        Self { handle, owned: false }
    }

    /// Get the raw handle value.
    pub fn get(&self) -> HANDLE {
        self.handle
    }

    /// Release ownership — the handle will NOT be closed on drop.
    /// Returns the raw handle.
    pub fn release(mut self) -> HANDLE {
        self.owned = false;
        self.handle
    }
}

#[cfg(windows)]
impl Drop for HandleGuard {
    fn drop(&mut self) {
        if self.owned && !self.handle.is_invalid() {
            let _ = unsafe { CloseHandle(self.handle) };
        }
    }
}

/// NTSTATUS codes for common error conditions.
#[cfg(windows)]
mod ntstatus {
    /// STATUS_OBJECT_NAME_NOT_FOUND (0xC0000034)
    pub const OBJECT_NAME_NOT_FOUND: i32 = 0xC000_0034_u32 as i32;
    /// STATUS_OBJECT_PATH_NOT_FOUND (0xC000003A)
    pub const OBJECT_PATH_NOT_FOUND: i32 = 0xC000_003A_u32 as i32;
}

/// Build an `OBJECT_ATTRIBUTES` for a handle-relative open.
///
/// # Safety
///
/// The `parent` handle must be a valid directory handle.
/// The `component` string must outlive the returned `OBJECT_ATTRIBUTES`.
#[cfg(windows)]
unsafe fn build_obj_attr(
    parent: HANDLE,
    component: &std::ffi::OsStr,
    obj_name: &mut windows::Win32::Foundation::UNICODE_STRING,
) -> windows::Wdk::Foundation::OBJECT_ATTRIBUTES {
    use windows::Wdk::Foundation::OBJECT_ATTRIBUTES;

    // Convert component to null-terminated UTF-16
    let wide: Vec<u16> = component
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // Initialize UNICODE_STRING via RtlInitUnicodeString
    unsafe {
        windows::Win32::System::WindowsProgramming::RtlInitUnicodeString(
            obj_name,
            windows::core::PCWSTR(wide.as_ptr()),
        );
    }

    let mut attr: OBJECT_ATTRIBUTES = std::mem::zeroed();
    attr.Length = std::mem::size_of::<OBJECT_ATTRIBUTES>() as u32;
    attr.RootDirectory = parent;
    attr.ObjectName = obj_name;

    attr
}

/// Check if a handle refers to a reparse point (symlink/junction).
///
/// Uses `NtQueryInformationFile(FileBasicInformation)` — the NT-native query
/// that requires `FILE_READ_ATTRIBUTES` on the handle (added to all opens).
///
/// Returns `true` if the handle's file has `FILE_ATTRIBUTE_REPARSE_POINT`.
/// Returns `true` on query failure (fail-closed: safe default).
///
/// # Safety
///
/// `handle` must be a valid file/directory handle opened with
/// `FILE_READ_ATTRIBUTES` in the access mask.
#[cfg(windows)]
unsafe fn is_reparse_point(handle: HANDLE) -> bool {
    use windows::Wdk::Storage::FileSystem::{
        NtQueryInformationFile, FILE_BASIC_INFORMATION,
        FileBasicInformation,
    };
    use windows::Win32::System::IO::IO_STATUS_BLOCK;

    let mut info: FILE_BASIC_INFORMATION = std::mem::zeroed();
    let mut iosb: IO_STATUS_BLOCK = std::mem::zeroed();

    let status = NtQueryInformationFile(
        handle,
        &mut iosb,
        &mut info as *mut _ as *mut _,
        std::mem::size_of::<FILE_BASIC_INFORMATION>() as u32,
        FileBasicInformation,
    );

    if status.0 < 0 {
        // Can't determine attributes — fail-closed: treat as potential reparse point.
        // This should not happen if FILE_READ_ATTRIBUTES is in the access mask.
        // Exposing NTSTATUS for diagnostics.
        eprintln!(
            "WARN: NtQueryInformationFile(FileBasicInformation) failed: NTSTATUS 0x{:08X}",
            status.0 as u32
        );
        return true;
    }

    // FILE_ATTRIBUTE_REPARSE_POINT = 0x400 = 1024
    info.FileAttributes & 0x400 != 0
}

/// Open an existing directory relative to a parent handle, without following
/// reparse points. Returns a handle to the directory.
///
/// If `create` is true, creates the directory if it doesn't exist, then
/// re-opens with `FILE_OPEN_REPARSE_POINT` to verify no race occurred.
///
/// # Safety
///
/// `parent` must be a valid directory handle.
#[cfg(windows)]
pub(crate) unsafe fn open_dir_relative(
    parent: HANDLE,
    component: &std::ffi::OsStr,
    create: bool,
) -> Result<HANDLE, crate::sandbox::WriteSafeError> {
    use crate::sandbox::WriteSafeError;
    use windows::Wdk::Storage::FileSystem::*;
    use windows::Win32::Storage::FileSystem::FILE_ACCESS_RIGHTS;

    let mut obj_name = windows::Win32::Foundation::UNICODE_STRING::default();
    let obj_attr = unsafe { build_obj_attr(parent, component, &mut obj_name) };

    let mut handle: HANDLE = HANDLE::default();
    let mut iosb: windows::Win32::System::IO::IO_STATUS_BLOCK = std::mem::zeroed();

    // Directory access: traverse + list + read-attributes + synchronize.
    // FILE_READ_ATTRIBUTES (0x80) is required for NtQueryInformationFile(FileBasicInformation).
    // Avoid FILE_READ_DATA / FILE_WRITE_DATA / FILE_APPEND_DATA / FILE_EXECUTE on directories.
    const DIR_ACCESS: u32 = 0x80 /* FILE_READ_ATTRIBUTES */
        | 0x20 /* FILE_TRAVERSE */
        | 0x01 /* FILE_LIST_DIRECTORY */
        | 0x100000 /* SYNCHRONIZE */;

    // Open existing with FILE_OPEN_REPARSE_POINT to not follow reparse points
    let open_status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_ACCESS_RIGHTS(DIR_ACCESS),
            &obj_attr,
            &mut iosb,
            None,
            windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY,
            windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(0),
            NTCREATEFILE_CREATE_DISPOSITION(1), // FILE_OPEN
            FILE_OPEN_REPARSE_POINT | FILE_DIRECTORY_FILE,
            None,
            0,
        )
    };

    if open_status.0 >= 0 {
        // Successfully opened — check if it's a reparse point
        if unsafe { is_reparse_point(handle) } {
            let _ = CloseHandle(handle);
            return Err(WriteSafeError::SymlinkDetected {
                component: component.to_string_lossy().into_owned(),
            });
        }
        return Ok(handle);
    }

    // Open failed — check if we should create
    let status_val = open_status.0;
    if !create
        || (status_val != ntstatus::OBJECT_NAME_NOT_FOUND
            && status_val != ntstatus::OBJECT_PATH_NOT_FOUND)
    {
        return Err(WriteSafeError::Io(std::io::Error::other(
            format!(
                "NtCreateFile open directory failed: NTSTATUS 0x{:08X}",
                status_val as u32
            ),
        )));
    }

    // Create the directory
    let mut new_handle: HANDLE = HANDLE::default();
    let mut new_iosb: windows::Win32::System::IO::IO_STATUS_BLOCK = std::mem::zeroed();

    // Re-build obj_attr (obj_name may have been consumed)
    let mut obj_name2 = windows::Win32::Foundation::UNICODE_STRING::default();
    let obj_attr2 = unsafe { build_obj_attr(parent, component, &mut obj_name2) };

    // Create directory: same minimal access as open (read-attributes for later verify)
    let create_status = unsafe {
        NtCreateFile(
            &mut new_handle,
            FILE_ACCESS_RIGHTS(DIR_ACCESS),
            &obj_attr2,
            &mut new_iosb,
            None,
            windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY,
            windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(0),
            NTCREATEFILE_CREATE_DISPOSITION(2), // FILE_CREATE
            FILE_DIRECTORY_FILE,
            None,
            0,
        )
    };

    if create_status.0 < 0 {
        return Err(WriteSafeError::Io(std::io::Error::other(
            format!(
                "NtCreateFile create directory failed: NTSTATUS 0x{:08X}",
                create_status.0 as u32
            ),
        )));
    }

    // Close the creation handle
    let _ = CloseHandle(new_handle);

    // Re-open with FILE_OPEN_REPARSE_POINT to verify no race
    let mut verify_handle: HANDLE = HANDLE::default();
    let mut verify_iosb: windows::Win32::System::IO::IO_STATUS_BLOCK = std::mem::zeroed();

    let mut obj_name3 = windows::Win32::Foundation::UNICODE_STRING::default();
    let obj_attr3 = unsafe { build_obj_attr(parent, component, &mut obj_name3) };

    let verify_status = unsafe {
        NtCreateFile(
            &mut verify_handle,
            FILE_ACCESS_RIGHTS(DIR_ACCESS),
            &obj_attr3,
            &mut verify_iosb,
            None,
            windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DIRECTORY,
            windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(0),
            NTCREATEFILE_CREATE_DISPOSITION(1), // FILE_OPEN
            FILE_OPEN_REPARSE_POINT | FILE_DIRECTORY_FILE,
            None,
            0,
        )
    };

    if verify_status.0 < 0 {
        return Err(WriteSafeError::Io(std::io::Error::other(
            format!(
                "NtCreateFile verify directory failed: NTSTATUS 0x{:08X}",
                verify_status.0 as u32
            ),
        )));
    }

    // Verify it's not a reparse point (race detection)
    if unsafe { is_reparse_point(verify_handle) } {
        let _ = CloseHandle(verify_handle);
        return Err(WriteSafeError::SymlinkDetected {
            component: component.to_string_lossy().into_owned(),
        });
    }

    Ok(verify_handle)
}

/// Open a file for writing relative to a parent directory handle.
/// Check that a file (final component) is not a reparse point.
/// Opens with FILE_READ_ATTRIBUTES only, checks, and closes.
/// Returns Ok(()) if clean, Err(SymlinkDetected) if reparse point.
///
/// # Safety
///
/// `parent` must be a valid directory handle.
#[cfg(windows)]
pub(crate) unsafe fn check_file_not_reparse(
    parent: HANDLE,
    filename: &std::ffi::OsStr,
) -> Result<(), crate::sandbox::WriteSafeError> {
    use crate::sandbox::WriteSafeError;
    use windows::Wdk::Storage::FileSystem::*;
    use windows::Win32::Storage::FileSystem::FILE_ACCESS_RIGHTS;

    let mut obj_name = windows::Win32::Foundation::UNICODE_STRING::default();
    let obj_attr = unsafe { build_obj_attr(parent, filename, &mut obj_name) };

    let mut handle: HANDLE = HANDLE::default();
    let mut iosb: windows::Win32::System::IO::IO_STATUS_BLOCK = std::mem::zeroed();

    // Open with FILE_READ_ATTRIBUTES + FILE_OPEN_REPARSE_POINT for reparse check
    let status = unsafe {
        NtCreateFile(
            &mut handle,
            FILE_ACCESS_RIGHTS(0x80), // FILE_READ_ATTRIBUTES
            &obj_attr,
            &mut iosb,
            None,
            windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL,
            windows::Win32::Storage::FileSystem::FILE_SHARE_MODE(
                windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0
                    | windows::Win32::Storage::FileSystem::FILE_SHARE_WRITE.0,
            ),
            NTCREATEFILE_CREATE_DISPOSITION(1), // FILE_OPEN
            FILE_OPEN_REPARSE_POINT | FILE_NON_DIRECTORY_FILE,
            None,
            0,
        )
    };

    if status.0 >= 0 {
        // File exists — check if it's a reparse point
        let is_reparse = unsafe { is_reparse_point(handle) };
        let _ = CloseHandle(handle);

        if is_reparse {
            return Err(WriteSafeError::SymlinkDetected {
                component: filename.to_string_lossy().into_owned(),
            });
        }
        return Ok(());
    }

    // File doesn't exist — new files can't be reparse points
    let not_found = status.0 == ntstatus::OBJECT_NAME_NOT_FOUND
        || status.0 == ntstatus::OBJECT_PATH_NOT_FOUND;
    if not_found {
        return Ok(());
    }

    // Some other error — report it
    Err(WriteSafeError::Io(std::io::Error::other(
        format!(
            "NtCreateFile reparse check failed for '{}': NTSTATUS 0x{:08X}",
            filename.to_string_lossy(),
            status.0 as u32
        ),
    )))
}

/// Open the workspace root directory as a handle using `CreateFileW`.
/// Uses `FILE_FLAG_BACKUP_SEMANTICS` to open directories.
#[cfg(windows)]
pub(crate) fn open_root_handle(
    workspace: &std::path::Path,
) -> Result<HANDLE, crate::sandbox::WriteSafeError> {
    use crate::sandbox::WriteSafeError;
    use windows::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_BACKUP_SEMANTICS,
        FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_SHARE_MODE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows::core::HSTRING;

    // FILE_GENERIC_READ includes FILE_READ_ATTRIBUTES (0x80) — sufficient for
    // passing to NtCreateFile child opens. Add SYNCHRONIZE for consistency.
    let desired_access = FILE_GENERIC_READ.0 | 0x100000; // SYNCHRONIZE

    let handle = unsafe {
        CreateFileW(
            &HSTRING::from(workspace.as_os_str()),
            desired_access,
            FILE_SHARE_MODE(FILE_SHARE_READ.0 | FILE_SHARE_WRITE.0),
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(FILE_FLAG_BACKUP_SEMANTICS.0),
            None,
        )
    }
    .map_err(|e| {
        WriteSafeError::Io(std::io::Error::other(
            format!("Failed to open workspace root handle: {}", e),
        ))
    })?;

    Ok(handle)
}
