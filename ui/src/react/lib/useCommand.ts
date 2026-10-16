import { useCallback, useRef, useState } from "react";

import { errorText } from "../../api/errors";

/**
 * 跑一个异步动作，顺手把"在跑 / 出错"管起来。
 *
 * 为什么要有它：现在`try { … } catch (error) { notify("error", errorText(error)) }`
 * 这个模板在用户侧写了 **56 遍**，每个动作都要手抄三件套，漏一个就是静默失败。
 *
 * 用法：
 * ```tsx
 * const save = useCommand(async (name: string) => api.rename(name), { onError: notifyError });
 * <Button disabled={save.loading} onClick={() => void save.run("abc")}>保存</Button>
 * ```
 *
 * 失败**不会抛**：错误文本放在 `error` 里，同时交给 `onError`（通常是外壳的通知）。
 * 调用方拿到的是 `undefined` —— 想分支就判它。
 */
export function useCommand<Args extends unknown[], Result>(
  action: (...args: Args) => Promise<Result>,
  options?: {
    /** 出错时喊一声（一般是外壳的 notify）。 */
    onError?: (text: string) => void;
    /** 成功时喊一声。 */
    onDone?: (result: Result) => void;
  },
): { run: (...args: Args) => Promise<Result | undefined>; loading: boolean; error: string | null } {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // 回调每次渲染都是新的，用 ref 存住，免得 run 的依赖天天变
  const latest = useRef({ action, options });
  latest.current = { action, options };

  const run = useCallback(async (...args: Args): Promise<Result | undefined> => {
    setLoading(true);
    setError(null);
    try {
      const result = await latest.current.action(...args);
      latest.current.options?.onDone?.(result);
      return result;
    } catch (thrown) {
      const text = errorText(thrown);
      setError(text);
      latest.current.options?.onError?.(text);
      return undefined;
    } finally {
      setLoading(false);
    }
  }, []);

  return { run, loading, error };
}
