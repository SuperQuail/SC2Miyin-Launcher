<script setup lang="ts">
import { onMounted, ref } from "vue";

import { BACKDROP, MIYIN } from "./api/art";
import { useLauncher } from "./composables/useLauncher";
import CampaignsView from "./views/CampaignsView.vue";
import SettingsView from "./views/SettingsView.vue";

const { installation, toast, bootstrap, isDesktop } = useLauncher();

type ViewId = "campaigns" | "settings";

/** 允许用 #settings 直接打开设置页（方便调试与截图）。 */
function initialView(): ViewId {
  return typeof window !== "undefined" && window.location.hash.includes("settings")
    ? "settings"
    : "campaigns";
}

const view = ref<ViewId>(initialView());

const tabs: { id: ViewId; label: string }[] = [
  { id: "campaigns", label: "战役" },
  { id: "settings", label: "设置" },
];

const backdropStyle = { backgroundImage: "url(" + BACKDROP + ")" };

onMounted(() => {
  void bootstrap();
});
</script>

<template>
  <div class="app">
    <div class="app-backdrop"></div>
    <div class="app-wallpaper" :style="backdropStyle"></div>

    <header class="topbar">
      <div class="brand">
        <span class="brand__avatar">
          <img :src="MIYIN.chibi" alt="弥音" />
        </span>
        <div class="brand__text">
          <span class="brand__name">弥音启动器</span>
          <span class="brand__sub">MiYin Launcher</span>
        </div>
      </div>

      <nav class="tabs">
        <button
          v-for="tab in tabs"
          :key="tab.id"
          class="tab"
          :class="{ 'tab--active': view === tab.id }"
          type="button"
          @click="view = tab.id"
        >
          {{ tab.label }}
        </button>
      </nav>

      <div class="topbar__right">
        <span v-if="!isDesktop" class="tag tag--demo">演示模式</span>
        <span v-if="installation" class="tag">{{ installation.version }}</span>
      </div>
    </header>

    <main class="content">
      <CampaignsView v-if="view === 'campaigns'" @open-settings="view = 'settings'" />
      <SettingsView v-else />
    </main>

    <Transition name="toast">
      <div v-if="toast" class="toast" :class="'toast--' + toast.kind">
        {{ toast.message }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
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
  padding: 0 22px;
  background: linear-gradient(120deg, #6d59b8 0%, #5a48a0 55%, #4a3b8c 100%);
  color: #fff;
  box-shadow: 0 2px 16px rgba(10, 6, 30, 0.45);
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
  color: #4a3b8c;
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
