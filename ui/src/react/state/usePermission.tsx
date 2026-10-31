import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";

import { api } from "../../api/bridge";
import { errorText } from "../../api/errors";

/**
 * 按需提权（docs/design/storage-and-elevation.md §3–§4）。
 *
 * 界面始终以普通权限运行、启动时不问任何存储状态：写操作照常在本进程里跑，
 * **撞上以 `[PERMISSION]` 开头的错误**才弹说明。用户同意后先拉起管理员助手
 * （UAC 只弹这一次），再把这个 op 原样重放一次 —— 界面状态不丢、不重启。
 *
 * 读操作不要走这里；游戏 / 编辑器启动也不走这里（那两件事永远在本进程做）。
 */
export const PERMISSION_PREFIX = "[PERMISSION]";

/** 重放要知道后端的 op 名与参数（参数与同名命令一致，见设计文档 §3）。 */
export interface PrivilegedRequest {
  op: string;
  args?: unknown;
}

/** 用户在对话框里选了「保持现状」：这次写入没有执行，界面照常可用。 */
export class PermissionDeclined extends Error {
  constructor() {
    super("需要管理员权限才能写入游戏目录或数据根：这次操作没有执行");
    this.name = "PermissionDeclined";
  }
}

/** 权限对话框的状态；`null` = 没弹。 */
export interface PermissionAsk {
  /** 后端给出的拒写原因（原话，含 `[PERMISSION]` 前缀）。 */
  reason: string;
  /** 上一次提权失败 / 取消的原因。 */
  failure: string | null;
  /** 正在拉起助手并重放。 */
  busy: boolean;
}

export interface PermissionApi {
  /** 对话框当前要问的话；没弹就是 null。 */
  ask: PermissionAsk | null;
  /**
   * 跑一个**写**动作。
   *
   * 走管理员助手重放时返回 `undefined`（助手的结果不保证与命令同形）——
   * 调用方应当接着重新读一遍状态，别拿它当命令返回值用。
   */
  run<T>(action: () => Promise<T>, request: PrivilegedRequest): Promise<T | undefined>;
  /** 对话框：「再次提权并同意」。 */
  elevate: () => Promise<void>;
  /** 对话框：「保持现状」——关掉弹窗，这次写入按失败处理。 */
  keepAsIs: () => void;
}

/** 去掉展示时没人想看的协议前缀。 */
export function permissionReason(text: string): string {
  return text.startsWith(PERMISSION_PREFIX) ? text.slice(PERMISSION_PREFIX.length).trim() : text;
}

/** 没套 Provider 时（组件单测）直接跑，不拦截。 */
const passthrough: PermissionApi = {
  ask: null,
  run: async function <T>(action: () => Promise<T>): Promise<T | undefined> {
    return action();
  },
  elevate: async () => undefined,
  keepAsIs: () => undefined,
};

const PermissionContext = createContext<PermissionApi | null>(null);

/** 外壳用：建一份提权状态。整个应用只有这一份（挂在 PermissionHost 里）。 */
export function usePermission(): PermissionApi {
  const [ask, setAsk] = useState<PermissionAsk | null>(null);
  /**
   * `workerRef` = 管理员助手已经拉起：之后的 [PERMISSION] 直接转给它重放，
   * 不再弹框。先在本机试一次仍然是对的 —— 存档、配置这类本来就不需要管理员。
   */
  const workerRef = useRef(false);
  const pending = useRef<{
    request: PrivilegedRequest;
    resolve: (value: undefined) => void;
    reject: (reason: unknown) => void;
  } | null>(null);

  /** 弹框并等用户裁决：同意 → 助手做完（resolve），保持现状 → reject。 */
  const askPermission = useCallback(
    (request: PrivilegedRequest, reason: string): Promise<undefined> =>
      new Promise<undefined>((resolve, reject) => {
        pending.current = { request, resolve, reject };
        setAsk({ reason, failure: null, busy: false });
      }),
    [],
  );

  const elevate = useCallback(async () => {
    const current = pending.current;
    if (!current) return;
    setAsk((state) => state && { ...state, busy: true, failure: null });
    try {
      // 先拉起助手；用户取消 UAC 时后端回 false，别当成功
      if (!(await api.requestPrivilegedWorker())) {
        throw new Error("提权没有完成：可能是在 UAC 里点了取消。");
      }
      await api.privilegedOp(current.request.op, current.request.args);
      pending.current = null;
      workerRef.current = true;
      setAsk(null);
      current.resolve(undefined);
    } catch (error) {
      // 失败留在框里：可以再点一次「再次提权」，或者保持现状
      setAsk((state) => state && { ...state, busy: false, failure: errorText(error) });
    }
  }, []);

  const keepAsIs = useCallback(() => {
    const current = pending.current;
    pending.current = null;
    setAsk(null);
    current?.reject(new PermissionDeclined());
  }, []);

  const run = useCallback(
    async <T,>(action: () => Promise<T>, request: PrivilegedRequest): Promise<T | undefined> => {
      try {
        return await action();
      } catch (error) {
        const text = errorText(error);
        if (!text.startsWith(PERMISSION_PREFIX)) throw error;
        // 助手已经起来了：同一批写操作直接交给它，别再弹一次
        if (workerRef.current) {
          await api.privilegedOp(request.op, request.args);
          return undefined;
        }
        return askPermission(request, text);
      }
    },
    [askPermission],
  );

  return { ask, run, elevate, keepAsIs };
}

/** 把这份状态交给页面（对话框画在 PermissionHost 里，这里只递状态）。 */
export function PermissionScope({ value, children }: { value: PermissionApi; children: ReactNode }) {
  return <PermissionContext.Provider value={value}>{children}</PermissionContext.Provider>;
}

/** 页面 / 领域 hook 用：拿全局的提权闸门（没套 Provider 时是不拦截的空实现）。 */
export function usePermissionGate(): PermissionApi {
  return useContext(PermissionContext) ?? passthrough;
}
