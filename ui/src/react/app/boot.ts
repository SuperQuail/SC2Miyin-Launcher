import { invoke } from "@tauri-apps/api/core";

import { isDesktop } from "../../api/bridge";

/**
 * 等后端把数据准备好（启动时的数据目录迁移 / 合并跑在后台线程上，见 src-tauri/src/lib.rs 的 run）。
 *
 * 这一步返回之前，**任何**碰后端 AppState 的命令都会让后端 panic —— 所以它必须排在
 * AppShell 之前（见 BootGate）。浏览器演示模式没有后端，直接过。
 *
 * shortcut: 这是唯一一条绕过 api/bridge.ts 直连 invoke 的调用（bridge.ts 这次不在改动范围），
 * 以后要收口就把它挪进 bridge 的 desktop/demo 两份实现里。
 */
export function waitForBoot(): Promise<void> {
  if (!isDesktop) return Promise.resolve();
  return invoke("boot_ready").then(() => undefined);
}
