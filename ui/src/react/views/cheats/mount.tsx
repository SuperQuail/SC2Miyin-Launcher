import { CheatsView } from "./CheatsView";
import { mountIsland } from "../../island";

/**
 * 给 Vue 外壳用的入口：`host` 是挂载点，`emit` 把事件（close / notify）交回外壳。
 *
 * 这样 React 视图**不需要知道外壳是谁**：它只认两个回调。
 */
export function mountCheats(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  return mountIsland(host, emit, props, () => (
    <CheatsView
      onClose={() => emit("close")}
      notify={(kind, text) => emit("notify", { kind, text })}
    />
  ));
}
