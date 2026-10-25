import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { SaveInventory } from "../../../api/types";
import { SavesView } from "./SavesView";

/**
 * 存档页：列出**真机存档**（Accounts/<账号>/<地域>/Saves/** 下的 .SC2Save）。
 *
 * 这一页以前读 <文档>/StarCraft II/Banks（真机不存在）所以永远空；
 * 现在钉三件事：能列出、能按账号/位置筛、什么都没有时给出像样的说明。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const inventory: SaveInventory = {
  accounts: ["598100452", "1466257849"],
  saves: [
    { account: "598100452", region: "5-S2-1-10106572", folder: "Saves/Unsaved/Campaign", name: "黑暗耳语", path: "/a/黑暗耳语.SC2Save", bytes: 19_000, modified: 1_761_300_000 },
    { account: "598100452", region: "5-S2-1-10106572", folder: "Saves/Campaign", name: "LotV - 艾尔", path: "/a/LotV - 艾尔.SC2Save", bytes: 41_000, modified: 1_761_200_000 },
    { account: "1466257849", region: "3-S2-1-8578610", folder: "Saves", name: "LibertyCampaignSave", path: "/b/Liberty.SC2Save", bytes: 7_000, modified: 1_700_000_000 },
  ],
  banks: [
    { account: "598100452", region: "5-S2-1-10106572", map: "5-S2-1-2668038", name: "COCampaign", path: "/b/COCampaign.SC2Bank", bytes: 1_000, modified: 1_761_000_000 },
  ],
};

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(data: SaveInventory | null = inventory) {
  const onScan = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(createElement(SavesView, { inventory: data, busy: false, onScan }));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  return { onScan };
}

describe("存档页", () => {
  it("列出每份真存档（名字 + 时间 + 大小 + 账号/地域 + 位置）", async () => {
    await mount();
    const rows = [...document.querySelectorAll(".saves__row")];
    expect(rows).toHaveLength(3);
    const text = document.body.textContent ?? "";
    expect(text).toContain("黑暗耳语");
    expect(text).toContain("LotV - 艾尔");
    expect(text).toContain("Saves/Unsaved/Campaign");
    expect(text).toContain("598100452");
  });

  it("没拿到清单时先扫一次", async () => {
    const { onScan } = await mount(null);
    expect(onScan).toHaveBeenCalledTimes(1);
    expect(document.body.textContent).toContain("正在扫描");
  });

  it("按账号筛选", async () => {
    await mount();
    const triggers = [...document.querySelectorAll<HTMLButtonElement>("button")].filter((button) =>
      (button.textContent ?? "").includes("全部账号"),
    );
    expect(triggers.length).toBe(1);
    await act(async () => {
      triggers[0].click();
    });
    const option = [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((item) =>
      (item.textContent ?? "").includes("1466257849"),
    );
    expect(option, "下拉里应有另一个账号").toBeTruthy();
    await act(async () => {
      option?.click();
    });
    const rows = [...document.querySelectorAll(".saves__row")];
    expect(rows).toHaveLength(1);
    expect(rows[0].textContent).toContain("LibertyCampaignSave");
  });

  it("一份都没有时给像样的说明", async () => {
    await mount({ accounts: [], saves: [], banks: [] });
    expect(document.querySelectorAll(".saves__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("没扫到存档");
  });
});
