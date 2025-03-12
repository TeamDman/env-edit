use crate::win_strings::utf16_from_bytes;
use crate::win_strings::MACHINE_ENV_SUB_KEY;
use eyre::bail;
use eyre::eyre;
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::Foundation::ERROR_MORE_DATA;
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows::Win32::System::Registry::KEY_QUERY_VALUE;
use windows::Win32::System::Registry::KEY_READ;
use windows::Win32::System::Registry::REG_EXPAND_SZ;
use windows::Win32::System::Registry::REG_SZ;
use windows::Win32::System::Registry::REG_VALUE_TYPE;
use windows::Win32::System::Registry::RegCloseKey;
use windows::Win32::System::Registry::RegEnumValueW;
use windows::Win32::System::Registry::RegOpenKeyExW;
use windows::Win32::System::Registry::RegQueryValueExW;
use windows::core::PCWSTR;
use windows::core::PWSTR;


pub fn list_machine_env_var() -> eyre::Result<Vec<EnvironmentVariable>> {
    let mut hkey: HKEY = HKEY::default();
    unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            MACHINE_ENV_SUB_KEY,
            None,
            KEY_READ,
            &mut hkey,
        )
        .ok()?;
    }

    let rtn = try {
        let mut index = 0;
        let mut rtn = Vec::new();
        loop {
            // We'll keep dynamic buffers that we can expand if needed
            let mut name_buf = vec![0u16; 256]; // 256 wide chars for the name
            let mut data_buf = vec![0u8; 1024]; // 1 KiB for data
            let mut name_len = name_buf.len() as u32; // length in wide chars for the name
            let mut data_len = data_buf.len() as u32; // length in bytes for the data
            let mut value_type = REG_VALUE_TYPE(0);

            let status = unsafe {
                RegEnumValueW(
                    hkey,
                    index,
                    Some(PWSTR(name_buf.as_mut_ptr())),
                    &mut name_len,
                    None,
                    Some(&mut value_type.0),
                    Some(data_buf.as_mut_ptr()),
                    Some(&mut data_len),
                )
            };
            match status {
                ERROR_NO_MORE_ITEMS => {
                    break rtn;
                }
                ERROR_MORE_DATA => {
                    // Our initial buffers weren't large enough, so let's reallocate
                    // using the exact size reported in `name_len` or `data_len`.
                    //
                    // name_len is the size in wide chars (without the null terminator).
                    // data_len is in bytes.
                    let mut bigger_name_buf = vec![0u16; name_len as usize];
                    let mut bigger_data_buf = vec![0u8; data_len as usize];

                    let status = unsafe {
                        RegEnumValueW(
                            hkey,
                            index,
                            Some(PWSTR(bigger_name_buf.as_mut_ptr())),
                            &mut name_len,
                            None,
                            Some(&mut value_type.0),
                            Some(bigger_data_buf.as_mut_ptr()),
                            Some(&mut data_len),
                        )
                    };
                    if status == ERROR_SUCCESS {
                        rtn.push(process_value(
                            &bigger_name_buf[..(name_len as usize)],
                            &bigger_data_buf[..(data_len as usize)],
                            value_type,
                        )?);
                    } else {
                        eprintln!("Failed to read bigger buffers: 0x{:X}", status.0);
                    }
                }
                ERROR_SUCCESS => {
                    // We got the data with our initial buffer
                    rtn.push(process_value(
                        &name_buf[..(name_len as usize)],
                        &data_buf[..(data_len as usize)],
                        value_type,
                    )?);
                }
                x => {
                    return Err(eyre!("RegEnumValueW error: 0x{:X}", x.0));
                }
            }
            index += 1;
        }
    };
    unsafe {
        RegCloseKey(hkey).ok()?;
    }
    rtn
}

/// Retrieve a machine-level environment variable value (if it exists).
/// Returns `Ok(None)` if the key was not found, or `Ok(Some(value))` otherwise.
pub fn get_machine_env_var(var_name: &str) -> eyre::Result<Option<String>> {
    // Windows registry calls typically want a wide (UTF-16) string with a null terminator
    let wide_name: Vec<u16> = var_name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        // Open the environment sub-key with KEY_QUERY_VALUE
        let mut hkey: HKEY = HKEY::default();
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            MACHINE_ENV_SUB_KEY,
            None,
            KEY_QUERY_VALUE,
            &mut hkey,
        )
        .ok()?;

        // We'll call RegQueryValueExW to get data size first
        let mut value_type = REG_VALUE_TYPE(0);
        let mut data_len: u32 = 0;
        let query_status = RegQueryValueExW(
            hkey,
            PCWSTR(wide_name.as_ptr()),
            None,
            Some(&mut value_type),
            None, // pass None to query the size needed
            Some(&mut data_len),
        );

        if let Err(e) = query_status.ok() {
            let code = WIN32_ERROR::from_error(&e).map(|c| c.0).unwrap_or_default();
            if code == ERROR_FILE_NOT_FOUND.0 {
                // Key does not exist
                RegCloseKey(hkey).ok()?;
                return Ok(None);
            } else {
                // Some other error
                RegCloseKey(hkey).ok()?;
                return Err(eyre!(
                    "Failed to query size for '{var_name}', code=0x{code:X}"
                ));
            }
        }

        // If data_len == 0, either the value is empty or something is off
        // We'll consider that as an empty string
        if data_len == 0 {
            RegCloseKey(hkey).ok()?;
            return Ok(Some(String::new()));
        }

        // Allocate a buffer for the actual data
        let mut data_buf = vec![0u8; data_len as usize];
        RegQueryValueExW(
            hkey,
            PCWSTR(wide_name.as_ptr()),
            None,
            Some(&mut value_type),
            Some(data_buf.as_mut_ptr()),
            Some(&mut data_len),
        )
        .ok()?;

        RegCloseKey(hkey).ok()?;

        // We only handle REG_SZ or REG_EXPAND_SZ in this example
        if value_type != REG_SZ {
            // If you also expect REG_EXPAND_SZ, you could handle that,
            // but for simplicity we just show REG_SZ
            return Ok(Some(utf16_from_bytes(&data_buf)));
        }
        Ok(Some(utf16_from_bytes(&data_buf)))
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EnvironmentVariable {
    pub key: String,
    pub value: String,
    pub value_expanded: Option<String>,
    pub kind: REG_VALUE_TYPE,
}
impl EnvironmentVariable {
    pub fn get_value(&self) -> &str {
        self.value_expanded.as_ref().unwrap_or(&self.value)
    }
}

/// Helper to interpret and print the registry value from raw buffers.
fn process_value(
    name_wchars: &[u16],
    data_bytes: &[u8],
    value_type: REG_VALUE_TYPE,
) -> eyre::Result<EnvironmentVariable> {
    // Convert the name from UTF-16
    let name_end = name_wchars
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(name_wchars.len());
    let name = String::from_utf16(&name_wchars[..name_end])?;

    match value_type {
        REG_SZ => {
            let value = utf16_from_bytes(data_bytes);
            return Ok(EnvironmentVariable {
                key: name,
                value,
                value_expanded: None,
                kind: value_type,
            });
            // println!("{name} (REG_SZ) = {data_str}");
        }
        REG_EXPAND_SZ => {
            let value = utf16_from_bytes(data_bytes);
            let expanded = expand_env_wstring(&value);
            // println!("{name} (REG_EXPAND_SZ) = {raw_str}");
            // println!("                   expanded => {expanded}");
            return Ok(EnvironmentVariable {
                key: name,
                value,
                value_expanded: Some(expanded),
                kind: value_type,
            });
        }
        _ => {
            bail!("{name} = (not a REG_SZ/REG_EXPAND_SZ type: {value_type:?})");
        }
    }
}

/// Expand variables like `%SystemRoot%` in a string
fn expand_env_wstring(input: &str) -> String {
    use std::iter;
    let wchars: Vec<u16> = input.encode_utf16().chain(iter::once(0)).collect();
    unsafe {
        let len = ExpandEnvironmentStringsW(PCWSTR(wchars.as_ptr()), None);
        if len == 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize];
        ExpandEnvironmentStringsW(PCWSTR(wchars.as_ptr()), Some(&mut buf));
        let expanded_end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..expanded_end])
    }
}
