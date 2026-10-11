import { ModsView } from "./ModsView";
import type { ModsHost } from "./useMods";
import { mountIsland } from "../../island";

/**
 * 给 Vue 外壳用的入口。
 *
 * 外壳递进来的东西：安装目录（决定几个按钮能不能点）、三个动作
 * （notify / reveal / bootstrap），以及全局右键菜单 —— 菜单在外壳里，
 * React 侧只说"在哪儿弹、有哪些项"。
 */
export function mountMods(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  const installation = props.installation as { root?: string } | null;

  const modsHost: ModsHost = {
    installationRoot: installation?.root ?? null,
    notify: (kind, text) => emit("notify", { kind, text }),
    reveal: (path) => emit("reveal", path),
    bootstrap: () => emit("bootstrap"),
    showMenu: (event, items, onPick) => emit("menu", { event, items, onPick }),
  };

  return mountIsland(host, emit, props, () => <ModsView {...modsHost} />);
}
