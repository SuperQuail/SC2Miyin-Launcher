<script setup lang="ts">
import { computed } from "vue";

import { slotArt } from "../api/art";
import type { SlotView } from "../api/types";

const props = defineProps<{ slot: SlotView }>();
const emit = defineEmits<{ open: [slug: string] }>();

const art = computed(() => slotArt(props.slot.slug));
const coverStyle = computed(() =>
  art.value ? { backgroundImage: "url(" + art.value + ")" } : undefined,
);

const isVanilla = computed(() => props.slot.active === null);
const variantCount = computed(() => props.slot.variants.length);
const statusText = computed(() =>
  props.slot.active_name ? "已启用：" + props.slot.active_name : "原版战役",
);
</script>

<template>
  <button class="slot" type="button" @click="emit('open', slot.slug)">
    <span class="slot__cover" :style="coverStyle">
      <span class="slot__scrim"></span>
      <span class="slot__badge" :class="{ 'slot__badge--mod': !isVanilla }">
        {{ isVanilla ? "原版" : "已切换" }}
      </span>
      <span v-if="variantCount" class="slot__count">{{ variantCount }} 个版本</span>
    </span>

    <span class="slot__body">
      <span class="slot__name">{{ slot.display_name }}</span>
      <span class="slot__status" :class="{ 'slot__status--active': !isVanilla }">
        {{ statusText }}
      </span>
    </span>
  </button>
</template>

<style scoped>
.slot {
  display: flex;
  flex-direction: column;
  padding: 0;
  overflow: hidden;
  text-align: left;
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--outline) 40%, transparent);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-2);
  transition: transform var(--duration) var(--ease), box-shadow var(--duration) var(--ease),
    border-color var(--duration) var(--ease);
}

.slot:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-3);
  border-color: color-mix(in srgb, var(--accent) 45%, transparent);
}

.slot__cover {
  position: relative;
  display: block;
  aspect-ratio: 16 / 8.2;
  background-color: #131a2c;
  background-size: cover;
  background-position: center;
}

.slot__scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(10, 16, 32, 0) 42%, rgba(10, 16, 32, 0.7) 100%);
}

.slot__badge,
.slot__count {
  position: absolute;
  top: 10px;
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  font-size: 11.5px;
  font-weight: 600;
  color: #fff;
  backdrop-filter: blur(6px);
}

.slot__badge {
  left: 10px;
  background: rgba(70, 78, 96, 0.85);
}

.slot__badge--mod {
  background: color-mix(in srgb, var(--accent) 88%, transparent);
}

.slot__count {
  right: 10px;
  background: rgba(20, 28, 46, 0.7);
}

.slot__body {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 13px 16px 15px;
}

.slot__name {
  font-size: 15.5px;
  font-weight: 700;
  color: var(--on-surface);
}

.slot__status {
  font-size: 12.5px;
  color: var(--on-surface-variant);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.slot__status--active {
  color: var(--accent);
  font-weight: 600;
}
</style>
