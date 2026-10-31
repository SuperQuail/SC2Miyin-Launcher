//! Windows 原语：命名管道、进程身份校验、提权启动。
//!
//! 这里只有 FFI 与薄封装，不含任何业务判断。
//!
//! **为什么是裸 FFI**：管道要 CreateNamedPipeW（Win32_System_Pipes）、ACL 要
//! Win32_Security，而 src-tauri/Cargo.toml 里的 windows crate 没开这两个 feature，
//! 那份清单不在提权工作组范围内（见设计文档 §7）。这里只调 kernel32 / advapi32 /
//! shell32 / bcrypt 的稳定导出，不碰未文档化接口；把这些 feature 加进清单后
//! 可以整个换成 windows crate，对外函数不用改。

use std::ffi::{OsStr, c_void};
use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, RawHandle};
use std::path::{Path, PathBuf};
use std::ptr::null_mut;
use std::time::Duration;

type Handle = *mut c_void;

/// CreateFile / CreateNamedPipe 的失败返回值。
const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;

const PIPE_ACCESS_DUPLEX: u32 = 0x0000_0003;
const FILE_FLAG_FIRST_PIPE_INSTANCE: u32 = 0x0008_0000;
const PIPE_TYPE_BYTE: u32 = 0x0000_0000;
const PIPE_READMODE_BYTE: u32 = 0x0000_0000;
const PIPE_WAIT: u32 = 0x0000_0000;
const PIPE_REJECT_REMOTE_CLIENTS: u32 = 0x0000_0008;
const PIPE_BUFFER: u32 = 64 * 1024;
const ERROR_PIPE_CONNECTED: u32 = 535;
const ERROR_PIPE_BUSY: i32 = 231;
const SDDL_REVISION_1: u32 = 1;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_USER_CLASS: u32 = 1;
const SW_SHOWNORMAL: i32 = 1;
const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x0000_0002;
const ERROR_CANCELLED: u32 = 1223;
const SE_ERR_ACCESSDENIED: u32 = 5;

/// SECURITY_ATTRIBUTES。管道句柄不继承，所以 bInheritHandle 恒为 0。
#[repr(C)]
struct SecurityAttributes {
    n_length: u32,
    descriptor: *mut c_void,
    inherit: i32,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    #[link_name = "CreateNamedPipeW"]
    fn create_named_pipe(
        name: *const u16,
        open_mode: u32,
        pipe_mode: u32,
        instances: u32,
        out_size: u32,
        in_size: u32,
        timeout: u32,
        attributes: *const SecurityAttributes,
    ) -> Handle;
    #[link_name = "ConnectNamedPipe"]
    fn connect_named_pipe(pipe: Handle, overlapped: *mut c_void) -> i32;
    #[link_name = "GetNamedPipeClientProcessId"]
    fn named_pipe_client_pid(pipe: Handle, pid: *mut u32) -> i32;
    #[link_name = "GetNamedPipeServerProcessId"]
    fn named_pipe_server_pid(pipe: Handle, pid: *mut u32) -> i32;
    #[link_name = "CloseHandle"]
    fn close_handle(handle: Handle) -> i32;
    #[link_name = "LocalFree"]
    fn local_free(memory: *mut c_void) -> *mut c_void;
    #[link_name = "GetCurrentProcess"]
    fn current_process() -> Handle;
    #[link_name = "OpenProcess"]
    fn open_process(access: u32, inherit: i32, pid: u32) -> Handle;
    #[link_name = "OpenProcessToken"]
    fn open_process_token(process: Handle, access: u32, token: *mut Handle) -> i32;
    #[link_name = "GetTokenInformation"]
    fn token_information(
        token: Handle,
        class: u32,
        info: *mut c_void,
        size: u32,
        needed: *mut u32,
    ) -> i32;
    #[link_name = "QueryFullProcessImageNameW"]
    fn process_image_name(process: Handle, flags: u32, buffer: *mut u16, size: *mut u32) -> i32;
    #[link_name = "GetWindowsDirectoryW"]
    fn windows_directory(buffer: *mut u16, size: u32) -> u32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    #[link_name = "ConvertStringSecurityDescriptorToSecurityDescriptorW"]
    fn sddl_to_descriptor(
        sddl: *const u16,
        revision: u32,
        descriptor: *mut *mut c_void,
        size: *mut u32,
    ) -> i32;
    #[link_name = "ConvertSidToStringSidW"]
    fn sid_to_string(sid: *mut c_void, out: *mut *mut u16) -> i32;
}

#[link(name = "shell32")]
unsafe extern "system" {
    #[link_name = "ShellExecuteW"]
    fn shell_execute(
        window: *mut c_void,
        verb: *const u16,
        file: *const u16,
        parameters: *const u16,
        directory: *const u16,
        show: i32,
    ) -> isize;
}

#[link(name = "bcrypt")]
unsafe extern "system" {
    #[link_name = "BCryptGenRandom"]
    fn gen_random(algorithm: Handle, buffer: *mut u8, size: u32, flags: u32) -> i32;
}

/// 窄字符转 UTF-16 + NUL 结尾。
fn wide(text: &OsStr) -> Vec<u16> {
    text.encode_wide().chain(std::iter::once(0)).collect()
}

/// 管道全名。命令行上写裸名字也认。
pub(crate) fn pipe_path(name: &str) -> String {
    if name.starts_with("\\\\") {
        name.to_string()
    } else {
        format!("\\\\.\\pipe\\{name}")
    }
}

/// 系统目录（%SystemRoot%）。不读环境变量：环境可能被人改过，这里要系统自己说。
pub(crate) fn windows_dir() -> Option<PathBuf> {
    let mut buffer = vec![0u16; 260];
    // SAFETY: 缓冲区与长度一致；返回值不小于长度表示被截断，按失败处理。
    let length = unsafe { windows_directory(buffer.as_mut_ptr(), buffer.len() as u32) };
    if length == 0 || length as usize >= buffer.len() {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(
        &buffer[..length as usize],
    )))
}

/// 取一串系统随机字节，十六进制。管道名与握手 nonce 都用它。
pub(crate) fn random_hex(bytes: usize) -> Result<String, String> {
    let mut buffer = vec![0u8; bytes];
    // SAFETY: buffer 由我们分配且足够长；NULL 算法句柄 + 系统首选 RNG 是文档允许的组合。
    let status = unsafe {
        gen_random(
            null_mut(),
            buffer.as_mut_ptr(),
            bytes as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status != 0 {
        return Err(format!("系统随机数生成失败（NTSTATUS {status:#x}）"));
    }
    Ok(buffer.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// 当前用户的 SID 字节形态（SID 自带长度，可以直接当 PSID 使用）。
fn current_user_sid() -> Result<Vec<u8>, String> {
    let mut token: Handle = null_mut();
    // SAFETY: 取当前进程的真句柄，失败时 token 保持 null 且下面不会用它。
    let ok = unsafe { open_process_token(current_process(), TOKEN_QUERY, &mut token) };
    if ok == 0 {
        return Err(format!(
            "拿不到本进程的令牌：{}",
            io::Error::last_os_error()
        ));
    }
    let sid = token_user_sid(token);
    // SAFETY: token 是本函数刚打开的有效句柄，只关一次。
    unsafe { close_handle(token) };
    sid
}

/// 令牌里的用户 SID。TOKEN_USER 的第一个字段就是 PSID。
fn token_user_sid(token: Handle) -> Result<Vec<u8>, String> {
    let mut needed = 0u32;
    // SAFETY: 先问长度；这次调用按文档必然失败并填 needed，缓冲区传 null 是允许的。
    unsafe { token_information(token, TOKEN_USER_CLASS, null_mut(), 0, &mut needed) };
    if needed == 0 {
        return Err("令牌里查不到用户 SID".to_string());
    }

    let mut buffer = vec![0u8; needed as usize];
    // SAFETY: 缓冲区长度就是刚问到的 needed。
    let ok = unsafe {
        token_information(
            token,
            TOKEN_USER_CLASS,
            buffer.as_mut_ptr() as *mut c_void,
            needed,
            &mut needed,
        )
    };
    if ok == 0 {
        return Err(format!(
            "读令牌用户信息失败：{}",
            io::Error::last_os_error()
        ));
    }

    // SAFETY: 上面成功返回的缓冲区就是 TOKEN_USER，首字段为 PSID；SID 自述长度，
    // 落在同一个缓冲区内，拷贝完就与缓冲区无关。
    let sid = unsafe {
        let sid = *(buffer.as_ptr() as *const *const u8);
        if sid.is_null() {
            return Err("令牌里没有用户 SID".to_string());
        }
        // SID = 1 字节修订号 + 1 字节子权限个数 + 6 字节机构 + 每个子权限 4 字节
        let count = *sid.add(1) as usize;
        let length = 8 + 4 * count;
        if count > 15 || length > buffer.len() {
            return Err("令牌里的用户 SID 长度不合理".to_string());
        }
        std::slice::from_raw_parts(sid, length).to_vec()
    };
    Ok(sid)
}

/// SID 的字符串写法（`S-1-5-21-…`），写 SDDL 用。
fn sid_text(sid: &[u8]) -> Result<String, String> {
    let mut out: *mut u16 = null_mut();
    // SAFETY: sid 是自述长度的完整 SID；输出指针由系统分配，下面立刻拷走并 LocalFree。
    let ok = unsafe { sid_to_string(sid.as_ptr() as *mut c_void, &mut out) };
    if ok == 0 || out.is_null() {
        return Err(format!("SID 转字符串失败：{}", io::Error::last_os_error()));
    }
    // SAFETY: out 是本函数刚拿到的 NUL 结尾宽字符串。
    let text = unsafe { read_wide(out) };
    // SAFETY: 系统分配，只能 LocalFree 一次。
    unsafe { local_free(out as *mut c_void) };
    Ok(text)
}

/// 读 NUL 结尾的宽字符串。
unsafe fn read_wide(text: *const u16) -> String {
    let mut length = 0usize;
    // SAFETY: 调用方保证指针指向 NUL 结尾的宽字符串。
    while unsafe { *text.add(length) } != 0 {
        length += 1;
    }
    // SAFETY: 上面已经量出长度，且末尾是 NUL。
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) })
}

/// 建一个只允许「当前用户 + SYSTEM」连接的管道服务端。
///
/// FILE_FLAG_FIRST_PIPE_INSTANCE：名字被别人抢先建过就直接失败 —— 这样 UI 先建、
/// 再启动助手，中间没有被人插一脚换掉服务端的窗口。
pub(crate) fn create_server(name: &str) -> Result<File, String> {
    let sid = current_user_sid()?;
    // D:P = 保护 DACL，禁止从父对象继承任何 ACE —— 否则「只给当前用户 + SYSTEM」
    // 会被进程 / 目录的继承项悄悄放宽。
    let sddl = format!("D:P(A;;GA;;;SY)(A;;GA;;;{})", sid_text(&sid)?);
    let sddl_wide = wide(OsStr::new(&sddl));

    let mut descriptor: *mut c_void = null_mut();
    // SAFETY: 输入是 NUL 结尾的 SDDL，输出指针由系统分配，下面无论成败都会释放。
    let ok = unsafe {
        sddl_to_descriptor(
            sddl_wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            null_mut(),
        )
    };
    if ok == 0 || descriptor.is_null() {
        return Err(format!(
            "安全描述符创建失败：{}",
            io::Error::last_os_error()
        ));
    }

    let attributes = SecurityAttributes {
        n_length: std::mem::size_of::<SecurityAttributes>() as u32,
        descriptor,
        inherit: 0,
    };
    let path = wide(OsStr::new(&pipe_path(name)));
    // SAFETY: 名字与安全属性都在本函数内有效，返回的句柄下面交给 File 管理。
    let handle = unsafe {
        create_named_pipe(
            path.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            1,
            PIPE_BUFFER,
            PIPE_BUFFER,
            0,
            &attributes,
        )
    };
    // SAFETY: 上面系统分配的描述符，内核在 CreateNamedPipeW 里已经拷走。
    unsafe { local_free(descriptor) };

    if handle == INVALID_HANDLE_VALUE {
        return Err(format!(
            "建命名管道失败（{name}）：{}",
            io::Error::last_os_error()
        ));
    }
    // SAFETY: handle 是本函数刚创建的独占句柄，之后只由 File 负责关闭。
    Ok(unsafe { File::from_raw_handle(handle as RawHandle) })
}

/// 等客户端连上来（阻塞；超时由调用方自己掐）。
pub(crate) fn accept(pipe: &File) -> Result<(), String> {
    // SAFETY: pipe 是有效的管道服务端句柄；同步模式下 OVERLAPPED 传 null 是文档写法。
    let ok = unsafe { connect_named_pipe(pipe.as_raw_handle(), null_mut()) };
    if ok != 0 {
        return Ok(());
    }
    let code = io::Error::last_os_error().raw_os_error().unwrap_or(0);
    // 客户端在 ConnectNamedPipe 之前就接上了：这是成功，不是错误。
    if code as u32 == ERROR_PIPE_CONNECTED {
        return Ok(());
    }
    Err(format!(
        "等待助手连接失败：{}",
        io::Error::from_raw_os_error(code)
    ))
}

/// 取管道客户端（UI 是服务端时，就是助手）的进程号。
pub(crate) fn client_pid(pipe: &File) -> Result<u32, String> {
    pipe_peer_pid(pipe, false)
}

/// 取管道服务端的进程号（助手是客户端时，用它验对面是谁）。
pub(crate) fn server_pid(pipe: &File) -> Result<u32, String> {
    pipe_peer_pid(pipe, true)
}

/// server = true 取服务端，否则取客户端。
fn pipe_peer_pid(pipe: &File, server: bool) -> Result<u32, String> {
    let mut pid = 0u32;
    // SAFETY: 句柄有效，输出缓冲区是本函数的栈变量。
    let ok = unsafe {
        if server {
            named_pipe_server_pid(pipe.as_raw_handle(), &mut pid)
        } else {
            named_pipe_client_pid(pipe.as_raw_handle(), &mut pid)
        }
    };
    if ok == 0 {
        return Err(format!(
            "拿不到管道对端进程：{}",
            io::Error::last_os_error()
        ));
    }
    Ok(pid)
}

/// 对端进程是不是「本启动器 + 同一个用户」。
///
/// 两道都要过：可执行文件路径与本进程完全一致（别人拷一份改个名也不认），
/// 以及令牌里的用户 SID 与本进程一致（提权不改变 SID，所以管理员助手仍然相等）。
pub(crate) fn peer_is_launcher(pid: u32) -> Result<(), String> {
    let image = process_image(pid)?;
    let exe = std::env::current_exe().map_err(|error| format!("拿不到本进程路径：{error}"))?;
    let same = image
        .to_string_lossy()
        .eq_ignore_ascii_case(&exe.to_string_lossy());
    if !same {
        return Err(format!(
            "管道对端不是本启动器：{}（本进程 {}）",
            image.display(),
            exe.display()
        ));
    }
    if process_user_sid(pid)? != current_user_sid()? {
        return Err(format!("管道对端（pid {pid}）不是同一个用户"));
    }
    Ok(())
}

/// 某个进程的可执行文件路径。
fn process_image(pid: u32) -> Result<PathBuf, String> {
    // SAFETY: 只申请查询权限；句柄用完即关。
    let process = unsafe { open_process(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if process.is_null() {
        return Err(format!("打不开进程 {pid}：{}", io::Error::last_os_error()));
    }
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    // SAFETY: 缓冲区足够大，size 是它的容量。
    let ok = unsafe { process_image_name(process, 0, buffer.as_mut_ptr(), &mut size) };
    // SAFETY: 上面打开的句柄，只关一次。
    unsafe { close_handle(process) };
    if ok == 0 {
        return Err(format!(
            "拿不到进程 {pid} 的路径：{}",
            io::Error::last_os_error()
        ));
    }
    Ok(PathBuf::from(String::from_utf16_lossy(
        &buffer[..size as usize],
    )))
}

/// 某个进程令牌里的用户 SID。
fn process_user_sid(pid: u32) -> Result<Vec<u8>, String> {
    // SAFETY: 只申请查询权限；两个句柄用完即关。
    unsafe {
        let process = open_process(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return Err(format!("打不开进程 {pid}：{}", io::Error::last_os_error()));
        }
        let mut token: Handle = null_mut();
        let ok = open_process_token(process, TOKEN_QUERY, &mut token);
        close_handle(process);
        if ok == 0 {
            return Err(format!(
                "打不开进程 {pid} 的令牌：{}",
                io::Error::last_os_error()
            ));
        }
        let sid = token_user_sid(token);
        close_handle(token);
        sid
    }
}

/// 助手端：连上 UI 建好的管道。UI 已经先建了服务端实例，这里一般一次就成。
pub(crate) fn connect(name: &str) -> Result<File, String> {
    let path = pipe_path(name);
    let mut last = String::new();
    for _ in 0..40 {
        match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => return Ok(file),
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                last = error.to_string();
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(format!("连不上管道 {path}：{error}")),
        }
    }
    Err(format!("管道 {path} 一直被占用：{last}"))
}

/// 用 runas 拉起当前 exe（会弹 UAC）。返回值小于等于 32 一律算失败。
///
/// ShellExecuteW 不等进程结束 —— 助手有没有真的起来，靠后面的握手判定。
pub(crate) fn run_as_admin(exe: &Path, arguments: &str) -> Result<(), String> {
    let verb = wide(OsStr::new("runas"));
    let file = wide(exe.as_os_str());
    let parameters = wide(OsStr::new(arguments));
    let directory = exe
        .parent()
        .map(|parent| wide(parent.as_os_str()))
        .unwrap_or_else(|| wide(OsStr::new(".")));

    // SAFETY: 四个字符串都是本函数内的 NUL 结尾缓冲区；窗口句柄传 null 表示无父窗口。
    let result = unsafe {
        shell_execute(
            null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            directory.as_ptr(),
            SW_SHOWNORMAL,
        )
    };
    let code = result as u32;
    if code > 32 {
        return Ok(());
    }

    Err(match code {
        ERROR_CANCELLED => "你取消了管理员授权（UAC），这次写入没有执行。需要时再点一次提权，或者从右键菜单「以管理员身份运行」打开启动器。".to_string(),
        SE_ERR_ACCESSDENIED => {
            "系统拒绝了管理员授权（UAC）。可以再试一次，或用「以管理员身份运行」启动启动器。"
                .to_string()
        }
        2 | 3 => format!("找不到启动器程序：{}", exe.display()),
        other => format!(
            "提权启动失败（ShellExecuteW 返回 {other}）：{}",
            exe.display()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipe_path_accepts_bare_and_full_names() {
        assert_eq!(pipe_path("abc"), "\\\\.\\pipe\\abc");
        assert_eq!(pipe_path("\\\\.\\pipe\\abc"), "\\\\.\\pipe\\abc");
    }

    #[test]
    fn random_hex_has_the_requested_size_and_differs() {
        let first = random_hex(16).expect("随机数");
        let second = random_hex(16).expect("随机数");
        assert_eq!(first.len(), 32);
        assert!(first.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn current_user_has_a_sid() {
        let sid = current_user_sid().expect("当前用户 SID");
        assert!(sid.len() >= 8);
        let text = sid_text(&sid).expect("SID 字符串");
        assert!(text.starts_with("S-1-"), "{text}");
    }

    #[test]
    fn peer_check_rejects_foreign_processes() {
        // 自己人：本进程
        peer_is_launcher(std::process::id()).expect("本进程应当通过");
        // 别人的进程（System Idle 之类取不到路径）必须被拒
        assert!(peer_is_launcher(0).is_err());
        assert!(peer_is_launcher(4).is_err());
    }
}
