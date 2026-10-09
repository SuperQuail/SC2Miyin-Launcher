<script setup lang="ts">
/**
 * 可选外部工具。
 *
 * 这些不是启动器的必需件 —— 不装照样能用，只是少了某项能力。
 * 所以装不装、装哪个版本都由用户定，**但安装本身要静默**：
 * 点了就下、下完就位，不去弹浏览器、也不让用户自己解压。
 */
import { onMounted, ref } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type { ToolRelease, ToolStatus } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const { notify } = useLauncher();

const tools = ref<ToolStatus[]>([]);
const loading = ref(true);
const busy = ref<string | null>(null);
/** 展开版本列表的那个工具 id。 */
const picking = ref<string | null>(null);
const releases = ref<Record<string, ToolRelease[]>>({});

onMounted(load);

async function load(): Promise<void> {
  loading.value = true;
  try {
    tools.value = await api.listTools();
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
  }
}

/** 装最新版（或指定版本）。 */
async function install(id: string, version: string | null): Promise<void> {
  busy.value = id;
  try {
    const updated = await api.installTool(id, version);
    tools.value = tools.value.map((item) => (item.id === id ? updated : item));
    picking.value = null;
    notify("success", updated.name + " " + (updated.version ?? "") + " 已装好");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

async function uninstall(id: string): Promise<void> {
  busy.value = id;
  try {
    await api.uninstallTool(id);
    await load();
    notify("success", "已卸载");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/** 展开版本列表（第一次展开才去查）。 */
async function toggleVersions(id: string): Promise<void> {
  if (picking.value === id) {
    picking.value = null;
    return;
  }

  picking.value = id;
  if (releases.value[id]) return;

  busy.value = id;
  try {
    releases.value = { ...releases.value, [id]: await api.toolReleases(id) };
  } catch (error) {
    notify("error", errorText(error));
    picking.value = null;
  } finally {
    busy.value = null;
  }
}

function versionLabel(release: ToolRelease): string {
  const date = release.published_at.slice(0, 10);
  return release.version + (release.prerelease ? "（预发行）" : "") + " · " + date + " · " + formatBytes(release.size);
}
</script>

<template>
  <section class="card panel">
    <header class="panel__head">
      <h3 class="panel__title">可选工具</h3>
      <span class="panel__hint">不装也能正常用</span>
    </header>

    <p v-if="loading" class="notes__empty">正在读取…</p>

    <article v-for="tool in tools" :key="tool.id" class="tool">
      <div class="tool__head">
        <div class="tool__title">
          <span class="tool__name">{{ tool.name }}</span>
          <span v-if="tool.installed" class="tag tag--ok">已安装 {{ tool.version }}</span>
          <span v-else class="tag">未安装</span>
        </div>
        <div class="tool__actions">
          <button class="btn btn-text btn--tiny" type="button" @click="api.openToolRepo(tool.id)">
            仓库
          </button>
          <button
            class="btn btn-text btn--tiny"
            type="button"
            :disabled="busy !== null"
            @click="toggleVersions(tool.id)"
          >
            选版本 ▾
          </button>
          <button
            class="btn btn-primary btn--tiny"
            type="button"
            :disabled="busy !== null"
            @click="install(tool.id, null)"
          >
            {{ busy === tool.id ? "处理中…" : tool.installed ? "更新到最新" : "一键安装" }}
          </button>
          <button
            v-if="tool.installed"
            class="btn btn-text btn--tiny btn--danger"
            type="button"
            :disabled="busy !== null"
            @click="uninstall(tool.id)"
          >
            卸载
          </button>
        </div>
      </div>

      <p class="tool__about">{{ tool.about }}</p>

      <!-- 自动装失败过：说清为什么之后不再自动重试 -->
      <p v-if="tool.auto_failed" class="tool__failed">
        上次自动安装失败<template v-if="tool.auto_error">：{{ tool.auto_error }}</template>。
        <strong>之后启动不会再自动尝试</strong> —— 想装的话点上面的「一键安装」。
      </p>
      <p v-if="tool.installed && tool.path" class="tool__path" :title="tool.path">
        {{ tool.path }} · {{ formatBytes(tool.size_bytes) }}
      </p>

      <!-- 版本列表 -->
      <ul v-if="picking === tool.id" class="versions">
        <li v-if="busy === tool.id" class="versions__empty">正在查询…</li>
        <li v-else-if="!releases[tool.id]?.length" class="versions__empty">
          这个工具还没有可安装的版本
        </li>
        <li v-for="release in releases[tool.id] ?? []" :key="release.tag" class="version">
          <span class="version__label">{{ versionLabel(release) }}</span>
          <button
            class="btn btn-text btn--tiny"
            type="button"
            :disabled="busy !== null || !release.has_asset"
            @click="install(tool.id, release.version)"
          >
            {{ release.has_asset ? "装这个版本" : "此版本没有附件" }}
          </button>
        </li>
      </ul>
    </article>
  </section>
</template>

<style scoped>
.tool {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding: 12px 0;
  border-top: 1px solid color-mix(in srgb, var(--outline) 40%, transparent);
}

.tool:first-of-type {
  border-top: none;
  padding-top: 4px;
}

.tool__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.tool__title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tool__name {
  font-size: 13.5px;
  font-weight: 700;
}

.tool__actions {
  display: flex;
  gap: 4px;
}

.tool__about {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.tool__failed {
  margin: 0;
  padding: 7px 11px;
  border-radius: var(--radius-sm);
  background: var(--warning-soft, color-mix(in srgb, #b26a00 14%, transparent));
  color: var(--warning, #b26a00);
  font-size: 12px;
  line-height: 1.7;
}

.tool__path {
  margin: 0;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  color: var(--on-surface-variant);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tag--ok {
  background: color-mix(in srgb, #00897b 18%, transparent);
  color: #00796b;
}

.versions {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 6px 0 0;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  list-style: none;
  max-height: 200px;
  overflow: auto;
}

.versions__empty {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.version {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.version__label {
  font-size: 12px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
}

.btn--tiny {
  padding: 3px 10px;
  font-size: 11.5px;
}

.btn--danger {
  color: var(--danger, #c62828);
}

.notes__empty {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}
</style>
