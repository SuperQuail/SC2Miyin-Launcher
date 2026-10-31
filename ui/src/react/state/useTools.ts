import { useCallback } from "react";

import { api } from "../../api/bridge";
import type { Notify } from "./useNotify";

/**
 * 启动时把可选工具装上 —— **默认静默安装**。
 *
 * 约定：没有就自动装；装不上提示一句「该部分功能不可用」，之后启动不再尝试
 * （后端记了标记，下次直接返回 skipped），用户想再试到设置里手动装。
 *
 * 这里**绝不能抛** —— 一个可选工具装不上，不该影响启动器本身能用。
 */
export function useTools(options: { notify: Notify }): { ensureTools: () => Promise<void> } {
  const { notify } = options;

  const ensureTools = useCallback(async () => {
    try {
      const result = await api.ensureTool("sc2diff");

      if (result.action === "installed") {
        notify("success", result.name + " 已自动安装");
      } else if (result.action === "failed" && result.message) {
        notify("warning", result.message);
      }
      // already_installed / skipped 都不打扰用户
    } catch {
      // 静默：这条路失败不影响启动
    }
  }, [notify]);

  return { ensureTools };
}
