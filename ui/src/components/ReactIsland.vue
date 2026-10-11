<script setup lang="ts">
/**
 * **迁移过渡件**：把一个 React 视图挂进 Vue 外壳。
 *
 * 外壳还是 Vue，逐个视图整块换成 React —— 不在同一个文件里混写两套框架。
 * 迁完之后外壳也换成 React，这个文件连同 `react/island.tsx` 一起删掉。
 *
 * `mount` 由 React 侧提供：`(host, emit) => 卸载函数`；
 * React 视图要跟外壳说话就用 `emit`（外壳在这边接成自己的事件）。
 */
import { onBeforeUnmount, onMounted, ref } from "vue";

const props = defineProps<{
  mount: (host: HTMLElement, emit: (event: string, payload?: unknown) => void) => () => void;
}>();

const emit = defineEmits<{
  close: [];
  notify: [payload: { kind: "success" | "error"; text: string }];
}>();

const host = ref<HTMLElement | null>(null);
let teardown: (() => void) | null = null;

onMounted(() => {
  if (!host.value) return;
  teardown = props.mount(host.value, (event, payload) => {
    if (event === "close") emit("close");
    if (event === "notify") emit("notify", payload as { kind: "success" | "error"; text: string });
  });
});

onBeforeUnmount(() => {
  teardown?.();
  teardown = null;
});
</script>

<template>
  <div ref="host"></div>
</template>
