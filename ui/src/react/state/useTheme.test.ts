import { beforeEach, describe, expect, it } from "vitest";

import { activeTheme, readStoredTheme, THEME_KEY, writeStoredTheme } from "./useTheme";

describe("主题偏好", () => {
  beforeEach(() => localStorage.clear());

  it("没存过就跟随系统", () => {
    expect(readStoredTheme()).toBe("system");
  });

  it("存进去能原样读回来", () => {
    writeStoredTheme("dark");
    expect(localStorage.getItem(THEME_KEY)).toBe("dark");
    expect(readStoredTheme()).toBe("dark");
  });

  it("存了坏值当没存过", () => {
    localStorage.setItem(THEME_KEY, "蓝");
    expect(readStoredTheme()).toBe("system");
  });

  it("跟随系统时按系统深浅合一档", () => {
    expect(activeTheme("system", true)).toBe("dark");
    expect(activeTheme("system", false)).toBe("light");
    expect(activeTheme("light", true)).toBe("light");
    expect(activeTheme("dark", false)).toBe("dark");
  });
});
