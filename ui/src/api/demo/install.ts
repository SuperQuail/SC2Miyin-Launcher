// 演示后端：可选外部工具。
import type { LauncherApi, ToolEnsure, ToolStatus } from "../types";

import { delay } from "./fixtures";

export const installDemo = {
  ensureTool: () => delay({ id: "sc2diff", name: "SC2Diff", ready: false, action: "skipped", error: null, message: null } as ToolEnsure),
  listTools: () =>
    delay([
      {
        id: "sc2diff",
        name: "SC2Diff",
        about: "对地图和模组做语义 diff 与版本控制，模组「只存差异」靠它",
        repo: "SuperQuail/SC2Diff",
        installed: false,
        version: null,
        path: null,
        size_bytes: 0,
        auto_failed: false,
        auto_error: null,
      },
    ]),
  toolReleases: () =>
    delay([
      {
        version: "0.1.0a2",
        tag: "v0.1.0a2",
        published_at: "2026-10-08T05:21:12Z",
        prerelease: true,
        download_url: "https://example.invalid/sc2diff.exe",
        size: 1_410_000,
        has_asset: true,
      },
    ]),
  installTool: () =>
    delay({
      id: "sc2diff",
      name: "SC2Diff",
      about: "演示",
      repo: "SuperQuail/SC2Diff",
      installed: true,
      version: "0.1.0a2",
      path: "D:\\demo\\sc2diff.exe",
      size_bytes: 1_410_000,
      auto_failed: false,
      auto_error: null,
    } as ToolStatus),
  uninstallTool: () => delay(undefined),
  openToolRepo: () => delay(undefined),
  openSevenZip: () => delay(undefined),
  closeSevenZip: () => delay(undefined),
  // 演示模式没有真实挂载，自然没有启动挂载警告
  takeMountWarnings: () => delay([]),
} satisfies Partial<LauncherApi>;
