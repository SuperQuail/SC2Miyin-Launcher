import { describe, expect, it } from "vitest";

import { comboOf, shouldBlock } from "./webview";

const key = (init: KeyboardEventInit) => new KeyboardEvent("keydown", init);

describe("webview 快捷键拦截", () => {
  it("拦掉浏览器味儿的组合", () => {
    expect(shouldBlock(key({ key: "r", ctrlKey: true }))).toBe(true);
    expect(shouldBlock(key({ key: "F5" }))).toBe(true);
    expect(shouldBlock(key({ key: "p", ctrlKey: true }))).toBe(true);
    expect(shouldBlock(key({ key: "+", ctrlKey: true }))).toBe(true);
    expect(shouldBlock(key({ key: "F12" }))).toBe(true);
    expect(shouldBlock(key({ key: "i", ctrlKey: true, shiftKey: true }))).toBe(true);
    expect(shouldBlock(key({ key: "w", ctrlKey: true }))).toBe(true);
  });

  it("不误伤编辑与复制粘贴", () => {
    expect(shouldBlock(key({ key: "c", ctrlKey: true }))).toBe(false);
    expect(shouldBlock(key({ key: "v", ctrlKey: true }))).toBe(false);
    expect(shouldBlock(key({ key: "z", ctrlKey: true }))).toBe(false);
    expect(shouldBlock(key({ key: "s", ctrlKey: true }))).toBe(false);
    expect(shouldBlock(key({ key: "a" }))).toBe(false);
    expect(shouldBlock(key({ key: "ArrowDown", altKey: true }))).toBe(false);
  });

  it("allow 能放行（开发者页要留 Ctrl+F 与开发者工具）", () => {
    expect(shouldBlock(key({ key: "f", ctrlKey: true }), ["Ctrl+F"])).toBe(false);
    expect(shouldBlock(key({ key: "F12" }), ["F12"])).toBe(false);
    // 放行是精确匹配，不会把别的一起放掉
    expect(shouldBlock(key({ key: "r", ctrlKey: true }), ["Ctrl+F"])).toBe(true);
  });

  it("页面自己处理过的不再拦（避免和 CodeMirror 之类抢）", () => {
    // 合成事件默认 cancelable 为 false，不标上 preventDefault 就不生效
    const handled = key({ key: "f", ctrlKey: true, cancelable: true });
    handled.preventDefault();
    expect(shouldBlock(handled)).toBe(false);
  });

  it("组合串是归一化的", () => {
    expect(comboOf(key({ key: "R", ctrlKey: true, shiftKey: true }))).toBe("ctrl+shift+r");
    expect(comboOf(key({ key: "F5" }))).toBe("f5");
  });
});
