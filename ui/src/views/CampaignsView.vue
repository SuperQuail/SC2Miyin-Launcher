<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { MIYIN } from "../api/art";
import ImportDialog from "../components/ImportDialog.vue";
import SlotCard from "../components/SlotCard.vue";
import { useContextMenu } from "../composables/useContextMenu";
import { useLauncher } from "../composables/useLauncher";
import SlotMenuView from "./SlotMenuView.vue";

const emit = defineEmits<{ "open-settings": []; "open-cheats": [] }>();

/** 页面空白处的右键菜单。 */
function showPageMenu(event: MouseEvent): void {
  menu.show(
    event,
    [
      { id: "import", label: "导入战役包…" },
      { id: "reload", label: "重新读取战役库" },
      { id: "cheats", label: "作弊码查询", separatorBefore: true },
      { id: "settings", label: "设置" },
    ],
    (id) => {
      if (id === "import") void importer.value?.startImport();
      if (id === "reload") void refresh();
      if (id === "cheats") emit("open-cheats");
      if (id === "settings") emit("open-settings");
    },
  );
}

/** 顶部「小工具」：作弊码这类查询工具挂这里，不单开标签页。 */
function openTools(event: MouseEvent): void {
  menu.show(
    event,
    [
      { id: "cheats", label: "作弊码查询" },
      { id: "reload", label: "重新读取战役库", separatorBefore: true },
    ],
    (id) => {
      if (id === "cheats") emit("open-cheats");
      if (id === "reload") void refresh();
    },
  );
}

const {
  installation,
  slots,
  loading,
  launch,
  reveal,
  refresh,
  chooseGameDirectory,
  bootstrap,
} = useLauncher();

/** 当前打开的战役槽位。 */
const opened = ref<string | null>(null);

onMounted(() => {
  void bootstrap();

  // 支持 #slot=wol 直接打开某个战役（调试与截图用）
  const match = /slot=([a-z]+)/.exec(typeof window === "undefined" ? "" : window.location.hash);
  if (match) opened.value = match[1];
});

const menu = useContextMenu();

/** 当前打开的那个战役；点了卡片才有值。 */
const openSlot = computed(() => slots.value.find((slot) => slot.slug === opened.value) ?? null);

/** 区域标签：国服 / 其他。 */
const regionLabel = computed(() => {
  const branch = installation.value?.branch;
  if (!branch) return "未知区域";
  return branch.toLowerCase() === "cn" ? "国服" : branch.toUpperCase();
});

const modCount = computed(() =>
  slots.value.reduce((total, slot) => total + slot.variants.length, 0),
);
const activeCount = computed(() => slots.value.filter((slot) => slot.active !== null).length);

/** 这个页面只列四大**原版**战役；自制战役有自己的标签页。 */
const officialSlots = computed(() => slots.value.filter((slot) => slot.slug !== "custom"));

/**
 * 导入面板抽成了独立组件（两个页面都要用），这里只留一个引用。
 */
const importer = ref<InstanceType<typeof ImportDialog> | null>(null);

/** 导入完成后直接进那个战役的菜单页。 */
function onImported(slot: string): void {
  opened.value = slot;
}

</script>

<template>
  <div class="page" @contextmenu.self="showPageMenu">
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
          <button class="btn btn-text" type="button" @click="openTools">小工具 ▾</button>
          <button class="btn btn-primary" type="button" :disabled="importer?.busy" @click="importer?.startImport()">
            {{ importer?.busy ? "核对中…" : "＋ 导入战役包" }}
          </button>
        </div>
      </div>

      <p class="section-hint">
        导入时启动器会读取包里的战役信息，自动归入对应的战役；每部战役默认是原版，
        点开可以导入并切换不同玩家制作的版本，多个版本同时保留、互不覆盖。
      </p>

      <ImportDialog ref="importer" entry="campaign" @imported="onImported" />

      <div class="grid">
          <SlotCard
            v-for="slot in officialSlots"
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
  /*
   * 半透明 + 背景模糊：底下的壁纸能透出来一点，整块不至于像贴上去的白色板子。
   * 颜色仍然走 --surface-*，浅色/深色主题都不会翻车。
   */
  background:
    radial-gradient(
      360px 280px at 86% 74%,
      color-mix(in srgb, var(--accent) 22%, transparent),
      transparent 72%
    ),
    color-mix(in srgb, var(--surface-1) 74%, transparent);
  backdrop-filter: blur(14px) saturate(1.15);
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

.tag--patch {
  background: var(--warning-soft);
  color: var(--warning);
}

.import__source {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--accent);
}

.import__evidence {
  margin-top: 3px;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.import__note {
  margin: 12px 0 0;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 12.5px;
  line-height: 1.6;
}

.import__field {
  margin-top: 14px;
}

.import__label {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--on-surface-variant);
}

.import__field .targets {
  margin-top: 8px;
}

.check {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  font-size: 12.5px;
  line-height: 1.7;
  cursor: pointer;
}

.check input {
  margin-top: 3px;
}

.check em {
  display: block;
  font-style: normal;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

/* 强改归属的警告条 */
.warn {
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  border-left: 3px solid var(--warning);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 12.5px;
  line-height: 1.7;
}

.sheet__badge--warn {
  background: var(--warning-soft);
}

.sheet__badge--warn .sheet__icon {
  stroke: var(--warning);
}

/* 原版战役 / 自制战役 两组 */
.group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 18px;
}

.group__title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
}

.group__hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.conflict {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  background: var(--accent-soft);
}

.conflict__text {
  font-size: 12.5px;
  color: var(--on-surface);
  line-height: 1.6;
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
