<script setup lang="ts">
import { computed, ref } from "vue";

import { slotArt } from "../api/art";
import type { SlotView, Variant } from "../api/types";
import { useLauncher } from "../composables/useLauncher";
import VariantCard from "../components/VariantCard.vue";

const props = defineProps<{ slot: SlotView }>();
const emit = defineEmits<{ back: [] }>();

const { activate, removeVariant, launch, busy } = useLauncher();

/** 当前选中的版本；null 表示原版战役。 */
const selected = ref<string | null>(props.slot.active);

const bannerStyle = computed(() => {
  const art = slotArt(props.slot.slug);
  return art ? { backgroundImage: "url(" + art + ")" } : undefined;
});

const dirty = computed(() => selected.value !== props.slot.active);

/** 启用当前选中的版本。 */
async function apply(): Promise<void> {
  const ok = await activate(props.slot.slug, selected.value);
  if (ok) emit("back");
}

/** 启用并立刻启动游戏。 */
async function applyAndPlay(): Promise<void> {
  const ok = await activate(props.slot.slug, selected.value);
  if (ok) await launch();
}

/** 删除一个已导入的版本。 */
async function drop(variant: Variant): Promise<void> {
  const ok = await removeVariant(props.slot.slug, variant.id);
  if (ok && selected.value === variant.id) selected.value = null;
}
</script>

<template>
  <div class="menu">
    <button class="back" type="button" @click="emit('back')">← 返回战役列表</button>

    <!-- 头条：用该战役的官方美术做背景 -->
    <header class="banner" :style="bannerStyle">
      <div class="banner__scrim"></div>
      <div class="banner__body">
        <h2 class="banner__title">{{ slot.display_name }}</h2>
        <p class="banner__sub">
          选择要游玩的版本 —— 原版战役，或导入的玩家版本
        </p>
      </div>
      <button class="btn btn-primary banner__play" type="button" @click="launch">
        开始游戏
      </button>
    </header>

    <!-- 版本卡片：原版永远排第一 -->
    <div class="section-head">
      <h3 class="section-head__title">
        可选版本
        <span class="section-head__count">{{ slot.variants.length + 1 }}</span>
      </h3>
      <span class="section-head__hint">导入新版本请回到战役列表页</span>
    </div>

    <div class="grid">
      <VariantCard
        :slot="slot"
        :variant="null"
        :active="slot.active === null"
        :selected="selected === null"
        @pick="selected = null"
      />
      <VariantCard
        v-for="item in slot.variants"
        :key="item.id"
        :slot="slot"
        :variant="item"
        :active="slot.active === item.id"
        :selected="selected === item.id"
        @pick="selected = item.id"
        @drop="drop(item)"
      />
    </div>

    <p v-if="!slot.variants.length" class="hint">
      还没有导入任何玩家版本。回到战役列表页点「导入战役包」，
      启动器会自动判断它属于哪部战役。
    </p>

    <p v-if="slot.notice" class="hint">{{ slot.notice }}</p>

    <!-- 底部操作条 -->
    <footer class="actions">
      <span class="actions__label">
        当前选择：<strong>{{ selected === null ? "原版战役" : selected }}</strong>
      </span>
      <span class="actions__spacer"></span>
      <button class="btn btn-outline" type="button" :disabled="busy" @click="applyAndPlay">
        启用并开始游戏
      </button>
      <button class="btn btn-primary" type="button" :disabled="!dirty || busy" @click="apply">
        {{ dirty ? "启用这个版本" : "已是当前版本" }}
      </button>
    </footer>
  </div>
</template>

<style scoped>
/* 注意：本组件的根是 flex 项，不能再写 margin: 0 auto ——
   自动外边距会吞掉交叉轴的 stretch，整页会缩成一条。 */
.menu {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  padding-bottom: 4px;
}

.back {
  align-self: flex-start;
  padding: 6px 14px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.16);
  color: #fff;
  font-size: 13px;
  font-weight: 600;
}

.back:hover {
  background: rgba(255, 255, 255, 0.26);
}

/* ---------- 头条 ---------- */

.banner {
  position: relative;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
  min-height: 168px;
  padding: 22px 26px;
  border-radius: var(--radius-lg);
  background-color: #101a2e;
  background-size: cover;
  background-position: center 32%;
  box-shadow: var(--shadow-2);
  overflow: hidden;
}

.banner__scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    100deg,
    rgba(8, 14, 28, 0.92) 8%,
    rgba(8, 14, 28, 0.55) 52%,
    rgba(8, 14, 28, 0.2) 100%
  );
}

.banner__body {
  position: relative;
  z-index: 1;
}

.banner__title {
  margin: 0;
  font-size: 28px;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.4px;
  text-shadow: 0 2px 10px rgba(0, 0, 0, 0.55);
}

.banner__sub {
  margin: 5px 0 0;
  font-size: 13px;
  color: rgba(255, 255, 255, 0.82);
}

.banner__play {
  position: relative;
  z-index: 1;
}

/* ---------- 列表 ---------- */

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

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(252px, 1fr));
  gap: 16px;
}

.hint {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.75;
  color: rgba(255, 255, 255, 0.8);
  text-shadow: 0 1px 5px rgba(0, 0, 0, 0.5);
}

.section-head__hint {
  font-size: 12.5px;
  color: rgba(255, 255, 255, 0.75);
  text-shadow: 0 1px 5px rgba(0, 0, 0, 0.5);
}

.tag {
  padding: 1px 7px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  color: var(--on-surface-variant);
  font-size: 11px;
  font-weight: 600;
}

/* ---------- 底部操作条 ---------- */

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--surface-1) 90%, transparent);
  border: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
  box-shadow: var(--shadow-2);
}

.actions__label {
  font-size: 13px;
  color: var(--on-surface-variant);
}

.actions__label strong {
  color: var(--on-surface);
}

.actions__spacer {
  flex: 1;
}
</style>
