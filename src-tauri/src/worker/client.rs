//! UI 进程这一侧：建管道、提权拉起助手、握手、发 op。
//!
//! 角色是设计文档 §3 定的：**UI 建服务端、助手当客户端**。名字与 ACL 都由 UI 定，
//! 并且先建好管道再弹 UAC —— 助手一起步就有东西可连，中间也没有能被插一脚换掉
//! 服务端的窗口（服务端带 FILE_FLAG_FIRST_PIPE_INSTANCE）。
//!
//! 助手驻留复用：连上之后这条会话一直开着，后来的写操作直接用，不再弹 UAC。
//! UI 退出 → 管道断开 → 助手读到 EOF 自己退出，不会留下无主的管理员进程。

use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::sync::mpsc;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde_json::Value;

use super::protocol::{self, HELLO, Hello, PROTOCOL, Request, Response};
use super::win;

/// 当前这条会话。`None` = 助手没起来，或者上次传输失败后已经清掉。
static SESSION: Mutex<Option<Session>> = Mutex::new(None);

/// 等助手连上来的上限。UAC 弹窗由用户决定，太短会误杀。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(60);

struct Session {
    /// 写端（读端在 `reader` 里，是同一个管道的克隆句柄）。
    pipe: File,
    reader: BufReader<File>,
    next_id: u64,
}

fn lock() -> Result<MutexGuard<'static, Option<Session>>, String> {
    SESSION
        .lock()
        .map_err(|_| "内部状态异常，请重启启动器".to_string())
}

/// 已经有助手就直接返回；没有就提权拉起一个并握手。
pub(crate) fn ensure() -> Result<(), String> {
    let mut guard = lock()?;
    if guard.is_some() {
        return Ok(());
    }

    // 随机名字 + 一次性 nonce：管道名不可猜，握手能认出「是不是这一次拉起的」
    let name = format!("miyin-{}", win::random_hex(8)?);
    let nonce = win::random_hex(16)?;
    let server = win::create_server(&name)?;

    // ConnectNamedPipe 是阻塞的：放到单独的线程里等，主线程（这里是命令线程）
    // 才不会因为助手根本没起来而永远卡住。
    let expected = nonce.clone();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(handshake(server, &expected));
    });

    let exe = std::env::current_exe().map_err(|error| format!("拿不到启动器路径：{error}"))?;
    let arguments = format!(
        "--privileged-worker {} --nonce {nonce}",
        win::pipe_path(&name)
    );
    if let Err(error) = win::run_as_admin(&exe, &arguments) {
        // 用户取消了 UAC（最常见）：自己连一下把等待线程叫醒，否则它要挂到进程退出。
        // 握手线程会认出「对端就是本进程」而立刻结束。读写都开，与管道方向一致。
        let _ = OpenOptions::new()
            .read(true)
            .write(true)
            .open(win::pipe_path(&name));
        return Err(error);
    }

    let session = receiver.recv_timeout(CONNECT_TIMEOUT).map_err(|_| {
        "等助手连接超时：没拿到管理员权限的进程（UAC 是不是被关掉了？）".to_string()
    })??;
    *guard = Some(session);
    Ok(())
}

/// 把一条 op 交给助手执行。
///
/// 传输层出错说明助手没了（被结束、空闲退出）：清掉会话，让界面重新提权；
/// 助手**业务上**失败（比如找不到这个战役）只是把错误原样带回，会话仍然可用。
pub(crate) fn call(op: &str, args: Value) -> Result<Value, String> {
    let mut guard = lock()?;
    let session = guard
        .as_mut()
        .ok_or_else(|| "提权助手没有在运行，请先请求管理员权限".to_string())?;

    let id = session.next_id;
    session.next_id += 1;
    let response = match session.exchange(Request {
        id,
        op: op.to_string(),
        args,
    }) {
        Ok(response) => response,
        Err(error) => {
            *guard = None;
            return Err(error);
        }
    };
    response.into_result()
}

impl Session {
    /// 一问一答。任何传输 / 协议层的异常都算「这条会话废了」。
    fn exchange(&mut self, request: Request) -> Result<Response, String> {
        let id = request.id;
        protocol::write(&mut self.pipe, &request)?;
        let response: Response = protocol::read(&mut self.reader)?
            .ok_or_else(|| "助手断开了连接（可能已被结束）".to_string())?;
        if response.id != id {
            return Err(format!("助手响应错位：要 {id}，拿到 {}", response.id));
        }
        Ok(response)
    }
}

/// 等助手连上来 → 验明正身 → 收下它的介绍 → 回一句 ack。
fn handshake(server: File, nonce: &str) -> Result<Session, String> {
    win::accept(&server)?;

    // 1) 连上来的必须真的是**本可执行文件**的另一个进程，且是同一个用户。
    let pid = win::client_pid(&server)?;
    if pid == std::process::id() {
        return Err("管道对端就是本进程：这不是助手连上来了".to_string());
    }
    win::peer_is_launcher(pid)?;

    // 2) 它得报出自己的 nonce 与协议版本 —— 对不上就说明不是这次拉起的进程
    let reader = server
        .try_clone()
        .map(BufReader::new)
        .map_err(|error| format!("复制管道句柄失败：{error}"))?;
    let mut reader = reader;
    let hello =
        protocol::read::<Request, _>(&mut reader)?.ok_or_else(|| "助手连上就断开了".to_string())?;
    if hello.op != HELLO {
        return Err(format!("助手的第一帧不是握手，而是 {}", hello.op));
    }
    let body: Hello =
        serde_json::from_value(hello.args).map_err(|error| format!("握手内容不合法：{error}"))?;
    verify_hello(&body, nonce, pid)?;

    let mut pipe = server;
    protocol::write(
        &mut pipe,
        &Response::ok(hello.id, serde_json::json!({ "protocol": PROTOCOL })),
    )?;
    Ok(Session {
        pipe,
        reader,
        next_id: hello.id + 1,
    })
}

/// 握手内容的三道校验：协议版本、nonce、pid 都要对得上。
///
/// 单独抽出来是为了能不起进程、不弹 UAC 地测 —— 这三条是「连上来的确实是
/// 这一次拉起的助手」的全部依据。
fn verify_hello(body: &Hello, nonce: &str, pid: u32) -> Result<(), String> {
    if body.protocol != PROTOCOL {
        return Err(format!(
            "助手协议版本 {} 与本启动器 {PROTOCOL} 不一致，请重新构建",
            body.protocol
        ));
    }
    if body.nonce != nonce {
        return Err("助手握手 nonce 对不上：这条连接不是这次拉起的".to_string());
    }
    if body.pid != pid {
        return Err(format!("助手报的 pid {} 与管道对端 {pid} 不一致", body.pid));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hello(nonce: &str, pid: u32) -> Hello {
        Hello {
            protocol: PROTOCOL,
            nonce: nonce.to_string(),
            pid,
        }
    }

    #[test]
    fn handshake_accepts_the_process_we_just_launched() {
        verify_hello(&hello("9f2a", 4321), "9f2a", 4321).expect("三项都对应当通过");
    }

    #[test]
    fn handshake_rejects_a_stale_nonce_or_a_different_process() {
        // 早先留下的助手（或者别人拿同一个管道名起的进程）会被 nonce 挡住
        let error = verify_hello(&hello("old", 4321), "9f2a", 4321).expect_err("nonce 不符");
        assert!(error.contains("nonce"), "{error}");

        // 报的 pid 与管道对端不是同一个进程：也拒
        let error = verify_hello(&hello("9f2a", 7777), "9f2a", 4321).expect_err("pid 不符");
        assert!(error.contains("pid"), "{error}");
    }

    #[test]
    fn handshake_rejects_a_protocol_mismatch() {
        let mut frame = hello("9f2a", 4321);
        frame.protocol = PROTOCOL + 1;
        let error = verify_hello(&frame, "9f2a", 4321).expect_err("版本不符");
        assert!(error.contains("协议版本"), "{error}");
    }
}
