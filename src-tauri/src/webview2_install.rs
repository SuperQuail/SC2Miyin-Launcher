//! 缺 WebView2 运行时时的自动补全：原生 Win32 进度窗 + 后台线程下载 / 安装。
//!
//! 为什么进度不能用 Tauri 的界面：**没有 WebView2 就建不出窗口**，所以这段 UI 只能自己画
//! （STATIC 说明 + PROGRESS_CLASS 进度条 + 取消按钮）。下载与安装都在后台线程，
//! UI 线程只跑消息循环；进度写在共享状态里，后台线程用 WM_USER 消息叫 UI 刷新。
//!
//! 流程：检测 → 流式下载微软官方 Evergreen 引导程序 → 静默安装（必要时 UAC 提权）→
//! 轮询运行时是否就绪。安装源与失败排查见 AGENTS.md 第 14.9 节。

use std::ffi::c_void;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    COLOR_WINDOW, DEFAULT_GUI_FONT, GetStockObject, HBRUSH, UpdateWindow,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, PBM_SETMARQUEE, PBM_SETPOS,
    PBS_MARQUEE, PROGRESS_CLASSW,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
    DispatchMessageW, GWL_STYLE, GWLP_USERDATA, GetMessageW, GetWindowLongPtrW, GetWindowLongW,
    HMENU, IDC_ARROW, LoadCursorW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK, MESSAGEBOX_STYLE, MSG,
    MessageBoxW, PostMessageW, PostQuitMessage, RegisterClassW, SW_SHOW, SW_SHOWNORMAL,
    SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SendMessageW, SetTimer,
    SetWindowLongPtrW, SetWindowLongW, SetWindowPos, SetWindowTextW, ShowWindow, TranslateMessage,
    WINDOW_EX_STYLE, WM_CLOSE, WM_COMMAND, WM_KEYDOWN, WM_SETFONT, WM_TIMER, WM_USER, WNDCLASSW,
    WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU, WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

/// 微软官方 Evergreen 引导程序（会 302 到 MicrosoftEdgeWebview2Setup.exe）。
///
/// 失败文案里也用它 —— 用户点开就是官方下载页，不用我们再解释一遍。
const SOURCE_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
/// 安装包落地的文件名（放临时目录）。
const SETUP_NAME: &str = "miyin-webview2-setup.exe";
/// 下载块大小。
const CHUNK: usize = 64 * 1024;
/// 安装完成后最多等多久：UAC 那条路是异步的，用户可能还要花时间点「是」。
const INSTALL_WAIT: Duration = Duration::from_secs(180);
/// 后台线程拿来叫 UI 刷新的两个消息（WM_USER 之后都是自己的）。
const WM_APP_UPDATE: u32 = WM_USER;
const WM_APP_DONE: u32 = WM_USER + 1;
/// 收尾用的延时器编号。
const DONE_TIMER: usize = 1;
/// 取消按钮的控件 ID。
const CANCEL_ID: usize = 1001;
/// 后台线程留给 UI 的结论：Ok 表示可以继续启动。
type Outcome = Result<(), String>;

/// 后台线程报给 UI 的阶段。
#[derive(Clone, Copy, Default)]
enum Stage {
    #[default]
    Checking,
    Downloading,
    Installing,
    Done,
}

/// 进度快照：后台线程写、UI 线程读。
#[derive(Clone, Copy, Default)]
struct Progress {
    stage: Stage,
    /// 已下载字节。
    done: u64,
    /// 总字节；拿不到（分块传输）就是 None —— 进度条走不确定态。
    total: Option<u64>,
}

/// 启动时调用：缺运行时就把环境补上。返回 Ok(()) 表示可以继续建窗口。
///
/// 窗口建不出来（极少数被安全软件拦了 User32 的机器）时退回消息框：至少说清正在做什么、
/// 失败时给原因和手动下载地址。
pub(crate) fn ensure_runtime() -> Result<(), String> {
    let progress = Arc::new(Mutex::new(Progress::default()));
    let cancel = Arc::new(AtomicBool::new(false));
    let result = Arc::new(Mutex::new(None));

    let Ok(window) = create_window(progress.clone(), cancel.clone(), result.clone()) else {
        eprintln!("[miyin] 原生进度窗没能建出来，退回消息框");
        return headless(progress, cancel);
    };

    // 先自己刷一次：让「正在检测」那一行与跑马灯在消息循环一开跑就出现在屏幕上
    post(window, WM_APP_UPDATE);

    // 后台线程：动网络、动安装程序。它只写共享状态 + PostMessage，不碰别的东西。
    //
    // HWND 不是 Send，所以传地址过去、在线程里还原 —— PostMessage 本来就允许跨线程投递，
    // 窗口在消息循环结束前一直有效（提前销毁也只是投递失败，不会崩）。
    let window_addr = window.0 as usize;
    {
        let (progress, cancel, result) = (progress.clone(), cancel.clone(), result.clone());
        std::thread::spawn(move || {
            // SAFETY: 这个地址指向本进程里由 UI 线程创建的窗口，只拿来做 PostMessage。
            let window = HWND(window_addr as *mut c_void);
            let tick = || post(window, WM_APP_UPDATE);
            let outcome = work(&progress, &cancel, &tick);
            if let Ok(mut slot) = result.lock() {
                *slot = Some(outcome);
            }
            post(window, WM_APP_DONE);
        });
    }

    // UI 线程跑消息循环，直到后台线程报完成、或用户取消 / 关窗。
    let mut message = MSG::default();
    // SAFETY: 标准消息循环；GetMessageW 在收到 WM_QUIT 时返回 0，出错返回 -1。
    while unsafe { GetMessageW(&mut message, None, 0, 0).0 } > 0 {
        // SAFETY: message 已经由 GetMessageW 填好，只在当前线程使用。
        unsafe {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    // SAFETY: 窗口是当前线程建的，可以在当前线程销毁；销毁后后台线程的 PostMessage 只会失败。
    unsafe {
        let _ = DestroyWindow(window);
    }

    // 取消优先：就算后台线程紧接着也成功了，用户点了停就停。
    if cancel.load(Ordering::SeqCst) {
        return Err("已取消安装 WebView2 运行时".to_string());
    }
    match result.lock().ok().and_then(|slot| slot.clone()) {
        Some(Ok(())) => Ok(()),
        Some(Err(reason)) => Err(reason),
        None => Err("下载 / 安装线程意外结束，没能判断结果".to_string()),
    }
}

/// 调试开关：这台机器**已经装了**运行时，用它强制再走一遍安装界面，验证文案与进度。
///
/// 设 MIYIN_WEBVIEW2_DEMO=1 才生效；它只跳过「真的跑安装程序」那一步（见 work），
/// 其余（解析地址、下载、进度、收尾）都是真实流程。release 里也留着 —— 不设环境变量就没人踩到。
pub(crate) fn demo_mode() -> bool {
    std::env::var("MIYIN_WEBVIEW2_DEMO").is_ok_and(|value| value == "1")
}

/// 整个「下载 + 安装」的流程，跑在后台线程上。返回失败原因（已经能直接念给用户听）。
fn work(progress: &Mutex<Progress>, cancel: &AtomicBool, tick: &dyn Fn()) -> Result<(), String> {
    let publish = |stage: Stage, done: u64, total: Option<u64>| {
        if let Ok(mut slot) = progress.lock() {
            *slot = Progress { stage, done, total };
        }
        tick();
    };

    let setup = download(cancel, &publish)?;

    publish(Stage::Installing, 0, None);
    if demo_mode() {
        // 调试开关：只走界面与文案，不真跑安装程序（这台机器已经装了，跑也只会立刻退出）。
        std::thread::sleep(Duration::from_millis(900));
    } else {
        install(&setup)?;
        wait_ready(cancel)?;
    }
    let _ = std::fs::remove_file(&setup);

    publish(Stage::Done, 0, None);
    Ok(())
}

/// 流式下载官方引导程序，返回临时文件路径。
fn download(
    cancel: &AtomicBool,
    publish: &dyn Fn(Stage, u64, Option<u64>),
) -> Result<PathBuf, String> {
    // 代理走系统设置（环境变量 -> Windows 系统代理 -> 直连），和更新模块同一套判断，绝不写死。
    // shortcut: 这里读不到用户自己配的 network.json（那时 Tauri 还没起来），先按默认「自动」选。
    let settings = miyin_core::update::NetworkSettings::default();
    let client = miyin_core::update::net::build_client(
        miyin_core::update::net::detect_proxy(&settings).as_ref(),
        "miyin-launcher",
    )
    .map_err(|error| format!("建立下载连接失败：{error}"))?;

    // 直接 GET 官方短链，由 reqwest 跟随 302 到 MicrosoftEdgeWebview2Setup.exe ——
    // 落下来的最终地址就是真正的安装包（解析失败也照样下，回退就是这条短链本身）。
    let mut response = client
        .get(SOURCE_URL)
        .send()
        .map_err(|error| format!("连不上微软下载站（{SOURCE_URL}）：{error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "下载安装包失败（{SOURCE_URL}）：HTTP {}",
            response.status()
        ));
    }
    eprintln!("[miyin] WebView2 安装包地址：{}", response.url());

    let total = response.content_length();
    let (path, mut file) = setup_file()?;
    let mut buffer = vec![0u8; CHUNK];
    let mut written: u64 = 0;
    let mut last = Instant::now();

    loop {
        if cancel.load(Ordering::SeqCst) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err("已取消下载 WebView2 安装包".to_string());
        }
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("下载中断：{error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("写入临时文件失败（{}）：{error}", path.display()))?;
        written += read as u64;
        // 每 100ms 报一次就够（进度条 + 文案），别把消息队列刷爆
        if last.elapsed() >= Duration::from_millis(100) {
            last = Instant::now();
            publish(Stage::Downloading, written, total);
        }
    }

    file.flush()
        .map_err(|error| format!("写入临时文件失败（{}）：{error}", path.display()))?;
    drop(file);
    publish(Stage::Downloading, written, total);

    if written == 0 {
        return Err("下载到的安装包是 0 字节，请检查网络或代理设置".to_string());
    }
    Ok(path)
}

/// 安装包落地位置：先临时目录；临时目录写不进去（受限环境 / 沙箱）就退到 exe 旁边。
fn setup_file() -> Result<(PathBuf, std::fs::File), String> {
    let in_temp = std::env::temp_dir().join(SETUP_NAME);
    let beside_exe = std::env::current_exe()
        .ok()
        .map(|exe| exe.with_file_name(SETUP_NAME));

    for path in [Some(in_temp.clone()), beside_exe].into_iter().flatten() {
        match std::fs::File::create(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) => eprintln!("[miyin] 安装包写不进去 {}：{error}", path.display()),
        }
    }
    Err(format!(
        "临时目录和程序目录都写不进去，没法保存安装包（{}）。可以手动下载安装：{SOURCE_URL}",
        in_temp.display()
    ))
}

/// 静默安装：先直接跑（不需要提权的机器上就成了），740 或跑不通再走 UAC 提权。
fn install(setup: &Path) -> Result<(), String> {
    let mut last = String::new();
    for args in [["/silent", "/install"], ["--silent", "/install"]] {
        match std::process::Command::new(setup).args(args).status() {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => last = format!("安装程序退出码 {status}"),
            // 740 = ERROR_ELEVATION_REQUIRED：安装器要管理员权限，换参数也没用，直接提权
            Err(error) if error.raw_os_error() == Some(740) => break,
            Err(error) => last = format!("启动安装程序失败：{error}"),
        }
    }

    eprintln!("[miyin] 静默安装没直接成功（{last}），改用 UAC 提权");
    elevate(setup)
}

/// 用 ShellExecuteW 的 runas 提权跑安装器（会弹 UAC）。它不等进程结束 —— 完成靠 wait_ready 轮询。
fn elevate(setup: &Path) -> Result<(), String> {
    let file = wide(&setup.to_string_lossy());
    let parameters = wide("/silent /install");
    // SAFETY: 两个宽字符串在调用期间存活并以 NUL 结尾；其余参数都是字面量或空。
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(file.as_ptr()),
            PCWSTR(parameters.as_ptr()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // 约定：返回值 > 32 才算成功
    if result.0 as isize > 32 {
        return Ok(());
    }
    Err(format!(
        "提权安装没能启动（ShellExecute 返回 {}）：多半是在 UAC 里点了「否」，或被安全软件拦下。\
         手动下载安装：{SOURCE_URL}",
        result.0 as isize
    ))
}

/// 等运行时真正可用 —— 安装程序退出不等于装好了（UAC 那条路是异步的）。
fn wait_ready(cancel: &AtomicBool) -> Result<(), String> {
    let deadline = Instant::now() + INSTALL_WAIT;
    loop {
        if crate::platform::webview2_version().is_some() {
            return Ok(());
        }
        if cancel.load(Ordering::SeqCst) {
            return Err("已取消等待 WebView2 安装完成".to_string());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "安装程序已经结束，但系统里仍然找不到 WebView2 运行时（多半被安全软件拦下了）。\
                 手动下载安装：{SOURCE_URL}"
            ));
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// 窗口建不出来时的兜底：消息框说清楚在做什么，然后同步跑完。
fn headless(progress: Arc<Mutex<Progress>>, cancel: Arc<AtomicBool>) -> Result<(), String> {
    message_box(
        &format!(
            "这台机器上没有 WebView2 运行时，启动器正在自动下载安装。\n\n\
             一般一两分钟；中途如果弹出「用户账户控制」，请点「是」。\n\n\
             装不上可以手动下载：{SOURCE_URL}"
        ),
        MB_ICONINFORMATION,
    );
    let tick = || {};
    let outcome = work(&progress, &cancel, &tick);
    if let Err(reason) = &outcome {
        message_box(&failure_text(reason), MB_ICONERROR);
    }
    outcome
}

fn message_box(text: &str, icon: MESSAGEBOX_STYLE) {
    let text = wide(text);
    let caption = wide("弥音启动器");
    // SAFETY: 两个宽字符串在调用期间存活并以 NUL 结尾；父窗口传 None（还没有窗口）。
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_OK | icon,
        );
    }
}

/// 失败时留在窗口上的整段话：原因 + 手动链接 + 能动手的排查方向。
fn failure_text(reason: &str) -> String {
    format!(
        "自动安装 WebView2 运行时没有成功。\n\n\
         原因：{reason}\n\n\
         可以手动下载安装（微软官方）：{SOURCE_URL}\n\n\
         排查建议：\n\
         · 安装时弹出「用户账户控制」请点「是」，点了「否」就装不上；\n\
         · 被杀毒软件 / 安全卫士拦下的话，先允许它，或者右键启动器「以管理员身份运行」再试；\n\
         · 装好之后重新打开启动器即可。\n\n\
         按任意键退出。"
    )
}

/// 一句话状态 + 进度百分比（None = 总长未知，进度条走不确定态）。
fn describe(progress: &Progress) -> (String, Option<u16>) {
    let mb = |bytes: u64| bytes as f64 / (1024.0 * 1024.0);
    match progress.stage {
        // 检测阶段给 0%：进度条先老实地停在那儿。
        // 不给 None —— 那会让进度条进跑马灯，而跑马灯切回定点后 PBM_SETPOS 画不准（实测）。
        Stage::Checking => (
            "正在检测 WebView2 运行时…\n\n没找到的话会自动下载安装，请别关掉这个窗口。".to_string(),
            Some(0),
        ),
        Stage::Downloading => match progress.total.filter(|total| *total > 0) {
            Some(total) => {
                let percent = (progress.done.min(total).saturating_mul(100) / total) as u16;
                (
                    format!(
                        "正在下载 WebView2 安装包（{percent}%，{:.1} MB / {:.1} MB）\n\n\
                         安装包来自微软官方，下完会自动安装，不需要你再点下一步。",
                        mb(progress.done),
                        mb(total)
                    ),
                    Some(percent),
                )
            }
            None => (
                format!(
                    "正在下载 WebView2 安装包（已下载 {:.1} MB，总大小未知）\n\n\
                     服务器没给大小，进度条会一直转 —— 耐心等一下。",
                    mb(progress.done)
                ),
                None,
            ),
        },
        // 下载完了，进度条停在满格 —— 安装期间没有可量化的进度
        Stage::Installing => (
            "正在安装 WebView2 运行时（可能弹系统 UAC，请在弹窗点「是」）\n\n\
             安装期间别关掉这个窗口，装完会自动继续启动。"
                .to_string(),
            Some(100),
        ),
        Stage::Done => ("安装完成，正在启动启动器…".to_string(), Some(100)),
    }
}

/// 窗口里的控件句柄 + 与后台线程共享的状态。
struct WindowState {
    hwnd: HWND,
    caption: HWND,
    bar: HWND,
    cancel: Arc<AtomicBool>,
    progress: Arc<Mutex<Progress>>,
    result: Arc<Mutex<Option<Outcome>>>,
}

/// 建窗口与控件。失败返回一句人话，调用方退回消息框。
fn create_window(
    progress: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
    result: Arc<Mutex<Option<Outcome>>>,
) -> Result<HWND, String> {
    // 初始文案（STATIC 的文字要用到调用结束，所以先绑成变量）
    let initial = wide(&describe(&Progress::default()).0);
    let title = wide("弥音启动器 - 正在补全 WebView2 运行环境");

    unsafe {
        // SAFETY: 建窗口与子控件都在当前线程；宽字符串在本作用域内一直活着。
        let instance =
            GetModuleHandleW(None).map_err(|error| format!("拿不到模块句柄：{error}"))?;
        let instance = HINSTANCE(instance.0);

        let _ = InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        });

        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            // 系统色背景：不用自己管画刷
            hbrBackground: HBRUSH((COLOR_WINDOW.0 as usize + 1) as *mut c_void),
            lpszClassName: w!("MiYinWebView2Setup"),
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err("注册窗口类失败".to_string());
        }

        let window = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("MiYinWebView2Setup"),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            576,
            340,
            None,
            None,
            Some(instance),
            None,
        )
        .map_err(|error| format!("创建窗口失败：{error}"))?;

        let caption = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            PCWSTR(initial.as_ptr()),
            WS_CHILD | WS_VISIBLE,
            16,
            16,
            528,
            200,
            Some(window),
            None,
            Some(instance),
            None,
        )
        .map_err(|error| format!("创建说明文字失败：{error}"))?;

        // 一上来是定点模式；总长未知时再临时挂上 PBS_MARQUEE 走跑马灯（见 set_marquee）
        let bar = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PROGRESS_CLASSW,
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE,
            16,
            226,
            528,
            20,
            Some(window),
            None,
            Some(instance),
            None,
        )
        .map_err(|error| format!("创建进度条失败：{error}"))?;

        let cancel_button = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("取消"),
            WS_CHILD | WS_VISIBLE,
            456,
            256,
            88,
            30,
            Some(window),
            Some(HMENU(CANCEL_ID as *mut c_void)),
            Some(instance),
            None,
        )
        .map_err(|error| format!("创建取消按钮失败：{error}"))?;

        let font = GetStockObject(DEFAULT_GUI_FONT);
        for control in [caption, bar, cancel_button] {
            SendMessageW(
                control,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }

        let state = Box::into_raw(Box::new(WindowState {
            hwnd: window,
            caption,
            bar,
            cancel,
            progress,
            result,
        }));
        // 故意不回收：窗口活到进程继续启动或退出为止，回收反而容易留下悬垂指针
        SetWindowLongPtrW(window, GWLP_USERDATA, state as isize);

        let _ = ShowWindow(window, SW_SHOW);
        let _ = UpdateWindow(window);
        Ok(window)
    }
}

/// 窗口过程：刷新文案 / 收尾 / 退出，别的都交回系统。
unsafe extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: GWLP_USERDATA 里是 create_window 放进来的 WindowState，故意不回收，活着。
    let state = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const WindowState;
    if !state.is_null() {
        // SAFETY: 上面刚确认非空，且它指向我们自己的、不回收的 WindowState。
        let state = unsafe { &*state };
        match message {
            WM_APP_UPDATE => {
                refresh(state);
                return LRESULT(0);
            }
            WM_APP_DONE => {
                settle(state);
                return LRESULT(0);
            }
            // 控件 ID 在 wparam 的低位
            WM_COMMAND if (wparam.0 & 0xffff) == CANCEL_ID => {
                request_quit(state);
                return LRESULT(0);
            }
            WM_TIMER if wparam.0 == DONE_TIMER => {
                // SAFETY: PostQuitMessage 只往当前线程的消息队列放一条 WM_QUIT。
                unsafe { PostQuitMessage(0) };
                return LRESULT(0);
            }
            WM_CLOSE => {
                request_quit(state);
                return LRESULT(0);
            }
            WM_KEYDOWN => {
                // 失败页上「按任意键退出」
                // SAFETY: 同上，只投递 WM_QUIT。
                unsafe { PostQuitMessage(0) };
                return LRESULT(0);
            }
            _ => {}
        }
    }
    // SAFETY: 其余消息照系统的默认处理（父窗口创建消息、绘制等）。
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

/// 用户要停：后台还在跑才算「取消」；已经出结果时，这个动作只是关窗口。
fn request_quit(state: &WindowState) {
    let running = state
        .result
        .lock()
        .map(|slot| slot.is_none())
        .unwrap_or(false);
    if running {
        state.cancel.store(true, Ordering::SeqCst);
    }
    // SAFETY: PostQuitMessage 只往当前（UI）线程的消息队列里放一条 WM_QUIT。
    unsafe { PostQuitMessage(0) };
}

/// 把共享状态画到控件上（只能在 UI 线程调用）。
fn refresh(state: &WindowState) {
    let Some(snapshot) = state.progress.lock().ok().map(|slot| *slot) else {
        return;
    };
    let (text, percent) = describe(&snapshot);
    set_caption(state, &text);

    match percent {
        Some(percent) => {
            set_marquee(state.bar, false);
            // SAFETY: bar 是本窗口的子控件；消息只带整数参数，不解引用任何东西。
            unsafe {
                SendMessageW(
                    state.bar,
                    PBM_SETPOS,
                    Some(WPARAM(percent as usize)),
                    Some(LPARAM(0)),
                );
            }
        }
        None => set_marquee(state.bar, true),
    }
}

/// 跑马灯（总长未知时的不确定态）。
///
/// 光发 PBM_SETMARQUEE 是不动的 —— 控件得带 PBS_MARQUEE 样式才认这个模式，切回定点时还要摘掉。
/// 所以正常的下载（有 Content-Length）一次都不进跑马灯，只有服务器不给大小时才切进去
/// （实测：跑马灯切回定点后 PBM_SETPOS 画出来的宽度对不上，别来回切）。
fn set_marquee(bar: HWND, on: bool) {
    // SAFETY: bar 是本窗口的子控件；这两个调用只读写它的样式位与整数参数。
    unsafe {
        let style = GetWindowLongW(bar, GWL_STYLE) as u32;
        let wanted = if on {
            style | PBS_MARQUEE
        } else {
            style & !PBS_MARQUEE
        };
        if wanted != style {
            SetWindowLongW(bar, GWL_STYLE, wanted as i32);
            // 样式改了要 SWP_FRAMECHANGED 才真正生效，否则控件还按老样式画
            let _ = SetWindowPos(
                bar,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
            );
        }
        SendMessageW(
            bar,
            PBM_SETMARQUEE,
            Some(WPARAM(usize::from(on))),
            Some(LPARAM(if on { 40 } else { 0 })),
        );
    }
}

/// 后台线程结束：成功就显示最后一行、稍等一下自己退；失败把原因和链接留在窗口上等按键。
fn settle(state: &WindowState) {
    match state.result.lock().ok().and_then(|slot| slot.clone()) {
        Some(Ok(())) => {
            refresh(state);
            // SAFETY: 延时器挂在当前窗口上；传 None 表示到点发 WM_TIMER 给窗口过程。
            unsafe {
                SetTimer(Some(state.hwnd), DONE_TIMER, 700, None);
            }
        }
        Some(Err(reason)) => set_caption(state, &failure_text(&reason)),
        None => set_caption(
            state,
            &failure_text("下载 / 安装线程意外结束，没能判断结果"),
        ),
    }
}

/// 换掉说明文字（STATIC 支持多行与自动换行）。
fn set_caption(state: &WindowState, text: &str) {
    let text = wide(text);
    // SAFETY: caption 是本窗口的子控件；宽字符串在调用期间存活。
    unsafe {
        let _ = SetWindowTextW(state.caption, PCWSTR(text.as_ptr()));
    }
}

/// 往 UI 线程投一条刷新消息（窗口已经销毁时失败，忽略即可）。
fn post(window: HWND, message: u32) {
    // SAFETY: HWND 由 UI 线程创建；PostMessage 只投递、不解引用，窗口没了也只是返回错误。
    unsafe {
        let _ = PostMessageW(Some(window), message, WPARAM(0), LPARAM(0));
    }
}

/// 转成以 NUL 结尾的宽字符串 —— 调 Win32 之前的最后一步。
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn downloading(done: u64, total: Option<u64>) -> Progress {
        Progress {
            stage: Stage::Downloading,
            done,
            total,
        }
    }

    #[test]
    fn known_length_shows_percent_and_megabytes() {
        let (text, percent) = describe(&downloading(512 * 1024, Some(1024 * 1024)));
        assert_eq!(percent, Some(50));
        assert!(text.contains("50%"), "{text}");
        assert!(text.contains("0.5 MB / 1.0 MB"), "{text}");
    }

    #[test]
    fn over_read_never_exceeds_one_hundred_percent() {
        // 服务器给的长度偏小时，别让进度条冲出 100
        let (_, percent) = describe(&downloading(3 * 1024 * 1024, Some(1024 * 1024)));
        assert_eq!(percent, Some(100));
    }

    #[test]
    fn unknown_length_is_indeterminate() {
        let (text, percent) = describe(&downloading(1024 * 1024, None));
        assert_eq!(percent, None, "总长未知时进度条走不确定态");
        assert!(text.contains("总大小未知"), "{text}");
    }

    #[test]
    fn failure_text_carries_reason_and_manual_link() {
        let text = failure_text("连不上微软下载站");
        assert!(text.contains("连不上微软下载站"), "{text}");
        assert!(text.contains(SOURCE_URL), "{text}");
        assert!(text.contains("按任意键退出"), "{text}");
    }
}
