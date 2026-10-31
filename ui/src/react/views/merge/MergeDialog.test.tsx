import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { MergeStatus } from "../../../api/types";
import { MergeDialog } from "./MergeDialog";

/**
 * 合并对话框的三件事：**冲突才问**、**默认保留游戏目录那份**、**选哪条就报哪条**。
 *
 * 第三条是重点：Choice 的 onChange 收了但没接上去的话，类型照样过 ——
 * 用户点「两份都留」结果合并按默认跑，这种 bug 只有真的点一下才看得见。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const status: MergeStatus = {
  local: "D:\\Code\\Rust\\HSCL\\target\\debug\\data",
  target: "D:\\Game\\BLZ\\miyin_launcher_data",
  additions: [
    { slot: "wol", variant: "wol-reborn-150", variant_name: "自由之翼：重生 v1.5.0", bytes: 1_402_000_000 },
    { slot: "lotv", variant: "lotv-scroll", variant_name: "虚空之遗：残卷", bytes: 412_000_000 },
  ],
  conflicts: [
    {
      slot: "custom",
      variant: "scmr-8",
      variant_name: "Starcraft Mass Recall",
      local_bytes: 1_414_700_000,
      target_bytes: 1_406_900_000,
    },
  ],
  keep_target: ["active.json", "installed.json", "dev", "backup"],
};

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(overrides?: Partial<Parameters<typeof MergeDialog>[0]>) {
  const props = {
    status,
    choices: { "custom/scmr-8": "target" } as Record<string, "target" | "local" | "both">,
    applying: false,
    onChoose: vi.fn(),
    onApply: vi.fn(),
    onLater: vi.fn(),
    ...overrides,
  };
  const element = document.createElement("div");
  document.body.appendChild(element);
  const root = createRoot(element);
  await act(async () => {
    root.render(createElement(MergeDialog, props));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    element.remove();
  });
  return { element, props };
}

describe("数据合并对话框", () => {
  it("默认选中「保持游戏目录那份」，三条出路都在", async () => {
    const { element } = await mount();
    const radios = [...element.querySelectorAll<HTMLInputElement>("input[type=radio]")];
    expect(radios.map((radio) => radio.value)).toEqual(["target", "local", "both"]);
    expect(radios.filter((radio) => radio.checked).map((radio) => radio.value)).toEqual(["target"]);
  });

  it("点「两份都留」会带着 <slot>/<variant> 这个 key 报上去", async () => {
    const { element, props } = await mount();
    const both = [...element.querySelectorAll<HTMLInputElement>("input[type=radio]")].find(
      (radio) => radio.value === "both",
    );
    expect(both, "应当有「两份都留」这一项").not.toBeUndefined();
    await act(async () => {
      both?.click();
    });
    expect(props.onChoose).toHaveBeenCalledWith("custom/scmr-8", "both");
  });

  it("把不冲突的那部分列成只读摘要，并说明哪几样一律用游戏目录那份", async () => {
    const { element } = await mount();
    const text = element.textContent ?? "";
    expect(text).toContain("将合并 · 2 项");
    expect(text).toContain("自由之翼：重生 v1.5.0");
    expect(text).toContain("1.3 GB");
    expect(text).toContain("active.json · installed.json · dev · backup");
  });

  it("「开始合并」在跑的时候转圈且点不动", async () => {
    const { element, props } = await mount({ applying: true });
    const primary = element.querySelector<HTMLButtonElement>(".sheet__actions .btn-primary");
    expect(primary?.disabled).toBe(true);
    expect(primary?.getAttribute("aria-busy")).toBe("true");
    await act(async () => {
      primary?.click();
    });
    expect(props.onApply).not.toHaveBeenCalled();
  });
});
