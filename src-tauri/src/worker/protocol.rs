//! 助手协议：JSON Lines，一行一个 JSON 帧。
//!
//! 请求 `{"id":1,"op":"activate","args":{…}}`，响应 `{"id":1,"ok":true,"value":…}`
//! 或 `{"id":1,"ok":false,"error":"…"}`。第一帧固定是 `op = "hello"` 的握手
//! （见 `docs/design/storage-and-elevation.md` §3）。
//!
//! 这一层只管编解码，不认识任何具体 op —— 想单独测协议不用起进程、不用碰磁盘。

use std::io::{self, BufRead, Write};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 协议版本。帧格式一改就 +1：老 UI 连新助手必须握手失败，而不是各说各话。
pub(crate) const PROTOCOL: u32 = 1;

/// 握手帧的 op 名。它不是白名单里的写操作，只会出现在第一帧。
pub(crate) const HELLO: &str = "hello";

/// 一条请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Request {
    pub id: u64,
    pub op: String,
    /// 操作参数。握手帧里放的是 Hello。
    #[serde(default)]
    pub args: Value,
}

/// 一条响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Response {
    pub id: u64,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    /// 成功帧。
    pub(crate) fn ok(id: u64, value: Value) -> Self {
        Self {
            id,
            ok: true,
            value: Some(value),
            error: None,
        }
    }

    /// 失败帧：错误一律换算成给人看的一句话，前端直接展示。
    pub(crate) fn error(id: u64, message: impl Into<String>) -> Self {
        Self {
            id,
            ok: false,
            value: None,
            error: Some(message.into()),
        }
    }

    /// ok 为假就翻成 Err，调用方不用自己拆两个字段。
    pub(crate) fn into_result(self) -> Result<Value, String> {
        if self.ok {
            return Ok(self.value.unwrap_or(Value::Null));
        }
        Err(self
            .error
            .unwrap_or_else(|| "助手返回了失败，但没说明原因".to_string()))
    }
}

/// 握手内容。
///
/// nonce：UI 建管道时生成的随机串，随命令行交给助手；助手原样回送，服务端比对 ——
/// 证明连上来的确实是**这一次**拉起的进程，而不是早先留下的连接。
/// pid：助手自己的进程号；服务端还要拿它与 GetNamedPipeClientProcessId 对一遍。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Hello {
    pub protocol: u32,
    pub nonce: String,
    pub pid: u32,
}

/// 把一帧编成「一行 JSON + 换行」。
fn encode<T: Serialize>(frame: &T) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(frame).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// 读一帧。对端关掉连接（UI 退出）时返回 Ok(None) —— 那是正常收尾，不是错误。
pub(crate) fn read<T: DeserializeOwned, R: BufRead>(reader: &mut R) -> Result<Option<T>, String> {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|error| format!("读管道失败：{error}"))?;
    if read == 0 {
        return Ok(None);
    }
    let frame = serde_json::from_str(line.trim_end())
        .map_err(|error| format!("协议帧解析失败：{error}"))?;
    Ok(Some(frame))
}

/// 写一帧并立刻刷出去（不刷 = 对端永远在等下一行）。
pub(crate) fn write<T: Serialize, W: Write>(writer: &mut W, frame: &T) -> Result<(), String> {
    let bytes = encode(frame).map_err(|error| format!("协议帧编码失败：{error}"))?;
    writer
        .write_all(&bytes)
        .and_then(|()| writer.flush())
        .map_err(|error| format!("写管道失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn request_round_trips_through_a_line() {
        let request = Request {
            id: 7,
            op: "activate".to_string(),
            args: serde_json::json!({ "slot": "wol", "variantId": "abc" }),
        };

        let mut buffer = Vec::new();
        write(&mut buffer, &request).expect("写一帧");
        // 一行一帧：帧尾必须正好一个换行
        assert_eq!(buffer.iter().filter(|byte| **byte == b'\n').count(), 1);

        let mut reader = Cursor::new(buffer);
        let back: Request = read(&mut reader).expect("读一帧").expect("应当读到");
        assert_eq!(back, request);
    }

    #[test]
    fn response_uses_the_documented_shape() {
        let accepted = Response::ok(3, serde_json::json!(["a"]));
        assert_eq!(
            serde_json::to_value(&accepted).expect("序列化"),
            serde_json::json!({ "id": 3, "ok": true, "value": ["a"] })
        );
        assert_eq!(
            accepted.into_result().expect("成功帧"),
            serde_json::json!(["a"])
        );

        let failed = Response::error(4, "没有这个战役");
        assert_eq!(
            serde_json::to_value(&failed).expect("序列化"),
            serde_json::json!({ "id": 4, "ok": false, "error": "没有这个战役" })
        );
        assert_eq!(failed.into_result().expect_err("失败帧"), "没有这个战役");
    }

    #[test]
    fn hello_round_trips() {
        let hello = Hello {
            protocol: PROTOCOL,
            nonce: "9f2a".to_string(),
            pid: 4321,
        };
        let request = Request {
            id: 0,
            op: HELLO.to_string(),
            args: serde_json::to_value(&hello).expect("序列化"),
        };
        let mut buffer = Vec::new();
        write(&mut buffer, &request).expect("写一帧");

        let back: Request = read(&mut Cursor::new(buffer))
            .expect("读一帧")
            .expect("读到");
        assert_eq!(back.op, HELLO);
        assert_eq!(
            serde_json::from_value::<Hello>(back.args).expect("解析握手"),
            hello
        );
    }

    #[test]
    fn eof_is_not_an_error() {
        let mut reader = Cursor::new(Vec::new());
        let frame: Option<Request> = read(&mut reader).expect("EOF 不是错误");
        assert!(frame.is_none());
    }

    #[test]
    fn broken_json_is_reported_not_panicked() {
        let mut reader = Cursor::new(b"{\"id\":1,".to_vec());
        let error = read::<Request, _>(&mut reader).expect_err("坏帧必须报错");
        assert!(error.contains("协议帧解析失败"), "{error}");
    }
}
