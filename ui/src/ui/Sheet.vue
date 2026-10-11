<script setup lang="ts">
/**
 * 遮罩弹窗。
 *
 * 原先有 **16 处**手写的 `div.sheet > div.sheet__card > h3.sheet__title …`，
 * 结构一模一样，只是每个页面又抄了一遍。这里收成一处。
 *
 * 样式仍来自 base.css 的 `.sheet` 系列（AGENTS §15.3 特意要求它们放全局，
 * 别写进某个组件的 scoped —— 那样别的组件用同名类会完全没样式）。
 */
withDefaults(
  defineProps<{
    /** 标题；要自定义标题就用 #title 插槽。 */
    title?: string;
    /** 点遮罩关不关。默认关（大多数弹窗都能点外面取消）。 */
    dismissOnBackdrop?: boolean;
  }>(),
  { dismissOnBackdrop: true },
);

const emit = defineEmits<{ close: [] }>();
</script>

<template>
  <div class="sheet" @click.self="dismissOnBackdrop && emit('close')">
    <div class="sheet__card">
      <slot name="title">
        <h3 v-if="title" class="sheet__title">{{ title }}</h3>
      </slot>

      <slot />

      <div v-if="$slots.actions" class="sheet__actions">
        <slot name="actions" />
      </div>
    </div>
  </div>
</template>
