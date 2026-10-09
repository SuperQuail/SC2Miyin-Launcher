/**
 * 启动器的全局状态与动作。
 *
 * 用模块级单例而不是 Pinia：状态很少，单例足够且没有额外依赖。
 */
import { computed, ref } from "vue";

import { api, isDesktop } from "../api/bridge";
import type {
  Installation,
  SlotView,
  Staged,
  UpdateCheck,
  UpdateProgress,
} from "../api/types";

export type ToastKind = "info" | "success" | "warning" | "error";

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
/* ---------------- 自动更新 ----------------
 *
 * 状态放在这里而不是组件里，因为**顶栏与设置页都要读它**：
 * 顶栏显示"有新版本"的角标，设置页显示终端与进度条。
 */

/** 更新过程的日志（内嵌终端的内容），最多留 400 行。 */
const updateLogs = ref<string[]>([]);
/** 最近一次检查结果。 */
const updateCheck = ref<UpdateCheck | null>(null);
/** 下载好、等着装的东西。 */
const updateStaged = ref<Staged | null>(null);
const updateProgress = ref<UpdateProgress | null>(null);
const updateChecking = ref(false);
const updateDownloading = ref(false);
/** 下载完成后弹的那个"要重启了"对话框。 */
const showRestartPrompt = ref(false);
/** 页面标识。新增页面时这里加一个，App.vue 的标签页跟着加。 */
export type ViewId = "campaigns" | "custom" | "mods" | "settings";

/** 当前页面。放在这里而不是 App.vue 里，是为了让更新公告也能切页面。 */
const currentView = ref<ViewId>("campaigns");

/** 启动器自己的版本（对外写法，如 0.1.0a3）。顶栏与更新面板共用。 */
const launcherVersion = ref("");

/** 启动时的更新公告（渲染 Release 正文的那个弹窗）。 */
const updateNoticeVisible = ref(false);
/** 公告上的「不再提示这个版本」勾选状态。 */
const updateNoticeMuted = ref(false);

/** 「不再提示」记在这里；只记版本号，下次开新版还会提示。 */
const MUTED_KEY = "miyin.update.mutedVersion";

function readMutedVersion(): string {
  try {
    return localStorage.getItem(MUTED_KEY) ?? "";
  } catch {
    // 隐私模式之类可能不让读，忽略即可
    return "";
  }
}

function writeMutedVersion(version: string): void {
  try {
    if (version) localStorage.setItem(MUTED_KEY, version);
    else localStorage.removeItem(MUTED_KEY);
  } catch {
    /* 存不了就算了，顶多多提示几次 */
  }
}

/** 关掉公告（这次不提示了，下次启动还会提示）。 */
function dismissUpdateNotice(): void {
  updateNoticeVisible.value = false;
  updateNoticeMuted.value = false;
}

/** 「不再提示这个版本」：记下版本号，以后不再弹；等出更新的版本再弹。 */
function muteThisVersion(): void {
  const version = updateCheck.value?.latest?.version ?? "";
  writeMutedVersion(version);
  updateNoticeVisible.value = false;
  updateNoticeMuted.value = false;
  pushUpdateLog("已设置为不再提示 " + version);
}

/** 有可用更新时存版本号，供顶栏提示；null 表示没有。 */
const updateAvailable = computed(() => {
  const result = updateCheck.value;
  return result?.available ? (result.latest?.version ?? null) : null;
});

function pushUpdateLog(line: string): void {
  const stamp = new Date().toTimeString().slice(0, 8);
  updateLogs.value = [...updateLogs.value.slice(-399), "[" + stamp + "] " + line];
}

let updateEventsBound = false;

/** 订阅后端推来的日志与进度（只订阅一次）。
 *
 * **不要在这里判断 isDesktop** —— 浏览器演示模式的桥接也提供了这两个订阅，
 * 加了判断的话演示模式就收不到任何日志（终端永远只有一行），踩过一次。
 */
async function bindUpdateEvents(): Promise<void> {
  if (updateEventsBound) return;
  updateEventsBound = true;
  try {
    await api.onUpdateLog((line) => pushUpdateLog(line));
    await api.onUpdateProgress((progress) => {
      updateProgress.value = progress;
    });
  } catch {
    updateEventsBound = false;
  }
}

/** 跑一次检查。quiet=true 时不弹提示（启动时自动扫描用）。 */
async function checkUpdateNow(quiet = false): Promise<void> {
  if (updateChecking.value) return;
  updateChecking.value = true;
  updateProgress.value = null;
  updateStaged.value = null;

  try {
    await bindUpdateEvents();
    pushUpdateLog("—— 开始检查更新 ——");
    const result = await api.checkUpdate();
    updateCheck.value = result;

    if (result.error) {
      pushUpdateLog("检查失败：" + result.error);
      if (!quiet) notify("info", "检查更新失败：" + result.error);
    } else if (result.available && result.latest) {
      if (!quiet) notify("success", "发现新版本 " + result.latest.version);
    } else if (!quiet) {
      notify("info", "已经是最新版本");
    }
  } catch (error) {
    pushUpdateLog("检查出错：" + errorText(error));
    if (!quiet) notify("error", errorText(error));
  } finally {
    updateChecking.value = false;
  }
}

/** 启动时自动扫一次：**不需要用户手点**；有新版就把公告弹出来。 */
async function autoCheckUpdate(): Promise<void> {
  await checkUpdateNow(true);

  if (updateCheck.value?.available && updateCheck.value.latest) {
    const version = updateCheck.value.latest.version;

    if (readMutedVersion() === version) {
      // 用户勾过「不再提示这个版本」—— 角标照常有，但不再弹窗
      pushUpdateLog("已设为不再提示 " + version + "，只显示角标");
    } else {
      updateNoticeVisible.value = true;
    }

    // 浏览器演示模式：把下载流程也演一遍，
    // 好让开发与截图能看到进度条与重启弹窗的实际样子（真实运行不会这样）。
    if (!isDesktop) {
      setTimeout(() => void downloadUpdateNow(), 6000);
    }
  }
}

/** 下载更新包。 */
async function downloadUpdateNow(): Promise<void> {
  const release = updateCheck.value?.latest;
  if (!release || updateDownloading.value) return;

  updateDownloading.value = true;
  updateProgress.value = null;
  pushUpdateLog("—— 开始下载 " + release.version + " ——");

  try {
    const staged = await api.downloadUpdate(release.version);
    updateStaged.value = staged;
    // 公告与重启弹窗不同时出现
    updateNoticeVisible.value = false;
    pushUpdateLog("下载完成，点「安装并重启」即可换上 " + staged.version);
    // 下载完成 → 弹窗告诉用户需要重启
    showRestartPrompt.value = true;
  } catch (error) {
    pushUpdateLog("下载失败：" + errorText(error));
    notify("error", errorText(error));
  } finally {
    updateDownloading.value = false;
  }
}

/**
 * 公告里点「下载并安装」。
 *
 * **不在公告里直接下载** —— 那样用户只看到一个转圈，看不到进度条和内嵌终端，
 * 容易以为卡死了。正确做法是：收起公告 -> 跳到设置页的更新面板 -> 在那里开始下载，
 * 用户能看着它一步步做完。
 */
async function downloadFromNotice(): Promise<void> {
  dismissUpdateNotice();
  currentView.value = "settings";

  // 等面板挂载并滚到位置，再开始下载。
  // 太早开始的话，进度条会先于用户视线出现，看着莫名其妙。
  await new Promise((resolve) => setTimeout(resolve, 350));
  await downloadUpdateNow();
}

/** 换上新版本（程序会退出，由替换脚本接管）。 */
async function applyUpdateNow(): Promise<void> {
  try {
    pushUpdateLog("正在准备替换，启动器即将退出…");
    await api.applyUpdate();
  } catch (error) {
    pushUpdateLog("替换失败：" + errorText(error));
    notify("error", errorText(error));
    showRestartPrompt.value = false;
  }
}

/** 被拖进窗口的压缩包路径；战役页取走后会清空。 */
const droppedPackage = ref<string | null>(null);

const toast = ref<Toast | null>(null);

let toastTimer: ReturnType<typeof setTimeout> | null = null;

/** 弹一条提示。 */
/**
 * 启动时把可选工具装上 —— **默认静默安装**。
 *
 * 约定：没有就自动装；装不上提示一句「该部分功能不可用」，之后启动不再尝试
 * （后端记了标记，下次直接返回 skipped），用户想再试到设置里手动装。
 *
 * 这里**绝不能抛** —— 一个可选工具装不上，不该影响启动器本身能用。
 */
async function ensureTools(): Promise<void> {
  try {
    const result = await api.ensureTool("sc2diff");

    if (result.action === "installed") {
      notify("success", result.name + " 已自动装好");
    } else if (result.action === "failed" && result.message) {
      notify("warning", result.message);
    }
    // already_installed / skipped 都不打扰用户
  } catch {
    // 静默：这条路失败不影响启动
  }
}

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
  // 启动器版本：界面要显示的是**自己**的版本，不是游戏构建号
  void api
    .appVersion()
    .then((version) => {
      launcherVersion.value = version;
    })
    .catch(() => {
      /* 拿不到就不显示，不该因此打扰用户 */
    });

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

/** 待确认的启用：预演已经算好，等用户点头。 */
type PendingActivation = {
  slot: string;
  variantId: string;
  preview: NonNullable<Awaited<ReturnType<typeof api.previewActivation>>>;
};

const pendingActivation = ref<PendingActivation | null>(null);

/** 游戏目录里已经装着的自制战役（还没收进库的那些）。 */
const installedCampaigns = ref<Awaited<ReturnType<typeof api.listInstalledCampaigns>>>([]);

async function refreshInstalled(): Promise<void> {
  try {
    installedCampaigns.value = await api.listInstalledCampaigns();
  } catch {
    installedCampaigns.value = [];
  }
}

/**
 * 把游戏目录里已经装着的战役**收进库**。
 *
 * 原目录**一个字节都不动** —— 那是用户自己放的文件夹，删不删他自己决定。
 * 想清干净就先用启动器"启用"一次库里那份，再自己删掉外面这个。
 */
async function collectCampaign(dir: string): Promise<void> {
  busy.value = true;
  try {
    await api.importPackageWith(dir, null, "rename");
    await refresh();
    await refreshInstalled();
    notify("success", "已收进库 —— 原目录没动，战役列表里能看到了");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/** 存档（issue #20）：现在这份 + 已经备份的那些。 */
const saves = ref<Awaited<ReturnType<typeof api.listSaves>> | null>(null);
const saveBackups = ref<Awaited<ReturnType<typeof api.listSaveBackups>>>([]);

async function refreshSaves(): Promise<void> {
  try {
    saves.value = await api.listSaves();
    saveBackups.value = await api.listSaveBackups();
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 备份现在的存档。label 一般填战役名。 */
async function backupSaves(label: string): Promise<void> {
  busy.value = true;
  try {
    const name = await api.backupSaves(label);
    await refreshSaves();
    notify("success", "存档已备份：" + name);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/** 还原一份备份。后端会先把现在的存档另存一份。 */
async function restoreSaves(name: string): Promise<void> {
  busy.value = true;
  try {
    const safety = await api.restoreSaves(name);
    await refreshSaves();
    notify("success", safety ? "已还原，还原前那份存为 " + safety : "已还原");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/** 日间 / 夜间。存在 localStorage —— 这是界面偏好，不必让后端知道。 */
/** `?theme=dark` 能强制一次 —— 截图和排查用，不写进 localStorage。 */
const forcedTheme = new URLSearchParams(location.search).get("theme");

const theme = ref<"light" | "dark">(
  forcedTheme === "dark" || forcedTheme === "light"
    ? forcedTheme
    : typeof localStorage !== "undefined" && localStorage.getItem("miyin.theme") === "dark"
      ? "dark"
      : "light",
);

function applyTheme(): void {
  document.documentElement.dataset.theme = theme.value;
  if (forcedTheme) return;
  try {
    localStorage.setItem("miyin.theme", theme.value);
  } catch {
    // 隐私模式下 localStorage 会抛；主题当次有效就行
  }
}

function toggleTheme(): void {
  theme.value = theme.value === "dark" ? "light" : "dark";
  applyTheme();
}

applyTheme();

/**
 * 请求启用 —— **先算预演，再让用户点头**。
 *
 * 写盘闸门放开到「安装目录里任意位置」之后，这是唯一一眼：
 * 会新增什么、覆盖什么（覆盖前会备份）、把谁的东西接管过来、删掉什么。
 * 切回原版不拦（那只是撤下我们装的东西）。
 */
async function requestActivation(slot: string, variantId: string | null): Promise<boolean> {
  if (variantId === null) return await activate(slot, null);
  busy.value = true;
  try {
    // 游戏跑着的时候换文件，Windows 上会删不掉 / 覆盖失败，留下记了一半的清单
    const running = await api.sc2Running();
    if (running) {
      notify("error", "星际争霸 II 正在运行（" + running + "），先退出游戏再切换。");
      return false;
    }
  } finally {
    busy.value = false;
  }
  busy.value = true;
  try {
    const preview = await api.previewActivation(slot, variantId);
    if (!preview) return await activate(slot, variantId);
    const touched =
      preview.add.length + preview.overwrite.length + preview.takeover.length + preview.delete.length;
    // 没什么可看的就别拦路
    if (touched === 0) return await activate(slot, variantId);
    pendingActivation.value = { slot, variantId, preview };
    return false;
  } catch (error) {
    notify("error", errorText(error));
    return false;
  } finally {
    busy.value = false;
  }
}

/** 用户在预演里点了「继续」。 */
async function confirmActivation(): Promise<boolean> {
  const pending = pendingActivation.value;
  pendingActivation.value = null;
  return pending ? await activate(pending.slot, pending.variantId) : false;
}

function cancelActivation(): void {
  pendingActivation.value = null;
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
    launcherVersion,
    currentView,
    downloadFromNotice,
    updateNoticeVisible,
    updateNoticeMuted,
    dismissUpdateNotice,
    muteThisVersion,
    updateAvailable,
    updateLogs,
    updateCheck,
    updateStaged,
    updateProgress,
    updateChecking,
    updateDownloading,
    showRestartPrompt,
    checkUpdateNow,
    autoCheckUpdate,
  ensureTools,
    downloadUpdateNow,
    applyUpdateNow,
    droppedPackage,
    isDesktop,
    installation: computed(() => installation.value),
    slots: computed(() => slots.value),
    libraryRoot: computed(() => libraryRoot.value),
    loading: computed(() => loading.value),
    busy: computed(() => busy.value),
    toast: computed(() => toast.value),
    bootstrap,
    refresh,
    chooseGameDirectory,
    activate,
  requestActivation,
  confirmActivation,
  cancelActivation,
  pendingActivation,
  theme,
  toggleTheme,
  installedCampaigns,
  refreshInstalled,
  collectCampaign,
  saves,
  saveBackups,
  refreshSaves,
  backupSaves,
  restoreSaves,
    removeVariant,
    launch,
    reveal,
    notify,
  };
}
