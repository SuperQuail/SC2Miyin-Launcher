import { describe, expect, it } from "vitest";

import {
  forcedThemeFromSearch,
  resolveStartView,
  skipsAutoUpdate,
  viewFromSearch,
  wantsRoom,
} from "./flags";

describe("调试开关", () => {
  it("?view= 认得页签里的那几页，其余回战役页", () => {
    expect(viewFromSearch("?view=settings")).toBe("settings");
    expect(viewFromSearch("?view=custom")).toBe("custom");
    // room 不在页签里，只能从设置页的开发者模式进
    expect(viewFromSearch("?view=room")).toBe("campaigns");
    expect(viewFromSearch("?view=nope")).toBe("campaigns");
    expect(viewFromSearch("")).toBe("campaigns");
  });

  it("?theme= 只认 light / dark", () => {
    expect(forcedThemeFromSearch("?theme=light")).toBe("light");
    expect(forcedThemeFromSearch("?theme=dark")).toBe("dark");
    expect(forcedThemeFromSearch("?theme=system")).toBeNull();
    expect(forcedThemeFromSearch("")).toBeNull();
  });

  it("?noupdate 只看在不在", () => {
    expect(skipsAutoUpdate("?noupdate=1")).toBe(true);
    expect(skipsAutoUpdate("?view=mods")).toBe(false);
  });

  it("#room 仍要求开发者模式开着", () => {
    expect(wantsRoom("#room", true)).toBe(true);
    expect(wantsRoom("#room", false)).toBe(false);
    expect(wantsRoom("", true)).toBe(false);
  });

  it("启动落在哪一页：房间优先于 view", () => {
    expect(resolveStartView({ search: "?view=mods", hash: "", devMode: false })).toBe("mods");
    expect(resolveStartView({ search: "", hash: "#room", devMode: true })).toBe("room");
    expect(resolveStartView({ search: "?view=mods", hash: "#room", devMode: false })).toBe("mods");
  });
});
