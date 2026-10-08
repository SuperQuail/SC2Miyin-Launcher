//! 手动跑一次更新检查，用来看真实的网络往返。
//!
//! `@bash
//! cargo run -p miyin-core --example check-update
//! `@

use miyin_core::update::Reporter;
use miyin_core::update::check;
use miyin_core::update::net::{self, NetworkSettings, ProxyMode};

fn main() {
    let mut settings = NetworkSettings::default();

    // 允许用环境变量临时试直连：MIYIN_PROXY=off
    if std::env::var("MIYIN_PROXY").is_ok_and(|value| value.eq_ignore_ascii_case("off")) {
        settings.proxy_mode = ProxyMode::Off;
    }

    println!("当前版本 : {}", miyin_core::update::current_version());
    println!("代理模式 : {:?}", settings.proxy_mode);
    println!("镜像加速 : {}", settings.use_mirrors);
    println!("含预发行 : {}", settings.include_prerelease);
    println!("探测代理 : {:?}", net::detect_proxy(&settings));
    println!("--------------------------------------------");

    let say = |message: &str| println!("  [日志] {message}");
    let result = check::check(
        &miyin_core::update::current_version(),
        &settings,
        Reporter::with_log(&say),
    );

    println!("查询走的路: {:?}", result.via);
    match (&result.error, &result.latest) {
        (Some(error), _) => println!("检查失败  : {error}"),
        (None, Some(latest)) => {
            println!("最新版本  : {}（{}）", latest.version, latest.tag);
            println!("发布时间  : {}", latest.published_at);
            println!("预发行    : {}", latest.prerelease);
            println!("发行页    : {}", latest.html_url);
            for asset in &latest.assets {
                println!(
                    "  资产    : {}  {} 字节  摘要={}",
                    asset.name,
                    asset.size,
                    asset.sha256.as_deref().unwrap_or("（无）")
                );
            }
            match latest.platform_asset() {
                Some(asset) => {
                    let urls = check::asset_urls(asset, &settings);
                    println!("可下载    : {}", asset.name);
                    println!("候选地址  : {} 个", urls.len());
                    for url in urls.iter().take(3) {
                        println!("   - {url}");
                    }
                }
                None => println!("可下载    : （没有适合 Windows 的包）"),
            }
        }
        (None, None) => println!("没有任何发行版"),
    }

    println!("有更新    : {}", result.available);
}
