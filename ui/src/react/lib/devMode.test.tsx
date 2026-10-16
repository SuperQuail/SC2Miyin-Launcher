import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { isDevMode, setDevMode, useDevMode } from "./devMode";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * 开发者模式存在 localStorage 里 —— 读写的每一处都可能碰上隐私模式
 * （读抛、写抛）。这两条路径都不许把异常放出来：一个开关失败不该让设置页崩掉。
 */
const KEY = "miyin.devMode";

const mounted: Array<() => Promise<void>> = [];

beforeEach(() => {
  localStorage.clear();
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  localStorage.clear();
  vi.restoreAllMocks();
});

describe("开发者模式的存取", () => {
  it("没设置过就是关", () => {
    expect(isDevMode()).toBe(false);
  });

  it("打开写进去、关掉清掉", () => {
    setDevMode(true);
    expect(localStorage.getItem(KEY)).toBe("1");
    expect(isDevMode()).toBe(true);

    setDevMode(false);
    expect(localStorage.getItem(KEY)).toBeNull();
    expect(isDevMode()).toBe(false);
  });

  it("只有恰好是 \"1\" 才算打开", () => {
    localStorage.setItem(KEY, "true");
    expect(isDevMode()).toBe(false);

    localStorage.setItem(KEY, "0");
    expect(isDevMode()).toBe(false);
  });

  it("读不了（隐私模式）时当作关，不抛", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("隐私模式下读不了 localStorage");
    });

    expect(() => isDevMode()).not.toThrow();
    expect(isDevMode()).toBe(false);
  });

  it("写不进去也不报错", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("配额满了");
    });

    expect(() => setDevMode(true)).not.toThrow();
  });

  it("清不掉也不报错", () => {
    vi.spyOn(Storage.prototype, "removeItem").mockImplementation(() => {
      throw new Error("配额满了");
    });

    expect(() => setDevMode(false)).not.toThrow();
  });
});

async function mountDevMode() {
  let latest: [boolean, (on: boolean) => void] | null = null;

  function Harness(): null {
    latest = useDevMode();
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

  const read = (): [boolean, (on: boolean) => void] => {
    if (!latest) throw new Error("harness 还没渲染出来");
    return latest;
  };

  return {
    get enabled() {
      return read()[0];
    },
    async set(next: boolean) {
      await act(async () => {
        read()[1](next);
      });
    },
    async fireStorageEvent() {
      await act(async () => {
        window.dispatchEvent(new Event("storage"));
      });
    },
  };
}

describe("useDevMode —— 设置页的那个开关", () => {
  it("初值来自 localStorage", async () => {
    localStorage.setItem(KEY, "1");
    const view = await mountDevMode();

    expect(view.enabled).toBe(true);
  });

  it("没设置过时初始是关", async () => {
    const view = await mountDevMode();

    expect(view.enabled).toBe(false);
  });

  it("切换会同时改状态和存储（下次进设置页才是新值）", async () => {
    const view = await mountDevMode();

    await view.set(true);
    expect(view.enabled).toBe(true);
    expect(localStorage.getItem(KEY)).toBe("1");

    await view.set(false);
    expect(view.enabled).toBe(false);
    expect(localStorage.getItem(KEY)).toBeNull();
  });

  it("别的标签页改了会跟上（storage 事件）", async () => {
    const view = await mountDevMode();
    expect(view.enabled).toBe(false);

    localStorage.setItem(KEY, "1");
    await view.fireStorageEvent();

    expect(view.enabled).toBe(true);
  });
});
