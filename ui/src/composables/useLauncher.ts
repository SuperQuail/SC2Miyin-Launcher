/**
 * 启动器的全局状态与动作。
 *
 * 用模块级单例而不是 Pinia：状态很少，单例足够且没有额外依赖。
 */
import { computed, ref } from "vue";

import { api, isDesktop } from "../api/bridge";
import type { Installation, SlotView, Variant } from "../api/types";

export type ToastKind = "info" | "success" | "error";

export interface Toast {
  kind: ToastKind;
  message: string;
}

const installation = ref<Installation | null>(null);
const slots = ref<SlotView[]>([]);
const libraryRoot = ref("");
const loading = ref(false);
const busy = ref(false);
const ready = ref(false);
/** 被拖进窗口的压缩包路径；战役页取走后会清空。 */
const droppedPackage = ref<string | null>(null);

/** 有可用更新时存版本号，供顶栏提示；null 表示没有。 */
const updateAvailable = ref<string | null>(null);

/** 静默检查一次更新：**失败不打扰用户**，悄悄把提示清掉就行。 */
async function refreshUpdateBadge(): Promise<void> {
  try {
    const result = await api.checkUpdate();
    updateAvailable.value = result.available ? (result.latest?.version ?? null) : null;
  } catch {
    updateAvailable.value = null;
  }
}

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

/** 首次进入时探测游戏并载入战役库。 */
async function bootstrap(): Promise<void> {
  if (ready.value) return;
  loading.value = true;
  try {
    installation.value = await api.detectInstallation();
    libraryRoot.value = await api.libraryRoot();
    slots.value = await api.listSlots();
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
    ready.value = true;
  }
}

/** 重新读取槽位。 */
async function refresh(): Promise<void> {
  loading.value = true;
  try {
    slots.value = await api.listSlots();
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

/** 启用某个版本（variantId 为 null 表示切回原版战役）。 */
async function activate(slot: string, variantId: string | null): Promise<boolean> {
  busy.value = true;
  try {
    const warnings = await api.activateVariant(slot, variantId);
    await refresh();
    if (warnings.length) {
      notify("info", warnings[0]);
    } else if (variantId === null) {
      notify("success", "已切回原版战役");
    } else {
      notify("success", "已启用：" + variantId);
    }
    return true;
  } catch (error) {
    notify("error", errorText(error));
    return false;
  } finally {
    busy.value = false;
  }
}

/** 从库里删除一个版本。 */
async function removeVariant(slot: string, variantId: string): Promise<boolean> {
  busy.value = true;
  try {
    await api.deleteVariant(slot, variantId);
    await refresh();
    notify("success", "已删除：" + variantId);
    return true;
  } catch (error) {
    notify("error", errorText(error));
    return false;
  } finally {
    busy.value = false;
  }
}

/** 把包导入到某个槽位。 */
async function importInto(slot: string, path: string): Promise<Variant | null> {
  busy.value = true;
  try {
    const created = await api.importPackage(path, slot);
    await refresh();
    notify("success", "已导入：" + created.name);
    return created;
  } catch (error) {
    notify("error", errorText(error));
    return null;
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
    updateAvailable,
    refreshUpdateBadge,
    droppedPackage,
    isDesktop,
    installation: computed(() => installation.value),
    slots: computed(() => slots.value),
    libraryRoot: computed(() => libraryRoot.value),
    loading: computed(() => loading.value),
    busy: computed(() => busy.value),
    toast: computed(() => toast.value),
    online: computed(() => installation.value !== null),
    bootstrap,
    refresh,
    chooseGameDirectory,
    activate,
    removeVariant,
    importInto,
    launch,
    reveal,
    notify,
  };
}
