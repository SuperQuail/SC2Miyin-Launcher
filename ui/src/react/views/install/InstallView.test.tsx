import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { ImportPreview, SlotView, Variant } from "../../../api/types";
import { InstallView } from "./InstallView";

/**
 * 批量安装的**接线**测试。
 *
 * 重点只有一个：一次拖进来好几个包时，**每一行独立成败** ——
 * 一个包失败必须记在它自己那一行，后面的照装。这种"中间停下来了"的 bug
 * 编译和类型都过得去（循环里少个 try/catch 而已），只能真的挂起来点一下。
 */

const prepareImport = vi.fn<(path: string, entry?: string) => Promise<ImportPreview>>();
const importPackageWith = vi.fn<(path: string, slot: string | null, mode: string) => Promise<Variant>>();
const pickPackage = vi.fn<() => Promise<string | null>>();

vi.mock("../../../api/bridge", () => ({
  api: {
    prepareImport: (path: string, entry?: string) => prepareImport(path, entry),
    importPackageWith: (path: string, slot: string | null, mode: string) => importPackageWith(path, slot, mode),
    pickPackage: () => pickPackage(),
  },
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const slots: SlotView[] = ["hots", "custom"].map((slug) => ({
  slug,
  display_name: slug === "hots" ? "虫群之心" : "自制战役",
  sub_directory: null,
  variants: [],
  active: null,
  active_name: null,
  notice: null,
}));

/** 一个包的预检结果；slot 传 null 就是"认不出归属"。 */
function preview(path: string, slot: string | null, name: string, withConflict = false): ImportPreview {
  return {
    path,
    source: slot ? "inferred" : "manual",
    slot,
    conflict: withConflict
      ? {
          existing_id: "old",
          existing_name: name,
          existing_version: "1.0",
          incoming_version: "2.0",
          same_id: true,
          relation: "newer",
        }
      : null,
    inspection: {
      path,
      installable: true,
      format: "ccm",
      name,
      author: "某作者",
      version: "2.0",
      description: null,
      campaign_type: "hots",
      kind: "campaign",
      id: null,
      requires: [],
      priority: null,
      payloads: [],
      identification: slot
        ? { campaign_type: "hots", evidence: "map_dependency", detail: "Swarm Story" }
        : null,
      cover: null,
      tags: [],
      suggested_slot: slot,
      content_root: "",
      suggested_dir_name: name,
      map_count: 8,
      map_count_note: null,
      mod_count: 1,
      entry_count: 9,
      unpacked_bytes: 1024,
      issues: [],
    },
  } as unknown as ImportPreview;
}

const variant = { id: "v", name: "装好的" } as unknown as Variant;

const mounted: Array<() => Promise<void>> = [];
const notify = vi.fn<(kind: "info" | "success" | "warning" | "error", message: string) => void>();
const onInstalled = vi.fn<(slot: string) => void>();

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

beforeEach(() => {
  prepareImport.mockReset();
  importPackageWith.mockReset();
  pickPackage.mockReset();
  notify.mockClear();
  onInstalled.mockClear();
});

/** 让 hook 里的异步（预检、安装）跑完。 */
async function flush(times = 4): Promise<void> {
  for (let index = 0; index < times; index += 1) {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }
}

async function mount(paths: string[]): Promise<void> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(InstallView, {
        entry: "campaign" as const,
        slots,
        droppedPackages: paths,
        consumeDropped: () => undefined,
        notify,
        onBack: () => undefined,
        onInstalled,
      }),
    );
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  await flush();
}

/** 点底部那个主按钮（单个包时是「安装」，多个包时是「全部安装」）。 */
async function clickPrimary(): Promise<void> {
  const button = document.querySelector<HTMLButtonElement>(".install__foot .btn-primary");
  if (!button) throw new Error("没有主按钮");
  await act(async () => {
    button.click();
  });
  await flush();
}

const rows = () => [...document.querySelectorAll(".install__row")];

describe("批量安装", () => {
  it("两个包：一个装上一个失败，成功的那个真的被装了，失败的记在它自己那行", async () => {
    prepareImport.mockImplementation(async (path) =>
      path === "good.zip" ? preview(path, "hots", "好包") : preview(path, "custom", "坏包"),
    );
    importPackageWith.mockImplementation(async (path) => {
      if (path === "bad.zip") throw new Error("压缩包坏了：CRC 校验失败");
      return variant;
    });

    await mount(["good.zip", "bad.zip"]);
    expect(rows()).toHaveLength(2);

    await clickPrimary();

    // 两个都试过，而且顺序就是清单顺序 —— 一个失败没有把后面那个带停
    expect(importPackageWith.mock.calls.map((call) => call[0])).toEqual(["good.zip", "bad.zip"]);
    expect(importPackageWith).toHaveBeenCalledWith("good.zip", "hots", "overwrite");
    // 有失败就不回来源页（留在这一页让用户看见哪几行红了）
    expect(onInstalled).not.toHaveBeenCalled();
    // toast 说清是哪个包、为什么
    const warning = notify.mock.calls.find((call) => call[0] === "warning");
    expect(warning).toBeTruthy();
    expect(String(warning?.[1])).toContain("坏包");
    expect(String(warning?.[1])).toContain("CRC");
    // 行上也留着结果
    const text = document.body.textContent ?? "";
    expect(text).toContain("✓ 已安装");
    expect(text).toContain("✗ 失败：压缩包坏了：CRC 校验失败");
  });

  it("全部成功：整批一个 toast，装完回来源页", async () => {
    prepareImport.mockImplementation(async (path) => preview(path, "hots", "包" + path));
    importPackageWith.mockResolvedValue(variant);

    await mount(["a.zip", "b.zip", "c.zip"]);
    await clickPrimary();

    expect(importPackageWith).toHaveBeenCalledTimes(3);
    expect(notify).toHaveBeenCalledWith("success", "已安装 3 个包");
    expect(onInstalled).toHaveBeenCalledTimes(1);
    expect(onInstalled).toHaveBeenCalledWith("hots");
  });

  it("认不出归属的那行：没选战役就只算它失败，别的照装", async () => {
    prepareImport.mockImplementation(async (path) =>
      path === "unknown.zip" ? preview(path, null, "认不出的包") : preview(path, "hots", "认得出的包"),
    );
    importPackageWith.mockResolvedValue(variant);

    await mount(["unknown.zip", "known.zip"]);
    // 那一行的下拉是空的，别的行预选好了
    const selects = [...document.querySelectorAll<HTMLSelectElement>(".install__row-select")];
    expect(selects.map((select) => select.value)).toEqual(["", "hots"]);
    expect(document.body.textContent).toContain("认不出来");

    await clickPrimary();

    // 认得出的那个照装上了
    expect(importPackageWith).toHaveBeenCalledTimes(1);
    expect(importPackageWith).toHaveBeenCalledWith("known.zip", "hots", "overwrite");
    expect(document.body.textContent).toContain("还没选装到哪部战役");
  });

  it("预检就失败的包：那行报「读不了这个包」，别的照装", async () => {
    prepareImport.mockImplementation(async (path) => {
      if (path === "broken.zip") throw new Error("不是压缩包");
      return preview(path, "hots", "正常的包");
    });
    importPackageWith.mockResolvedValue(variant);

    await mount(["broken.zip", "ok.zip"]);
    expect(document.body.textContent).toContain("读不了这个包");
    expect(document.body.textContent).toContain("不是压缩包");

    await clickPrimary();

    // 预检失败的不会被拿去装，正常的那个照装
    expect(importPackageWith.mock.calls.map((call) => call[0])).toEqual(["ok.zip"]);
    const error = notify.mock.calls.find((call) => call[0] === "warning");
    expect(String(error?.[1])).toContain("broken.zip");
  });

  it("冲突整批一个选择：清单顶上给一次，有冲突的行有标记", async () => {
    prepareImport.mockImplementation(async (path) => preview(path, "hots", "包" + path, true));
    importPackageWith.mockResolvedValue(variant);

    await mount(["a.zip"]);
    const choices = [...document.querySelectorAll("input[type=radio]")].map((input) => (input as HTMLInputElement).value);
    expect(choices).toEqual(["overwrite", "rename"]);
    expect(document.body.textContent).toContain("冲突");

    // 切成"重命名后导入"，整批按这个跑
    const rename = [...document.querySelectorAll<HTMLInputElement>("input[type=radio]")].find(
      (input) => input.value === "rename",
    );
    await act(async () => {
      rename?.click();
    });
    await clickPrimary();
    expect(importPackageWith).toHaveBeenCalledWith("a.zip", "hots", "rename");
  });

  it("同一个文件进来两次是两行，各行独立（不能拿路径当身份）", async () => {
    prepareImport.mockImplementation(async (path) => preview(path, "hots", "同一个包"));
    importPackageWith.mockResolvedValue(variant);

    await mount(["dup.zip", "dup.zip"]);
    expect(rows()).toHaveLength(2);

    await clickPrimary();
    expect(importPackageWith).toHaveBeenCalledTimes(2);
    expect(importPackageWith.mock.calls.map((call) => call[0])).toEqual(["dup.zip", "dup.zip"]);
  });

  it("单个包：按钮还是「安装」，装完照样回来源页", async () => {
    prepareImport.mockImplementation(async (path) => preview(path, "hots", "一个包"));
    importPackageWith.mockResolvedValue(variant);

    await mount(["one.zip"]);
    expect(document.querySelector(".install__foot .btn-primary")?.textContent).toBe("安装");

    await clickPrimary();
    expect(notify).toHaveBeenCalledWith("success", "已安装：一个包");
    expect(onInstalled).toHaveBeenCalledWith("hots");
  });
});
