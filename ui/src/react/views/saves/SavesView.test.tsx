import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SaveBackup, SaveIsolation, SaveTarget } from "../../../api/types";
import { SavesView } from "./SavesView";

/**
 * 存档页：**一行 = 一个战役/版本**（已启用的各占一格，哪怕还没存档、哪怕它是原版）。
 *
 * 这一页只负责「存档归哪个战役」—— 切换战役在战役页，所以行里只有「管理存档」一个按钮。
 * 钉四件事：已启用各占一行、空分类照样出现、未启用的折在对应战役下、没有任何重复标签。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const targets: SaveTarget[] = [
  { key: "wol", label: "自由之翼（原版）", enabled: true },
  { key: "wol/重生", label: "自由之翼 · 自由之翼：重生", enabled: false },
  { key: "hots", label: "虫群之心（原版）", enabled: true },
  { key: "custom/SCMR", label: "自制战役 · SCMR", enabled: true },
];

const groups: SaveBackup[] = [
  { name: "自由之翼-1790000100", label: "自由之翼", bytes: 1_220_000, files: 2 },
  { name: "原版-1790000000", label: "原版", bytes: 1_842_000, files: 3, legacy: true },
  // 同一战役的空壳目录：不该在页面上再出现一次（用户报的「没去重」）
  { name: "自由之翼_·_重生-1790000099", label: "自由之翼 · 重生", bytes: 0, files: 0 },
];

const isolation: SaveIsolation = {
  enabled: true,
  active: "自由之翼-1790000100",
  assignments: { "自由之翼-1790000100": "wol" },
};

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(overrides?: { targets?: SaveTarget[]; groups?: SaveBackup[]; isolation?: SaveIsolation }) {
  const onRefresh = vi.fn();
  const onOpenSettings = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(SavesView, {
        targets: overrides?.targets ?? targets,
        isolation: overrides?.isolation ?? isolation,
        groups: overrides?.groups ?? groups,
        banks: { files: [{ name: "ENS.SC2Bank", bytes: 611_000 }], bytes: 1_842_000, missing: false },
        busy: false,
        onRefresh,
        onOpenSettings,
        onGroupFiles: vi.fn(async () => []),
        onMoveToKey: vi.fn(async () => {}),
      }),
    );
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  return { onRefresh, onOpenSettings };
}

describe("存档页", () => {
  it("已启用的战役/版本各占一行（哪怕还没存档）", async () => {
    await mount();
    const rows = [...document.querySelectorAll(".saves__row")];
    expect(rows).toHaveLength(3);
    const text = document.body.textContent ?? "";
    expect(text).toContain("自由之翼（原版）");
    expect(text).toContain("2 个文件");
    expect(text).toContain("虫群之心（原版）");
    expect(text).toContain("还没有存档");
  });

  it("当前在用的那一行有标记", async () => {
    await mount();
    expect(document.querySelector(".saves__row--active")?.textContent).toContain("当前在用");
  });

  it("行里只有「管理存档」一个按钮（切换战役属于战役页）", async () => {
    await mount();
    const texts = [...document.querySelectorAll(".saves__ops button")].map((button) => button.textContent?.trim());
    expect(texts).toEqual(["管理存档"]);
  });

  it("未启用的旧组默认折叠，展开后折到对应战役下面", async () => {
    await mount();
    expect(document.body.textContent).not.toContain("未分类（升级前的存档）");
    const toggle = [...document.querySelectorAll("button")].find((button) =>
      (button.textContent ?? "").includes("显示未启用"),
    );
    expect(toggle, "要有折叠开关").toBeTruthy();
    await act(async () => {
      toggle?.click();
    });
    expect(document.body.textContent).toContain("未分类（升级前的存档）");
    // 空壳目录不该重复出现
    const labels = [...document.querySelectorAll(".saves__title-text")].map((item) => item.textContent);
    expect(new Set(labels).size).toBe(labels.length);
  });

  it("既没启用分类、也没有旧组时给空状态说明", async () => {
    await mount({ targets: [], groups: [] });
    expect(document.querySelectorAll(".saves__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("还没有存档分类");
  });
});
