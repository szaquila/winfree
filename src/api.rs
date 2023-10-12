use std::{
    ffi::{OsStr, OsString},
    iter::once,
    mem,
    os::windows::ffi::{OsStrExt, OsStringExt},
    ptr::null_mut,
};
use winapi::{
    shared::{
        minwindef::{DWORD, HKEY, *},
        ntdef::*,
        windef::*,
        winerror::SEC_E_OK,
    },
    um::{
        processthreadsapi, psapi,
        winnt::{self, REG_BINARY},
        winreg::{RegOpenKeyW, RegQueryValueExW, RegSetValueExW, LSTATUS},
        winuser::{
            self, EnumWindows, GetSystemMetrics, MoveWindow, SM_CXSCREEN, SM_CYSCREEN, WINDOWINFO,
            WS_VISIBLE,
        },
    },
};

const LEN: usize = 1024;

unsafe extern "system" fn enum_window(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let text = GetWindowTextW(hwnd);
        // let text = String::from_utf16_lossy(&text[..len as usize]);

        let info = GetWindowInfo(hwnd);

        if !text.is_empty()
            // && info.dwStyle.contains(WS_VISIBLE)
			&& (info.dwStyle & WS_VISIBLE) > 0
            && !(info.rcWindow.left == 0 && (info.rcWindow.right - info.rcWindow.left) == CX)
        {
            // println!(
            //     "{:<8} ({:>4}, {:>4}, {:>4}, {:>4}) {:40}",
            //     window.clone().0,
            //     info.rcWindow.left,
            //     info.rcWindow.top,
            //     info.rcWindow.right - info.rcWindow.left,
            //     info.rcWindow.bottom - info.rcWindow.top,
            //     text.clone(),
            // );

            let (_, proc_id) = GetWindowThreadProcessId(hwnd);
            let h_process = OpenProcess(
                winnt::PROCESS_TERMINATE | winnt::PROCESS_QUERY_INFORMATION,
                0,
                proc_id,
            );
            let image_file_name = GetModuleFileNameExW(h_process);
            let name = image_file_name.into_string().unwrap();

            let mut checked = false;
            let mut left = info.rcWindow.left;
            let mut top = info.rcWindow.top;
            let mut width = info.rcWindow.right - info.rcWindow.left;
            let mut height = info.rcWindow.bottom - info.rcWindow.top;
            if SAVED.as_ref().unwrap().contains_key(&name) {
                checked = true;
                if !OC {
                    let saved = SAVED.as_ref().unwrap().get(&name).unwrap();
                    left = saved.left;
                    top = saved.top;
                    width = saved.width;
                    height = saved.height;
                    let _ = MoveWindow(hwnd, left, top, width, height, 1);
                }
            }
            LIST.as_mut().unwrap().insert(
                name.clone(),
                Item {
                    hwnd: hwnd as u32,
                    title: text.into_string().unwrap(),
                    checked,
                    left,
                    top,
                    width,
                    height,
                    name,
                },
            );
        }

        true.into()
    }
}

fn slice_to_os_string_trancate_nul(text: &[u16]) -> OsString {
    if let Some(new_len) = text.iter().position(|x| *x == 0) {
        OsString::from_wide(&text[0..new_len])
    } else {
        OsString::from_wide(&text)
    }
}

///
#[allow(dead_code)]
#[allow(non_snake_case)]
fn GetForegroundWindow() -> HWND {
    unsafe { winuser::GetForegroundWindow() }
}

/// returns `OsString`, nul char is truncated.
#[allow(dead_code)]
#[allow(non_snake_case)]
fn GetWindowTextW(hwnd: HWND) -> OsString {
    let text_len = unsafe { winuser::GetWindowTextLengthW(hwnd) };
    // println!("text_len: {}", text_len);

    let mut text = vec![0u16; (text_len + 1) as usize];
    unsafe {
        winuser::GetWindowTextW(hwnd, text.as_mut_ptr(), text_len + 1);
    }
    slice_to_os_string_trancate_nul(&text)
}

#[allow(dead_code)]
#[allow(non_snake_case)]
fn GetWindowInfo(hwnd: HWND) -> WINDOWINFO {
    let mut info: WINDOWINFO = unsafe { mem::zeroed() };
    let _ok = unsafe { winuser::GetWindowInfo(hwnd, &mut info) };
    info
}

/// returns `(thread_id, proc_id)`
#[allow(dead_code)]
#[allow(non_snake_case)]
fn GetWindowThreadProcessId(hwnd: HWND) -> (DWORD, DWORD) {
    let mut proc_id: DWORD = 0;
    let thread_id = unsafe { winuser::GetWindowThreadProcessId(hwnd, &mut proc_id) };
    (thread_id, proc_id)
}

/// returns process handle.
#[allow(dead_code)]
#[allow(non_snake_case)]
fn OpenProcess(dwDesiredAccess: DWORD, bInheritHandle: BOOL, dwProcessId: DWORD) -> HANDLE {
    unsafe { processthreadsapi::OpenProcess(dwDesiredAccess, bInheritHandle, dwProcessId) }
}

/// returns `OsString`, nul char is truncated.
#[allow(dead_code)]
#[allow(non_snake_case)]
fn GetProcessImageFileNameW(hProcess: HANDLE) -> OsString {
    let mut image_file_name = vec![0u16; LEN];
    let _ = unsafe {
        psapi::GetProcessImageFileNameW(
            hProcess,
            image_file_name.as_mut_ptr(),
            image_file_name.len() as u32,
        )
    };
    slice_to_os_string_trancate_nul(&image_file_name)
}

/// returns `OsString`, nul char is truncated.
#[allow(dead_code)]
#[allow(non_snake_case)]
fn GetModuleFileNameExW(hProcess: HANDLE) -> OsString {
    let mut image_file_name = vec![0u16; LEN];
    let _ = unsafe {
        psapi::GetModuleFileNameExW(
            hProcess,
            null_mut(),
            image_file_name.as_mut_ptr(),
            image_file_name.len() as u32,
        )
    };
    slice_to_os_string_trancate_nul(&image_file_name)
}

/// 打开注册表
/// [`main_hkey`] 是一个HKEY值，默认接收[`HKEY_CURRENT_USER`]等值
/// [`sub_key`] HKEY的子健
/// # Examples
/// Basic usage:
/// ```
/// let sub_key = "Software\\360\\333";
/// let hkey_result = reg_util::reg_open(HKEY_CURRENT_USER, sub_key);
/// ```
pub(crate) fn reg_open(main_hkey: HKEY, sub_key: &str) -> Result<HKEY, String> {
    unsafe {
        let mut hkey: HKEY = null_mut();
        let status = RegOpenKeyW(main_hkey, str_to_lpcwstr(sub_key).as_ptr(), &mut hkey);
        if status == SEC_E_OK {
            return Result::Ok(hkey);
        }
        return Result::Err(format!("status == {}", status));
    }
}

/// 查询注册表的REG_BINARY的值
pub(crate) fn reg_query_binary(hkey: &HKEY, key_name: &str) -> Vec<u8> {
    unsafe {
        let mut dword: DWORD = 0;
        let mut dtype: DWORD = 0;

        //查询
        let status = RegQueryValueExW(
            *hkey,
            str_to_lpcwstr(key_name).as_ptr(),
            null_mut(),
            &mut dtype,
            null_mut(),
            &mut dword,
        );

        let mut data_binary: Vec<u8> = vec![0; dword as usize];
        if status == SEC_E_OK {
            // 存在值

            RegQueryValueExW(
                *hkey,
                str_to_lpcwstr(key_name).as_ptr(),
                null_mut(),
                &mut dtype,
                data_binary.as_mut_ptr(),
                &mut dword,
            );
        }
        return data_binary;
    }
}

/// 保存REG_SZ类型的数据
pub(crate) fn reg_save_binary(hkey: &HKEY, key_name: &str, value: &mut Vec<u8>) -> LSTATUS {
    unsafe {
        let status = RegSetValueExW(
            *hkey,
            str_to_lpcwstr(key_name).as_ptr(),
            0,
            REG_BINARY,
            value.as_mut_ptr(),
            value.len() as u32,
        );
        return status;
    }
}

pub(crate) fn str_to_lpcwstr(str: &str) -> Vec<u16> {
    unsafe {
        let result: Vec<u16> = OsStr::new(str).encode_wide().chain(once(0)).collect();
        return result;
    }
}
