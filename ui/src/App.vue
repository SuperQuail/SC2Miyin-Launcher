<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import { api } from "./api/bridge";
import { BACKDROP, MIYIN } from "./api/art";
import { useLauncher } from "./composables/useLauncher";
import type { ViewId } from "./composables/useLauncher";
import ContextMenu from "./components/ContextMenu.vue";
import { contextMenuHandledRecently, useContextMenu } from "./composables/useContextMenu";
import UpdateNotice from "./components/UpdateNotice.vue";
import CampaignsView from "./views/CampaignsView.vue";
import CustomView from "./views/CustomView.vue";
import ModsView from "./views/ModsView.vue";
import CheatsView from "./views/CheatsView.vue";
import SettingsView from "./views/SettingsView.vue";

const {
  installation,
  toast,
  bootstrap,
  isDesktop,
  droppedPackage,
  updateAvailable,
  updateStaged,
  showRestartPrompt,
  autoCheckUpdate,
  applyUpdateNow,
  currentView,
  launcherVersion,
  ensureTools,
} = useLauncher();

/** 会出水波纹的元素。加新组件时把类名补进来就行。 */
const RIPPLE_TARGETS =
  ".btn, .chip, .tab, .variant, .slot, .map, .target, .ctx__item, .group__head, .update-badge";

const tabs: { id: ViewId; label: string }[] = [
  { id: "campaigns", label: "战役" },
  // 自制战役是**另一个顶层选项**，不是「战役」里的一个分组 ——
  // 两类的玩法根本不同（一个由游戏驱动，一个得用编辑器打开）
  { id: "custom", label: "自制战役" },
  { id: "mods", label: "模组" },
  { id: "settings", label: "设置" },
];

/** 作弊码这类查询工具走弹层，不占标签位。 */
const cheatsOpen = ref(false);

const menu = useContextMenu();

/**
 * 右键：**一律拦掉浏览器原生的那个**。
 *
 * WebView 的原生菜单是「刷新 / 另存为 / 检查元素」那一套 —— 放在桌面应用里
 * 格格不入，而且会把用户导向一个跟本应用无关的世界。所以整个窗口一律拦掉，
 * 换成我们自己的：
 *
 * - 组件自己处理过的（卡片、地图行…）用组件那份菜单 —— 靠时间戳认出来
 * - **输入框里给复制粘贴** —— 不然用户连粘贴个路径都做不到，这是拦掉的代价
 * - 其余地方落到默认菜单
 */
function onContextMenu(event: MouseEvent): void {
  event.preventDefault();
  if (contextMenuHandledRecently()) return;

  const editable = (event.target as HTMLElement | null)?.closest<HTMLElement>(
    "input, textarea, [contenteditable='true']",
  );

  if (editable) {
    menu.show(
      event,
      [
        { id: "cut", label: "剪切" },
        { id: "copy", label: "复制" },
        { id: "paste", label: "粘贴" },
        { id: "selectAll", label: "全选", separatorBefore: true },
      ],
      (id) => void editAction(editable, id),
    );
    return;
  }

  menu.show(
    event,
    [
      { id: "refresh", label: "刷新数据" },
      { id: "settings", label: "设置", separatorBefore: true },
      { id: "repo", label: "项目主页" },
    ],
    (id) => {
      if (id === "refresh") void bootstrap();
      if (id === "settings") currentView.value = "settings";
      if (id === "repo") void api.openUrl("https://github.com/SuperQuail/SC2Miyin-Launcher");
    },
  );
}

/** 输入框里的剪切 / 复制 / 粘贴 / 全选。 */
async function editAction(element: HTMLElement, action: string): Promise<void> {
  const field = element as HTMLInputElement | HTMLTextAreaElement;

  if (action === "copy" || action === "cut") {
    const selected = field.value?.slice(field.selectionStart ?? 0, field.selectionEnd ?? 0) ?? "";
    try {
      await navigator.clipboard.writeText(selected);
      if (action === "cut") {
        const start = field.selectionStart ?? 0;
        const end = field.selectionEnd ?? 0;
        field.value = field.value.slice(0, start) + field.value.slice(end);
        // 改完要通知 Vue —— 不然双向绑定还是旧值
        field.dispatchEvent(new Event("input", { bubbles: true }));
      }
    } catch {
      // 剪贴板被挡就算了，不弹红字
    }
    return;
  }

  if (action === "paste") {
    try {
      const text = await navigator.clipboard.readText();
      const start = field.selectionStart ?? field.value.length;
      const end = field.selectionEnd ?? field.value.length;
      field.value = field.value.slice(0, start) + text + field.value.slice(end);
      field.selectionStart = field.selectionEnd = start + text.length;
      field.dispatchEvent(new Event("input", { bubbles: true }));
    } catch {
      // 读剪贴板需要权限，被拒就安静收场
    }
    return;
  }

  if (action === "selectAll") {
    field.focus();
    field.select();
  }
}

const backdropStyle = { backgroundImage: "url(" + BACKDROP + ")" };

/**
 * 窗口按钮。
 *
 * 我们关掉了系统边框（`decorations: false`），所以最小化 / 最大化 / 关闭
 * 得自己画。浏览器演示模式下这些命令不存在，按钮直接不显示。
 */
const maximized = ref(false);

/**
 * 按住顶栏拖动窗口。
 *
 * 本来 `data-tauri-drag-region` 就够了，但它有个坑：**只认事件落在
 * 带这个属性的元素本身**。点了里面带文字的 span、或任何子元素，就不响应 ——
 * 用户的感觉就是「有时候能拖有时候不能」，很难受。
 *
 * 所以自己接管：落点不在按钮/链接/输入框/窗口控件上就调系统的拖动，
 * 整条顶栏（除了那几个控件）都是有效拖动区。
 */
function startWindowDrag(event: MouseEvent): void {
  if (event.button !== 0) return;

  const target = event.target as HTMLElement | null;
  if (target?.closest("button, a, input, select, textarea, .winctl")) return;

  void (async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // 浏览器演示模式没有这个能力，忽略
    }
  })();
}

/** 读一次最大化状态。双击标题栏 / Win+↑ 是系统改的，只能靠 resize 补上。 */
async function syncMaximized(): Promise<void> {
  try {
    maximized.value = await api.windowIsMaximized();
  } catch {
    // 拿不到就当没最大化
  }
}

onMounted(() => {
  if (!isDesktop) return;
  void syncMaximized();
  window.addEventListener("resize", () => void syncMaximized());
});

async function toggleMaximize(): Promise<void> {
  try {
    maximized.value = await api.windowToggleMaximize();
  } catch {
    // 忽略
  }
}

/** 有文件被拖到窗口上方。 */
const dragging = ref(false);
let stopWatching: (() => void) | null = null;

/**
 * 点击波纹：在按钮 / 卡片上按一下，从落点扩散一圈。
 *
 * 用一个全局监听而不是给每个组件加指令 —— 界面里的可点元素太多，
 * 与其到处挂，不如在这里按选择器统一处理，新加的组件自动就有。
 */
function spawnRipple(event: MouseEvent): void {
  const target = (event.target as HTMLElement | null)?.closest<HTMLElement>(RIPPLE_TARGETS);
  if (!target || target.hasAttribute("disabled")) return;

  // 宿主样式（定位 + 裁剪）在这里打类，免得 CSS 再抄一份选择器
  target.classList.add("ripple-host");

  const rect = target.getBoundingClientRect();
  // 直径取长边两倍，保证从任何角落点都能铺满
  const size = Math.max(rect.width, rect.height) * 2;
  const dot = document.createElement("span");
  dot.className = "ripple";
  dot.style.width = dot.style.height = size + "px";
  dot.style.left = event.clientX - rect.left - size / 2 + "px";
  dot.style.top = event.clientY - rect.top - size / 2 + "px";
  target.appendChild(dot);
  window.setTimeout(() => dot.remove(), 620);
}

onMounted(async () => {
  window.addEventListener("mousedown", spawnRipple);
  window.addEventListener("contextmenu", onContextMenu);
  void bootstrap();

  // **启动就自动扫描更新**，不需要用户手点。
  // 稍微延后，别和启动时的战役库读取抢时间。
  setTimeout(() => void autoCheckUpdate(), 1200);

  // **可选工具默认静默安装**：没有就装上；装不上只提示一句，之后不再重试。
  // 排在更新检查后面 —— 那是更要紧的事，工具装不上不影响启动器本身。
  setTimeout(() => void ensureTools(), 2600);

  // 浏览器演示模式没有这个 API，静默跳过
  if (!isDesktop) return;

  try {
    stopWatching = await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;

      if (payload.type === "over") {
        dragging.value = true;
      } else if (payload.type === "leave") {
        dragging.value = false;
      } else if (payload.type === "drop") {
        dragging.value = false;
        const path = payload.paths[0];
        if (!path) return;
        // 拖到哪个页面都行：切回战役页，交给它去预检
        currentView.value = "campaigns";
        droppedPackage.value = path;
      }
    });
  } catch {
    stopWatching = null;
  }
});

onUnmounted(() => {
  window.removeEventListener("mousedown", spawnRipple);
  window.removeEventListener("contextmenu", onContextMenu);
  stopWatching?.();
});
</script>

<template>
  <div class="app">
    <div class="app-backdrop"></div>
    <div class="app-wallpaper" :style="backdropStyle"></div>

    <!-- 顶栏同时是标题栏：空白处按住可以拖窗口 -->
    <header class="topbar" data-tauri-drag-region @mousedown="startWindowDrag">
      <div class="brand" data-tauri-drag-region>
        <span class="brand__avatar">
          <img :src="MIYIN.chibi" alt="弥音" />
        </span>
        <div class="brand__text">
          <span class="brand__name">弥音启动器</span>
          <span class="brand__sub">MiYin Launcher</span>
        </div>
      </div>

      <nav class="tabs" data-tauri-drag-region>
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="tab"
          :class="{ 'tab--active': currentView === tab.id }"
          type="button"
          @click="currentView = tab.id"
        >
          {{ tab.label }}
        </button>
      </nav>

      <div class="topbar__right">
        <button
          v-if="updateAvailable"
          class="update-badge"
          type="button"
          title="有新版本可用，点开设置页查看"
          @click="currentView = 'settings'"
        >
          <!-- 下载图标：向下箭头落进托盘，用户一眼就知道是"可以下载新版本" -->
          <svg class="update-badge__icon" viewBox="0 0 16 16" aria-hidden="true">
            <path d="M8 1.5v7.5" />
            <path d="M5 6.5 8 9.5l3-3" />
            <path d="M2.5 11v1.5A2 2 0 0 0 4.5 14.5h7a2 2 0 0 0 2-2V11" />
          </svg>
          <span>新版本 {{ updateAvailable }}</span>
        </button>
        <span v-if="!isDesktop" class="tag tag--demo">演示模式</span>
        <!-- 启动器自己的版本：以前这里只显示游戏构建号，很容易被当成启动器版本 -->
        <span v-if="launcherVersion" class="tag tag--app" title="弥音启动器版本">
          v{{ launcherVersion }}
        </span>
        <span v-if="installation" class="tag" title="星际争霸 II 版本">
          SC2 {{ installation.version }}
        </span>
      </div>

      <!-- 自绘的窗口按钮：系统边框已经关掉了 -->
      <div class="winctl">
        <button class="winctl__btn" type="button" title="最小化" @click="api.windowMinimize()">
          <svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6h7" /></svg>
        </button>
        <button
          class="winctl__btn"
          type="button"
          :title="maximized ? '还原' : '最大化'"
          @click="toggleMaximize"
        >
          <svg v-if="maximized" viewBox="0 0 12 12" aria-hidden="true">
            <rect x="2.5" y="4.5" width="5" height="5" rx="1" />
            <path d="M4.5 4.5v-2h5v5h-2" />
          </svg>
          <svg v-else viewBox="0 0 12 12" aria-hidden="true">
            <rect x="2.5" y="2.5" width="7" height="7" rx="1.5" />
          </svg>
        </button>
        <button
          class="winctl__btn winctl__btn--close"
          type="button"
          title="关闭"
          @click="api.windowClose()"
        >
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M3 3l6 6" />
            <path d="M9 3l-6 6" />
          </svg>
        </button>
      </div>
    </header>

    <!-- 拖拽导入 -->
    <div v-if="dragging" class="dropzone">
      <div class="dropzone__card">
        <div class="dropzone__title">松手即可导入</div>
        <div class="dropzone__text">
          支持 zip / 7z / rar / tar —— 启动器会自动判断它属于哪部战役，
          认不出来会让你选，不会瞎猜。
        </div>
      </div>
    </div>

    <main class="content">
      <!-- 切页时淡入上移，两个方向都给一点衔接 -->
      <Transition name="view" mode="out-in">
        <CampaignsView
          v-if="currentView === 'campaigns'"
          key="campaigns"
          @open-settings="currentView = 'settings'"
          @open-cheats="cheatsOpen = true"
        />
        <CustomView
          v-else-if="currentView === 'custom'"
          key="custom"
          @open-cheats="cheatsOpen = true"
        />
        <ModsView v-else-if="currentView === 'mods'" key="mods" />
        <SettingsView v-else key="settings" />
      </Transition>
    </main>

    <Transition name="toast">
      <div v-if="toast" class="toast" :class="'toast--' + toast.kind">
        {{ toast.message }}
      </div>
    </Transition>
    <!-- 作弊码：战役页 / 自制战役页的「小工具」里弹出来 -->
    <CheatsView v-if="cheatsOpen" @close="cheatsOpen = false" />

    <!-- 全局右键菜单：任何地方调 useContextMenu().show() 就能弹 -->
    <ContextMenu />

    <!-- 启动时的更新公告（渲染 Release 正文的 Markdown） -->
    <UpdateNotice />

    <!-- 更新下载完成后提示重启 -->
    <div v-if="showRestartPrompt" class="sheet">
      <div class="sheet__card">
        <div class="sheet__badge">
          <svg class="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3v10" />
            <path d="M8 9l4 4 4-4" />
            <path d="M4 16v2a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-2" />
          </svg>
        </div>
        <h3 class="sheet__title">更新已下载完成</h3>
        <p class="sheet__text">
          需要重启启动器才能完成安装 —— Windows 上没法覆盖正在运行的程序。
          <br />
          重启后版本会变成 <strong>{{ updateStaged?.version }}</strong>，
          你导入的战役、补丁和设置都不会受影响。
        </p>
        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="showRestartPrompt = false">
            稍后
          </button>
          <button class="btn btn-primary" type="button" @click="applyUpdateNow">
            立即重启并安装
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 拖进来的遮罩：覆盖整个窗口，告诉用户松手就能导入 */
.dropzone {
  position: fixed;
  inset: 0;
  z-index: 90;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(12, 18, 32, 0.55);
  backdrop-filter: blur(4px);
  pointer-events: none;
}

.dropzone__card {
  padding: 26px 34px;
  border-radius: var(--radius-lg);
  border: 2px dashed var(--accent);
  background: var(--surface-1);
  box-shadow: var(--shadow-3);
  text-align: center;
}

.dropzone__title {
  font-size: 19px;
  font-weight: 700;
  color: var(--accent);
}

.dropzone__text {
  margin-top: 8px;
  max-width: 420px;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.app {
  position: relative;
  display: flex;
  flex-direction: column;
  height: 100%;
}

.app-wallpaper {
  position: fixed;
  inset: 0;
  z-index: 0;
  background-size: cover;
  background-position: center 30%;
  opacity: 0.34;
  filter: saturate(1.1);
}

.topbar {
  position: relative;
  z-index: 3;
  display: flex;
  align-items: center;
  gap: 24px;
  height: var(--header-height);
  /* 右边留 6px 给自绘的窗口按钮 —— 系统边框已经关掉了 */
  padding: 0 6px 0 22px;
  background: linear-gradient(120deg, #5b8bf0 0%, #3b6ce0 55%, #2b57c4 100%);
  color: #fff;
  box-shadow: 0 2px 16px rgba(8, 18, 40, 0.42);
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 220px;
}

.brand__avatar {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 12px;
  overflow: hidden;
  background: #fff;
  box-shadow: 0 2px 8px rgba(8, 5, 22, 0.35);
}

/* 立绘自带白底，与白色头像框无缝衔接 */
.brand__avatar img {
  width: 118%;
  height: 118%;
  object-fit: cover;
  object-position: center 42%;
}

.brand__text {
  display: flex;
  flex-direction: column;
  line-height: 1.15;
}

.brand__name {
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 1px;
}

.brand__sub {
  font-size: 11px;
  opacity: 0.72;
  letter-spacing: 0.6px;
}

/* ---------- 自绘的窗口按钮 ---------- */
/*
 * 窗口按钮。
 *
 * 不加负外边距 —— 试过一次 `@margin-right: -18px`@ 想让它贴到窗口边缘，
 * 结果整组被顶出可视区，界面上直接看不见，而系统边框已经关掉了，
 * 那就等于没法关窗口。宁可和右边缘留一点间距。
 */
.winctl {
  display: flex;
  align-self: stretch;
  margin-left: 6px;
  /* 顶栏原本的右内边距对按钮来说太宽，收一点 */
  margin-right: -14px;
}

.winctl__btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 100%;
  padding: 0;
  border: none;
  background: none;
  color: #fff;
  cursor: pointer;
  transition: background 0.12s ease, color 0.12s ease;
}

.winctl__btn svg {
  width: 12px;
  height: 12px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.2;
  stroke-linecap: round;
}

.winctl__btn:hover {
  background: rgba(255, 255, 255, 0.16);
  color: #fff;
}

/* 关闭按钮悬停要变红 —— 这是所有桌面应用的共同约定 */
.winctl__btn--close:hover {
  background: #d13438;
  color: #fff;
}

.tabs {
  display: flex;
  gap: 6px;
  margin-left: 8px;
}

.tab {
  height: 36px;
  padding: 0 20px;
  border-radius: var(--radius-pill);
  color: rgba(255, 255, 255, 0.86);
  font-size: 14px;
  font-weight: 600;
  transition: background var(--duration) var(--ease), color var(--duration) var(--ease);
}

.tab:hover {
  background: rgba(255, 255, 255, 0.14);
}

.tab--active {
  background: #fff;
  color: #2b57c4;
}

.topbar__right {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.tag {
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.18);
  font-size: 12px;
  letter-spacing: 0.4px;
}

.update-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 13px 5px 10px;
  border-radius: var(--radius-pill);
  border: none;
  background: #fff;
  color: var(--accent);
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
  /* 轻轻呼吸一下，把注意力引过来 */
  animation: update-pulse 2.4s ease-in-out infinite;
}

.update-badge__icon {
  width: 15px;
  height: 15px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.7;
  stroke-linecap: round;
  stroke-linejoin: round;
}

@keyframes update-pulse {
  0%,
  100% {
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.18);
  }
  50% {
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 22%, transparent);
  }
}

@media (prefers-reduced-motion: reduce) {
  .update-badge {
    animation: none;
  }
}



.update-badge:hover {
  background: var(--accent-soft);
}

.tag--app {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 700;
}

.tag--demo {
  background: rgba(255, 214, 120, 0.24);
  color: #ffe6a8;
}

.content {
  position: relative;
  z-index: 1;
  flex: 1;
  overflow-y: auto;
  padding: 24px 26px 40px;
}

.toast {
  position: fixed;
  right: 24px;
  bottom: 24px;
  z-index: 40;
  max-width: 420px;
  padding: 13px 18px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  color: var(--on-surface);
  box-shadow: var(--shadow-3);
  border-left: 4px solid var(--accent);
  font-size: 13.5px;
}

.toast--warning {
  background: color-mix(in srgb, var(--warning, #b26a00) 92%, transparent);
  color: #fff;
}

.toast--success {
  border-left-color: var(--success);
}

.toast--error {
  border-left-color: var(--danger);
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 180ms var(--ease), transform 180ms var(--ease);
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(10px);
}
</style>
