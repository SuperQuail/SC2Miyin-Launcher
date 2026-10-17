import { useCallback, useState } from "react";

import { api } from "../../api/bridge";
import type { MergeChoice, MergeStatus } from "../../api/types";
import { useCommand } from "../lib/useCommand";
import type { Notify } from "./useNotify";

export interface MergeApi {
  /** 另一份数据；null = 没得并可并（或者用户这次已经关掉了）。 */
  status: MergeStatus | null;
  /** key `<slot>/<variant>` -> 怎么处理。默认全是「保持游戏目录那份」。 */
  choices: Record<string, MergeChoice>;
  setChoice: (key: string, choice: MergeChoice) => void;
  /** 启动时查一次（外壳挂载时调）。 */
  check: () => Promise<void>;
  /** 正在合并（按钮转圈用）。 */
  applying: boolean;
  apply: () => Promise<void>;
  /** 稍后：这次不问了，下次启动再问。 */
  dismiss: () => void;
}

/**
 * 启动时的数据合并（AGENTS §14.1）。
 *
 * 可能有多个启动器实例，各自带着自己的 data/，所以要把程序旁边那份**合并进**
 * 游戏目录同级那份。这里只管"问用户"这一段：不冲突的由后端直接并，
 * 冲突的后端只报上来、等 `merge_apply` 收选择再动手。
 */
export function useMerge(options: { notify: Notify }): MergeApi {
  const { notify } = options;
  const [status, setStatus] = useState<MergeStatus | null>(null);
  const [choices, setChoices] = useState<Record<string, MergeChoice>>({});
  const [dismissed, setDismissed] = useState(false);

  /** 启动时查一次。查不到不算错误 —— 静默跳过，多问一次比弹个红字强。 */
  const check = useCallback(async () => {
    try {
      const found = await api.mergeStatus();
      if (!found) return;
      setStatus(found);
      setDismissed(false);
      // 默认：冲突一律用游戏目录那份（本地那份还留在原地，没动它）
      setChoices(
        Object.fromEntries(found.conflicts.map((conflict) => [conflict.slot + "/" + conflict.variant, "target" as const])),
      );
    } catch {
      /* 启动时的探查不该打扰用户 */
    }
  }, []);

  const setChoice = useCallback((key: string, choice: MergeChoice) => {
    setChoices((current) => ({ ...current, [key]: choice }));
  }, []);

  const command = useCommand((choicesToApply: Record<string, MergeChoice>) => api.mergeApply(choicesToApply), {
    onError: (text) => notify("error", text),
    onDone: () => {
      const total = status ? status.additions.length + status.conflicts.length : 0;
      setStatus(null);
      notify("success", "已合并 " + total + " 个版本");
    },
  });

  const apply = useCallback(async () => {
    // merge_apply 返回 void，只有 onDone（成功）才会走到 —— 失败由 onError 喊
    await command.run(choices);
  }, [choices, command]);

  return {
    status: dismissed ? null : status,
    choices,
    setChoice,
    check,
    applying: command.loading,
    apply,
    dismiss: () => setDismissed(true),
  };
}
