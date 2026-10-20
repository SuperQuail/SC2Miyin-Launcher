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
 * 以及「切换到这个 / 设为当前」别再被写成同一个动作（一个是换内容，一个是把现在这份存回去）。
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
  const onAssign = vi.fn();
  const onSaveInto = vi.fn();
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
        onAssign,
        onSaveInto,
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
  return { onSwitch, onAssign, onSaveInto, onOpenSettings };
}

describe("存档页", () => {
  it("列出每个组：名字、文件数与大小、归属下拉的当前值", async () => {
    await mount();

    expect(document.querySelectorAll(".saves__row")).toHaveLength(2);
    const text = document.body.textContent ?? "";
    expect(text).toContain("原版");
    expect(text).toContain("3 个文件");
    // 归属下拉：原版那个组不属于任何战役，自由之翼那个挂在 wol
    const selects = [...document.querySelectorAll<HTMLSelectElement>(".saves__select")];
    expect(selects.map((select) => select.value)).toEqual(["", "wol"]);
    // 当前在用的那个组有标记
    expect(document.querySelector(".saves__row--active")?.textContent).toContain("当前在用");
  });

  it("改归属下拉真的会上报（手动定位到别的战役）", async () => {
    const { onAssign } = await mount();
    const select = document.querySelectorAll<HTMLSelectElement>(".saves__select")[1];

    await act(async () => {
      select.value = "hots";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(onAssign).toHaveBeenCalledWith("自由之翼-1790000100", "hots");
  });

  it("「切换到这个」和「设为当前」是两个不同的动作", async () => {
    const { onSwitch, onSaveInto, onAssign } = await mount();
    const row = document.querySelectorAll(".saves__row")[1];
    const buttons = [...row.querySelectorAll("button")];
    const switchButton = buttons.find((button) => button.textContent === "切换到这个");
    const saveButton = buttons.find((button) => button.textContent === "设为当前");
    expect(switchButton, "不在用的组要有「切换到这个」").toBeTruthy();
    expect(saveButton, "每组都要有「设为当前」").toBeTruthy();

    await act(async () => {
      switchButton?.click();
    });
    expect(onSwitch).toHaveBeenCalledWith("自由之翼-1790000100");

    await act(async () => {
      saveButton?.click();
    });
    expect(onSaveInto).toHaveBeenCalledWith("自由之翼-1790000100");
    expect(onAssign).not.toHaveBeenCalled();
  });

  it("没有组时给空状态说明", async () => {
    await mount({ groups: [] });
    expect(document.querySelectorAll(".saves__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("还没有存档组");
  });

  it("隔离开着时说明里不出现「去设置里打开」", async () => {
    const { onOpenSettings } = await mount();
    expect(document.body.textContent).not.toContain("去设置里打开");
    void onOpenSettings;
  });
});
