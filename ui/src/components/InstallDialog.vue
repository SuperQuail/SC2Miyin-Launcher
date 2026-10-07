<script setup lang="ts">
import { computed, ref } from "vue";

import { api } from "../api/bridge";
import { MIYIN, campaignTypeName, formatBytes } from "../api/art";
import type { PackageInspection } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const emit = defineEmits<{ close: [] }>();

const { install, busy, notify } = useLauncher();

const path = ref("");
const inspection = ref<PackageInspection | null>(null);
const inspecting = ref(false);

const canInstall = computed(() => inspection.value?.installable === true);

const formatLabel = computed(() => {
  switch (inspection.value?.format) {
    case "ccm":
      return "CCM 战役包";
    case "miyin":
      return "弥音标准包";
    case "plain":
      return "无元数据包";
    default:
      return "未知格式";
  }
});

/** 弹出文件选择框。 */
async function browse(): Promise<void> {
  try {
    const picked = await api.pickPackage();
    if (!picked) return;
    path.value = picked;
    await runInspect();
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 预检（这一步不写盘）。 */
async function runInspect(): Promise<void> {
  const target = path.value.trim();
  if (!target) return;

  inspecting.value = true;
  try {
    inspection.value = await api.inspectPackage(target);
  } catch (error) {
    inspection.value = null;
    notify("error", errorText(error));
  } finally {
    inspecting.value = false;
  }
}

/** 确认安装。 */
async function confirm(): Promise<void> {
  if (!inspection.value) return;
  const ok = await install(inspection.value.path);
  if (ok) emit("close");
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog card" role="dialog" aria-modal="true">
      <header class="dialog__head">
        <h3 class="dialog__title">安装战役包</h3>
        <button class="icon-btn" type="button" aria-label="关闭" @click="emit('close')">✕</button>
      </header>

      <div class="dialog__body">
        <div class="field">
          <span class="field__label">战役包路径</span>
          <div class="field__row">
            <input
              v-model="path"
              class="text-input"
              placeholder="选择一个 .zip 战役包，或把文件拖进窗口"
              spellcheck="false"
              @keyup.enter="runInspect"
            />
            <button class="btn btn-outline" type="button" @click="browse">浏览…</button>
            <button
              class="btn btn-tonal"
              type="button"
              :disabled="!path.trim() || inspecting"
              @click="runInspect"
            >
              {{ inspecting ? "核对中…" : "核对" }}
            </button>
          </div>
        </div>

        <div v-if="inspection" class="inspect">
          <div class="inspect__summary">
            <div class="inspect__name">{{ inspection.name ?? "未命名战役" }}</div>
            <div class="inspect__meta">
              <span>{{ formatLabel }}</span>
              <span>· {{ campaignTypeName(inspection.campaign_type) }}</span>
              <span v-if="inspection.author">· {{ inspection.author }}</span>
              <span v-if="inspection.version">· v{{ inspection.version }}</span>
            </div>
          </div>

          <dl class="facts">
            <div class="fact">
              <dt>地图</dt>
              <dd>{{ inspection.map_count }}</dd>
            </div>
            <div class="fact">
              <dt>模组</dt>
              <dd>{{ inspection.mod_count }}</dd>
            </div>
            <div class="fact">
              <dt>条目</dt>
              <dd>{{ inspection.entry_count }}</dd>
            </div>
            <div class="fact">
              <dt>解压后</dt>
              <dd>{{ formatBytes(inspection.unpacked_bytes) }}</dd>
            </div>
          </dl>

          <p class="target">
            将安装到：<code>{{ inspection.suggested_dir_name }}</code>
            <span v-if="inspection.content_root" class="target__note">
              （包内内容根 <code>{{ inspection.content_root }}</code> 会被剥离）
            </span>
          </p>

          <ul v-if="inspection.issues.length" class="issues">
            <li
              v-for="issue in inspection.issues"
              :key="issue.code"
              class="issue"
              :class="'issue--' + issue.level"
            >
              <strong>{{ issue.message }}</strong>
              <span v-if="issue.hint" class="issue__hint">{{ issue.hint }}</span>
            </li>
          </ul>
          <p v-else class="ok">✓ 核对通过，没有发现问题</p>
        </div>

        <div v-else class="placeholder">
          <img class="placeholder__art" :src="MIYIN.chibi" alt="" />
          <p class="placeholder__text">
            安装前会先读一遍压缩包：识别格式与元数据、解析包里的名称与资料片、
            检查有没有越界路径条目，并算出解压后的体积。<strong>这一步不会写入任何文件。</strong>
          </p>
        </div>
      </div>

      <footer class="dialog__foot">
        <button class="btn btn-text" type="button" @click="emit('close')">取消</button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="!canInstall || busy"
          @click="confirm"
        >
          {{ busy ? "正在安装…" : "确认安装" }}
        </button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 30;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 32px;
  background: rgba(10, 7, 22, 0.62);
  backdrop-filter: blur(4px);
}

.dialog {
  display: flex;
  flex-direction: column;
  width: min(680px, 100%);
  max-height: 100%;
  overflow: hidden;
  box-shadow: var(--shadow-3);
}

.dialog__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 18px 22px 12px;
}

.dialog__title {
  margin: 0;
  font-size: 17px;
}

.icon-btn {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-pill);
  color: var(--on-surface-variant);
  font-size: 15px;
}

.icon-btn:hover {
  background: var(--surface-2);
}

.dialog__body {
  flex: 1;
  overflow-y: auto;
  padding: 4px 22px 18px;
}

.field__label {
  display: block;
  margin-bottom: 7px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--on-surface-variant);
}

.field__row {
  display: flex;
  gap: 8px;
}

.inspect {
  margin-top: 18px;
  padding: 16px 18px;
  border-radius: var(--radius-md);
  background: var(--surface-2);
  border: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
}

.inspect__name {
  font-size: 15.5px;
  font-weight: 700;
}

.inspect__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  margin-top: 4px;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.facts {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin: 14px 0 0;
}

.fact {
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--surface-1);
  text-align: center;
}

.fact dt {
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.fact dd {
  margin: 2px 0 0;
  font-size: 15px;
  font-weight: 700;
}

.target {
  margin: 12px 0 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
  line-height: 1.7;
}

.target code,
.issues code {
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  background: var(--surface-3);
}

.target__note {
  opacity: 0.85;
}

.issues {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 12px 0 0;
  padding: 0;
  list-style: none;
}

.issue {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 11px;
  border-radius: var(--radius-sm);
  font-size: 12.5px;
  line-height: 1.5;
}

.issue--warning {
  background: var(--warning-soft);
  color: var(--warning);
}

.issue--broken {
  background: var(--danger-soft);
  color: var(--danger);
}

.issue__hint {
  opacity: 0.85;
}

.ok {
  margin: 12px 0 0;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--success);
}

.placeholder {
  display: flex;
  align-items: center;
  gap: 14px;
  margin: 18px 2px 4px;
  font-size: 13px;
  line-height: 1.8;
  color: var(--on-surface-variant);
}

.placeholder__art {
  flex: 0 0 auto;
  width: 96px;
  height: 96px;
  object-fit: contain;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 64%, transparent 84%);
  mask-image: radial-gradient(circle at 50% 52%, #000 64%, transparent 84%);
}

.placeholder__text {
  margin: 0;
}

.placeholder__text strong {
  color: var(--accent);
}

.dialog__foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 22px 18px;
  border-top: 1px solid color-mix(in srgb, var(--outline) 40%, transparent);
}
</style>
