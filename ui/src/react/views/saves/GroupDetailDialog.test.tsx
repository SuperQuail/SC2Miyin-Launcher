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
          { key: "lotv", label: "虚空之遗（原版）" },
          { key: "lotv/肉鸽", label: "虚空之遗 · 肉鸽" },
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

  it("选一个归属就把这份归过去（自家下拉，带对参数）", async () => {
    const { onMove } = await mount();
    // 自家下拉：先点开第二行那个触发器，再点目标选项
    const triggers = [...document.querySelectorAll<HTMLButtonElement>(".savedetail__move button")];
    expect(triggers.length).toBeGreaterThanOrEqual(2);
    await act(async () => {
      triggers[1].click();
    });
    const options = [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')];
    const target = options.find((item) => (item.textContent ?? "").includes("肉鸽"));
    expect(target, "要能选到「虚空之遗 · 肉鸽」").toBeTruthy();
    await act(async () => {
      target?.click();
    });
    expect(onMove).toHaveBeenCalledWith("Swarm01.SC2Bank", "lotv/肉鸽");
  });

  it("空组说一句话，不画空列表", async () => {
    await mount({ files: [] });
    expect(document.querySelectorAll(".savedetail__row")).toHaveLength(0);
    expect(document.body.textContent).toContain("此组暂无存档");
  });
});
