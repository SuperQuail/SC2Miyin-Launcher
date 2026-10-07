<script setup lang="ts">
import { computed, ref, watch } from "vue";

import { api } from "../api/bridge";
import { formatBytes, slotArt } from "../api/art";
import type { SlotView, Variant } from "../api/types";

const props = defineProps<{
  slot: SlotView;
  /** null 表示「原版战役」这张卡片。 */
  variant: Variant | null;
  active: boolean;
  selected: boolean;
}>();

const emit = defineEmits<{ pick: []; drop: [] }>();

/** 包内自带的封面（data URL）；没有就用该战役的官方美术。 */
const customCover = ref<string | null>(null);

watch(
  () => props.variant?.id ?? "",
  async (id) => {
    customCover.value = null;
    if (!id) return;
    try {
      customCover.value = await api.variantCover(props.slot.slug, id);
    } catch {
      customCover.value = null;
    }
  },
  { immediate: true },
);

const isVanilla = computed(() => props.variant === null);

const cover = computed(() => customCover.value ?? slotArt(props.slot.slug));
const coverStyle = computed(() =>
  cover.value ? { backgroundImage: "url(" + cover.value + ")" } : undefined,
);

const title = computed(() => (isVanilla.value ? "原版战役" : props.variant?.name ?? ""));

const meta = computed(() => {
  if (!props.variant) return "暴雪官方战役，不做任何修改";
  const parts: string[] = [];
  if (props.variant.author) parts.push(props.variant.author);
  parts.push(props.variant.map_count + " 张地图");
  parts.push(formatBytes(props.variant.size_bytes));
  return parts.join(" · ");
});

/** 用的是包内封面还是官方美术，给个角标说明。 */
const usingOwnCover = computed(() => customCover.value !== null);
</script>

<template>
  <article
    class="variant"
    :class="{ 'variant--selected': selected, 'variant--active': active }"
    role="button"
    tabindex="0"
    @click="emit('pick')"
    @keydown.enter="emit('pick')"
  >
    <div class="variant__cover" :style="coverStyle">
      <div class="variant__scrim"></div>

      <span v-if="selected" class="variant__check">✓</span>
      <span v-if="isVanilla" class="variant__badge variant__badge--left">原版</span>
      <span v-else-if="variant?.version" class="variant__badge variant__badge--left">
        v{{ variant.version }}
      </span>
      <span v-if="active" class="variant__badge variant__badge--right">当前启用</span>
    </div>

    <div class="variant__body">
      <h4 class="variant__name" :title="title">{{ title }}</h4>
      <p class="variant__meta">{{ meta }}</p>
      <p v-if="usingOwnCover" class="variant__own">使用包内封面</p>
    </div>

    <button
      v-if="!isVanilla"
      class="variant__drop"
      type="button"
      title="从库中删除这个版本"
      @click.stop="emit('drop')"
    >
      删除
    </button>
  </article>
</template>

<style scoped>
.variant {
  position: relative;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  cursor: pointer;
  background: var(--surface-1);
  border: 2px solid transparent;
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-2);
  transition: transform var(--duration) var(--ease), box-shadow var(--duration) var(--ease),
    border-color var(--duration) var(--ease);
}

.variant:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-3);
}

.variant--selected {
  border-color: var(--accent);
}

.variant--active .variant__name {
  color: var(--accent);
}

.variant__cover {
  position: relative;
  aspect-ratio: 16 / 8.4;
  background-color: #131a2c;
  background-size: cover;
  background-position: center;
}

.variant__scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(10, 16, 32, 0) 46%, rgba(10, 16, 32, 0.68) 100%);
}

.variant__check {
  position: absolute;
  top: 10px;
  left: 10px;
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 700;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
}

.variant__badge {
  position: absolute;
  top: 10px;
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  font-size: 11.5px;
  font-weight: 600;
  color: #fff;
  backdrop-filter: blur(6px);
}

.variant__badge--left {
  left: 10px;
  background: rgba(20, 28, 46, 0.72);
}

.variant--selected .variant__badge--left {
  left: 42px;
}

.variant__badge--right {
  right: 10px;
  background: color-mix(in srgb, var(--accent) 90%, transparent);
}

.variant__body {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 12px 15px 14px;
}

.variant__name {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
  color: var(--on-surface);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.variant__meta {
  margin: 0;
  font-size: 12px;
  color: var(--on-surface-variant);
}

.variant__own {
  margin: 0;
  font-size: 11.5px;
  color: var(--accent);
}

.variant__drop {
  position: absolute;
  right: 10px;
  bottom: 12px;
  padding: 4px 11px;
  border-radius: var(--radius-pill);
  font-size: 12px;
  color: var(--danger);
  background: var(--danger-soft);
  opacity: 0;
  transition: opacity var(--duration) var(--ease);
}

.variant:hover .variant__drop,
.variant:focus-within .variant__drop {
  opacity: 1;
}
</style>
