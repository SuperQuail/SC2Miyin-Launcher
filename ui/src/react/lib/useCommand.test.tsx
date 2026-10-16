import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { useCommand } from "./useCommand";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * 统一动作三件套（在跑 / 出错 / 回调）。
 *
 * 这份测试守的是三条约定：**失败不抛**、错误文本进 `error`、loading 一定落回 false。
 * 落了哪一条，调用方要么静默失败，要么按钮永远卡在禁用态 —— 两种都只有用户先发现。
 *
 * 没有 @testing-library/react（也不许装），所以自己用 createRoot + act 把 hook 挂起来，
 * 再从外部读它每次渲染后的值。
 */
interface CommandState<A extends unknown[], R> {
  run: (...args: A) => Promise<R | undefined>;
  loading: boolean;
  error: string | null;
}

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
});

/** 一个可以手动放行的闸门，用来观察「动作还在跑」的那一瞬间。 */
function createGate() {
  let release: () => void = () => {};
  const promise = new Promise<void>((resolve) => {
    release = resolve;
  });
  return { promise, release: () => release() };
}

async function mountCommand<A extends unknown[], R>(
  action: (...args: A) => Promise<R>,
  options?: { onError?: (text: string) => void; onDone?: (result: R) => void },
) {
  let latest: CommandState<A, R> | null = null;

  function Harness(): null {
    latest = useCommand(action, options);
    return null;
  }

  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(createElement(Harness));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  const read = (): CommandState<A, R> => {
    if (!latest) throw new Error("harness 还没渲染出来");
    return latest;
  };

  return {
    get run() {
      return read().run;
    },
    get loading() {
      return read().loading;
    },
    get error() {
      return read().error;
    },
  };
}

describe("useCommand", () => {
  it("成功：返回结果并交给 onDone，error 保持 null", async () => {
    const onDone = vi.fn();
    const onError = vi.fn();
    const view = await mountCommand(async (n: number) => n * 2, { onDone, onError });

    let result: number | undefined;
    await act(async () => {
      result = await view.run(21);
    });

    expect(result).toBe(42);
    expect(view.error).toBeNull();
    expect(view.loading).toBe(false);
    expect(onDone).toHaveBeenCalledWith(42);
    expect(onError).not.toHaveBeenCalled();
  });

  it("失败不抛：错误文本进 error、交给 onError、返回 undefined", async () => {
    const onError = vi.fn();
    const onDone = vi.fn();
    const view = await mountCommand(
      async () => {
        throw new Error("导入失败：包已损坏");
      },
      { onError, onDone },
    );

    let result: string | undefined = "占位";
    let rejected = false;
    await act(async () => {
      try {
        result = await view.run();
      } catch {
        rejected = true;
      }
    });

    expect(rejected, "失败路径不许把异常抛给调用方").toBe(false);
    expect(result).toBeUndefined();
    expect(view.error).toBe("导入失败：包已损坏");
    expect(onError).toHaveBeenCalledWith("导入失败：包已损坏");
    expect(onDone).not.toHaveBeenCalled();
    expect(view.loading, "出错后也必须落回 false").toBe(false);
  });

  it("跑的过程中 loading 是 true，结束才落回 false", async () => {
    const gate = createGate();
    const view = await mountCommand(async () => {
      await gate.promise;
      return "完成";
    });

    expect(view.loading).toBe(false);

    let pending: Promise<string | undefined> | undefined;
    await act(async () => {
      pending = view.run();
    });
    expect(view.loading, "动作还在跑，按钮应当禁用").toBe(true);

    gate.release();
    await act(async () => {
      await pending;
    });

    expect(view.loading).toBe(false);
    expect(view.error).toBeNull();
  });

  it("上一次的错误在下一次成功后清空", async () => {
    let attempt = 0;
    const view = await mountCommand(async () => {
      attempt += 1;
      if (attempt === 1) throw new Error("第一次失败");
      return "好了";
    });

    await act(async () => {
      await view.run();
    });
    expect(view.error).toBe("第一次失败");

    let second: string | undefined;
    await act(async () => {
      second = await view.run();
    });

    expect(second).toBe("好了");
    expect(view.error).toBeNull();
  });

  it("抛的不是 Error 也能读成文案（桥接层可能直接抛字符串）", async () => {
    const view = await mountCommand(async (): Promise<void> => {
      throw "桥接层直接抛了个字符串";
    });

    await act(async () => {
      await view.run();
    });

    expect(view.error).toBe("桥接层直接抛了个字符串");
  });

  it("不给 onError / onDone 也不炸", async () => {
    const ok = await mountCommand(async () => 1);
    await act(async () => {
      await ok.run();
    });
    expect(ok.error).toBeNull();

    const bad = await mountCommand(async (): Promise<number> => {
      throw new Error("失败");
    });
    await act(async () => {
      await bad.run();
    });
    expect(bad.error).toBe("失败");
  });
});
