//! 网络小工具：探一个地址通不通。
//!
//! 刻意和 [`crate::update::net`] 分开：那边是"下载更新"的重型设施
//! （代理选择、镜像竞速、摘要校验），这里只回答一个是非题，不该套上那一整套。

use std::time::Duration;

/// 这个 HTTPS 地址能不能用。
///
/// 判据是**拿到了任何一个 HTTP 响应**（哪怕 404 / 403）—— 那说明 TLS 握手过了、
/// 服务器在那儿；连不上、证书不被信任、超时，都算"用不了"，调用方退回 HTTP。
///
/// 代理走环境变量里那一套（reqwest 默认行为）：用户的浏览器要是得穿代理才出得去，
/// 这里也该穿 —— 不然探测结果和用户实际能打开的东西对不上。
pub fn https_usable(url: &str) -> bool {
    let Ok(client) = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(3))
        .timeout(Duration::from_secs(6))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("SC2Miyin-Launcher")
        .build()
    else {
        return false;
    };

    // HEAD 最省流量；有些服务器不认 HEAD，那就再试一次 GET（只读头，不读体）
    if client.head(url).send().is_ok() {
        return true;
    }
    client.get(url).send().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一个不存在的地址必须干净地返回 false，不能 panic。
    #[test]
    fn unreachable_host_is_false() {
        assert!(!https_usable("https://127.0.0.1:1/"));
    }
}
