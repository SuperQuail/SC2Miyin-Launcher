import { createRoot } from "react-dom/client";

import { SettingsView, type SettingsViewProps } from "./SettingsView";

/** 外壳递进来的数据（形状由 App.vue 那边给）。 */
type ShellProps = Omit<SettingsViewProps, "notify" | "actions">;

function build(props: Record<string, unknown>, emit: (event: string, payload?: unknown) => void): SettingsViewProps {
  const shell = props as unknown as ShellProps;
  return {
    ...shell,
    notify: (kind, text) => emit("notify", { kind, text }),
    actions: {
      chooseGameDirectory: () => emit("choose-game-dir"),
      reveal: (path) => emit("reveal", path),
      collectCampaign: (dir) => emit("collect", dir),
      toggleSaveIsolation: (enabled) => emit("toggle-isolation", enabled),
      saveCurrentSaves: () => emit("save-current"),
      switchSaveProfile: (name) => emit("switch-profile", name),
      assignSaveProfile: (name, slot) => emit("assign-profile", { name, slot }),
      backupSaves: () => emit("backup-saves"),
      restoreSaves: (name) => emit("restore-saves", name),
      refresh: () => emit("refresh"),
      checkUpdate: () => emit("check-update"),
      downloadUpdate: () => emit("download-update"),
      applyUpdate: () => emit("apply-update"),
      openDevPage: () => emit("open-dev"),
    },
  };
}

/**
 * 给 Vue 外壳用的入口。
 *
 * 这个视图**依赖外壳的数据**（安装信息、存档、更新状态），所以 props 变化必须重渲染 ——
 * 不能只按第一次的数据画（安置桥的 `mountIsland` 对这类视图不够用，这里自己拿 root）。
 */
export function mountSettings(
  host: HTMLElement,
  emit: (event: string, payload?: unknown) => void,
  props: Record<string, unknown>,
) {
  const root = createRoot(host);
  const render = (next: Record<string, unknown>) => root.render(<SettingsView {...build(next, emit)} />);
  render(props);
  return {
    update: render,
    teardown: () => root.unmount(),
  };
}
