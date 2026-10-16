import { describe, expect, it } from "vitest";

import { appendLog, MAX_LOG_LINES } from "./log";

describe("更新终端的一行", () => {
  it("时间戳是本地 HH:MM:SS", () => {
    const at = new Date(2026, 9, 16, 9, 5, 3);
    expect(appendLog([], "开始检查更新", at)).toEqual(["[09:05:03] 开始检查更新"]);
  });

  it("最多留 400 行，新的挤掉最老的", () => {
    let lines: string[] = [];
    for (let index = 0; index < MAX_LOG_LINES + 5; index += 1) {
      lines = appendLog(lines, String(index), new Date(2026, 9, 16, 0, 0, 0));
    }
    expect(lines).toHaveLength(MAX_LOG_LINES);
    expect(lines[0]).toContain("5");
    expect(lines[lines.length - 1]).toContain(String(MAX_LOG_LINES + 4));
  });
});
