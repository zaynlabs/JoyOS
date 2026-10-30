use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ,
    RegCloseKey, RegDeleteKeyValueW, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, RegSetKeyValueW,
};
use windows::Win32::Foundation::ERROR_MORE_DATA;
use windows::core::{HSTRING, PWSTR};

pub fn read_string(key: &str, value: &str) -> Option<String> {
    read_string_in(HKEY_CURRENT_USER, key, value)
}

/// Machine-wide settings, e.g. where GOG Galaxy records installed games.
pub fn read_local_machine_string(key: &str, value: &str) -> Option<String> {
    read_string_in(HKEY_LOCAL_MACHINE, key, value)
}

fn read_string_in(hive: HKEY, key: &str, value: &str) -> Option<String> {
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    // Ask for the size first, so long values (paths, command lines) aren't cut off.
    let mut size = 0u32;
    let err = unsafe { RegGetValueW(hive, &key, &value, RRF_RT_REG_SZ, None, None, Some(&mut size)) };
    if err.is_err() {
        return None;
    }
    // The value can grow between the two calls, so try a couple of times.
    for _ in 0..3 {
        let mut buf = vec![0u16; size as usize / 2 + 1];
        size = (buf.len() * 2) as u32;
        let err = unsafe {
            RegGetValueW(hive, &key, &value, RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size))
        };
        if err == ERROR_MORE_DATA {
            continue;
        }
        if err.is_err() {
            return None;
        }
        let len = (size as usize / 2).saturating_sub(1);
        return Some(String::from_utf16_lossy(&buf[..len]));
    }
    None
}

/// Names of the direct subkeys under a machine-wide key. Empty if it doesn't exist.
pub fn local_machine_subkeys(key: &str) -> Vec<String> {
    subkeys_in(HKEY_LOCAL_MACHINE, key)
}

fn subkeys_in(hive: HKEY, key: &str) -> Vec<String> {
    let mut handle = HKEY::default();
    let opened = unsafe { RegOpenKeyExW(hive, &HSTRING::from(key), Some(0), KEY_READ, &mut handle) };
    if opened.is_err() {
        return vec![];
    }
    let mut names = Vec::new();
    for index in 0.. {
        let mut buf = [0u16; 256];
        let mut len = buf.len() as u32;
        let err = unsafe {
            RegEnumKeyExW(handle, index, Some(PWSTR(buf.as_mut_ptr())), &mut len, None, None, None, None)
        };
        if err.is_err() {
            break;
        }
        names.push(String::from_utf16_lossy(&buf[..len as usize]));
    }
    unsafe {
        let _ = RegCloseKey(handle);
    }
    names
}

pub fn read_dword(key: &str, value: &str) -> Option<u32> {
    let mut data = 0u32;
    let mut size = 4u32;
    let err = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(value),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    };
    err.is_ok().then_some(data)
}

pub fn write_string(key: &str, value: &str, data: &str) -> bool {
    let wide: Vec<u16> = data.encode_utf16().chain(Some(0)).collect();
    unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            &HSTRING::from(key),
            &HSTRING::from(value),
            REG_SZ.0,
            Some(wide.as_ptr().cast()),
            (wide.len() * 2) as u32,
        )
        .is_ok()
    }
}

pub fn delete(key: &str, value: &str) {
    unsafe {
        let _ = RegDeleteKeyValueW(HKEY_CURRENT_USER, &HSTRING::from(key), &HSTRING::from(value));
    }
}
