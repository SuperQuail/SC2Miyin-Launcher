import { createRoot } from "react-dom/client";

import { askShell, ASK_ACTIVATE, ASK_REMOVE_VARIANT } from "../../../api/ask";
import type { Installation } from "../../../api/types";
import type { DocInfo, SlotView } from "../../../api/types";
import { CampaignsView } from "./CampaignsView";
import { CustomCampaignPanel } from "./CustomCampaignPanel";
import type { CampaignsHost, CampaignMenuItem } from "./host";
import { ImportDialog } from "./ImportDialog";

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
 * 给 Vue 外壳用的入口（战役列表 + 战役菜单 + 导入 / 自制战役面板，一整块）。
 *
 * 这个视图**依赖外壳的数据**（安装信息、槽位、库根目录），props 一变就得重渲染 ——
 * 和设置页一样自己拿 root，不用 `mountIsland`（那个只为不依赖外壳数据的视图准备）。
 *
 * 动作一律交回外壳：`useLauncher` 还管着启用预演、通知、导航这些外壳的事。
 * 单向的用 emit，要回执的走 `api/ask.ts` 那条事件通道。
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

/** 通知 / 揭示 —— 几个岛共用的搬运写法。 */
function hostChannels(emit: (event: string, payload?: unknown) => void) {
  return {
    notify: (kind: "info" | "success" | "error", text: string) => emit("notify", { kind, text }),
    reveal: (path: string) => emit("reveal", path),
  };
}

/**
 * 只挂导入对话框的岛（自制战役页还在 Vue，先借它用）。
 *
 * `importing` 是外壳的状态：岛把它改回去，外壳才好禁用按钮、换文案。
 */
export function mountImportDialog(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  const build = (next: Record<string, unknown>) => {
    const shell = next as unknown as {
      entry?: "campaign" | "custom";
      mode?: "import" | "update";
      slot?: string;
      variantId?: string;
      importNonce?: number;
      slots?: SlotView[];
      droppedPackage?: string | null;
      importing?: boolean;
    };
    return (
      <ImportDialog
        entry={shell.entry ?? "campaign"}
        mode={shell.mode ?? "import"}
        slot={shell.slot ?? ""}
        variantId={shell.variantId ?? ""}
        importNonce={shell.importNonce ?? 0}
        slots={shell.slots ?? []}
        droppedPackage={shell.droppedPackage ?? null}
        consumeDropped={() => emit("consume-drop")}
        refresh={() => emit("refresh")}
        onImported={(slug) => emit("imported", slug)}
        onBusy={(value) => emit("busy", value)}
        {...hostChannels(emit)}
      />
    );
  };

  const root = createRoot(host);
  root.render(build(props));
  return { update: (next: Record<string, unknown>) => root.render(build(next)), teardown: () => root.unmount() };
}

/** 只挂自制战役面板的岛（地图 + 模组 + 启动）。 */
export function mountCustomCampaignPanel(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  const shell = props as unknown as {
    slot: string;
    variant: import("../../../api/types").Variant;
    active: boolean;
  };
  const root = createRoot(host);
  root.render(
    <CustomCampaignPanel
      slot={shell.slot}
      variant={shell.variant}
      active={shell.active}
      refresh={() => emit("refresh")}
      onOpenDoc={(doc: DocInfo) => emit("open-doc", doc)}
      showMenu={(event, items: CampaignMenuItem[], onPick) => emit("menu", { event, items, onPick })}
      {...hostChannels(emit)}
    />,
  );
  return { update: () => undefined, teardown: () => root.unmount() };
}