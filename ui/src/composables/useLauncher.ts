/**
 * 启动器的全局状态与动作。
 *
 * 用模块级单例而不是 Pinia：状态很少，单例足够且没有额外依赖。
 */
import { computed, ref } from "vue";

import { api, isDesktop } from "../api/bridge";
import type { Campaign, Installation, PackageInspection } from "../api/types";

export type ToastKind = "info" | "success" | "error";

export interface Toast {
  kind: ToastKind;
  message: string;
}

const installation = ref<Installation | null>(null);
const campaigns = ref<Campaign[]>([]);
const loading = ref(false);
const busy = ref(false);
const ready = ref(false);
const toast = ref<Toast | null>(null);

let toastTimer: ReturnType<typeof setTimeout> | null = null;

/** 弹一条提示。 */
export function notify(kind: ToastKind, message: string): void {
  toast.value = { kind, message };
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = null;
  }, 4200);
}

/** 把任意异常转成可展示的文本。 */
export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

/** 首次进入时探测游戏并载入战役列表。 */
async function bootstrap(): Promise<void> {
  if (ready.value) return;
  loading.value = true;
  try {
    installation.value = await api.detectInstallation();
    if (installation.value) {
      campaigns.value = await api.listCampaigns();
    }
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
    ready.value = true;
  }
}

/** 重新扫描战役列表。 */
async function refresh(): Promise<void> {
  if (!installation.value) return;
  loading.value = true;
  try {
    campaigns.value = await api.listCampaigns();
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
  }
}

/** 手动指定游戏目录。 */
async function chooseGameDirectory(): Promise<void> {
  try {
    const path = await api.pickGameDirectory();
    if (!path) return;
    installation.value = await api.setInstallation(path);
    await refresh();
    notify("success", "已设置游戏目录");
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 选择战役包并预检。 */
async function pickAndInspect(): Promise<PackageInspection | null> {
  try {
    const path = await api.pickPackage();
    if (!path) return null;
    return await api.inspectPackage(path);
  } catch (error) {
    notify("error", errorText(error));
    return null;
  }
}

/** 安装一个已预检过的包。 */
async function install(path: string): Promise<boolean> {
  busy.value = true;
  try {
    const campaign = await api.installPackage(path);
    await refresh();
    notify("success", "已安装：" + campaign.name);
    return true;
  } catch (error) {
    notify("error", errorText(error));
    return false;
  } finally {
    busy.value = false;
  }
}

/** 卸载战役。 */
async function uninstall(id: string): Promise<void> {
  busy.value = true;
  try {
    await api.uninstallCampaign(id);
    await refresh();
    notify("success", "已卸载：" + id);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/** 启动游戏。 */
async function launch(): Promise<void> {
  try {
    await api.launchGame();
    notify("success", "正在启动星际争霸 II…");
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 在资源管理器中打开路径。 */
async function reveal(path: string): Promise<void> {
  try {
    await api.revealPath(path);
  } catch (error) {
    notify("error", errorText(error));
  }
}

export function useLauncher() {
  return {
    isDesktop,
    installation: computed(() => installation.value),
    campaigns: computed(() => campaigns.value),
    loading: computed(() => loading.value),
    busy: computed(() => busy.value),
    toast: computed(() => toast.value),
    online: computed(() => installation.value !== null),
    bootstrap,
    refresh,
    chooseGameDirectory,
    pickAndInspect,
    install,
    uninstall,
    launch,
    reveal,
    notify,
  };
}
