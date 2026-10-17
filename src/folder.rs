#[cfg(windows)]
#[cfg_attr(coverage_nightly, coverage(off))]
pub fn open_directory(directory: &std::path::Path) -> Result<(), String> {
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    open_directory_with(directory, |path| {
        // SAFETY: The path is NUL-terminated and remains alive throughout the call.
        // The remaining string arguments are static literals or null pointers.
        unsafe {
            ShellExecuteW(
                HWND::default(),
                w!("open"),
                PCWSTR(path.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
            .0
        }
    })
}

#[cfg(windows)]
fn open_directory_with(
    directory: &std::path::Path,
    execute: impl FnOnce(&[u16]) -> isize,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let path = null_terminated_path(directory.as_os_str().encode_wide())?;
    shell_execute_result(execute(&path))
}

fn null_terminated_path(path: impl Iterator<Item = u16>) -> Result<Vec<u16>, String> {
    let mut path: Vec<u16> = path.collect();
    if path.contains(&0) {
        return Err("Directory path contains a NUL character".to_string());
    }
    path.push(0);
    Ok(path)
}

fn shell_execute_result(result: isize) -> Result<(), String> {
    // ShellExecuteW returns an integer status in HINSTANCE, not a handle.
    if result > 32 {
        return Ok(());
    }
    Err(format!(
        "Failed to open directory with ShellExecuteW (error code: {result})"
    ))
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn directory_path_preserves_unicode_and_spaces() {
        let path = "C:\\My Screenshots\\日本語 📷";
        let encoded = null_terminated_path(path.encode_utf16()).unwrap();

        assert_eq!(encoded.last(), Some(&0));
        assert_eq!(
            String::from_utf16(&encoded[..encoded.len() - 1]).unwrap(),
            path
        );
    }

    #[test]
    fn directory_path_preserves_unpaired_surrogates() {
        assert_eq!(
            null_terminated_path([b'C' as u16, b':' as u16, 0xd800].into_iter()).unwrap(),
            vec![b'C' as u16, b':' as u16, 0xd800, 0]
        );
    }

    #[test]
    fn directory_path_rejects_interior_nul() {
        assert!(null_terminated_path("C:\\safe\0\\other".encode_utf16()).is_err());
    }

    #[test]
    fn shell_execute_success_requires_a_value_above_32() {
        for result in [33, 42, isize::MAX] {
            assert_eq!(shell_execute_result(result), Ok(()));
        }
    }

    #[test]
    fn shell_execute_errors_include_the_native_code() {
        for result in [0, 2, 5, 32] {
            let error = shell_execute_result(result).unwrap_err();
            assert!(error.contains(&result.to_string()), "{error}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn open_directory_with_validates_and_dispatches_null_terminated_path() {
        let execute = |wide_path: &[u16]| {
            assert_eq!(wide_path.last(), Some(&0));
            42
        };
        let dir = std::path::Path::new("C:\\Screenshots");
        assert_eq!(open_directory_with(dir, execute), Ok(()));

        let invalid = std::path::Path::new("C:\\Screenshots\0bad");
        assert!(open_directory_with(invalid, execute).is_err());
    }
}
