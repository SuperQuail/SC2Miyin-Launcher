//! 平台相关实现：单实例锁、外部浏览器、资源管理器、WebView2 调教、崩溃现场记录。
//!
//! 这里只放和具体操作系统打交道的东西，业务规则都在 `miyin-core` / `commands` 里。

use std::path::Path;

use crate::{CRASH_LOG, PANIC_QUIET};

/// 当前进程的完整性级别（Windows RID）：0x1000 = Low、0x2000 = Medium、0x3000 = High。
///
/// 解压在**低完整性目录**里的 exe 会被 Windows 以低完整性启动；低完整性进程写不了
/// `%LOCALAPPDATA%` 里的 WebView2 profile，浏览器进程必崩。启动前用它给一句人话，
/// 比让用户看到“窗口没建出来”好排查。
#[cfg(windows)]
pub(crate) fn process_integrity_level() -> Option<u32> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
        TOKEN_QUERY, TokenIntegrityLevel,
    };
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // SAFETY: 只查询当前进程令牌；缓冲区按 GetTokenInformation 返回的长度分配，
    // SID 指针来自该缓冲区，生命周期覆盖随后的取值。
    unsafe {
        let mut token = HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return None;
        }

        let mut size = 0u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut size);
        if size == 0 {
            let _ = CloseHandle(token);
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let ok = GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr() as *mut _),
            size,
            &mut size,
        )
        .is_ok();
        let level = if ok {
            let label = &*(buffer.as_ptr() as *const TOKEN_MANDATORY_LABEL);
            let sid = label.Label.Sid;
            let count = *GetSidSubAuthorityCount(sid);
            Some(*GetSidSubAuthority(sid, u32::from(count - 1)))
        } else {
            None
        };
        let _ = CloseHandle(token);
        level
    }
}

/// 进程是否以低于 Medium 的完整性运行（典型：exe 放在低完整性目录）。
#[cfg(windows)]
pub(crate) fn is_low_integrity() -> bool {
    process_integrity_level().is_some_and(|rid| rid < 0x2000)
}

/// 单实例：拿一个 `Local\net.miyin.launcher.single` 命名内核对象。
///
/// 返回 true = 拿到锁，可以继续启动；false = 已经有启动器在跑。
/// 句柄**故意不关**（Windows 句柄不随变量销毁而关闭，进程退出时系统回收）——
/// 提前 CloseHandle 就等于把锁放了。
#[cfg(windows)]
pub(crate) fn claim_single_instance() -> bool {
    use windows::Win32::Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, SetLastError, WIN32_ERROR,
    };
    use windows::Win32::System::Threading::{CreateEventW, CreateMutexW};
    use windows::core::w;

    // SAFETY: 对象名是静态 NUL 结尾字符串；拿到的句柄不释放（见上），
    // 「已有实例」分支用完立刻关掉。
    unsafe {
        SetLastError(WIN32_ERROR(0));
        if let Ok(handle) = CreateMutexW(None, true, w!("Local\\net.miyin.launcher.single")) {
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return false;
            }
            // 句柄不再关闭：关掉就等于释放锁。Windows 句柄不随变量销毁而关闭，
            // 进程退出时由系统统一回收。
            return true;
        }

        // CreateMutexW 用不了（极老系统 / 受限沙箱）时退回事件对象
        SetLastError(WIN32_ERROR(0));
        match CreateEventW(None, false, false, w!("Local\\net.miyin.launcher.single")) {
            Ok(handle) => {
                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = CloseHandle(handle);
                    return false;
                }
                // 同上：锁要跟着进程活
                true
            }
            Err(error) => {
                // 连内核对象都建不出来：宁可放过（否则用户永远打不开），只记一笔
                eprintln!("[miyin] 单实例锁拿不到：{error}");
                true
            }
        }
    }
}

/// 非 Windows：不做单实例限制。
#[cfg(not(windows))]
pub(crate) fn claim_single_instance() -> bool {
    true
}

/// 把已有启动器的窗口拎到前台。找不到窗口返回 false。
#[cfg(windows)]
pub(crate) fn focus_existing_window() -> bool {
    use windows::Win32::UI::WindowsAndMessaging::{
        FindWindowW, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
    };
    use windows::core::w;

    // SAFETY: 窗口标题是静态 NUL 结尾字符串；拿到的 HWND 只用来显示 / 置前。
    unsafe {
        let Ok(window) = FindWindowW(None, w!("弥音启动器")) else {
            return false;
        };
        let _ = ShowWindow(window, SW_RESTORE);
        let _ = ShowWindow(window, SW_SHOW);
        let _ = SetForegroundWindow(window);
        true
    }
}

/// 非 Windows：没有窗口可找。
#[cfg(not(windows))]
pub(crate) fn focus_existing_window() -> bool {
    false
}

/// 已有实例、又找不到它的窗口：说清楚再退出。
pub(crate) fn notify_already_running() {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Info)
        .set_title("弥音启动器")
        .set_description("启动器已经在运行了。\n\n如果没看到它的窗口，去任务栏里找一下。")
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// 用系统默认浏览器打开链接。
#[cfg(windows)]
pub(crate) fn open_in_browser(url: &str) -> std::io::Result<()> {
    // 不能直接 `start url`：cmd 会把 & 当分隔符。用 rundll32 更稳
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
}

#[cfg(not(windows))]
pub(crate) fn open_in_browser(url: &str) -> std::io::Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .spawn()
        .map(|_| ())
}

/// Windows：打开这个目录，或者**在它的文件夹里选中它**。
///
/// **不走 explorer 命令行** —— 那条路我赌错了两次（`/select,` 的参数解析、以及给 arg 手工加引号
/// 会把引号字符本身传进去，于是 explorer 拿到无效参数、**静默回退到「文档」**）。
/// 现在走 Shell API：目录用 `ShellExecuteW("open")`，文件用 `SHOpenFolderAndSelectItems`。
/// 失败会**明确报错**，不会再悄悄打开一个不相干的地方。
#[cfg(windows)]
pub(crate) fn reveal_in_file_manager(target: &Path) -> Result<(), String> {
    use windows::Win32::UI::Shell::{ILCreateFromPathW, SHOpenFolderAndSelectItems, ShellExecuteW};
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    use windows::core::PCWSTR;

    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let buffer = wide(target.as_os_str());

    if target.is_dir() {
        // 目录：等价于在资源管理器里双击它
        // SAFETY: buffer 以 NUL 结尾、在调用期间存活；其余参数传 null 表示用默认值。
        let result = unsafe {
            ShellExecuteW(
                None,
                windows::core::w!("open"),
                PCWSTR(buffer.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        // 约定：返回值 > 32 才算成功
        return if result.0 as isize > 32 {
            Ok(())
        } else {
            Err(format!("无法打开目录：{}", target.display()))
        };
    }

    // 文件：在它所在的文件夹里选中它
    // SAFETY: pidl 由系统分配，用完立刻 CoTaskMemFree；SHOpenFolderAndSelectItems 期间 buffer 存活。
    unsafe {
        let pidl = ILCreateFromPathW(PCWSTR(buffer.as_ptr()));
        if pidl.is_null() {
            return Err(format!("无法定位这个文件：{}", target.display()));
        }
        let outcome = SHOpenFolderAndSelectItems(pidl, None, 0);
        windows::Win32::System::Com::CoTaskMemFree(Some(pidl as *const _));
        outcome.map_err(|error| format!("无法在资源管理器里选中它：{error}"))
    }
}

/// 其它平台暂不支持。
#[cfg(not(windows))]
pub(crate) fn reveal_in_file_manager(_target: &Path) -> Result<(), String> {
    Err("当前平台暂不支持打开资源管理器".to_string())
}

/// 让 webview 不像浏览器：关掉 WebView2 自带的**浏览器加速键**（Ctrl+F/P/U/R、F5、缩放…）。
///
/// **不碰右键菜单** —— 那是页面自己拦的（两个入口都有 `onContextMenu`，AGENTS §18.2），
/// 在这里再关一遍是重复的一层。
///
/// 为什么不用 Tauri 的接口：Tauri 2.11 没有把 wry 的 `with_browser_accelerator_keys` /
/// `with_default_context_menus` 暴露出来（wry 0.55 内部有）；这里顺着 `with_webview`
/// 拿到 `ICoreWebView2Controller`，自己去关 —— 这是**唯一**能把浏览器快捷键彻底摘掉的地方，
/// 页面里的 keydown 拦截只盖得住一部分（窗口级组合键拦不住）。
///
/// 关掉 ≠ 快捷键没了：要用的快捷键改走原生菜单 accelerators，或页面自己的 keydown
/// （AGENTS §18.2）。失败也不影响功能：页面那层还兜着（`ui/src/api/webview.ts`）。
#[cfg(windows)]
pub(crate) fn tame_webview(webview: &tauri::webview::PlatformWebview) {
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
    use windows::core::Interface;

    // SAFETY: 回调运行在 WebView2 自己的 UI 线程上，controller 在回调期间有效。
    unsafe {
        let Ok(core) = webview.controller().CoreWebView2() else {
            return;
        };
        let Ok(settings) = core.Settings() else {
            return;
        };
        match settings.cast::<ICoreWebView2Settings3>() {
            Ok(settings3) => {
                if let Err(error) = settings3.SetAreBrowserAcceleratorKeysEnabled(false) {
                    eprintln!("[miyin] 关 WebView2 浏览器加速键失败：{error}");
                }
            }
            Err(error) => eprintln!("[miyin] WebView2 太旧，拿不到 Settings3：{error}"),
        }
    }
}

/// 往 exe 同级的 `miyin-crash.log` 追加一段现场。写不进去就算了 —— 这里**绝不能**再出事。
fn append_crash_log(text: &str) {
    let Some(path) = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(|dir| dir.join(CRASH_LOG)))
    else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    use std::io::Write;

    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|span| span.as_secs())
        .unwrap_or(0);
    let _ = writeln!(file, "==== {at}\n{text}");
}

/// 启动失败时弹的原生框。
///
/// release 是 `windows_subsystem = "windows"`（没有控制台），Tauri 起不来时
/// 只写 stderr 等于没写；用户看到的会是「双击没反应」或「窗口一闪就没、进程还在」。
/// 只用在最后一道出口，弹完调用方就退出。
pub(crate) fn fatal_dialog(text: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
    use windows::core::HSTRING;

    let text = HSTRING::from(text);
    let caption = HSTRING::from("弥音启动器");
    // SAFETY: 纯 Win32 调用，窗口句柄为空；失败也不影响调用方的退出流程。
    unsafe {
        let _ = MessageBoxW(None, &text, &caption, MB_OK | MB_ICONERROR);
    }
}

/// panic 的载荷一般是 &str 或 String，别的类型就只能含糊一点了。
pub(crate) fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "（panic 没有文本信息）".to_string()
    }
}

/// 装 panic 现场记录：写进 exe 同级的 `miyin-crash.log`，再弹一个原生框。
///
/// 为什么要它：release 版是 `windows_subsystem = "windows"`（main.rs:2），没有控制台，
/// stderr 是黑洞。用户报「白屏 / 双击没反应」时，这个文件是唯一能拿到现场的东西。
///
/// 规矩：hook 里只做加法、失败一律忽略 —— 在这里再 panic 就是递归，会直接把进程带走。
pub(crate) fn install_crash_hook() {
    std::panic::set_hook(Box::new(|info| {
        let name = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        let location = info
            .location()
            .map(|spot| format!("{}:{}:{}", spot.file(), spot.line(), spot.column()))
            .unwrap_or_else(|| "位置未知".to_string());
        let message = panic_text(info.payload());
        let text = format!("[线程 {name}] {location}\n{message}");

        eprintln!("[miyin] panic：{text}");
        append_crash_log(&text);

        // 后台启动线程的 panic 有更合适的出口（boot_ready -> 界面上那页失败说明）
        if PANIC_QUIET.try_with(std::cell::Cell::get).unwrap_or(false) {
            return;
        }
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("弥音启动器出错了")
            .set_description(format!(
                "启动器遇到了一个没处理的问题。原因已经写进程序目录的 {CRASH_LOG}，\n\
                 反馈时可以把它一起发给我们。\n\n{text}"
            ))
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }));
}

/// 绿色版不带安装器：这台机器上可能压根没有 WebView2 运行时。
///
/// 缺它时 Tauri 要到**建窗口**那一步才会失败（tauri-runtime-wry 2.11.4 src/lib.rs:4757
/// 会先弹一个英文框），随后整个进程 panic —— 用户看到的是「闪一下就没」。
/// 所以进 Tauri 之前先问一句：缺了交给 [`crate::webview2_install`] 自己补上。
///
/// 探测优先用 Tauri 的官方接口（`tauri::webview_version`，走 WebView2 自己的加载器）；
/// 它读不到时再查注册表 —— 运行时装了但 API 探测拿不到版本（少数精简 / 多用户配置的系统）时，
/// 不该把人赶去重装一遍。
#[cfg(windows)]
pub(crate) fn webview2_version() -> Option<String> {
    tauri::webview_version().ok().or_else(registry_version)
}

/// 注册表兜底：WebView2 运行时在 EdgeUpdate 客户端里登记着固定的产品 GUID。
#[cfg(windows)]
fn registry_version() -> Option<String> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

    const CLIENT: &str = r"Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}";
    // 机器级装在 WOW6432Node 下（安装器是 32 位）；用户级装在 HKCU
    for (root, prefix) in [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\"),
        (HKEY_CURRENT_USER, r"SOFTWARE\"),
    ] {
        let Ok(key) = RegKey::predef(root).open_subkey(format!("{prefix}{CLIENT}")) else {
            continue;
        };
        let Ok(version) = key.get_value::<String, _>("pv") else {
            continue;
        };
        let version = version.trim();
        if !version.is_empty() && version != "0.0.0.0" {
            return Some(version.to_string());
        }
    }
    None
}
