import { invoke } from "@tauri-apps/api/core";

import { isDesktop } from "../../api/bridge";
import type { Notify } from "../state/useNotify";

/**
 * 等后端把数据准备好（启动时的数据目录迁移 / 合并跑在后台线程上，见 src-tauri/src/lib.rs 的 run）。
 *
 * 这一步返回之前，**任何**碰后端 AppState 的命令都会让后端 panic —— 所以它必须排在
 * AppShell 之前（见 BootGate）。浏览器演示模式没有后端，直接过。
 *
 * shortcut: 这是唯一一条绕过 api/bridge.ts 直连 invoke 的调用 —— boot_ready 要在
 * demo 后端之外单独调；以后收口可以把它挪进 bridge 的桌面实现里。
 */
export function waitForBoot(): Promise<void> {
  if (!isDesktop) return Promise.resolve();
  return invoke("boot_ready").then(() => undefined);
}

/**
 * 把启动时自动挂载模组攒下的提示告诉用户：前 3 条走 toast，其余只进控制台。
 *
 * 后端在启动时就把提示收好了（见 `take_mount_warnings`）；这里只负责怎么给人看。
 */
export function reportMountWarnings(notify: Notify, warnings: string[]): void {
  warnings.slice(0, 3).forEach((warning) => notify("warning", warning));
  warnings.slice(3).forEach((warning) => console.warn("[miyin] 模组挂载警告：" + warning));
}
