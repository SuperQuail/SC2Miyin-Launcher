import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SaveBackup, SaveIsolation, SaveTarget } from "../../../api/types";
import { SavesView } from "./SavesView";

/**
 * 存档页的渲染与接线。
 *
 * 两件事最容易坏：**归属下拉真的接到 onAssign**（这是"手动定位到别的战役"的唯一入口），
 * 以及「切换为该分类 / 设为当前」别再被写成同一个动作（一个是换内容，一个是把现在这份存回去）。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const targets: SaveTarget[] = [
  { key: "wol", label: "自由之翼" },
  { key: "hots", label: "虫群之心" },
  { key: "custom", label: "自制战役" },
];

const groups: SaveBackup[] = [
  { name: "原版-1790000000", label: "原版", bytes: 1_842_000, files: 3 },
  { name: "自由之翼-1790000100", label: "自由之翼", bytes: 611_000, files: 2 },
];

const isolation: SaveIsolation = {
  enabled: true,
  active: "原版-1790000000",
  assignments: { "自由之翼-1790000100": "wol" },
};

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(overrides?: { isolation?: SaveIsolation; groups?: SaveBackup[] }) {
  const onSwitch = vi.fn();
  const onSaveInto = vi.fn();
  const onGroupFiles = vi.fn(async () => []);
  const onRefresh = vi.fn();
  const onOpenSettings = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(SavesView, {
        targets,
        isolation: overrides?.isolation ?? isolation,
        groups: overrides?.groups ?? groups,
        banks: { files: [{ name: "ENS.SC2Bank", bytes: 611_000 }], bytes: 1_842_000, missing: false },
        busy: false,
        onSwitch,
        onMoveToKey: vi.fn(async () => {}),
        onSaveInto,
        onGroupFiles,
        onRefresh,
        onOpenSettings,
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
  it("列出每个组：名字、文件数与大小；没归属的组标「未指定」", async () => {
    await mount();

    expect(document.querySelectorAll(".saves__row")).toHaveLength(2);
    const text = document.body.textContent ?? "";
    expect(text).toContain("原版");
    expect(text).toContain("3 个文件");
    // 没有归属的组标「未指定」；归属由系统按组名自动维护，界面上不给改
    expect(document.querySelector(".saves__loose")?.textContent).toBe("未指定");
    // 当前在用的那个组有标记
    expect(document.querySelector(".saves__row--active")?.textContent).toContain("当前在用");
  });

  it("没有存档的组自动隐藏（当前在用的那个例外）", async () => {
    const empty: SaveBackup[] = [
      { name: "原版-1790000000", label: "原版", bytes: 1_842_000, files: 3 },
      { name: "空组-1790000200", label: "空组", bytes: 0, files: 0 },
    ];
    await mount({ groups: empty });
    const rows = [...document.querySelectorAll(".saves__row")];
    expect(rows).toHaveLength(1);
    expect(document.body.textContent).not.toContain("空组");
  });

  it("「切换为该分类」和「设为当前」是两个不同的动作", async () => {
    const { onSwitch, onSaveInto } = await mount();
    // 分区之后行顺序会变，按名字找那一行（自由之翼那个组）
    const row = [...document.querySelectorAll(".saves__row")].find((item) =>
      (item.textContent ?? "").includes("自由之翼"),
    ) as HTMLElement;
    const buttons = [...row.querySelectorAll("button")];
    const switchButton = buttons.find((button) => button.textContent === "切换为该分类");
    const saveButton = buttons.find((button) => button.textContent === "设为当前");
    expect(switchButton, "不在用的组要有「切换为该分类」").toBeTruthy();
    expect(saveButton, "每组都要有「设为当前」").toBeTruthy();

    await act(async () => {
      switchButton?.click();
    });
    expect(onSwitch).toHaveBeenCalledWith("自由之翼-1790000100");

    await act(async () => {
      saveButton?.click();
    });
    expect(onSaveInto).toHaveBeenCalledWith("自由之翼-1790000100");
  });

  it("没有分类时给空状态说明", async () => {
    await mount({ groups: [] });
    expect(document.querySelectorAll(".saves__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("还没有存档分类");
  });

  it("隔离开着时说明里不出现「去设置里打开」", async () => {
    const { onOpenSettings } = await mount();
    expect(document.body.textContent).not.toContain("去设置里打开");
    void onOpenSettings;
  });
});
