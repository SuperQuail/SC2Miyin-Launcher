<script setup lang="ts">
/**
 * 右键菜单。由 App.vue 挂一次，全局共用。
 *
 * 位置会**夹在视口内** —— 在窗口右下角右键时不至于弹到屏幕外面去。
 */
import { computed, onMounted, onUnmounted, nextTick, ref, watch } from "vue";

import { useContextMenu } from "../composables/useContextMenu";

const { visible, at, items, pick, close } = useContextMenu();

const box = ref<HTMLElement | null>(null);
/** 量到真实尺寸之后再定位置，免得先闪一下再跳。 */
const position = ref({ x: 0, y: 0 });

watch(visible, async (open) => {
  if (!open) return;
  position.value = { x: at.value.x, y: at.value.y };
  await nextTick();

  const element = box.value;
  if (!element) return;

  const margin = 8;
  const maxX = window.innerWidth - element.offsetWidth - margin;
  const maxY = window.innerHeight - element.offsetHeight - margin;
  position.value = {
    x: Math.max(margin, Math.min(at.value.x, maxX)),
    y: Math.max(margin, Math.min(at.value.y, maxY)),
  };
});

const style = computed(() => ({ left: position.value.x + "px", top: position.value.y + "px" }));

/** 点别处、滚动、按 Esc 都关掉。 */
function onPointerDown(event: MouseEvent): void {
  if (!visible.value) return;
  const element = box.value;
  if (element && event.target instanceof Node && element.contains(event.target)) return;
  close();
}

function onKey(event: KeyboardEvent): void {
  if (event.key === "Escape") close();
}

onMounted(() => {
  window.addEventListener("mousedown", onPointerDown, true);
  window.addEventListener("wheel", close, { passive: true });
  window.addEventListener("keydown", onKey);
});

onUnmounted(() => {
  window.removeEventListener("mousedown", onPointerDown, true);
  window.removeEventListener("wheel", close);
  window.removeEventListener("keydown", onKey);
});
</script>

<template>
  <div v-if="visible" ref="box" class="ctx" :style="style" @contextmenu.prevent>
    <template v-for="item in items" :key="item.id">
      <div v-if="item.separatorBefore" class="ctx__sep"></div>
      <button
        class="ctx__item"
        :class="{ 'ctx__item--danger': item.danger }"
        type="button"
        :disabled="item.disabled"
        @click="pick(item.id)"
      >
        {{ item.label }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.ctx {
  position: fixed;
  z-index: 90;
  min-width: 168px;
  padding: 5px;
  border-radius: var(--radius-sm);
  background: var(--surface-1);
  box-shadow: var(--shadow-3);
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
  /* 从鼠标位置长出来，别硬邦邦地出现 */
  animation: ctx-in 0.12s ease-out;
}

@keyframes ctx-in {
  from {
    opacity: 0;
    transform: scale(0.96) translateY(-2px);
  }
}

.ctx__item {
  display: block;
  width: 100%;
  padding: 6px 12px;
  border: none;
  border-radius: 6px;
  background: none;
  color: var(--on-surface);
  font-family: inherit;
  font-size: 12.5px;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s ease;
}

.ctx__item:hover:not(:disabled) {
  background: var(--accent-soft);
}

.ctx__item:disabled {
  color: var(--on-surface-variant);
  opacity: 0.55;
  cursor: default;
}

.ctx__item--danger {
  color: var(--danger, #c62828);
}

.ctx__item--danger:hover:not(:disabled) {
  background: color-mix(in srgb, var(--danger, #c62828) 12%, transparent);
}

.ctx__sep {
  height: 1px;
  margin: 4px 8px;
  background: color-mix(in srgb, var(--outline) 55%, transparent);
}
</style>
