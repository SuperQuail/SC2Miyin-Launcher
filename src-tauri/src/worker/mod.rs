//! 提权助手：同一个 exe 的 `--privileged-worker` 模式 + 命名管道 IPC（见设计文档 §3）。
//!
//! 三条对外口子：
//! - [`run_if_requested`]：`main()` 最前面分流。是助手就跑完退出，**不加载 Tauri / WebView2**；
//! - [`request_privileged_worker`]：UI 侧，建管道 + ShellExecuteW(runas)，返回是否已就绪；
//! - [`privileged_op`]：UI 侧，把一条写操作交给已经连好的助手执行。
//!
//! 模块分工：`protocol` 帧编解码、`win` Win32 原语、`client` / `server` 两端会话、
//! `ops` 白名单与核心库调用。读操作永远在 UI 进程，这里一律不碰。

use serde_json::{Map, Value};

mod ops;
mod protocol;

#[cfg(windows)]
mod client;
#[cfg(windows)]
mod server;
#[cfg(windows)]
mod win;

/// 助手被要求做什么：连哪个管道、握手时回哪串 nonce。
struct Invocation {
    pipe: String,
    nonce: String,
}

/// 若当前进程是 `--privileged-worker` 模式，就在这里跑完并返回 true（不加载 Tauri）。
///
/// 助手是一条独立的分支：不建窗口、不碰 WebView2、也不碰单实例锁 —— 它就是给
/// 已经在跑的启动器打工的，拿管道名连上去，干完活随 UI 退出。
pub(crate) fn run_if_requested() -> bool {
    let Some(invocation) = invocation() else {
        return false;
    };

    #[cfg(windows)]
    if let Err(error) = server::serve(&invocation.pipe, &invocation.nonce) {
        // release 没有控制台；UI 那边看不到这条消息时，至少这里留一行
        eprintln!("[miyin] 提权助手退出：{error}");
    }
    #[cfg(not(windows))]
    {
        let _ = (&invocation.pipe, &invocation.nonce);
        eprintln!("[miyin] 提权助手只在 Windows 上支持");
    }

    true
}

/// 从命令行认出助手模式。
///
/// 命令行形态：`--privileged-worker \\.\pipe\<random> --nonce <hex>`。
/// 管道名与 nonce 都必须有 —— 只有 UI 会带上它们；不接受环境变量之类的旁路，
/// 免得「设个变量就能让启动器进助手模式」成为一条额外的攻击面。
fn invocation() -> Option<Invocation> {
    parse_args(std::env::args().skip(1))
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Option<Invocation> {
    let mut pipe = None;
    let mut nonce = String::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--privileged-worker" => pipe = args.next(),
            "--nonce" => nonce = args.next().unwrap_or_default(),
            _ => {}
        }
    }
    let pipe = pipe?;
    if pipe.trim().is_empty() || nonce.trim().is_empty() {
        return None;
    }
    Some(Invocation { pipe, nonce })
}

/// 拉起（或复用）管理员助手；返回是否已就绪。
///
/// 用户拒绝 UAC 时返回一句能读懂的错误（不是 "os error 1223"）。
#[tauri::command(async)]
pub(crate) fn request_privileged_worker() -> Result<bool, String> {
    #[cfg(windows)]
    {
        client::ensure()?;
        Ok(true)
    }
    #[cfg(not(windows))]
    {
        Err("提权助手只在 Windows 上有".to_string())
    }
}

/// 把一个写操作转给管理员助手执行（参数与命令同名同义）。
///
/// 数据根 / 游戏根由**本进程**注入：前端只传 op 自己的参数，管不到根目录。
#[tauri::command(async)]
pub(crate) fn privileged_op(
    op: String,
    args: Value,
    state: tauri::State<'_, crate::AppState>,
) -> Result<Value, String> {
    #[cfg(windows)]
    {
        let args = with_roots(&state, args)?;
        client::call(&op, args)
    }
    #[cfg(not(windows))]
    {
        let _ = (op, args, state);
        Err("提权助手只在 Windows 上有".to_string())
    }
}

/// 把这条写操作的两个根塞进参数里。
#[cfg(windows)]
fn with_roots(state: &tauri::State<'_, crate::AppState>, args: Value) -> Result<Value, String> {
    let mut fields = match args {
        Value::Null => Map::new(),
        Value::Object(fields) => fields,
        _ => return Err("args 必须是 JSON 对象".to_string()),
    };

    let installation = state
        .installation
        .lock()
        .map_err(crate::lock_error)?
        .clone()
        .ok_or_else(|| "尚未设置星际争霸 II 安装目录，请到「设置」中选择".to_string())?;

    let data_root = state.library.root();
    fields.insert(
        "dataRoot".to_string(),
        Value::String(data_root.to_string_lossy().into_owned()),
    );
    fields.insert(
        "gameRoot".to_string(),
        Value::String(installation.root.to_string_lossy().into_owned()),
    );
    Ok(Value::Object(fields))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Option<Invocation> {
        parse_args(list.iter().map(|item| item.to_string()))
    }

    #[test]
    fn worker_mode_is_recognised() {
        let found = args(&[
            "--privileged-worker",
            r"\\.\pipe\miyin-abc",
            "--nonce",
            "deadbeef",
        ])
        .expect("应当认出助手模式");
        assert_eq!(found.pipe, r"\\.\pipe\miyin-abc");
        assert_eq!(found.nonce, "deadbeef");
    }

    #[test]
    fn ordinary_launch_is_not_a_worker() {
        assert!(args(&[]).is_none());
        assert!(args(&["--elevated"]).is_none());
        // 缺了管道名不算
        assert!(args(&["--privileged-worker"]).is_none());
    }

    #[test]
    fn pipe_without_nonce_is_not_a_worker() {
        // nonce 是握手的一半凭据，缺了就只能继续当普通启动器 —— 不能进助手模式。
        assert!(args(&["--privileged-worker", "miyin-abc"]).is_none());
        assert!(args(&["--privileged-worker", "miyin-abc", "--nonce", " "]).is_none());
    }
}
