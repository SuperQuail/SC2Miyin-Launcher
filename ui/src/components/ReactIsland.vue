<script setup lang="ts">
/**
 * **迁移过渡件**：把一个 React 视图挂进 Vue 外壳。
 *
 * 两个通道：
 * - `props`：外壳把数据递进去；**变了就 update**（React 自己 diff，不重建整棵树）
 * - `emit`：React 侧要说话就 emit（外壳在这边接成自己的事件）
 *
 * React 视图**不知道外壳是谁** —— 迁完之后外壳也换 React，这个文件就删。
 */
import { onBeforeUnmount, onMounted, ref, watch } from "vue";

interface Handle {
  update: (props: Record<string, unknown>) => void;
  teardown: () => void;
}

const props = defineProps<{
  mount: (
    host: HTMLElement,
    emit: (event: string, payload?: unknown) => void,
    props: Record<string, unknown>,
  ) => Handle;
  /** 递给 React 视图的数据。变了会自动 update。 */
  viewProps?: Record<string, unknown>;
}>();

const emit = defineEmits<{
  [key: string]: [payload?: unknown];
}>();

const host = ref<HTMLElement | null>(null);
let handle: Handle | null = null;

onMounted(() => {
  if (!host.value) return;
  handle = props.mount(
    host.value,
    (event, payload) => emit(event, payload),
    props.viewProps ?? {},
  );
});

// 外壳的数据变了 → 递进去（React 侧自己 diff）
watch(
  () => props.viewProps,
  (next) => handle?.update(next ?? {}),
  { deep: true },
);

onBeforeUnmount(() => {
  handle?.teardown();
  handle = null;
});
</script>

<template>
  <div ref="host"></div>
</template>
