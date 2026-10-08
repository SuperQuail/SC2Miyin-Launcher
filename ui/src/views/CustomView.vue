<script setup lang="ts">
/**
 * 自制战役页。
 *
 * 与「战役」页是**并列的顶层选项**，不是它的一个分组 —— 因为两者根本不是一类：
 * 原版战役是游戏自己驱动的，自制战役得靠编辑器打开、还要单独挂模组。
 * 混在一页里会让人以为「自制战役也就是某个战役的改版」。
 */
import { computed, onMounted, ref } from "vue";

import CustomCampaignPanel from "../components/CustomCampaignPanel.vue";
import DocViewer from "../components/DocViewer.vue";
import ImportDialog from "../components/ImportDialog.vue";
import VariantCard from "../components/VariantCard.vue";
import { useContextMenu } from "../composables/useContextMenu";
import { useLauncher } from "../composables/useLauncher";
import type { DocInfo } from "../api/types";

const emit = defineEmits<{ "open-cheats": [] }>();

const { slots, loading, refresh, bootstrap } = useLauncher();
const menu = useContextMenu();

const importer = ref<InstanceType<typeof ImportDialog> | null>(null);
/** 正在看哪一部自制战役。 */
const opened = ref<string | null>(null);
/** 正在读的说明文档。 */
const openedDoc = ref<DocInfo | null>(null);

onMounted(() => void bootstrap());

const customSlot = computed(() => slots.value.find((slot) => slot.slug === "custom") ?? null);
const variants = computed(() => customSlot.value?.variants ?? []);
const openedVariant = computed(
  () => variants.value.find((item) => item.id === opened.value) ?? null,
);

/** 导入完直接打开刚进来的那部。 */
function onImported(): void {
  const newest = variants.value[0];
  if (newest) opened.value = newest.id;
}

/** 顶部「小工具」：作弊码这类查询工具挂在这里，不单开标签页。 */
function openTools(event: MouseEvent): void {
  menu.show(
    event,
    [
      { id: "cheats", label: "作弊码查询" },
      { id: "reload", label: "重新读取库", separatorBefore: true },
    ],
    (id) => {
      if (id === "cheats") emit("open-cheats");
      if (id === "reload") void refresh();
    },
  );
}
</script>

<template>
  <div class="page">
    <!--
      看板：这一页的内容直接压在壁纸上，没有这块半透明底板的话深色文字根本读不出来。
      样式与战役页的 hero 保持一致（半透明 + 背景模糊）。
    -->
    <section class="hero">
      <div class="hero__body">
        <p class="hero__eyebrow">独立整部战役</p>
        <h2 class="hero__title">自制战役</h2>
        <p class="hero__text">
          <strong>不装进游戏目录</strong> —— 地图留在启动器里，
          用游戏编辑器打开来玩。
        </p>
      </div>

      <div class="hero__actions">
        <button class="btn btn-text" type="button" :disabled="loading" @click="openTools">
          小工具 ▾
        </button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="importer?.busy"
          @click="importer?.startImport()"
        >
          {{ importer?.busy ? "核对中…" : "＋ 导入战役包" }}
        </button>
      </div>
    </section>

    <ImportDialog ref="importer" @imported="onImported" />

    <section v-if="!variants.length" class="empty">
      <p class="empty__title">还没有自制战役</p>
      <p class="empty__text">
        点「＋ 导入战役包」选一个包，启动器会自动认出它属于自制战役。
      </p>
    </section>

    <div v-else class="grid">
      <VariantCard
        v-for="item in variants"
        :key="item.id"
        :slot="customSlot!"
        :variant="item"
        :active="customSlot?.active === item.id"
        :selected="opened === item.id"
        @pick="opened = item.id"
      />
    </div>

    <CustomCampaignPanel
      v-if="openedVariant && customSlot"
      :slot="customSlot.slug"
      :variant="openedVariant"
      @open-doc="openedDoc = $event"
    />

    <DocViewer
      v-if="openedDoc && openedVariant && customSlot"
      :slot="customSlot.slug"
      :variant-id="openedVariant.id"
      :doc="openedDoc"
      @close="openedDoc = null"
    />
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: var(--content-max);
  margin: 0 auto;
}

/* 和战役页的 hero 同一套写法，两页看起来才是一家的 */
/* 卡片网格：scoped 样式不共享，每个用到 .grid 的视图都得自己定义一份 */
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(268px, 1fr));
  gap: 18px;
}

.hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  flex-wrap: wrap;
  padding: 24px 28px;
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--surface-1) 74%, transparent);
  backdrop-filter: blur(14px) saturate(1.15);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.hero__body {
  max-width: 620px;
}

.hero__eyebrow {
  margin: 0 0 6px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--accent);
}

.hero__title {
  margin: 0 0 8px;
  font-size: 26px;
  font-weight: 700;
}

.hero__text {
  margin: 0;
  font-size: 13px;
  line-height: 1.8;
  color: var(--on-surface-variant);
}

.hero__actions {
  display: flex;
  gap: 8px;
  flex: none;
}

.empty {
  padding: 22px 24px;
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--surface-1) 74%, transparent);
  backdrop-filter: blur(14px) saturate(1.15);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.empty__title {
  margin: 0 0 5px;
  font-size: 14px;
  font-weight: 700;
}

.empty__text {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.8;
  color: var(--on-surface-variant);
}

code {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--surface-2);
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 12px;
}
</style>
