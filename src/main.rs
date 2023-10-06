use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use std::{ffi::OsString, fmt, mem, os::windows::ffi::OsStringExt, ptr::null_mut};
// use windows::Win32::{ Foundation::*, System::ProcessStatus::*, System::Threading::*, UI::WindowsAndMessaging::* };
use fmt::{Display, Formatter, Result};
use serde::{Deserialize, Serialize};
use winapi::{
    shared::{minwindef::*, ntdef::*, windef::*},
    um::{processthreadsapi, psapi, winnt, winuser},
};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Item {
    hwnd: u32,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    title: String,
    name: String,
    checked: bool,
}

impl Display for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{:<8} ({:>4}, {:>4}, {:>4}, {:>4}) {:40} {:40}",
            self.hwnd, self.left, self.top, self.width, self.height, self.title, self.name
        )
    }
}

static mut LIST: Vec<Item> = Vec::new();
static mut CX: i32 = 0;
static mut CY: i32 = 0;
const LEN: usize = 1024;

fn main() {
    dioxus_desktop::launch_cfg(
        app,
        Config::default().with_window(
            WindowBuilder::new()
                .with_title("桌面整理")
                .with_resizable(false)
                .with_inner_size(LogicalSize::new(640.0, 480.0)),
        ),
    );
}

fn app(cx: Scope) -> Element {
    unsafe {
        // let win = dioxus_desktop::use_window(&cx);
        // 我们将窗口设置为无边框的，然后我们可以自己实现标题栏。
        // win.set_decorations(false);

        CX = winuser::GetSystemMetrics(winuser::SM_CXSCREEN);
        CY = winuser::GetSystemMetrics(winuser::SM_CYSCREEN);

        LIST.clear();
        let _ = winuser::EnumWindows(Some(enum_window), 0 as LPARAM);
        // println!("{:?}", LIST);

        let hwnd = use_state(&cx, || "0".to_string());
        let left = use_state(&cx, || "0".to_string());
        let top = use_state(&cx, || "0".to_string());
        let width = use_state(&cx, || "0".to_string());
        let height = use_state(&cx, || "0".to_string());
        let items = LIST.iter().enumerate().map(|x| {
			let id = format!("item_{}", x.0);
            rsx! {
                tr {
					onclick: move |_evt| {
						hwnd.set(x.1.hwnd.to_string());
						left.set(x.1.left.to_string());
						top.set(x.1.top.to_string());
						width.set(x.1.width.to_string());
						height.set(x.1.height.to_string());
					},
                    td {
						style: "width: 15%;",
						input {
							id: "{id}",
							r#type: "checkbox",
							checked: x.1.checked,
							onchange: move |_| {
							}
						}
						x.1.hwnd.to_string()
					}
                    td { style: "width: 24%;", x.1.left.to_string(), ",", x.1.top.to_string(), ",", x.1.width.to_string(), ",", x.1.height.to_string() }
                    td { style: "width: 30%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", x.1.title.to_string() }
                    td { style: "width: 24%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", x.1.name.to_string() }
                }
            }
        });

        cx.render(rsx! {
            // div {
            //     class: "titlebar",
            //     style: "background-color: black; color: white; padding: 0; text-align: right;",
            //     // 当鼠标被按下后，开始监听窗口的拖拽
            //     // 通过这种方法可以实现自定义 TopBar 的页面拖拽
            //     onmousedown: |_| { win.drag(); },
            //     a {
            //         class: "minimize",
            //         // 使用 cancel_bubble 拦截默认事件处理，因为我们需要全局绑定 drag
            //         onmousedown: |e| { e.stop_propagation(); },
            //         onclick: move |_| { win.set_minimized(true) },
            //         "最小化"
            //     }
            //     a {
            //         class: "close",
            //         onmousedown: |e| { e.stop_propagation(); },
            //         onclick: move |_| { win.close() },
            //         "关闭"
            //     }
            // }
			style { include_str!("./style.css") }
			div {
				style: "width: 100%; height: 30px;display: inline-block; white-space: nowrap; display: flex; justify-content: center; align-items: center;",
				input { style: "width: 22%;", placeholder: "左", value: "{left}", oninput: |evt| left.set(evt.value.clone()), }
				input { style: "width: 22%;", placeholder: "上", value: "{top}", oninput: |evt| top.set(evt.value.clone()),  }
				input { style: "width: 22%;", placeholder: "宽", value: "{width}", oninput: |evt| width.set(evt.value.clone()),  }
				input { style: "width: 22%;", placeholder: "高", value: "{height}", oninput: |evt| height.set(evt.value.clone()),  }
				button { onclick: move |_| {
					println!("当前值: {left} {top} {width} {height}");
					let _ = winuser::MoveWindow(hwnd.parse::<isize>().unwrap() as HWND, left.parse::<i32>().unwrap(), top.parse::<i32>().unwrap(), width.parse::<i32>().unwrap(), height.parse::<i32>().unwrap(), 1);
				}, "确定" }
			}
            div {
                table {
					style: "width: 100%",
					tr {
						td {
							style: "width: 15%; text-align: left;",
							input {
								id: "toggle-all",
								r#type: "checkbox",
								onchange: move |_| {
								}
							}
							label { r#for: "toggle-all", "句柄" }
						}
						td { style: "width: 24%;", "位置"}
						td { style: "width: 30%;", "标题"}
						td { style: "width: 24%;", "路径"}
					}
					items
                }
            }
        })
    }
}

unsafe extern "system" fn enum_window(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let text = GetWindowTextW(hwnd);
        // let text = String::from_utf16_lossy(&text[..len as usize]);

        let info = GetWindowInfo(hwnd);

        if !text.is_empty()
            // && info.dwStyle.contains(winuser::WS_VISIBLE)
			&& (info.dwStyle & winuser::WS_VISIBLE) > 0
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

            LIST.push(Item {
                hwnd: hwnd as usize as u32,
                left: info.rcWindow.left,
                top: info.rcWindow.top,
                width: info.rcWindow.right - info.rcWindow.left,
                height: info.rcWindow.bottom - info.rcWindow.top,
                title: text.into_string().unwrap(),
                name: image_file_name.into_string().unwrap(),
                checked: false,
            });
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
fn GetWindowInfo(hwnd: HWND) -> winuser::WINDOWINFO {
    let mut info: winuser::WINDOWINFO = unsafe { mem::zeroed() };
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
