<script setup lang="ts">
/**
 * 更新面板：检查 / 下载 / 安装，外加一个**内嵌终端**告诉用户正在发生什么。
 *
 * 状态放在 useLauncher 里（顶栏也要读），这里只负责渲染。
 */
import { computed, nextTick, onMounted, ref, watch } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type { NetworkSettings } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const {
  notify,
  updateLogs,
  updateCheck,
  updateStaged,
  updateProgress,
  updateChecking,
  updateDownloading,
  checkUpdateNow,
  downloadUpdateNow,
  applyUpdateNow,
} = useLauncher();

const current = ref("");
const network = ref<NetworkSettings>({
  proxy_mode: "auto",
  proxy_url: null,
  use_mirrors: true,
  include_prerelease: true,
});
const proxy = ref<{ url: string; source: string } | null>(null);
const savingNetwork = ref(false);
const showNotes = ref(false);
const logBox = ref<HTMLElement | null>(null);
/** 整个面板；从更新公告跳过来时滚到自己身上。 */
const panelBox = ref<HTMLElement | null>(null);

onMounted(async () => {
  try {
    current.value = await api.appVersion();
    network.value = await api.networkSettings();
    proxy.value = await api.detectedProxy();
  } catch (error) {
    notify("error", errorText(error));
  }
});

// 从更新公告跳过来时自动开始下载 —— 把面板滚到视线里，
// 否则用户只听到"在下载"却看不到进度条与终端
watch(updateDownloading, async (downloading) => {
  if (!downloading) return;
  await nextTick();
  panelBox.value?.scrollIntoView({ behavior: "smooth", block: "start" });
});

// 终端自动滚到底：用户进来看到的是最新一行，而不是最老的
watch(
  () => updateLogs.value.length,
  async () => {
    await nextTick();
    if (logBox.value) logBox.value.scrollTop = logBox.value.scrollHeight;
  },
);

const latest = computed(() => updateCheck.value?.latest ?? null);
const staged = computed(() => updateStaged.value);
const progress = computed(() => updateProgress.value);
const logs = computed(() => updateLogs.value);

const networkLabel = computed(() => {
  if (updateCheck.value?.via) return updateCheck.value.via;
  if (proxy.value) return proxy.value.url + "（" + proxy.value.source + "）";
  return "直连";
});

/** 进度条百分比；总长度未知时给个不确定态的宽度。 */
const percent = computed(() => progress.value?.percent ?? 0);
const hasPercent = computed(() => progress.value?.percent != null);

function progressText(): string {
  const value = progress.value;
  if (!value) return "";
  if (value.total) {
    return formatBytes(value.done) + " / " + formatBytes(value.total);
  }
  return formatBytes(value.done);
}

async function saveNetwork(): Promise<void> {
  savingNetwork.value = true;
  try {
    await api.setNetworkSettings(network.value);
    proxy.value = await api.detectedProxy();
    notify("success", "网络设置已保存");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    savingNetwork.value = false;
  }
}

function openRelease(): void {
  const url =
    latest.value?.html_url ||
    "https://github.com/SuperQuail/SC2Miyin-Launcher/releases";
  void api.openUrl(url).catch((error) => notify("error", errorText(error)));
}

const notes = computed(() => latest.value?.notes?.trim() ?? "");
</script>

<template>
  <section ref="panelBox" class="panel">
    <header class="panel__head">
      <h4 class="panel__title">更新</h4>
      <span class="panel__version">当前 {{ current || "…" }}</span>
    </header>

    <p class="panel__note">
      启动时会<strong>自动检查</strong>一次更新，有新版本会在右上角显示下载标记。
      国内直连较慢时会自动并发试多个镜像；代理被限流时会自动改直连重试。
    </p>

    <div class="row">
      <button
        class="btn btn-tonal"
        type="button"
        :disabled="updateChecking"
        @click="checkUpdateNow(false)"
      >
        {{ updateChecking ? "检查中…" : "重新检查" }}
      </button>
      <button class="btn btn-text" type="button" @click="openRelease">打开发行页面</button>
      <span class="row__hint">网络：{{ networkLabel }}</span>
    </div>

    <!-- 有更新 -->
    <div v-if="latest && updateCheck?.available" class="release">
      <div class="release__head">
        <span class="release__version">{{ latest.version }}</span>
        <span v-if="latest.prerelease" class="tag tag--warn">预发行</span>
        <span class="release__date">{{ latest.published_at.slice(0, 10) }}</span>
      </div>

      <div v-if="notes" class="release__notes" :class="{ 'release__notes--folded': !showNotes }">
        {{ notes }}
      </div>
      <button
        v-if="notes.length > 160"
        class="btn btn-text"
        type="button"
        @click="showNotes = !showNotes"
      >
        {{ showNotes ? "收起说明" : "展开说明" }}
      </button>

      <!-- 进度条 -->
      <div v-if="updateDownloading || progress" class="progress">
        <div class="progress__track">
          <div
            class="progress__fill"
            :class="{ 'progress__fill--unknown': !hasPercent }"
            :style="hasPercent ? { width: percent + '%' } : undefined"
          ></div>
        </div>
        <div class="progress__meta">
          <span v-if="hasPercent" class="progress__percent">{{ Math.round(percent) }}%</span>
          <span class="progress__bytes">{{ progressText() }}</span>
        </div>
      </div>

      <div class="row">
        <button
          class="btn btn-primary"
          type="button"
          :disabled="updateDownloading"
          @click="downloadUpdateNow"
        >
          {{ updateDownloading ? "下载中…" : staged ? "重新下载" : "下载" }}
        </button>
        <button class="btn btn-primary" type="button" :disabled="!staged" @click="applyUpdateNow">
          安装并重启
        </button>
        <span v-if="staged" class="row__hint">已就绪：{{ staged.version }}</span>
      </div>
    </div>

    <p v-else-if="updateCheck && !updateCheck.error && !updateCheck.available" class="panel__ok">
      已经是最新版本。
    </p>
    <p v-else-if="updateCheck?.error" class="panel__warn">{{ updateCheck.error }}</p>

    <!-- 内嵌终端 -->
    <div class="console">
      <div class="console__head">
        <span class="console__title">更新日志</span>
        <span class="console__hint">这里能看到刚才到底做了什么</span>
      </div>
      <div ref="logBox" class="console__body">
        <p v-if="!logs.length" class="console__empty">
          还没有日志。启动时的自动检查跑完就会有内容。
        </p>
        <p v-for="(line, index) in logs" :key="index" class="console__line">{{ line }}</p>
      </div>
    </div>

    <hr class="divider" />

    <h5 class="panel__sub">网络与更新通道</h5>

    <label class="field">
      <span class="field__label">代理</span>
      <select v-model="network.proxy_mode" class="field__input">
        <option value="auto">自动（环境变量 → 系统代理 → 直连）</option>
        <option value="off">不使用代理</option>
        <option value="manual">手动指定</option>
      </select>
    </label>

    <label v-if="network.proxy_mode === 'manual'" class="field">
      <span class="field__label">代理地址</span>
      <input
        v-model="network.proxy_url"
        class="field__input"
        type="text"
        placeholder="http://127.0.0.1:7897"
      />
    </label>

    <label class="check">
      <input v-model="network.use_mirrors" type="checkbox" />
      <span>下载时自动尝试 GitHub 镜像加速（国内推荐）</span>
    </label>

    <label class="check">
      <input v-model="network.include_prerelease" type="checkbox" />
      <span>接收预发行版本（alpha / beta）</span>
    </label>

    <div class="row">
      <button class="btn btn-tonal" type="button" :disabled="savingNetwork" @click="saveNetwork">
        {{ savingNetwork ? "保存中…" : "保存网络设置" }}
      </button>
    </div>
  </section>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 16px 18px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
}

.panel__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.panel__title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
}

.panel__version {
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.panel__sub {
  margin: 0;
  font-size: 13.5px;
  font-weight: 700;
}

.panel__note,
.panel__ok,
.panel__warn {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.panel__ok {
  color: var(--success, #2e7d32);
}

.panel__warn {
  padding: 8px 11px;
  border-radius: var(--radius-sm);
  background: var(--warning-soft);
  color: var(--warning);
}

.row {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 8px;
}

.row__hint {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.release {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  background: var(--accent-soft);
}

.release__head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.release__version {
  font-size: 15px;
  font-weight: 700;
  color: var(--accent);
}

.release__date {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.release__notes {
  font-size: 12.5px;
  line-height: 1.7;
  white-space: pre-wrap;
  color: var(--on-surface);
}

.release__notes--folded {
  max-height: 120px;
  overflow: hidden;
  -webkit-mask-image: linear-gradient(#000 60%, transparent);
  mask-image: linear-gradient(#000 60%, transparent);
}

/* ---------- 进度条 ---------- */

.progress {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.progress__track {
  height: 8px;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  overflow: hidden;
}

.progress__fill {
  height: 100%;
  border-radius: var(--radius-pill);
  background: var(--accent);
  transition: width 0.25s ease;
}

/* 总长度未知时来回扫，表示"在动但不知道还有多久" */
.progress__fill--unknown {
  width: 35%;
  animation: progress-slide 1.4s ease-in-out infinite;
}

@keyframes progress-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(340%);
  }
}

.progress__meta {
  display: flex;
  gap: 10px;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.progress__percent {
  font-weight: 700;
  color: var(--accent);
}

/* ---------- 内嵌终端 ---------- */

.console {
  border-radius: var(--radius-sm);
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--outline) 65%, transparent);
}

.console__head {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 6px 11px;
  background: var(--surface-3);
}

.console__title {
  font-size: 12px;
  font-weight: 700;
  color: var(--on-surface);
}

.console__hint {
  font-size: 11px;
  color: var(--on-surface-variant);
}

.console__body {
  height: 168px;
  overflow-y: auto;
  padding: 8px 11px;
  background: #10141c;
}

.console__empty {
  margin: 0;
  font-size: 11.5px;
  color: #7c8798;
}

.console__line {
  margin: 0 0 2px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.65;
  color: #c8d2e0;
  white-space: pre-wrap;
  word-break: break-all;
}

.divider {
  margin: 6px 0 2px;
  border: none;
  border-top: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
}

.field {
  display: block;
}

.field__label {
  display: block;
  margin-bottom: 4px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--on-surface-variant);
}

.field__input {
  width: 100%;
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 13px;
}

.check {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12.5px;
  color: var(--on-surface);
}

.tag--warn {
  background: var(--warning-soft);
  color: var(--warning);
}
</style>
