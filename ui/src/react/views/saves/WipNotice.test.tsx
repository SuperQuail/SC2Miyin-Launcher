import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("../../../api/bridge", () => ({
  api: {
    listTools: async () => [],
    networkSettings: async () => ({
      proxy_mode: "auto",
      proxy_url: null,
      use_mirrors: true,
      include_prerelease: true,
    }),
    detectedProxy: async () => null,
  },
}));

import { SettingsView } from "../settings/SettingsView";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mounted: Array<() => Promise<void>> = [];
afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

async function mount(isolationWarning: string | null = null) {
  const toggleSaveIsolation = vi.fn();
  const noop = () => {};
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      createElement(SettingsView, {
        notify: noop,
        installation: null,
        libraryRoot: "C:/lib",
        isDesktop: false,
        installedCampaigns: [],
        saves: null,
        saveBackups: [],
        saveIsolation: { enabled: false, active: null, assignments: {} },
        saveIsolationWarning: isolationWarning,
        slots: [],
        busy: false,
        launcherVersion: "0.1.0a14",
        update: { check: null, staged: null, progress: null, logs: [], checking: false, downloading: false },
        actions: {
          chooseGameDirectory: noop, reveal: noop, collectCampaign: noop,
          toggleSaveIsolation, saveCurrentSaves: noop, switchSaveProfile: noop,
          assignSaveProfile: noop, backupSaves: noop, restoreSaves: noop,
          refresh: noop, checkUpdate: noop, downloadUpdate: noop, applyUpdate: noop,
          openRoom: noop, openDevPage: noop,
        },
      }),
    );
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  return { toggleSaveIsolation };
}

function isolationCheckbox(): HTMLInputElement {
  const label = [...document.querySelectorAll(".isolation__switch")].find((item) =>
    (item.textContent ?? "").includes("按战役隔离存档"),
  );
  return label?.querySelector("input") as HTMLInputElement;
}

describe("设置页开启隔离：先确认", () => {
  it("勾开关只弹确认框，不直接开", async () => {
    const { toggleSaveIsolation } = await mount();
    await act(async () => {
      isolationCheckbox().click();
    });
    expect(document.querySelector(".sheet"), "要弹二次确认").not.toBeNull();
    expect(document.body.textContent).toContain("可能导致存档损坏");
    expect(toggleSaveIsolation).not.toHaveBeenCalled();
  });

  it("确认后才真的开", async () => {
    const { toggleSaveIsolation } = await mount();
    await act(async () => {
      isolationCheckbox().click();
    });
    const confirm = [...document.querySelectorAll(".sheet__actions button")].find((item) =>
      (item.textContent ?? "").includes("仍要开启"),
    );
    expect(confirm).toBeTruthy();
    await act(async () => {
      (confirm as HTMLButtonElement).click();
    });
    expect(toggleSaveIsolation).toHaveBeenCalledWith(true);
  });

  it("取消就什么都不做", async () => {
    const { toggleSaveIsolation } = await mount();
    await act(async () => {
      isolationCheckbox().click();
    });
    const cancel = [...document.querySelectorAll(".sheet__actions button")].find((item) =>
      (item.textContent ?? "").includes("取消"),
    );
    await act(async () => {
      (cancel as HTMLButtonElement).click();
    });
    expect(toggleSaveIsolation).not.toHaveBeenCalled();
    expect(document.querySelector(".sheet")).toBeNull();
  });

  it("设置页也有同样的 WIP 警示", async () => {
    await mount();
    const note = document.querySelector('[role="note"]');
    expect(note).not.toBeNull();
    expect(note?.textContent).toContain("存档管理（WIP）");
  });

  it("状态文件读不出来时把提示摆出来（不静默当关闭）", async () => {
    await mount("存档隔离的状态文件读不出来");
    const alert = document.querySelector('[role="alert"]');
    expect(alert, "要给一条警示，别让用户以为分组凭空没了").not.toBeNull();
    expect(alert?.textContent).toContain("状态文件读不出来");
    // WIP 警示照旧在（只增不减）
    expect(document.querySelector('[role="note"]')?.textContent).toContain("存档管理（WIP）");
  });
});
