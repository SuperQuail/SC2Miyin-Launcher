import { createRoot } from "react-dom/client";

import { askShell, ASK_ACTIVATE, ASK_REMOVE_VARIANT } from "../../../api/ask";
import type { SlotView } from "../../../api/types";
import type { CampaignMenuItem } from "../campaigns/host";
import { CustomView } from "./CustomView";
import type { CustomHost } from "./host";

/** 外壳递进来的数据。 */
interface ShellProps {
  slots: SlotView[];
  libraryRoot: string;
  loading: boolean;
}

/**
 * 给 Vue 外壳用的入口（自制战役页）。
 *
 * 和战役流程 / 设置页一样自己拿 root：这个视图依赖外壳的数据（槽位、库根目录），
 * props 一变就得重渲染。动作一律交回外壳（见 `host.ts`），要回执的两个走
 * `api/ask.ts` 那条事件通道。
 */
export function mountCustom(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  const root = createRoot(host);

  const render = (next: Record<string, unknown>) => {
    const shell = next as unknown as ShellProps;
    const customHost: CustomHost = {
      slots: shell.slots ?? [],
      libraryRoot: shell.libraryRoot ?? "",
      loading: shell.loading,
      notify: (kind, text) => emit("notify", { kind, text }),
      bootstrap: () => emit("bootstrap"),
      refresh: () => emit("refresh"),
      activate: (slot, variantId) => askShell(ASK_ACTIVATE, slot, variantId),
      removeVariant: (slot, variantId) => askShell(ASK_REMOVE_VARIANT, slot, variantId),
      showMenu: (event, items: CampaignMenuItem[], onPick) => emit("menu", { event, items, onPick }),
      openCheats: () => emit("open-cheats"),
    };
    root.render(<CustomView {...customHost} />);
  };

  render(props);
  return { update: render, teardown: () => root.unmount() };
}
