import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../../../../api/bridge";
import type { BoundPatch } from "../../../../api/types";
import { PatchPanel } from "./PatchPanel";

/**
 * 补丁删除入口：bug 是「导入的补丁没有删除按钮，删不掉」。
 * 这里钉住：每个补丁行都有删除按钮，确认后走 deletePatch。
 */
vi.mock("../../../../api/bridge", () => ({
  api: {
    listBindings: vi.fn(),
    listAvailablePatches: vi.fn(),
    deletePatch: vi.fn(),
    updatePatch: vi.fn(),
    configurePatch: vi.fn(),
    unbindPatch: vi.fn(),
    bindPatch: vi.fn(),
    autoBindPatches: vi.fn(),
    importPatch: vi.fn(),
    pickPackage: vi.fn(),
    pickExportPath: vi.fn(),
    exportPatch: vi.fn(),
  },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const boundPatch: BoundPatch = {
  id: "p1",
  name: "幼儿园补丁",
  author: null,
  version: "3.0",
  description: null,
  registration_id: null,
  priority: 100,
  requires: [],
  payloads: [],
  imported_at: 0,
  size_bytes: 1,
  mod_count: 1,
  enabled: true,
  matched: false,
};

const mounted: Array<() => Promise<void>> = [];

beforeEach(() => {
  vi.mocked(api.listBindings).mockReset();
  vi.mocked(api.listAvailablePatches).mockReset();
  vi.mocked(api.deletePatch).mockReset();
  vi.mocked(api.listBindings).mockResolvedValue([boundPatch]);
  vi.mocked(api.listAvailablePatches).mockResolvedValue([]);
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(): Promise<{ notify: ReturnType<typeof vi.fn> }> {
  const notify = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(PatchPanel, {
        slot: "wol",
        notify,
        exporting: false,
        onExporting: () => undefined,
      }),
    );
  });
  // 等 listBindings / listAvailablePatches 两个 promise 落定
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  return { notify };
}

describe("补丁面板 · 删除", () => {
  it("每个补丁都有删除按钮，确认后调用 deletePatch", async () => {
    const { notify } = await mount();

    const remove = [...document.querySelectorAll<HTMLButtonElement>("button")].find(
      (button) => button.textContent === "删除",
    );
    expect(remove, "补丁行上没有删除按钮").toBeTruthy();

    await act(async () => {
      remove?.click();
    });

    const confirm = document.querySelector<HTMLButtonElement>(".sheet__actions .btn-danger");
    expect(confirm?.textContent).toBe("删除");

    await act(async () => {
      confirm?.click();
      await Promise.resolve();
    });

    expect(vi.mocked(api.deletePatch)).toHaveBeenCalledWith("p1");
    expect(notify).toHaveBeenCalledWith("success", expect.stringContaining("幼儿园补丁"));
  });
});
