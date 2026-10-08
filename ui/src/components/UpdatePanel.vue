<script setup lang="ts">
/**
 * 更新面板：检查 GitHub Release、下载、换上新版本。
 *
 * 两条原则写在界面上：
 * - **网络失败不算错误** —— 检查失败只提示一下，不弹红字
 * - **不确定就不动** —— 版本号认不出来 / 摘要对不上，一律不提示"有新版本"
 */
import { computed, onMounted, onUnmounted, ref } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type { NetworkSettings, Staged, UpdateCheck, UpdateProgress } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const { notify, refreshUpdateBadge } = useLauncher();

const current = ref("");
const checking = ref(false);
const result = ref<UpdateCheck | null>(null);
const staged = ref<Staged | null>(null);
const progress = ref<UpdateProgress | null>(null);
const downloading = ref(false);
const applying = ref(false);
const showNotes = ref(false);

const network = ref<NetworkSettings>({
  proxy_mode: "auto",
  proxy_url: null,
  use_mirrors: true,
  include_prerelease: true,
});
const proxy = ref<{ url: string; source: string } | null>(null);
const savingNetwork = ref(false);

let unsubscribe: (() => void) | null = null;

onMounted(async () => {
  try {
    current.value = await api.appVersion();
    network.value = await api.networkSettings();
    proxy.value = await api.detectedProxy();
  } catch (error) {
    notify("error", errorText(error));
  }

  if (api.onUpdateProgress) {
    unsubscribe = await api.onUpdateProgress((payload) => {
      progress.value = payload;
    });
  }
});

onUnmounted(() => unsubscribe?.());

const latest = computed(() => result.value?.latest ?? null);

const progressText = computed(() => {
  const value = progress.value;
  if (!value) return "";
  const done = formatBytes(value.done);
  if (value.total) return done + " / " + formatBytes(value.total);
  return done;
});

/** 保存网络设置；改完立刻生效（下一次检查就用新设置）。 */
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

async function check(): Promise<void> {
  checking.value = true;
  staged.value = null;
  progress.value = null;
  try {
    result.value = await api.checkUpdate();
    await refreshUpdateBadge();

    if (result.value.error) {
      notify("info", "检查更新失败：" + result.value.error);
    } else if (result.value.available && result.value.latest) {
      notify("success", "发现新版本 " + result.value.latest.version);
    } else {
      notify("info", "已经是最新版本");
    }
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    checking.value = false;
  }
}

async function download(): Promise<void> {
  const release = latest.value;
  if (!release) return;

  downloading.value = true;
  progress.value = null;
  try {
    staged.value = await api.downloadUpdate(release.version);
    notify("success", "已下载 " + release.version + "，可以安装了");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    downloading.value = false;
  }
}

async function apply(): Promise<void> {
  applying.value = true;
  try {
    await api.applyUpdate();
    notify("info", "正在替换程序，启动器会自己重启…");
  } catch (error) {
    notify("error", errorText(error));
    applying.value = false;
  }
}

function openRelease(): void {
  const url = latest.value?.html_url || "https://github.com/SuperQuail/SC2Miyin-Launcher/releases";
  void api.openUrl(url).catch((error) => notify("error", errorText(error)));
}

/** 更新说明通常很长，截断显示。 */
const notes = computed(() => latest.value?.notes?.trim() ?? "");
</script>

<template>
  <section class="panel">
    <header class="panel__head">
      <h4 class="panel__title">更新</h4>
      <span class="panel__version">当前 {{ current || "…" }}</span>
    </header>

    <p class="panel__note">
      更新来自 GitHub Release。国内直连较慢时会<strong>自动并发试多个镜像</strong>，谁先下完用谁；
      代理默认按「环境变量 → Windows 系统代理 → 直连」自动挑。
    </p>

    <div class="row">
      <button class="btn btn-tonal" type="button" :disabled="checking" @click="check">
        {{ checking ? "检查中…" : "检查更新" }}
      </button>
      <button class="btn btn-text" type="button" @click="openRelease">打开发行页面</button>
      <span v-if="result?.via" class="row__hint">网络：{{ result.via }}</span>
      <span v-else-if="proxy" class="row__hint">网络：{{ proxy.url }}（{{ proxy.source }}）</span>
      <span v-else class="row__hint">网络：直连</span>
    </div>

    <!-- 有更新 -->
    <div v-if="latest && result?.available" class="release">
      <div class="release__head">
        <span class="release__version">{{ latest.version }}</span>
        <span v-if="latest.prerelease" class="tag tag--warn">预发行</span>
        <span class="release__date">{{ latest.published_at.slice(0, 10) }}</span>
      </div>

      <div v-if="notes" class="release__notes" :class="{ 'release__notes--folded': !showNotes }">
        {{ notes }}
      </div>
      <button v-if="notes.length > 160" class="btn btn-text" type="button" @click="showNotes = !showNotes">
        {{ showNotes ? "收起说明" : "展开说明" }}
      </button>

      <div class="row">
        <button
          class="btn btn-primary"
          type="button"
          :disabled="downloading || applying"
          @click="download"
        >
          {{ downloading ? "下载中…" : staged ? "重新下载" : "下载" }}
        </button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="!staged || applying"
          @click="apply"
        >
          {{ applying ? "正在替换…" : "安装并重启" }}
        </button>
        <span v-if="staged" class="row__hint">
          已下载到 {{ staged.root }}（来自 {{ staged.url.includes("github.com") ? "GitHub 直连" : "镜像" }}）
        </span>
      </div>

      <!-- 进度条 -->
      <div v-if="downloading || progress" class="bar">
        <div class="bar__fill" :style="{ width: (progress?.percent ?? 0) + '%' }"></div>
      </div>
      <div v-if="progress" class="bar__text">{{ progressText }}</div>
    </div>

    <!-- 已是最新 / 检查失败 -->
    <p v-else-if="result && !result.error && !result.available" class="panel__ok">
      已经是最新版本。
    </p>
    <p v-else-if="result?.error" class="panel__warn">{{ result.error }}</p>

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

.bar {
  height: 6px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  overflow: hidden;
}

.bar__fill {
  height: 100%;
  background: var(--accent);
  transition: width 0.2s ease;
}

.bar__text {
  font-size: 11.5px;
  color: var(--on-surface-variant);
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
