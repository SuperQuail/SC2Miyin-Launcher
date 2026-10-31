//! 助手进程这一侧：连上 UI 的管道、自我介绍、然后一条条执行白名单 op。
//!
//! 这一侧是**提权**的，所以进来先认人：管道服务端必须是本可执行文件的另一个进程、
//! 且是同一个用户。命令行上的管道名是 UI 给的，但名字本身不构成信任 ——
//! 谁都能拿着同一个名字启动我们，认的是**连上来的那个进程是谁**。
//!
//! UI 退出 / 断开 → 这里读到 EOF，正常收尾退出（不留管理员进程）。

use std::io::BufReader;

use super::ops;
use super::protocol::{self, HELLO, Hello, PROTOCOL, Request, Response};
use super::win;

/// 跑完整个会话，直到对端断开。
pub(crate) fn serve(pipe: &str, nonce: &str) -> Result<(), String> {
    let file = win::connect(pipe)?;

    // 先认服务端：不是本启动器 / 不是同一个用户，一句话都不多说
    let server_pid = win::server_pid(&file)?;
    win::peer_is_launcher(server_pid)?;

    let mut writer = file
        .try_clone()
        .map_err(|error| format!("复制管道句柄失败：{error}"))?;
    let mut reader = BufReader::new(file);

    // 自我介绍：把命令行带来的 nonce 原样回送。UI 用它确认「这就是我刚拉起的进程」，
    // 也顺便把协议版本摊开说 —— 版本不一致就让 UI 直接报错，别各说各话。
    let hello = Hello {
        protocol: PROTOCOL,
        nonce: nonce.to_string(),
        pid: std::process::id(),
    };
    let args = serde_json::to_value(&hello).map_err(|error| format!("握手序列化失败：{error}"))?;
    protocol::write(
        &mut writer,
        &Request {
            id: 0,
            op: HELLO.to_string(),
            args,
        },
    )?;
    let ack: Response =
        protocol::read(&mut reader)?.ok_or_else(|| "UI 没回握手就断开了".to_string())?;
    ack.into_result()?;

    loop {
        // 对端正常关掉（UI 退出）时是 None，不是错误
        let Some(request) = protocol::read::<Request, _>(&mut reader)? else {
            return Ok(());
        };
        let reply = dispatch(&request);
        protocol::write(&mut writer, &reply)?;
    }
}

/// 一条请求 → 一条响应。业务错误当场翻成错误帧，连接继续用。
fn dispatch(request: &Request) -> Response {
    if request.op == HELLO {
        return Response::error(request.id, "已经握过手了，重复的握手帧不接受");
    }
    match ops::run(&request.op, &request.args) {
        Ok(value) => Response::ok(request.id, value),
        Err(error) => Response::error(request.id, error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 全程真管道：建服务端 -> 助手连上来 -> 握手 -> 两条请求 -> 断开收尾。
    ///
    /// 助手与 UI 在同一进程里跑（测试没法弹 UAC），管道本身、ACL、帧编解码、
    /// 白名单分派都是真的。不会弹任何对话框。
    #[test]
    fn worker_serves_requests_over_a_real_pipe() {
        let name = format!("miyin-serve-{}", win::random_hex(6).expect("随机管道名"));
        let nonce = win::random_hex(8).expect("随机 nonce");

        let server = win::create_server(&name).expect("建管道");
        let worker_pipe = name.clone();
        let worker_nonce = nonce.clone();
        let worker = std::thread::spawn(move || serve(&worker_pipe, &worker_nonce));

        // —— UI 这一侧 ——
        win::accept(&server).expect("等助手连上来");
        let mut writer = server.try_clone().expect("复制管道句柄");
        let mut reader = std::io::BufReader::new(server);

        let hello: Request = protocol::read(&mut reader)
            .expect("读握手")
            .expect("应当有握手帧");
        assert_eq!(hello.op, HELLO);
        let body: Hello = serde_json::from_value(hello.args).expect("握手内容");
        assert_eq!(body.protocol, PROTOCOL);
        assert_eq!(body.nonce, nonce);
        protocol::write(&mut writer, &Response::ok(hello.id, json!({}))).expect("回 ack");

        // 未知 op：拒绝，但连接还能继续用
        protocol::write(
            &mut writer,
            &Request {
                id: 1,
                op: "rm -rf".to_string(),
                args: json!({}),
            },
        )
        .expect("发请求");
        let refused: Response = protocol::read(&mut reader)
            .expect("读响应")
            .expect("应当有响应");
        assert!(!refused.ok);
        assert!(
            refused.error.unwrap_or_default().contains("未知操作"),
            "未知 op 必须被白名单挡住"
        );

        // 白名单里的 op 走到了参数校验（没给根目录，必须先拒）
        protocol::write(
            &mut writer,
            &Request {
                id: 2,
                op: "activate".to_string(),
                args: json!({}),
            },
        )
        .expect("发请求");
        let missing: Response = protocol::read(&mut reader)
            .expect("读响应")
            .expect("应当有响应");
        assert!(!missing.ok);
        assert!(
            missing.error.unwrap_or_default().contains("dataRoot"),
            "应当报缺参数，说明分派确实走到了 ops::run"
        );

        // 断开：助手读到 EOF 自己收尾，不留后台进程
        drop(writer);
        drop(reader);
        let outcome = worker.join().expect("助手线程不该 panic");
        assert!(outcome.is_ok(), "助手应当正常退出：{outcome:?}");
    }
}
