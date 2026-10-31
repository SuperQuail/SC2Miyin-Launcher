import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { ActivationConflict } from "../../../api/bridge";
import { ActivationConflictDialog } from "./ActivationConflictDialog";

/**
 * 冲突对话框的渲染与接线。
 *
 * 这条路径写错的话，用户会在「已经覆盖了别人」之后才知道 —— 所以这里钉三件事：
 * 标题/条数对、超过 5 条折成「另有 N 个」、两个按钮各自接到对应回调上。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function conflict(index: number): ActivationConflict {
  return {
    target: "Mods/Shared" + index + ".SC2Mod",
    slot: "wol",
    slot_name: "自由之翼",
    variant: "自由之翼：重生 v1.4.2",
    variant_name: "自由之翼：重生",
  };
}

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(conflicts: ActivationConflict[], busy = false) {
  const onUseNew = vi.fn();
  const onKeepOther = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(ActivationConflictDialog, {
        conflicts,
        newCampaign: "虫群之心 · 进化重制",
        otherCampaign: "自由之翼 · 重生",
        busy,
        onUseNew,
        onKeepOther,
      }),
    );
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  return { onUseNew, onKeepOther };
}

describe("启用冲突对话框", () => {
  it("标题说清后果，并列出冲突条数与前 5 条", async () => {
    await mount(Array.from({ length: 7 }, (_, index) => conflict(index)));

    // 标题直接点名两个战役 —— 不写「这个」「另一个」这种没有指代对象的说法
    expect(document.querySelector(".sheet__title")?.textContent).toBe(
      "虫群之心 · 进化重制 与 自由之翼 · 重生 需要写入重名的文件",
    );
    expect(document.querySelector(".conflict__head")?.textContent).toContain("7 个");
    expect(document.querySelectorAll(".conflict__item")).toHaveLength(5);
    // 第 6 条起折成一句，别把弹窗撑成一面墙
    expect(document.querySelector(".conflict__more")?.textContent).toContain("另有 2 个");
    // 写明当前由谁使用
    const text = document.body.textContent ?? "";
    expect(text).toContain("Mods/Shared0.SC2Mod");
    expect(text).toContain("当前由 自由之翼 · 自由之翼：重生 使用");
  });

  it("两条不折行、也没有「等 N 个」", async () => {
    await mount([conflict(0), conflict(1)]);
    expect(document.querySelectorAll(".conflict__item")).toHaveLength(2);
    expect(document.querySelector(".conflict__more")).toBeNull();
  });

  it("两个按钮各自写着战役名、各自接到回调", async () => {
    const { onUseNew, onKeepOther } = await mount([conflict(0)]);
    const buttons = [...document.querySelectorAll<HTMLButtonElement>(".sheet__actions button")];
    const labels = buttons.map((button) => button.textContent);

    expect(labels).toEqual(["启用 自由之翼 · 重生", "启用 虫群之心 · 进化重制"]);

    await act(async () => {
      buttons[1].click();
    });
    expect(onUseNew).toHaveBeenCalledTimes(1);
    expect(onKeepOther).not.toHaveBeenCalled();

    await act(async () => {
      buttons[0].click();
    });
    expect(onKeepOther).toHaveBeenCalledTimes(1);
  });

  it("在跑的时候主按钮转圈且点不动", async () => {
    const { onUseNew } = await mount([conflict(0)], true);
    const primary = document.querySelector<HTMLButtonElement>(".sheet__actions .btn-primary");
    expect(primary?.disabled).toBe(true);
    await act(async () => {
      primary?.click();
    });
    expect(onUseNew).not.toHaveBeenCalled();
  });
});
