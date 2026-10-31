import { afterEach, describe, expect, it, vi } from "vitest";

import { reportMountWarnings } from "./boot";

/** 启动挂载警告怎么给人看：前 3 条 toast，其余控制台。 */

afterEach(() => {
  vi.restoreAllMocks();
});

describe("启动挂载警告", () => {
  it("前 3 条走 toast，其余进控制台", () => {
    const notify = vi.fn();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);

    reportMountWarnings(notify, ["a", "b", "c", "d", "e"]);

    expect(notify.mock.calls).toEqual([
      ["warning", "a"],
      ["warning", "b"],
      ["warning", "c"],
    ]);
    expect(warn.mock.calls).toEqual([
      ["[miyin] 模组挂载警告：d"],
      ["[miyin] 模组挂载警告：e"],
    ]);
  });

  it("没有警告就什么都不做", () => {
    const notify = vi.fn();
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);

    reportMountWarnings(notify, []);

    expect(notify).not.toHaveBeenCalled();
    expect(warn).not.toHaveBeenCalled();
  });
});
