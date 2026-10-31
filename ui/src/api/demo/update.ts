// 演示后端：自动更新与网络设置。
import type { LauncherApi } from "../types";

import { delay } from "./fixtures";

export const updateDemo = {
  appVersion: () => delay("0.1.0a3"),
  networkSettings: () =>
    delay({
      proxy_mode: "auto" as const,
      proxy_url: null,
      use_mirrors: true,
      include_prerelease: true,
    }),
  setNetworkSettings: () => delay(undefined),
  detectedProxy: () => delay({ url: "http://127.0.0.1:7897", source: "Windows 系统代理" }),
  // 演示模式假装有一个新版本，这样整块更新面板（版本卡 / 进度条 / 按钮）都能看到
  checkUpdate: () =>
    delay({
      current: "0.1.0a3",
      latest: {
        version: "0.1.0a4",
        tag: "v0.1.0a4",
        published_at: new Date().toISOString(),
        html_url: "https://github.com/SuperQuail/SC2Miyin-Launcher/releases",
        notes: [
          "## 这一版的新东西",
          "",
          "### 自动更新",
          "",
          "设置页新增「更新」面板，**不用再去 Releases 页面找包**：",
          "",
          "- 检查 → 下载（带进度条）→ 安装并重启",
          "- 下载后校验 SHA-256，对不上就删掉重来",
          "- 内嵌终端会逐行写清刚才做了什么",
          "",
          "### 国内可用性",
          "",
          "| 场景 | 处理 |",
          "| --- | --- |",
          "| 直连慢 | 展开「直连 + 6 个镜像」并发竞速，谁先下完用谁 |",
          "| 代理被限流 | 自动改直连重试（额度按出口 IP 算） |",
          "| 不确定 | 一律不动，宁可让你手动装 |",
          "",
          "### 一行命令验证",
          "",
          "```bash",
          "cargo run -p miyin-core --example check-update",
          "```",
          "",
          "> 更新只替换程序本体，data/ 里的战役与补丁一个字节都不碰。",
          "",
          "详见 [包格式规范](https://github.com/SuperQuail/SC2Miyin-Launcher/blob/main/docs/package-format.md)。",
          "（演示数据，真实运行时这里显示的是 Release 正文。）",
        ].join("\n"),
        prerelease: true,
        assets: [],
      },
      available: true,
      via: "直连（代理被限流）",
      error: null,
    }),
  downloadUpdate: (version) =>
    delay({
      version,
      archive: "C:\\demo\\data\\updates\\" + version + "\\package.zip",
      root: "C:\\demo\\data\\updates\\" + version,
      executable: "C:\\demo\\data\\updates\\" + version + "\\miyin-launcher.exe",
      extras: [],
      url: "https://gh.ddlc.top/https://github.com/...",
      bytes: 6_061_384,
    }),
  applyUpdate: () => delay(undefined),
  openUrl: () => delay(undefined),
  onUpdateProgress: async (handler) => {
    // 演示模式假装在下一次东西，让进度条也动起来
    let done = 0;
    const total = 6_061_384;
    const timer = setInterval(() => {
      done = Math.min(done + 900_000, total);
      handler({
        done,
        total,
        percent: (done / total) * 100,
      });
      if (done >= total) clearInterval(timer);
    }, 260);
    return () => clearInterval(timer);
  },
  onUpdateLog: async (handler) => {
    // 演示模式把一次真实的检查过程回放一遍，好让前端看到终端的实际样子
    const lines = [
      "开始检查更新（当前版本 0.1.0a2）",
      "网络：http://127.0.0.1:7897（Windows 系统代理）",
      "查询 GitHub Releases…",
      "代理被限流：HTTP 403 API rate limit exceeded for 58.152.46.170（剩余额度 0）",
      "改用直连重试…",
      "直连成功",
      "读到 3 个发行版本（含预发行）",
      "发现新版本 0.1.0a3（预发行，2026-10-08）",
      "可下载：SC2Miyin-Launcher-0.1.0a3-win64.zip（5.78 MB）",
    ];
    const timers = lines.map((line, index) =>
      setTimeout(() => handler(line), 260 * (index + 1)),
    );
    return () => timers.forEach(clearTimeout);
  },
} satisfies Partial<LauncherApi>;
