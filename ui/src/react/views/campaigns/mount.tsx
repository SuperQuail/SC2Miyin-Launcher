import { createRoot } from "react-dom/client";

import { askShell, ASK_ACTIVATE, ASK_REMOVE_VARIANT } from "../../../api/ask";
import type { Installation, SlotView } from "../../../api/types";
import { CampaignsView } from "./CampaignsView";
import type { CampaignsHost, CampaignMenuItem } from "./host";

/** 外壳递进来的数据。 */
interface ShellProps {
  installation: Installation | null;
  slots: SlotView[];
  libraryRoot: string;
  loading: boolean;
  busy: boolean;
  droppedPackage: string | null;
}

/**
 * 给 Vue 外壳用的入口（战役列表 + 战役菜单 + 导入，一整块）。
 *
 * 这个视图**依赖外壳的数据**（安装信息、槽位、库根目录），props 一变就得重渲染 ——
 * 和设置页一样自己拿 root，不用 `mountIsland`（那个只为不依赖外壳数据的视图准备）。
 *
 * 动作一律交回外壳：`useLauncher` 还管着启用预演、通知、导航这些外壳的事。
 * 单向的用 emit，要回执的走 `api/ask.ts` 那条事件通道。
 *
 * （原来还在这里挂了「只含导入对话框」「只含自制战役面板」两个小岛 —— 自制战役页
 * 迁到 React 之后它们没有调用方了，跟着删掉。）
 */
export function mountCampaigns(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  const root = createRoot(host);

  const render = (next: Record<string, unknown>) => {
    const shell = next as unknown as ShellProps;
    const campaignHost: CampaignsHost = {
      installation: shell.installation,
      slots: shell.slots ?? [],
      libraryRoot: shell.libraryRoot ?? "",
      loading: shell.loading,
      busy: shell.busy,
      droppedPackage: shell.droppedPackage ?? null,
      consumeDropped: () => emit("consume-drop"),
      notify: (kind, text) => emit("notify", { kind, text }),
      reveal: (path) => emit("reveal", path),
      bootstrap: () => emit("bootstrap"),
      chooseGameDirectory: () => emit("choose-game-dir"),
      showMenu: (event, items: CampaignMenuItem[], onPick) => emit("menu", { event, items, onPick }),
      launch: () => emit("launch"),
      activate: (slot, variantId) => askShell(ASK_ACTIVATE, slot, variantId),
      removeVariant: (slot, variantId) => askShell(ASK_REMOVE_VARIANT, slot, variantId),
      refresh: () => emit("refresh"),
      pendingActivation: (next.pendingActivation ?? null) as CampaignsHost["pendingActivation"],
      confirmActivation: () => askShell(ASK_ACTIVATE, "__confirm__", null),
      cancelActivation: () => emit("cancel-activation"),
      openCheats: () => emit("open-cheats"),
      openSettings: () => emit("open-settings"),
    };
    root.render(<CampaignsView {...campaignHost} />);
  };

  render(props);
  return { update: render, teardown: () => root.unmount() };
}
