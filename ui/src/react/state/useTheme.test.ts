import { beforeEach, describe, expect, it } from "vitest";

import { readStoredTheme, THEME_KEY, writeStoredTheme } from "./useTheme";

describe("主题偏好", () => {
  beforeEach(() => localStorage.clear());

  it("没存过默认夜间", () => {
    expect(readStoredTheme()).toBe("dark");
  });

  it("日间 / 夜间都能原样读回来", () => {
    writeStoredTheme("light");
    expect(localStorage.getItem(THEME_KEY)).toBe("light");
    expect(readStoredTheme()).toBe("light");

    writeStoredTheme("dark");
    expect(readStoredTheme()).toBe("dark");
  });

  it("存了坏值默认夜间", () => {
    localStorage.setItem(THEME_KEY, "蓝");
    expect(readStoredTheme()).toBe("dark");
  });

  it("旧的 system 值落到夜间", () => {
    localStorage.setItem(THEME_KEY, "system");
    expect(readStoredTheme()).toBe("dark");
  });
});
