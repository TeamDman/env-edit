use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use windows::core::PCWSTR;
use windows::core::w;

pub const MACHINE_ENV_SUB_KEY: PCWSTR =
    w!("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment");

/// Converts a Rust `&str` to a null-terminated wide string (`Vec<u16>`).
pub fn to_wide_null(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(once(0)) // Append null terminator
        .collect()
}

/// Convert pairs of bytes to UTF-16, then to a Rust string. Stop at the first null terminator.
pub fn utf16_from_bytes(bytes: &[u8]) -> String {
    let wide_data: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let str_end = wide_data
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(wide_data.len());
    String::from_utf16_lossy(&wide_data[..str_end])
}
