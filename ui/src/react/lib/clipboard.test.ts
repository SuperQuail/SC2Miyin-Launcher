import { afterEach, describe, expect, it, vi } from "vitest";

import { copyText } from "./clipboard";

/**
 * 复制有两条路：先问 navigator.clipboard，不行再退回 textarea + execCommand。
 *
 * WebView 里哪条通不一定，所以两条都得管用；而且**失败只返回 false**（不抛）——
 * 复制失败弹一句提示就够了，不该让作弊码页崩掉。
 */
function installClipboard(writeText: (() => Promise<void>) | null): void {
  if (writeText === null) {
    Reflect.deleteProperty(navigator, "clipboard");
    return;
  }
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
}

/** 回退路径用的 document.execCommand —— jsdom 压根没实现，自己塞一个。 */
function installExecCommand(exec: () => boolean) {
  const mock = vi.fn(exec);
  document.execCommand = mock;
  return mock;
}

afterEach(() => {
  Reflect.deleteProperty(navigator, "clipboard");
  Reflect.deleteProperty(document, "execCommand");
  vi.restoreAllMocks();
  document.body.innerHTML = "";
});

describe("copyText", () => {
  it("首选路径能用就用它，不碰 execCommand", async () => {
    const writeText = vi.fn(async () => {});
    installClipboard(writeText);
    const exec = installExecCommand(() => true);

    await expect(copyText("show me the money")).resolves.toBe(true);

    expect(writeText).toHaveBeenCalledWith("show me the money");
    expect(exec).not.toHaveBeenCalled();
  });

  it("Clipboard API 抛错时退回 textarea + execCommand", async () => {
    installClipboard(async () => {
      throw new Error("这个 WebView 不许写剪贴板");
    });
    const exec = installExecCommand(() => true);

    await expect(copyText("ophelia")).resolves.toBe(true);

    expect(exec).toHaveBeenCalledWith("copy");
    expect(document.querySelector("textarea"), "临时输入框用完必须摘掉").toBeNull();
  });

  it("这台 WebView 根本没有 navigator.clipboard 时也走回退", async () => {
    installClipboard(null);
    const exec = installExecCommand(() => true);

    await expect(copyText("x")).resolves.toBe(true);

    expect(exec).toHaveBeenCalledWith("copy");
  });

  it("两条路都失败就返回 false，不抛", async () => {
    installClipboard(async () => {
      throw new Error("不许写");
    });
    installExecCommand(() => false);

    await expect(copyText("x")).resolves.toBe(false);
    expect(document.querySelector("textarea"), "失败也要摘掉临时输入框").toBeNull();
  });

  it("执意要复制空串也照常走（不给调用方挖坑）", async () => {
    const writeText = vi.fn(async () => {});
    installClipboard(writeText);

    await expect(copyText("")).resolves.toBe(true);
    expect(writeText).toHaveBeenCalledWith("");
  });
});
