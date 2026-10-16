import { describe, expect, it } from "vitest";

import { errorText } from "./errors";

/**
 * errorText 是错误提示的**唯一出口**（Vue 侧与 React 侧共用）。
 *
 * 它要是把 Error 读成 "[object Object]"，用户看到的就是一句废话 ——
 * 而错误路径平时没人走，只有真出事的那一刻才暴露。
 */
describe("errorText", () => {
  it("字符串原样返回（桥接层常常直接抛字符串）", () => {
    expect(errorText("路径不存在：D:\\Game\\StarCraft II")).toBe("路径不存在：D:\\Game\\StarCraft II");
  });

  it("Error 取 message", () => {
    expect(errorText(new Error("磁盘空间不足"))).toBe("磁盘空间不足");
  });

  it("Error 子类一样取 message", () => {
    class BridgeError extends Error {}
    expect(errorText(new BridgeError("桥接调用失败"))).toBe("桥接调用失败");
  });

  it("其它值 String 化，不抛", () => {
    expect(errorText(42)).toBe("42");
    expect(errorText(true)).toBe("true");
    expect(errorText(null)).toBe("null");
    expect(errorText(undefined)).toBe("undefined");
    expect(errorText({ code: 500 })).toBe("[object Object]");
  });
});
