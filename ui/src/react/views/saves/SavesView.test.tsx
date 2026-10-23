import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SaveBackup, SaveIsolation, SaveTarget } from "../../../api/types";
import { SavesView } from "./SavesView";

/**
 * 存档页的渲染与接线。
 *
 * 这一页的核心是**一行 = 一个战役/版本**（已启用的各占一格，哪怕还没存档、哪怕它是原版），
 * 没启用的旧组默认折叠。这里钉四件事：已启用各占一行、空分类照样出现、折叠开关、
 * 以及「切换为该分类 / 设为当前」别被写成同一个动作。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const targets: SaveTarget[] = [
  { key: "wol", label: "自由之翼（原版）", enabled: true },
  { key: "wol/重生", label: "自由之翼 · 自由之翼：重生", enabled: false },
  { key: "hots", label: "虫群之心（原版）", enabled: true },
];

const groups: SaveBackup[] = [
  { name: "自由之翼-1790000100", label: "自由之翼", bytes: 1_220_000, files: 2 },
  { name: "原版-1790000000", label: "原版", bytes: 1_842_000, files: 3, legacy: true },
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
  const onSwitch = vi.fn();
  const onSaveInto = vi.fn();
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
        onSwitch,
        onSaveInto,
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
  return { onSwitch, onSaveInto, onOpenSettings };
}

describe("存档页", () => {
  it("已启用的战役/版本各占一行（哪怕还没存档）", async () => {
    await mount();
    const rows = [...document.querySelectorAll(".saves__row")];
    // wol（有 2 个文件）+ hots（还没有组 -> 「还没有存档」）
    expect(rows).toHaveLength(2);
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

  it("未启用的旧组默认折叠，点开才出现", async () => {
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
  });

  it("「切换为该分类」和「设为当前」是两个不同的动作", async () => {
    // active 置空，这样那一行不是"正在用"，两个动作都点得动
    const { onSwitch, onSaveInto } = await mount({
      isolation: { enabled: true, active: null, assignments: isolation.assignments },
    });
    const row = [...document.querySelectorAll(".saves__row")].find((item) =>
      (item.textContent ?? "").includes("自由之翼（原版）"),
    ) as HTMLElement;
    const buttons = [...row.querySelectorAll("button")];
    await act(async () => {
      buttons.find((button) => button.textContent === "切换为该分类")?.click();
    });
    expect(onSwitch).toHaveBeenCalledWith("自由之翼-1790000100");

    await act(async () => {
      buttons.find((button) => button.textContent === "设为当前")?.click();
    });
    expect(onSaveInto).toHaveBeenCalledWith("自由之翼-1790000100");
  });

  it("既没启用分类、也没有旧组时给空状态说明", async () => {
    await mount({ targets: [], groups: [] });
    expect(document.querySelectorAll(".saves__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("还没有存档分类");
  });
});
