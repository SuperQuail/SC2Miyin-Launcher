import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { GroupDetailDialog } from "./GroupDetailDialog";

/**
 * 组详情：能看见组里有什么、能把某一份挪到别的组。
 *
 * 这条路径坏了，用户就**没法修分错的存档** —— 所以钉三件事：
 * 文件列出来、选目标组会带对参数、空组给一句话。
 */

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(overrides: { files?: Array<{ name: string; bytes: number }> | null } = {}) {
  const onMove = vi.fn();
  const onReload = vi.fn();
  const onClose = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(GroupDetailDialog, {
        groupLabel: "自由之翼-1790",
        files: overrides.files === undefined
          ? [
              { name: "Terran01.SC2Bank", bytes: 131_072 },
              { name: "Swarm01.SC2Bank", bytes: 65_536 },
            ]
          : overrides.files,
        choices: [
          { name: "原版-1790", label: "原版（不属于任何战役）" },
          { name: "虫群之心-1790", label: "虫群之心 → 虫群之心" },
        ],
        busy: false,
        onMove,
        onReload,
        onClose,
      }),
    );
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  return { onMove, onReload, onClose };
}

describe("存档组详情", () => {
  it("列出组里的每一份存档（名字 + 大小）", async () => {
    await mount();
    const rows = [...document.querySelectorAll(".savedetail__row")];
    expect(rows).toHaveLength(2);
    const text = document.body.textContent ?? "";
    expect(text).toContain("Terran01.SC2Bank");
    expect(text).toContain("128 KB");
    expect(text).toContain("Swarm01.SC2Bank");
  });

  it("选一个目标组就把这份挪过去（带对参数）", async () => {
    const { onMove } = await mount();
    const selects = [...document.querySelectorAll<HTMLSelectElement>(".savedetail__move")];
    expect(selects).toHaveLength(2);
    await act(async () => {
      selects[1].value = "虫群之心-1790";
      selects[1].dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(onMove).toHaveBeenCalledWith("Swarm01.SC2Bank", "虫群之心-1790");
  });

  it("空组说一句话，不画空列表", async () => {
    await mount({ files: [] });
    expect(document.querySelectorAll(".savedetail__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("这个组里还没有存档");
  });
});
