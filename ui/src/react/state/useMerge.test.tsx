import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { MergeChoice, MergeStatus } from "../../api/types";

/**
 * 起手的两件事：**启动时会去查**，以及**冲突默认保留游戏目录那份**。
 *
 * 这两条都在「点了没反应 / 默认选错」的范畴里 —— 类型过得去，只有真挂起来才看得见。
 */

const mergeStatus = vi.fn<() => Promise<MergeStatus | null>>();
const mergeApply = vi.fn<(choices: Record<string, MergeChoice>) => Promise<void>>();

vi.mock("../../api/bridge", () => ({
  api: {
    mergeStatus: () => mergeStatus(),
    mergeApply: (choices: Record<string, MergeChoice>) => mergeApply(choices),
  },
}));

import { useMerge, type MergeApi } from "./useMerge";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const status: MergeStatus = {
  local: "D:\\a\\data",
  target: "D:\\Game\\miyin_launcher_data",
  additions: [{ slot: "wol", variant: "w1", variant_name: "重生", bytes: 1 }],
  conflicts: [
    { slot: "custom", variant: "scmr-8", variant_name: "SCMR", local_bytes: 10, target_bytes: 20 },
    { slot: "hots", variant: "evo", variant_name: "进化", local_bytes: 30, target_bytes: 40 },
  ],
  keep_target: ["active.json"],
};

const mounted: Array<() => Promise<void>> = [];
let latest: MergeApi | null = null;

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
  latest = null;
});

/** 把 hook 挂起来，返回它当下的那套 API。 */
async function mountHook(): Promise<MergeApi> {
  function Probe() {
    latest = useMerge({ notify: () => undefined });
    return null;
  }
  const element = document.createElement("div");
  document.body.appendChild(element);
  const root = createRoot(element);
  await act(async () => {
    root.render(createElement(Probe));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    element.remove();
  });
  if (!latest) throw new Error("hook 没挂上");
  return latest;
}

describe("useMerge", () => {
  it("check() 查到东西就弹出来，冲突默认全是「保持游戏目录那份」", async () => {
    mergeStatus.mockResolvedValue(status);
    const api = await mountHook();
    expect(api.status, "还没查之前不该有东西").toBeNull();

    await act(async () => {
      await api.check();
    });

    expect(latest?.status?.conflicts).toHaveLength(2);
    expect(latest?.choices).toEqual({ "custom/scmr-8": "target", "hots/evo": "target" });
  });

  it("没有另一份数据时什么都不弹", async () => {
    mergeStatus.mockResolvedValue(null);
    const api = await mountHook();
    await act(async () => {
      await api.check();
    });
    expect(latest?.status).toBeNull();
  });

  it("apply() 把选好的那份原样交给后端，成功后收起来", async () => {
    mergeStatus.mockResolvedValue(status);
    const api = await mountHook();
    await act(async () => {
      await api.check();
    });
    await act(async () => {
      latest?.setChoice("hots/evo", "both");
    });
    await act(async () => {
      await latest?.apply();
    });

    expect(mergeApply).toHaveBeenCalledWith({ "custom/scmr-8": "target", "hots/evo": "both" });
    expect(latest?.status).toBeNull();
  });

  it("「稍后」之后就收起来（下次启动再问）", async () => {
    mergeStatus.mockResolvedValue(status);
    const api = await mountHook();
    await act(async () => {
      await api.check();
    });
    await act(async () => {
      latest?.dismiss();
    });
    expect(latest?.status).toBeNull();
  });
});
