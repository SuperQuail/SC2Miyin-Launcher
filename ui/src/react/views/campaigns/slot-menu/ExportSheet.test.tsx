import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { api } from "../../../../api/bridge";
import type { ExportPreview, Variant } from "../../../../api/types";
import { ExportSheet } from "./ExportSheet";

/**
 * 导出向导的接线：格式切换 → 预检 → 丢失清单；以及「换格式才能导出」的拦截。
 *
 * 这几条弄错会让用户**以为完整导出**，实际丢内容 —— 所以把提示与按钮状态钉住。
 */

vi.mock("../../../../api/bridge", () => ({
  api: {
    previewExportVariant: vi.fn(),
    exportVariant: vi.fn(),
    pickExportPath: vi.fn(),
  },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const variant: Variant = {
  id: "v1",
  name: "自由之翼：重生",
  author: "SomeCreator",
  version: "1.4.2",
  description: null,
  format: "ccm",
  imported_at: 0,
  source: null,
  map_count: 32,
  mod_count: 2,
  size_bytes: 1024,
  target_sub: null,
  cover: null,
  tags: [],
  registration_id: null,
  declared_mods: [],
  doc: null,
};

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

beforeEach(() => {
  vi.mocked(api.previewExportVariant).mockReset();
  vi.mocked(api.exportVariant).mockReset();
  vi.mocked(api.pickExportPath).mockReset();
});

async function mount() {
  const notify = vi.fn();
  const onClose = vi.fn();
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(ExportSheet, {
        slot: "wol",
        variant,
        notify,
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
  return { notify, onClose };
}

const preview = (supported: boolean): ExportPreview => ({
  maps: 32,
  mods: 2,
  files: 35,
  supported,
  losses: [
    { kind: "cover", label: "包内封面图", detail: "星际枢纽格式没有封面字段。" },
    { kind: "patch", label: "已启用的补丁", detail: "补丁不会合并进导出包。" },
  ],
});

describe("导出向导", () => {
  it("默认是弥音包格式，不显示丢失清单", async () => {
    await mount();
    expect(document.querySelector(".export-losses")).toBeNull();
    expect(document.querySelector(".export-format--on")?.textContent).toContain("弥音包格式");
    expect(vi.mocked(api.previewExportVariant)).not.toHaveBeenCalled();
  });

  it("切到星际枢纽格式后先预检，把会丢的内容逐条列出来", async () => {
    vi.mocked(api.previewExportVariant).mockResolvedValue(preview(true));
    await mount();

    const formats = [...document.querySelectorAll<HTMLButtonElement>(".export-format")];
    await act(async () => {
      formats[1].click();
    });
    await act(async () => {
      await Promise.resolve();
    });

    expect(vi.mocked(api.previewExportVariant)).toHaveBeenCalledWith("wol", "v1", true, "scnexus");
    const text = document.body.textContent ?? "";
    expect(text).toContain("以下内容不会进入星际枢纽包");
    expect(text).toContain("包内封面图");
    expect(text).toContain("已启用的补丁");
    expect(document.querySelector<HTMLButtonElement>(".sheet__actions .btn-primary")?.disabled).toBe(false);
  });

  it("预检说不支持时，导出按钮被拦下", async () => {
    vi.mocked(api.previewExportVariant).mockResolvedValue(preview(false));
    await mount();

    await act(async () => {
      document.querySelectorAll<HTMLButtonElement>(".export-format")[1].click();
    });
    await act(async () => {
      await Promise.resolve();
    });

    expect(document.querySelector<HTMLButtonElement>(".sheet__actions .btn-primary")?.disabled).toBe(true);
    expect(document.body.textContent).toContain("不能导出为星际枢纽兼容包");
  });

  it("选完保存位置才写文件：exportVariant 收到格式参数，成功关窗", async () => {
    vi.mocked(api.previewExportVariant).mockResolvedValue(preview(true));
    vi.mocked(api.pickExportPath).mockResolvedValue("D:\\exports\\reborn.zip");
    vi.mocked(api.exportVariant).mockResolvedValue({
      path: "D:\\exports\\reborn.zip",
      files: 35,
      maps: 32,
      mods: 2,
      expanded: [],
      patches: [],
      format: "scnexus",
      losses: preview(true).losses,
    });
    const { notify, onClose } = await mount();

    await act(async () => {
      document.querySelectorAll<HTMLButtonElement>(".export-format")[1].click();
    });
    await act(async () => {
      await Promise.resolve();
    });
    await act(async () => {
      document.querySelector<HTMLButtonElement>(".sheet__actions .btn-primary")?.click();
    });
    await act(async () => {
      await Promise.resolve();
    });

    expect(vi.mocked(api.exportVariant)).toHaveBeenCalledWith(
      "wol",
      "v1",
      "D:\\exports\\reborn.zip",
      true,
      "scnexus",
    );
    expect(notify).toHaveBeenCalledWith(
      "success",
      expect.stringContaining("有 2 项内容没有进入星际枢纽包"),
    );
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
