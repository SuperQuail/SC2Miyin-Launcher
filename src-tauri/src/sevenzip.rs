//! 内嵌 7-Zip（LGPL-2.1+）：导入压缩包时不再依赖用户机器上装没装 7-Zip，
//! 「工具」里的解压界面也直接用 7-Zip 自己的窗口（嵌入到启动器窗口里）。
//!
//! 资产放在 src-tauri/assets/7zip/，构建时 include_bytes! 进 exe；
//! 运行时解到 %LOCALAPPDATA%\net.miyin.launcher\tools\7zip\<版本>\（用户目录可写，
//! 不碰游戏数据根），并通过 MIYIN_7Z 告诉 core 用哪个 7z.exe。
//!
//! 许可：7-Zip 是 LGPL-2.1+，随包附带 License.txt。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

const VERSION: &str = "2605";

/// 解压工具窗口的调试日志（release 没控制台，出问题只能靠它）。
fn log_line(text: &str) {
    let Some(base) = std::env::var_os("LOCALAPPDATA") else {
        return;
    };
    let path = PathBuf::from(base)
        .join("net.miyin.launcher")
        .join("sevenzip-tool.log");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{text}");
    }
}

struct Asset {
    name: &'static str,
    bytes: &'static [u8],
}

const ASSETS: &[Asset] = &[
    Asset {
        name: "7z.exe",
        bytes: include_bytes!("../assets/7zip/7z.exe"),
    },
    Asset {
        name: "7z.dll",
        bytes: include_bytes!("../assets/7zip/7z.dll"),
    },
    Asset {
        name: "7zFM.exe",
        bytes: include_bytes!("../assets/7zip/7zFM.exe"),
    },
    Asset {
        name: "7zG.exe",
        bytes: include_bytes!("../assets/7zip/7zG.exe"),
    },
    Asset {
        name: "License.txt",
        bytes: include_bytes!("../assets/7zip/License.txt"),
    },
    Asset {
        name: "readme.txt",
        bytes: include_bytes!("../assets/7zip/readme.txt"),
    },
    Asset {
        name: "Lang/zh-cn.txt",
        bytes: include_bytes!("../assets/7zip/Lang/zh-cn.txt"),
    },
];

#[derive(Clone)]
pub(crate) struct SevenZip {
    pub console: PathBuf,
    pub gui: PathBuf,
}

/// 首次调用时把内嵌的 7-Zip 解出来；之后复用。
pub(crate) fn ensure() -> Result<SevenZip, String> {
    static CACHE: OnceLock<Result<SevenZip, String>> = OnceLock::new();
    CACHE.get_or_init(prepare).clone()
}

fn prepare() -> Result<SevenZip, String> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "找不到 %LOCALAPPDATA%".to_string())?;
    let dir = base
        .join("net.miyin.launcher")
        .join("tools")
        .join(format!("7zip-{VERSION}"));

    let marker = dir.join(".ready");
    let missing = !marker.is_file();
    std::fs::create_dir_all(&dir).map_err(|e| format!("建 7-Zip 目录失败：{e}"))?;

    for asset in ASSETS {
        let path = dir.join(asset.name);
        if path.is_file() && !missing {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("建目录失败：{e}"))?;
        }
        std::fs::write(&path, asset.bytes).map_err(|e| format!("写 {} 失败：{e}", asset.name))?;
    }
    let _ = std::fs::write(&marker, VERSION.as_bytes());

    let console = dir.join("7z.exe");
    let gui = dir.join("7zFM.exe");
    if !console.is_file() || !gui.is_file() {
        return Err("内嵌 7-Zip 解压失败".to_string());
    }

    // SAFETY: 这条在 run() 启动早期调用（起线程/Tauri 之前），没有并发读环境变量。
    unsafe {
        std::env::set_var("MIYIN_7Z", &console);
    }

    Ok(SevenZip { console, gui })
}

/// 启动入口里调用：失败只记日志，不拦启动（导入时还有系统 7z 兜底）。
pub(crate) fn prepare_for_startup() {
    match ensure() {
        Ok(paths) => eprintln!("[miyin] 内嵌 7-Zip：{}", paths.console.display()),
        Err(error) => eprintln!("[miyin] 内嵌 7-Zip 不可用：{error}"),
    }
}

/// 顶栏高度（与 UI 的 --header-height 一致）：7-Zip 只占它下面的区域。
const TOPBAR_HEIGHT: f64 = 48.0;

/// 当前嵌进主窗口的 7-Zip 子窗口（子窗口句柄, 顶栏像素高度）。None = 没开。
static TOOL_CHILD: std::sync::Mutex<Option<(isize, i32)>> = std::sync::Mutex::new(None);

/// 打开「解压工具」：把 7-Zip 文件管理器窗口嵌进主窗口**顶栏下方**的区域。
///
/// 顶栏（我们的标题栏 / 导航）继续由 WebView2 渲染；WebView2 用窗口区域裁到只剩顶栏，
/// 这样 7-Zip 的 DirectComposition 不会把顶栏盖掉。关闭时恢复整块 webview。
#[tauri::command(async)]
pub(crate) fn open_sevenzip(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;

    log_line("open_sevenzip: start");
    ensure().map_err(|error| {
        log_line(&format!("open_sevenzip: ensure failed: {error}"));
        error
    })?;

    let Some(main) = app.get_webview_window("main") else {
        return Err("主窗口还没准备好".to_string());
    };
    let host = main
        .hwnd()
        .map_err(|error| {
            log_line(&format!("open_sevenzip: no main hwnd: {error}"));
            format!("拿不到主窗口句柄：{error}")
        })?
        .0 as isize;
    let top = (TOPBAR_HEIGHT * main.scale_factor().unwrap_or(1.0)).round() as i32;

    // 已经开着：重新铺一下并保持顶栏裁剪
    if let Ok(mut slot) = TOOL_CHILD.lock() {
        if let Some((child, top)) = *slot {
            if is_window(child) {
                place_overlay(host, child, top);
                clip_webview(host, top);
                return Ok(());
            }
            *slot = None;
        }
    }

    let gui = ensure()?.gui;
    let app_handle = app.clone();
    std::thread::spawn(move || embed_gui(app_handle, host, top, gui));
    Ok(())
}

/// 关闭 7-Zip（顶栏「退出 7-Zip」按钮用）：只发 WM_CLOSE，收尾交给监控线程。
#[tauri::command(async)]
pub(crate) fn close_sevenzip() -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};

        if let Ok(slot) = TOOL_CHILD.lock() {
            if let Some((child, _)) = *slot {
                unsafe {
                    let _ =
                        PostMessageW(Some(HWND(child as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
                }
            }
        }
    }
    Ok(())
}

/// 后台线程：拉起 7zFM，SetParent 到顶栏下方，并跟随窗口尺寸。
fn embed_gui(app: tauri::AppHandle, host: isize, top: i32, gui: PathBuf) {
    use tauri::Emitter;

    let mut child = match Command::new(&gui)
        .current_dir(gui.parent().unwrap_or(Path::new(".")))
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            log_line(&format!("embed: spawn failed: {error}"));
            return;
        }
    };
    log_line(&format!("embed: spawned pid={}", child.id()));

    let pid = child.id();
    let gui_hwnd = match wait_for_window(pid, 50) {
        Some(hwnd) => hwnd,
        None => {
            log_line("embed: 7-Zip window not found");
            let _ = child.kill();
            return;
        }
    };

    if !embed_window(host, gui_hwnd) {
        log_line("embed: SetParent failed");
    } else {
        log_line("embed: SetParent ok");
        place_overlay(host, gui_hwnd, top);
        clip_webview(host, top);
        if let Ok(mut slot) = TOOL_CHILD.lock() {
            *slot = Some((gui_hwnd, top));
        }
        let _ = app.emit("sevenzip://open", true);
    }

    loop {
        if !is_window(host) {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(_) => break,
        }
        place_overlay(host, gui_hwnd, top);
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    restore_webview(host);
    if let Ok(mut slot) = TOOL_CHILD.lock() {
        *slot = None;
    }
    let _ = app.emit("sevenzip://open", false);
    log_line("embed: closed");
}

/// 找宿主窗口里某个类名的子窗口（WebView2 宿主叫 WRY_WEBVIEW）。
#[cfg(windows)]
fn find_child_class(host: isize, class: &str) -> Option<isize> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetClassNameW};
    use windows::core::BOOL;

    struct Search<'a> {
        class: &'a str,
        found: Option<isize>,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let search = &mut *(lparam.0 as *mut Search);
            let mut buffer = [0u16; 64];
            let len = GetClassNameW(hwnd, &mut buffer);
            let name = String::from_utf16_lossy(&buffer[..len.max(0) as usize]);
            if name == search.class {
                search.found = Some(hwnd.0 as isize);
                return BOOL(0);
            }
            BOOL(1)
        }
    }

    let mut search = Search { class, found: None };
    unsafe {
        let _ = EnumChildWindows(
            Some(HWND(host as *mut _)),
            Some(callback),
            LPARAM(&mut search as *mut _ as isize),
        );
    }
    search.found
}

#[cfg(not(windows))]
fn find_child_class(_host: isize, _class: &str) -> Option<isize> {
    None
}

/// 把 webview 裁到只剩顶栏：WebView2 的 DirectComposition 会盖住 7-Zip，
/// 裁掉顶栏以下的部分之后，7-Zip 只在下方区域显示，顶栏仍由我们的页面渲染。
#[cfg(windows)]
fn clip_webview(host: isize, top: i32) {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, SetWindowRgn};
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    let Some(webview) = find_child_class(host, "WRY_WEBVIEW") else {
        log_line("embed: WRY_WEBVIEW not found");
        return;
    };
    let mut rect = RECT::default();
    unsafe {
        if GetClientRect(HWND(host as *mut _), &mut rect).is_err() {
            return;
        }
        let width = rect.right - rect.left;
        let region = CreateRectRgn(0, 0, width, top);
        if region.is_invalid() {
            log_line("embed: CreateRectRgn failed");
            return;
        }
        // 区域交给系统，成功后 windows 自己负责释放。
        let _ = SetWindowRgn(HWND(webview as *mut _), Some(region), true);
        log_line("embed: webview clipped to topbar");
    }
}

#[cfg(not(windows))]
fn clip_webview(_host: isize, _top: i32) {}

/// 恢复 webview 的整块区域。
#[cfg(windows)]
fn restore_webview(host: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::SetWindowRgn;

    if let Some(webview) = find_child_class(host, "WRY_WEBVIEW") {
        unsafe {
            let _ = SetWindowRgn(HWND(webview as *mut _), None, true);
        }
        log_line("embed: webview region restored");
    }
}

#[cfg(not(windows))]
fn restore_webview(_host: isize) {}

/// 等 7zFM 的主窗口（类名 7-Zip::FM）出现，最多约 5 秒。
fn wait_for_window(pid: u32, tries: usize) -> Option<isize> {
    for _ in 0..tries {
        if let Some(hwnd) = find_window(pid) {
            return Some(hwnd);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    None
}

#[cfg(windows)]
fn find_window(_pid: u32) -> Option<isize> {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowThreadProcessId,
    };
    use windows::core::BOOL;

    struct Search {
        pid: u32,
        found: Option<isize>,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let search = &mut *(lparam.0 as *mut Search);
            let mut owner = 0u32;
            let _ = GetWindowThreadProcessId(hwnd, Some(&mut owner));
            if owner != search.pid {
                return BOOL(1);
            }
            let mut class = [0u16; 64];
            let len = GetClassNameW(hwnd, &mut class);
            let name = String::from_utf16_lossy(&class[..len.max(0) as usize]);
            if name == "7-Zip::FM" {
                search.found = Some(hwnd.0 as isize);
                return BOOL(0);
            }
            BOOL(1)
        }
    }

    let mut search = Search {
        pid: _pid,
        found: None,
    };
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut search as *mut _ as isize));
    }
    search.found
}

#[cfg(not(windows))]
fn find_window(_pid: u32) -> Option<isize> {
    None
}

#[cfg(windows)]
fn embed_window(host: isize, child: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_STYLE, GetWindowLongW, HWND_TOP, SW_SHOW, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE,
        SWP_SHOWWINDOW, SetParent, SetWindowLongW, SetWindowPos, ShowWindow, WS_CAPTION, WS_CHILD,
        WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_POPUP, WS_SYSMENU, WS_THICKFRAME, WS_VISIBLE,
    };

    let host = HWND(host as *mut _);
    let child = HWND(child as *mut _);
    unsafe {
        let style = GetWindowLongW(child, GWL_STYLE);
        let cleaned = style
            & !(WS_POPUP.0 as i32
                | WS_CAPTION.0 as i32
                | WS_THICKFRAME.0 as i32
                | WS_MINIMIZEBOX.0 as i32
                | WS_MAXIMIZEBOX.0 as i32
                | WS_SYSMENU.0 as i32);
        let new_style = cleaned | WS_CHILD.0 as i32 | WS_VISIBLE.0 as i32;
        SetWindowLongW(child, GWL_STYLE, new_style);
        if SetParent(child, Some(host)).is_err() {
            return false;
        }
        let _ = SetWindowPos(
            child,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_SHOWWINDOW | SWP_NOMOVE | SWP_NOSIZE,
        );
        let _ = ShowWindow(child, SW_SHOW);
        true
    }
}

#[cfg(not(windows))]
fn embed_window(_host: isize, _child: isize) -> bool {
    false
}

/// 把 7-Zip 子窗口铺到顶栏下方，并保持在最上层子窗口位置。
#[cfg(windows)]
fn place_overlay(host: isize, child: isize, top: i32) {
    use windows::Win32::Foundation::{HWND, RECT};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClientRect, HWND_TOP, SWP_NOACTIVATE, SWP_SHOWWINDOW, SetWindowPos,
    };

    let host = HWND(host as *mut _);
    let child = HWND(child as *mut _);
    let mut rect = RECT::default();
    unsafe {
        if GetClientRect(host, &mut rect).is_err() {
            return;
        }
        let width = rect.right - rect.left;
        let height = (rect.bottom - rect.top - top).max(0);
        let _ = SetWindowPos(
            child,
            Some(HWND_TOP),
            0,
            top,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }
}

#[cfg(not(windows))]
fn place_overlay(_host: isize, _child: isize, _top: i32) {}

#[cfg(windows)]
fn is_window(hwnd: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::IsWindow;
    unsafe { IsWindow(Some(HWND(hwnd as *mut _))).as_bool() }
}

#[cfg(not(windows))]
fn is_window(_hwnd: isize) -> bool {
    false
}
