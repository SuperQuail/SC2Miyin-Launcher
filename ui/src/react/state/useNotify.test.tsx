import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TOAST_MAX, TOAST_MS, useNotify, type NotifyApi } from "./useNotify";

/**
 * 提示队列。这三件事都是"类型过得去、真跑起来才看得见"的：
 * **上限**（第 4 条该挤掉最旧的）、**悬停暂停**（倒计时真的冻住）、
 * **精确关闭**（✕ 只关那一条，不是把整叠清掉）。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let latest: NotifyApi | null = null;
const mounted: Array<() => Promise<void>> = [];

async function mountHook(): Promise<NotifyApi> {
  function Probe() {
    latest = useNotify();
    return null;
  }
  const element = document.createElement("div");
  document.body.appendChild(element);
  const root = createRoot(element);
  await act(async () => {
    root.render(createElement(Probe));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    element.remove();
  });
  if (!latest) throw new Error("hook 没挂上");
  return latest;
}

/** 往前拨时钟 —— 走表和 Date.now() 一起走，不然到点判断不成立。 */
async function advance(ms: number) {
  await act(async () => {
    vi.advanceTimersByTime(ms);
  });
}

/** 推一条并等 React 刷完。 */
async function push(api: NotifyApi, message: string) {
  await act(async () => {
    api.notify("info", message);
  });
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
  latest = null;
  vi.useRealTimers();
});

describe("useNotify", () => {
  it("往队列里推，最新那条排在最前", async () => {
    const api = await mountHook();
    expect(latest?.toasts).toHaveLength(0);

    await push(api, "第一条");
    await push(api, "第二条");

    const toasts = latest?.toasts ?? [];
    expect(toasts.map((toast) => toast.message)).toEqual(["第二条", "第一条"]);
    // 每条都有自己的号 —— ✕ 靠它精确关闭
    expect(new Set(toasts.map((toast) => toast.id)).size).toBe(2);
  });

  it("同时最多 3 条：第 4 条挤掉最旧的", async () => {
    const api = await mountHook();
    for (const text of ["1", "2", "3", "4"]) await push(api, text);

    const toasts = latest?.toasts ?? [];
    expect(toasts).toHaveLength(TOAST_MAX);
    expect(toasts.map((toast) => toast.message)).toEqual(["4", "3", "2"]);
  });

  it("到点先退场、再卸载（两段计时）", async () => {
    const api = await mountHook();
    await push(api, "会自己走");

    await advance(TOAST_MS - 100);
    expect(latest?.toasts, "还没到点就不该走").toHaveLength(1);

    // 过了 TOAST_MS：标记 leaving，退出动画开始
    await advance(200);
    expect(latest?.toasts[0]?.leaving, "该进入退场了").toBe(true);

    // 再等退场动画放完才真正卸掉
    await advance(200);
    expect(latest?.toasts).toHaveLength(0);
  });

  it("悬停暂停：倒计时冻住，移开接着走", async () => {
    const api = await mountHook();
    await push(api, "悬停时别走");
    await advance(1000);

    await act(async () => {
      api.pause();
    });
    // 暂停期间过掉的时间远超 TOAST_MS，但一条都不该消失
    await advance(TOAST_MS * 3);
    expect(latest?.toasts, "鼠标还压在上面").toHaveLength(1);
    expect(latest?.toasts[0]?.leaving).toBe(false);

    await act(async () => {
      api.resume();
    });
    // 移开后接着走完剩下的 —— 剩余时间不丢
    await advance(TOAST_MS - 1000);
    expect(latest?.toasts[0]?.leaving, "剩下的时间走完就该退场").toBe(true);
  });

  it("✕ 只关那一条，别的照旧", async () => {
    const api = await mountHook();
    await push(api, "旧的");
    await push(api, "新的");
    const older = (latest?.toasts ?? []).find((toast) => toast.message === "旧的");
    expect(older).toBeTruthy();

    await act(async () => {
      api.dismiss(older!.id);
    });

    expect(latest?.toasts.find((toast) => toast.id === older!.id)?.leaving, "被点的这条该退场").toBe(true);
    expect(latest?.toasts.find((toast) => toast.message === "新的")?.leaving, "没被点的不该动").toBe(false);

    await advance(200);
    expect(latest?.toasts.map((toast) => toast.message)).toEqual(["新的"]);
  });

  it("没有提示时不挂走表（空队列不空转）", async () => {
    const api = await mountHook();
    const before = vi.getTimerCount();
    await advance(1000);
    expect(latest?.toasts).toHaveLength(0);
    expect(vi.getTimerCount(), "空队列不该有定时器在跑").toBe(before);
    expect(api.notify).toBeTypeOf("function");
  });
});
