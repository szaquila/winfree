use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use std::fmt;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};

#[derive(Debug)]
struct Item {
    hwnd: i32,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    title: String,
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<8} ({:>4}, {:>4}, {:>4}, {:>4}) {:40}",
            self.hwnd, self.left, self.top, self.width, self.height, self.title,
        )
    }
}

static mut LIST: Vec<Item> = Vec::new();
static mut CX: i32 = 0;
static mut CY: i32 = 0;

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

        CX = GetSystemMetrics(SM_CXSCREEN);
        CY = GetSystemMetrics(SM_CYSCREEN);

        LIST.clear();
        let _ = EnumWindows(Some(enum_window), LPARAM(0));
        // println!("{:?}", LIST);

        let hwnd = use_state(&cx, || "0".to_string());
        let left = use_state(&cx, || "0".to_string());
        let top = use_state(&cx, || "0".to_string());
        let width = use_state(&cx, || "0".to_string());
        let height = use_state(&cx, || "0".to_string());
        let items = LIST.iter().enumerate().map(|x| {
            rsx! {
                tr {
					onclick: move |_evt| {
						hwnd.set(x.1.hwnd.to_string());
						left.set(x.1.left.to_string());
						top.set(x.1.top.to_string());
						width.set(x.1.width.to_string());
						height.set(x.1.height.to_string());
					},
                    td { style: "width: 60px;", x.1.hwnd.to_string() }
                    td { style: "width: 80px;", x.1.left.to_string(), ",", x.1.top.to_string(), ",", x.1.width.to_string(), ",", x.1.height.to_string() }
                    td { style: "width: 200px; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;", x.1.title.to_string() }
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
				style: "width: 98%; height: 30px;display: inline-block; white-space: nowrap; display: flex; justify-content: center; align-items: center;",
				input { style: "width: 22%;", placeholder: "左", value: "{left}", oninput: |evt| left.set(evt.value.clone()), }
				input { style: "width: 22%;", placeholder: "上", value: "{top}", oninput: |evt| top.set(evt.value.clone()),  }
				input { style: "width: 22%;", placeholder: "宽", value: "{width}", oninput: |evt| width.set(evt.value.clone()),  }
				input { style: "width: 22%;", placeholder: "高", value: "{height}", oninput: |evt| height.set(evt.value.clone()),  }
				button { onclick: move |_| {
					println!("当前值: {left} {top} {width} {height}");
					let _ = MoveWindow(HWND(hwnd.parse::<isize>().unwrap()), left.parse::<i32>().unwrap(), top.parse::<i32>().unwrap(), width.parse::<i32>().unwrap(), height.parse::<i32>().unwrap(), true);
				}, "确定" }
			}
            div {
                table {
					tr {
						td { style: "width: 60px;", "句柄" }
						td { style: "width: 80px;", "位置"}
						td { style: "width: 200px;", "标题"}
					}
					items
                }
            }
        })
    }
}

unsafe extern "system" fn enum_window(window: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let mut text: [u16; 512] = [0; 512];
        let len = GetWindowTextW(window, &mut text);
        let text = String::from_utf16_lossy(&text[..len as usize]);

        let mut info = WINDOWINFO {
            cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
            ..Default::default()
        };
        GetWindowInfo(window, &mut info).unwrap();

        if !text.is_empty()
            && info.dwStyle.contains(WS_VISIBLE)
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
            LIST.push(Item {
                hwnd: window.0 as usize as i32,
                left: info.rcWindow.left,
                top: info.rcWindow.top,
                width: info.rcWindow.right - info.rcWindow.left,
                height: info.rcWindow.bottom - info.rcWindow.top,
                title: text.into(),
            });
        }

        true.into()
    }
}
