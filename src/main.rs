use eyre::Result;
use eyre::eyre;
use windows::Win32::Foundation::ERROR_MORE_DATA;
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::System::Registry::HKEY;
use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
use windows::Win32::System::Registry::KEY_READ;
use windows::Win32::System::Registry::REG_VALUE_TYPE;
use windows::Win32::System::Registry::RegCloseKey;
use windows::Win32::System::Registry::RegEnumValueW;
use windows::Win32::System::Registry::RegOpenKeyExW;
use windows::core::*;

fn main() -> Result<()> {
    let sub_key = w!("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment");

    let mut hkey: HKEY = HKEY::default();
    unsafe {
        RegOpenKeyExW(HKEY_LOCAL_MACHINE, sub_key, None, KEY_READ, &mut hkey).ok()?;
    }

    let mut index = 0;
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

        if status == ERROR_NO_MORE_ITEMS {
            break; // no more values
        }
        if status == ERROR_MORE_DATA {
            // Our initial buffers weren't large enough, so let's reallocate
            // using the exact size reported in `name_len` or `data_len`.
            //
            // name_len is the size in wide chars (without the null terminator).
            // data_len is in bytes.
            let mut bigger_name_buf = vec![0u16; name_len as usize];
            let mut bigger_data_buf = vec![0u8; data_len as usize];

            let status2 = unsafe {
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
            if status2 == ERROR_SUCCESS {
                process_value(
                    &bigger_name_buf[..(name_len as usize)],
                    &bigger_data_buf[..(data_len as usize)],
                    value_type,
                );
            } else {
                eprintln!("Failed to read bigger buffers: 0x{:X}", status2.0);
            }
        } else if status == ERROR_SUCCESS {
            // We got the data with our initial buffer
            process_value(
                &name_buf[..(name_len as usize)],
                &data_buf[..(data_len as usize)],
                value_type,
            );
        } else {
            return Err(eyre!("RegEnumValueW error: 0x{:X}", status.0));
        }

        index += 1;
    }

    unsafe {
        RegCloseKey(hkey).ok()?;
    }

    Ok(())
}

/// Helper to interpret and print the registry value from raw buffers.
fn process_value(name_wchars: &[u16], data_bytes: &[u8], value_type: REG_VALUE_TYPE) {
    // Convert the name from UTF-16
    let name_end = name_wchars
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(name_wchars.len());
    let name = String::from_utf16_lossy(&name_wchars[..name_end]);

    match value_type.0 {
        1 => {
            // REG_SZ
            let data_str = utf16_from_byte_slice(data_bytes);
            println!("{name} (REG_SZ) = {data_str}");
        }
        2 => {
            // REG_EXPAND_SZ
            let raw_str = utf16_from_byte_slice(data_bytes);
            let expanded = expand_env_wstring(&raw_str);
            println!("{name} (REG_EXPAND_SZ) = {raw_str}");
            println!("                   expanded => {expanded}");
        }
        _ => {
            println!("{name} = (not a REG_SZ/REG_EXPAND_SZ type: {value_type:?})");
        }
    }
}

/// Convert pairs of bytes to UTF-16, then to a Rust string. Stop at the first null terminator.
fn utf16_from_byte_slice(bytes: &[u8]) -> String {
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
