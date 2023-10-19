#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use dioxus::prelude::*;
use dioxus_desktop::{
    tao::platform::windows::WindowBuilderExtWindows, Config, LogicalSize, WindowBuilder,
};
use fmt::{Display, Formatter, Result};
use platform_dirs::AppDirs;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap as Map,
    env, fmt,
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
    path::Path,
};
use tray_item::{IconSource, TrayItem};
use windows::Win32::{
    Foundation::*, System::ProcessStatus::*, System::Threading::*, UI::WindowsAndMessaging::*,
};
use winreg::{enums::*, RegKey};

#[derive(RustEmbed)]
#[folder = "./assets/"]
struct Assets;

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Item {
    hwnd: u32,
    title: String,
    checked: bool,
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    name: String,
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

const SAVED_FILE: &str = "winfree.json";
static mut LIST: Option<Map<String, Item>> = None;
static mut SAVED: Option<Map<String, Item>> = None;
static mut CX: i32 = 0;
static mut CY: i32 = 0;
static mut ROWCLICK: bool = false;
static mut HIDE: bool = false;
static mut STARTUP: bool = false;
static mut WIN: isize = 0;
static mut EXE: String = String::new();

fn main() {
    load();

    let mut tray = TrayItem::new("整理桌面", IconSource::Resource("main-icon")).unwrap();
    tray.add_menu_item("显示/隐藏", || {
        unsafe {
            HIDE = !HIDE;
            // WIN.set_visible(!HIDE);
            if HIDE {
                ShowWindow(HWND(WIN), SW_MINIMIZE);
            } else {
                ShowWindow(HWND(WIN), SW_RESTORE);
            }
        }
    })
    .unwrap();
    tray.inner_mut().add_separator().unwrap();
    tray.add_menu_item("刷新", || unsafe {
        ROWCLICK = false;
        LIST = Some(Map::new());
        let _ = EnumWindows(Some(enum_window), LPARAM(0));
    })
    .unwrap();
    tray.add_menu_item("退出", || {
        std::process::exit(0);
    })
    .unwrap();

    dioxus_desktop::launch_cfg(
        app,
        Config::default()
            // .with_custom_head(
            //     r#"<link rel="stylesheet" href="assets/tailwind.css">"#.to_string(),
            // )
            .with_custom_index(
                r#"
				<!DOCTYPE html>
				<html lang="zh-CN" class="dark h-full">
					<head>
						<title>Dioxus app</title>
						<meta name="viewport" content="width=device-width, initial-scale=1.0" />
						<link rel="stylesheet" href="assets/tailwind.css">
						<style>
							input::-webkit-outer-spin-button,
							input::-webkit-inner-spin-button {
								-webkit-appearance: none;
							}
							input[type="number"] {
								-moz-appearance: textfield;
							}
						</style>
					</head>
					<body class="m-0 p-0 antialiased text-slate-500 dark:text-slate-400 bg-white dark:bg-slate-900">
						<div id="main"></div>
					</body>
				</html>
				"#
                .into(),
            )
            .with_window(
                WindowBuilder::new()
                    .with_title("桌面整理")
                    .with_resizable(false)
                    .with_minimizable(false)
                    .with_skip_taskbar(true)
                    .with_inner_size(LogicalSize::new(640.0, 480.0)),
            ),
    );
}

fn app(cx: Scope) -> Element {
    unsafe {
        CX = GetSystemMetrics(SM_CXSCREEN);
        CY = GetSystemMetrics(SM_CYSCREEN);

        LIST = Some(Map::new());
        let _ = EnumWindows(Some(enum_window), LPARAM(0));
        // println!("{:?}", LIST);

        let window = dioxus_desktop::use_window(&cx);
        // 我们将窗口设置为无边框的，然后我们可以自己实现标题栏。
        // window.set_decorations(false);
        // window.set_visible(!HIDE);
        window.set_minimized(HIDE);

        let hwnd = use_state(&cx, || "0".to_string());
        let title = use_state(&cx, || "0".to_string());
        let checked = use_state(&cx, || "0".to_string());
        let left = use_state(&cx, || "".to_string());
        let top = use_state(&cx, || "".to_string());
        let width = use_state(&cx, || "".to_string());
        let height = use_state(&cx, || "".to_string());
        let name = use_state(&cx, || "0".to_string());

        let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (run, _disp) = hkcu.create_subkey(path).ok()?;
        for (k, _v) in run.enum_values().map(|x| x.unwrap()) {
            // println!("{} = {:?}", k, v);
            if k == "winfree" {
                // && v.to_string() == EXE.clone() {
                STARTUP = true;
            }
        }

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
			// style { include_str!("./style.css") }
			div {
				class: "container w-full h-full bg-white dark:bg-slate-900 rounded-lg px-2 py-2 ring-1 ring-slate-900/5 shadow-xl",
				// style: "padding: 5px;",
				div {
					class: "flex h-8 leading-6 align-middle",
					// style: "width: 100%; height: 30px; margin-bottom: 10px; white-space: nowrap; display: flex; justify-content: center; align-items: center;",
					input {
						class: "w-1/6 mx-1.5 mb-2 px-1 py-1 bg-white dark:bg-slate-800 border shadow-sm border-slate-300 placeholder-slate-400 focus:outline-none focus:border-sky-500 focus:ring-sky-500 rounded-md sm:text-sm focus:ring-1",
						// style: "width: 22%; margin-left: 5px;",
						r#type: "number",
						id: "id_left",
						name: "id_left",
						value: "{left}",
						placeholder: "请输入左坐标",
						oninput: |evt| left.set(evt.value.clone()),
						// autofocus: "true",
					}
					input {
						class: "w-1/6 mx-1.5 mb-2 px-1 py-1 bg-white dark:bg-slate-800 border shadow-sm border-slate-300 placeholder-slate-400 focus:outline-none focus:border-sky-500 focus:ring-sky-500 rounded-md sm:text-sm focus:ring-1",
						// style: "width: 22%; margin-left: 5px;",
						r#type: "number",
						id: "id_top",
						name: "id_top",
						value: "{top}",
						placeholder: "请输入上坐标",
						oninput: |evt| top.set(evt.value.clone()),
					}
					input {
						class: "w-1/6 mx-1.5 mb-2 px-1 py-1 bg-white dark:bg-slate-800 border shadow-sm border-slate-300 placeholder-slate-400 focus:outline-none focus:border-sky-500 focus:ring-sky-500 rounded-md sm:text-sm focus:ring-1",
						// style: "width: 22%; margin-left: 5px;",
						r#type: "number",
						id: "id_width",
						name: "id_width",
						value: "{width}",
						placeholder: "请输入宽度",
						oninput: |evt| width.set(evt.value.clone()),
					}
					input {
						class: "w-1/6 mx-1.5 mb-2 px-1 py-1 bg-white dark:bg-slate-800 border shadow-sm border-slate-300 placeholder-slate-400 focus:outline-none focus:border-sky-500 focus:ring-sky-500 rounded-md sm:text-sm focus:ring-1",
						// style: "width: 22%; margin-left: 5px;",
						r#type: "number",
						id: "id_height",
						name: "id_height",
						value: "{height}",
						placeholder: "请输入高度",
						oninput: |evt| height.set(evt.value.clone()),
					}
					button {
						class: "mx-1.5 mb-2 inline-flex items-center text-white bg-green-500 border-0 py-1 px-3 hover:bg-green-700 rounded",
						// style: "margin-left: 5px; margin-right: 5px;",
						onmousedown: |evt| evt.stop_propagation(),
						onclick: move |_| {
							// println!("当前值: {left} {top} {width} {height}");
							let chwnd = hwnd.parse::<isize>().unwrap();
							let ctitle = title.parse::<String>().unwrap();
							if chwnd > 0 && !left.is_empty() && !top.is_empty() && !width.is_empty() && !height.is_empty() {
								let cchecked = checked.parse::<bool>().unwrap();
								let cleft = left.parse::<i32>().unwrap();
								let ctop = top.parse::<i32>().unwrap();
								let cwidth = width.parse::<i32>().unwrap();
								let cheight = height.parse::<i32>().unwrap();
								let cname = name.parse::<String>().unwrap();
								if cchecked {
									SAVED.as_mut().unwrap().insert(
										cname.clone(),
										Item {
											hwnd: chwnd as u32,
											title: ctitle,
											checked: true,
											left: cleft,
											top: ctop,
											width: cwidth,
											height: cheight,
											name: cname,
										},
									);
									save();
								}
								let _ = MoveWindow(HWND(chwnd), cleft, ctop, cwidth, cheight, true);
							}
						},
						"确定"
					}
					label {
						class: "flex align-middle items-center",
						r#for: "id_startup",
						input {
							r#type: "checkbox",
							id: "id_startup",
							name: "id_startup",
							checked: "{STARTUP}",
							onchange: move |evt| {
								if evt.value.clone().parse::<bool>().unwrap() {
									STARTUP = true;
									let _ = run.set_value("winfree", &EXE);
								} else {
									STARTUP = false;
									let _ = run.delete_value("winfree");
								}
							},
						}
						"开机启动"
					}
				}
				div {
					table {
						class: "table-fixed border-collapse w-full border border-slate-400 dark:border-slate-500 bg-white dark:bg-slate-600 text-sm shadow-sm",
						thead {
							tr {
								class: "bg-slate-200 dark:bg-slate-800",
								th {
									class: "w-1/6 border border-slate-300 dark:border-slate-600 font-semibold p-1.5 text-slate-900 dark:text-slate-200 text-left",
									"句柄"
								}
								th {
									class: "w-1/4 border border-slate-300 dark:border-slate-600 font-semibold p-1.5 text-slate-900 dark:text-slate-200 text-left",
									"位置"
								}
								th {
									class: "w-1/3 border border-slate-300 dark:border-slate-600 font-semibold p-1.5 text-slate-900 dark:text-slate-200 text-left",
									"标题"
								}
								th {
									class: "w-1/4 border border-slate-300 dark:border-slate-600 font-semibold p-1.5 text-slate-900 dark:text-slate-200 text-left",
									"路径"
								}
							}
						}
						tbody {
							for (k, v) in LIST.as_ref().unwrap().iter() {
								tr {
									class: "odd:bg-slate-500 even:bg-slate-600 hover:bg-yellow-100",
									onclick: move |_evt| {
										hwnd.set(v.hwnd.to_string());
										title.set(v.title.to_string());
										checked.set(v.checked.to_string());
										left.set(v.left.to_string());
										top.set(v.top.to_string());
										width.set(v.width.to_string());
										height.set(v.height.to_string());
										name.set(v.name.to_string());
										ROWCLICK = true;
									},
									ondblclick: move |_evt| {
										let mut info = WINDOWINFO {
											cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
											..Default::default()
										};
										GetWindowInfo(HWND(v.hwnd as isize), &mut info).unwrap();
										hwnd.set(v.hwnd.to_string());
										title.set(v.title.to_string());
										checked.set(v.checked.to_string());
										left.set(info.rcWindow.left.to_string());
										top.set(info.rcWindow.top.to_string());
										width.set((info.rcWindow.right - info.rcWindow.left).to_string());
										height.set((info.rcWindow.bottom - info.rcWindow.top).to_string());
										name.set(v.name.to_string());
									},
									td {
										class: "border border-slate-300 dark:border-slate-700 p-1 text-slate-400",
										input {
											id: "id_{k}",
											name: "id_{k}",
											r#type: "checkbox",
											checked: v.checked,
											onchange: move |evt| {
												if evt.value.clone().parse::<bool>().unwrap() {
													checked.set("1".to_string());
													SAVED.as_mut().unwrap().insert(
														v.name.clone(),
														Item {
															hwnd: v.hwnd as u32,
															title: v.title.clone(),
															checked: true,
															left: v.left,
															top: v.top,
															width: v.width,
															height: v.height,
															name: v.name.clone(),
														},
													);
												} else {
													checked.set("0".to_string());
													SAVED.as_mut().unwrap().remove(&v.name.clone());
												}
												save();
											}
										}
										label {
											r#for: "id_{k}",
											v.hwnd.to_string()
										}
									}
									td {
										class: "border border-slate-300 dark:border-slate-700 p-1 text-slate-400",
										v.left.to_string(), ",", v.top.to_string(), ",", v.width.to_string(), ",", v.height.to_string()
									}
									td {
										class: "overflow-hidden text-ellipsis whitespace-nowrap border border-slate-300 dark:border-slate-700 p-1 text-slate-400",
										v.title.to_string()
									}
									td {
										class: "overflow-hidden text-ellipsis whitespace-nowrap border border-slate-300 dark:border-slate-700 p-1 text-slate-400",
										v.name.to_string()
									}
								}
							}
						}
					}
				}
			}
	})
    }
}

pub fn stacks_icon(cx: Scope) -> Element {
    cx.render(rsx!(
        svg {
            fill: "none",
            stroke: "currentColor",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            stroke_width: "2",
            class: "w-10 h-10 text-white p-2 bg-indigo-500 rounded-full",
            view_box: "0 0 24 24",
            path { d: "M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"}
        }
    ))
}

pub fn right_arrow_icon(cx: Scope) -> Element {
    cx.render(rsx!(
        svg {
            fill: "none",
            stroke: "currentColor",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            stroke_width: "2",
            class: "w-4 h-4 ml-1",
            view_box: "0 0 24 24",
            path { d: "M5 12h14M12 5l7 7-7 7"}
        }
    ))
}

fn load() {
    unsafe {
        match env::current_exe() {
            Ok(exe_path) => EXE = exe_path.to_str().unwrap().to_string(),
            Err(e) => println!("failed to get current exe path: {e}"),
        };

        SAVED = Some(Map::new());
        let app_dirs = AppDirs::new(Some(SAVED_FILE), true).unwrap();
        // println!("{:?}", app_dirs);
        let path = Path::new(&app_dirs.config_dir);
        if path.exists() {
            let reader = File::open(&app_dirs.config_dir).unwrap();
            let items: Vec<Item> = serde_json::from_reader(reader).unwrap();
            for item in items {
                SAVED
                    .as_mut()
                    .unwrap()
                    .insert(item.name.clone(), item.clone());
            }
        } else {
            // println!("配置文件不存在，创建上级目录!");
            let _ = create_dir_all(path.parent().unwrap());
        }

        let path = Path::new("assets/tailwind.css");
        if !path.exists() {
            let _ = create_dir_all(path.parent().unwrap());
            let mut buffer = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("./assets/tailwind.css")
                .unwrap();
            match Assets::get("tailwind.css") {
                Some(assets) => {
                    let _ = buffer.write_all(assets.data.as_ref());
                }
                None => {
                    println!("get embed file error!")
                }
            }
        }
    }
}

fn save() {
    unsafe {
        let app_dirs = AppDirs::new(Some(SAVED_FILE), true).unwrap();
        let writer = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&app_dirs.config_dir)
            .unwrap();

        let mut items = Vec::new();
        for (_k, v) in SAVED.as_ref().unwrap().iter() {
            items.push(v);
        }
        let _ = serde_json::to_writer_pretty(writer, &items);
    }
}

unsafe extern "system" fn enum_window(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let mut text = [0; 512];
        let len = GetWindowTextW(hwnd, &mut text);
        let text = String::from_utf16_lossy(&text[..len as usize]);

        let mut info = WINDOWINFO {
            cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
            ..Default::default()
        };
        GetWindowInfo(hwnd, &mut info).unwrap();

        if !text.is_empty()
            && info.dwStyle.contains(WS_VISIBLE)
			// && (info.dwStyle & WS_VISIBLE) > 0
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

            let mut proc_id = 0u32;
            let mut name = "".to_string();
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut proc_id));
            match OpenProcess(
                PROCESS_TERMINATE | PROCESS_QUERY_INFORMATION,
                false,
                proc_id,
            ) {
                Ok(h_process) => {
                    let mut cname = [0; 512];
                    let len = GetModuleFileNameExW(h_process, HMODULE(0), &mut cname);
                    name = String::from_utf16_lossy(&cname[..len as usize]);
                    if name.clone() == EXE.clone() {
                        WIN = hwnd.0;
                    }
                }
                Err(err) => {
                    println!("错误：{}", err);
                }
            }

            let mut checked = false;
            let mut left = info.rcWindow.left;
            let mut top = info.rcWindow.top;
            let mut width = info.rcWindow.right - info.rcWindow.left;
            let mut height = info.rcWindow.bottom - info.rcWindow.top;
            if SAVED.as_ref().unwrap().contains_key(&name) {
                checked = true;
                let saved = SAVED.as_ref().unwrap().get(&name).unwrap();
                left = saved.left;
                top = saved.top;
                width = saved.width;
                height = saved.height;
                if !ROWCLICK {
                    let _ = MoveWindow(hwnd, left, top, width, height, true);
                }
            }
            LIST.as_mut().unwrap().insert(
                name.clone(),
                Item {
                    hwnd: hwnd.0 as u32,
                    title: text.into(),
                    checked,
                    left,
                    top,
                    width,
                    height,
                    name: name.into(),
                },
            );
        }

        true.into()
    }
}
