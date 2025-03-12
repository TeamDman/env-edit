use eyre::Result;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows::Win32::System::Registry::KEY_SET_VALUE;
use windows::Win32::System::Registry::REG_SZ;
use windows::Win32::System::Registry::RegCloseKey;
use windows::Win32::System::Registry::RegOpenKeyExW;
use windows::Win32::System::Registry::RegSetValueExW;
use windows::core::*;

use crate::win_strings::MACHINE_ENV_SUB_KEY;

/// Registry path for machine-level environment variables

/// Create or update a machine-level environment variable to the given string value (REG_SZ).
///
/// * `var_name` = the name of the variable, e.g. "ENV_EDIT_TEST"
/// * `value` = the new string value
pub fn set_machine_env_var(var_name: &str, value: &str) -> Result<()> {
    // Convert name and value to wide strings
    let wide_name: Vec<u16> = var_name.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_val: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        // Open the registry key with KEY_SET_VALUE
        let mut hkey: HKEY = HKEY::default();
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            MACHINE_ENV_SUB_KEY,
            None,
            KEY_SET_VALUE,
            &mut hkey,
        )
        .ok()?;

        // We set it as REG_SZ (typical for normal variables).
        // If you want expansions (e.g. %SystemRoot%), you could use REG_EXPAND_SZ instead.
        let data = wide_val.align_to::<u8>().1;
        let set_result = RegSetValueExW(
            hkey,
            PCWSTR(wide_name.as_ptr()),
            Some(0),
            REG_SZ,
            Some(data),
        )
        .ok();

        // want to close even if set failed
        RegCloseKey(hkey).ok()?;

        set_result?;
    }

    broadcast_changes()?;

    Ok(())
}

pub fn broadcast_changes() -> eyre::Result<()> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::WindowsAndMessaging::HWND_BROADCAST;
    use windows::Win32::UI::WindowsAndMessaging::SMTO_ABORTIFHUNG;
    use windows::Win32::UI::WindowsAndMessaging::SendMessageTimeoutW;
    use windows::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE;

    unsafe {
        let mut result = 0;
        let lparam = LPARAM(w!("Environment").as_ptr() as _);
        SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            lparam,
            SMTO_ABORTIFHUNG,
            5000,
            Some(&mut result),
        );
    }
    Ok(())
}
