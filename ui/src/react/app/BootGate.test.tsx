import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

/**
 * 外壳要等后端把数据准备好（启动时的迁移 / 合并在后台线程上跑，见 src-tauri/src/lib.rs）。
 * 在它准备好之前挂上 AppShell，那些命令会让后端 panic —— 所以这层门必须挡住。
 */

let boot: () => Promise<void> = () => Promise.resolve();
vi.mock("./boot", () => ({ waitForBoot: () => boot() }));

import { BootGate } from "./BootGate";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let unmount: (() => Promise<void>) | null = null;

afterEach(async () => {
  await unmount?.();
  unmount = null;
  document.body.innerHTML = "";
});

async function mount(): Promise<HTMLElement> {
  const element = document.createElement("div");
  document.body.appendChild(element);
  const root = createRoot(element);
  await act(async () => {
    root.render(
      createElement(BootGate, null, createElement("span", { className: "shell" }, "外壳")),
    );
  });
  unmount = async () => {
    await act(async () => root.unmount());
  };
  return element;
}

describe("BootGate", () => {
  it("数据没准备好：先给「正在准备数据」，不挂外壳", async () => {
    boot = () => new Promise<void>(() => undefined);
    const element = await mount();

    expect(element.textContent).toContain("正在准备数据");
    expect(element.querySelector(".shell")).toBeNull();
  });

  it("准备好了才挂外壳", async () => {
    let release: () => void = () => undefined;
    boot = () =>
      new Promise<void>((resolve) => {
        release = resolve;
      });
    const element = await mount();
    await act(async () => release());

    expect(element.querySelector(".shell")).not.toBeNull();
    expect(element.textContent).not.toContain("正在准备数据");
  });

  it("后端没起来：给出一页能看的失败说明", async () => {
    boot = () => Promise.reject(new Error("后端没响应"));
    const element = await mount();

    expect(element.textContent).toContain("启动失败");
    expect(element.textContent).toContain("后端没响应");
  });
});
