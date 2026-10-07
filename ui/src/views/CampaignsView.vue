<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { api } from "../api/bridge";
import { MIYIN, formatBytes } from "../api/art";
import type { PackageInspection } from "../api/types";
import SlotCard from "../components/SlotCard.vue";
import { errorText, useLauncher } from "../composables/useLauncher";
import SlotMenuView from "./SlotMenuView.vue";

const emit = defineEmits<{ "open-settings": [] }>();

const {
  installation,
  slots,
  loading,
  launch,
  reveal,
  refresh,
  chooseGameDirectory,
  bootstrap,
  importInto,
  notify,
} = useLauncher();

/** 当前打开的战役槽位。 */
const opened = ref<string | null>(null);

onMounted(() => {
  void bootstrap();

  // 支持 #slot=wol 直接打开某个战役（调试与截图用）
  const match = /slot=([a-z]+)/.exec(typeof window === "undefined" ? "" : window.location.hash);
  if (match) opened.value = match[1];
});

/** 正在处理的导入：选好文件、预检完、等用户确认。 */
const pending = ref<{ path: string; inspection: PackageInspection; slot: string } | null>(null);
const importing = ref(false);

const openSlot = computed(() => slots.value.find((slot) => slot.slug === opened.value) ?? null);

const regionLabel = computed(() => {
  const branch = installation.value?.branch;
  if (!branch) return "未知区域";
  return branch.toLowerCase() === "cn" ? "国服" : branch.toUpperCase();
});

const modCount = computed(() =>
  slots.value.reduce((total, slot) => total + slot.variants.length, 0),
);
const activeCount = computed(() => slots.value.filter((slot) => slot.active !== null).length);

/** 没有自动识别出归属时，让用户选。 */
const needsTarget = computed(() => pending.value !== null && !pending.value.inspection.suggested_slot);

const pendingName = computed(
  () => pending.value?.inspection.name ?? pending.value?.inspection.suggested_dir_name ?? "未命名战役",
);

/** 自动判断出的目标战役（展示用中文名，而不是槽位标识）。 */
const suggestedName = computed(() => {
  const slug = pending.value?.inspection.suggested_slot;
  return slots.value.find((slot) => slot.slug === slug)?.display_name ?? null;
});

/** 选择压缩包并预检；能自动判断归属时直接给出目标战役。 */
async function startImport(): Promise<void> {
  try {
    const path = await api.pickPackage();
    if (!path) return;

    importing.value = true;
    const inspection = await api.inspectPackage(path);
    pending.value = { path, inspection, slot: inspection.suggested_slot ?? "" };
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}

/** 确认导入，并直接进入目标战役的菜单。 */
async function confirmImport(): Promise<void> {
  const current = pending.value;
  if (!current || !current.slot) return;

  const created = await importInto(current.slot, current.path);
  pending.value = null;
  if (created) opened.value = current.slot;
}
</script>

<template>
  <div class="page">
    <!-- 尚未设置游戏目录 -->
    <section v-if="!installation" class="empty card">
      <img class="empty__art" :src="MIYIN.cry" alt="" />
      <h2 class="empty__title">还没有找到星际争霸 II</h2>
      <p class="empty__text">
        启动器会先读取注册表里的安装位置。如果没有找到，请手动指定游戏根目录
        （即包含 <code>StarCraft II.exe</code> 的那一层）。
      </p>
      <div class="empty__actions">
        <button class="btn btn-primary btn-lg" type="button" @click="chooseGameDirectory">
          选择游戏目录
        </button>
        <button class="btn btn-outline" type="button" @click="emit('open-settings')">
          前往设置
        </button>
      </div>
    </section>

    <!-- 进入某个战役自己的菜单：选择原版 / 已导入的玩家版本 -->
    <SlotMenuView v-else-if="openSlot" :slot="openSlot" @back="opened = null" />

    <template v-else>
      <!-- 游戏状态 + 看板娘 -->
      <section class="hero card">
        <div class="hero__body">
          <p class="hero__eyebrow">星际争霸 II · {{ regionLabel }}</p>
          <h2 class="hero__title">{{ installation.version ?? "版本未知" }}</h2>
          <p class="hero__path" :title="installation.root">{{ installation.root }}</p>

          <div class="hero__actions">
            <button class="btn btn-primary btn-lg" type="button" @click="launch">开始游戏</button>
            <button class="btn btn-outline" type="button" @click="reveal(installation.root)">
              打开游戏目录
            </button>
          </div>

          <dl class="hero__stats">
            <div class="stat">
              <dt>官方战役</dt>
              <dd>{{ slots.length }}</dd>
            </div>
            <div class="stat">
              <dt>已导入版本</dt>
              <dd>{{ modCount }}</dd>
            </div>
            <div class="stat">
              <dt>已切换</dt>
              <dd>{{ activeCount }}</dd>
            </div>
          </dl>
        </div>

        <div class="hero__kanban" aria-hidden="true">
          <img :src="MIYIN.wink" alt="" />
        </div>
      </section>

      <!-- 导入战役包：放在外面，自动判断属于哪个战役 -->
      <div class="section-head">
        <h3 class="section-head__title">
          战役
          <span class="section-head__count">{{ slots.length }}</span>
        </h3>
        <div class="section-head__actions">
          <button class="btn btn-text" type="button" :disabled="loading" @click="refresh">
            {{ loading ? "读取中…" : "重新读取" }}
          </button>
          <button class="btn btn-primary" type="button" :disabled="importing" @click="startImport">
            {{ importing ? "核对中…" : "＋ 导入战役包" }}
          </button>
        </div>
      </div>

      <p class="section-hint">
        导入时启动器会读取包里的战役信息，自动归入对应的战役；每部战役默认是原版，
        点开可以导入并切换不同玩家制作的版本，多个版本同时保留、互不覆盖。
      </p>

      <!-- 导入预览 -->
      <section v-if="pending" class="import">
        <div class="import__head">
          <div>
            <div class="import__title">
              将导入：{{ pendingName }}
              <span v-if="pending.inspection.version" class="tag">
                v{{ pending.inspection.version }}
              </span>
            </div>
            <div class="import__meta">
              {{ pending.inspection.map_count }} 张地图 ·
              {{ formatBytes(pending.inspection.unpacked_bytes) }}
              <span v-if="pending.inspection.author"> · {{ pending.inspection.author }}</span>
            </div>
          </div>

          <div class="import__target">
            <template v-if="suggestedName">
              自动识别归属：<strong>{{ suggestedName }}</strong>
            </template>
            <template v-else>
              <span class="import__ask">包内没有标明归属，请选择要导入到哪部战役：</span>
            </template>
          </div>
        </div>

        <!-- 认不出来时的手动选择 -->
        <div v-if="needsTarget" class="targets">
          <button
            v-for="item in slots"
            :key="item.slug"
            class="target"
            :class="{ 'target--on': pending.slot === item.slug }"
            type="button"
            @click="pending.slot = item.slug"
          >
            {{ item.display_name }}
          </button>
        </div>

        <ul v-if="pending.inspection.issues.length" class="issues">
          <li
            v-for="issue in pending.inspection.issues"
            :key="issue.code"
            class="issue"
            :class="'issue--' + issue.level"
          >
            {{ issue.message }}
          </li>
        </ul>

        <div class="import__actions">
          <button class="btn btn-text" type="button" @click="pending = null">取消</button>
          <button
            class="btn btn-primary"
            type="button"
            :disabled="!pending.slot || !pending.inspection.installable"
            @click="confirmImport"
          >
            确认导入
          </button>
        </div>
      </section>

      <div class="grid">
        <SlotCard
          v-for="slot in slots"
          :key="slot.slug"
          :slot="slot"
          @open="opened = $event"
        />
      </div>
    </template>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 18px;
  max-width: var(--content-max);
  margin: 0 auto;
}

/* ---------- 顶部看板 ---------- */

.hero {
  position: relative;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  overflow: hidden;
  padding: 24px 28px;
  background:
    radial-gradient(360px 280px at 86% 74%, rgba(150, 185, 255, 0.45), transparent 72%),
    linear-gradient(118deg, #ffffff 0%, #fbfdff 58%, #f2f7ff 100%);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.hero__body {
  position: relative;
  z-index: 2;
  max-width: 520px;
}

.hero__eyebrow {
  margin: 0 0 6px;
  font-size: 12.5px;
  letter-spacing: 1.4px;
  text-transform: uppercase;
  color: var(--accent);
  font-weight: 700;
}

.hero__title {
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: var(--on-surface);
}

.hero__path {
  margin: 8px 0 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
  word-break: break-all;
}

.hero__actions {
  display: flex;
  gap: 10px;
  margin-top: 18px;
}

.hero__stats {
  display: flex;
  gap: 26px;
  margin: 18px 0 0;
  padding-top: 14px;
  border-top: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
}

.stat dt {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.stat dd {
  margin: 2px 0 0;
  font-size: 20px;
  font-weight: 700;
  color: var(--accent);
}

/* 看板娘：立绘自带白底，用径向遮罩把方形边缘化开 */
.hero__kanban {
  position: relative;
  z-index: 1;
  flex: 0 0 auto;
  width: 218px;
  margin: -24px -10px -24px 0;
  pointer-events: none;
}

.hero__kanban img {
  display: block;
  width: 100%;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 58%, transparent 78%);
  mask-image: radial-gradient(circle at 50% 52%, #000 58%, transparent 78%);
}

/* ---------- 列表头 ---------- */

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.section-head__title {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 0;
  font-size: 17px;
  font-weight: 700;
  color: #fff;
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.55);
}

.section-head__count {
  padding: 1px 9px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.22);
  font-size: 12.5px;
}

.section-head__actions {
  display: flex;
  gap: 8px;
}

.section-head__actions .btn-text {
  color: #fff;
  background: rgba(255, 255, 255, 0.16);
}

.section-head__actions .btn-text:not(:disabled):hover {
  background: rgba(255, 255, 255, 0.26);
}

.section-hint {
  margin: -6px 0 0;
  font-size: 12.5px;
  line-height: 1.7;
  color: rgba(255, 255, 255, 0.78);
  text-shadow: 0 1px 5px rgba(0, 0, 0, 0.5);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(268px, 1fr));
  gap: 18px;
}

/* ---------- 导入面板 ---------- */

.import {
  padding: 16px 18px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  box-shadow: var(--shadow-2);
}

.import__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.import__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 700;
}

.import__meta {
  margin-top: 5px;
  font-size: 12px;
  color: var(--on-surface-variant);
}

.import__target {
  font-size: 12.5px;
  color: var(--on-surface-variant);
  text-align: right;
  max-width: 320px;
}

.import__target strong {
  color: var(--accent);
}

.import__ask {
  color: var(--warning);
}

.targets {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.target {
  padding: 7px 16px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--outline);
  background: var(--surface-1);
  font-size: 13px;
  font-weight: 600;
}

.target--on {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
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
  padding: 7px 10px;
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

.tag {
  padding: 1px 7px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  color: var(--on-surface-variant);
  font-size: 11px;
  font-weight: 600;
}

.import__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}

/* ---------- 空状态 ---------- */

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 10px;
  padding: 40px 32px 46px;
}

.empty__art {
  width: 176px;
  height: 176px;
  object-fit: contain;
  margin-bottom: -6px;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 62%, transparent 82%);
  mask-image: radial-gradient(circle at 50% 52%, #000 62%, transparent 82%);
}

.empty__title {
  margin: 4px 0 0;
  font-size: 19px;
}

.empty__text {
  margin: 0;
  max-width: 560px;
  color: var(--on-surface-variant);
  font-size: 13.5px;
  line-height: 1.7;
}

.empty__text code {
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  background: var(--surface-3);
  font-size: 12.5px;
}

.empty__actions {
  display: flex;
  gap: 10px;
  margin-top: 12px;
}
</style>
