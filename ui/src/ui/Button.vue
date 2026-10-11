<script setup lang="ts">
/**
 * 按钮。
 *
 * 全项目原先有 100 多处手写的 `<button class="btn btn-xxx">`，样式来自 base.css ——
 * 所以这一层**只收结构**：把"哪个变体、多大、禁用没"变成 props，
 * 样式仍然由 base.css 提供（观感零变化）。
 *
 * 之后要换视觉语言（比如迁 React 时换成自家控件），只改这一处。
 */
withDefaults(
  defineProps<{
    /** 对应 base.css 的 .btn-* 那一组。默认 text（界面里用得最多的一种）。 */
    variant?: "primary" | "tonal" | "text" | "outline" | "danger";
    size?: "md" | "lg";
    disabled?: boolean;
    title?: string;
    type?: "button" | "submit";
  }>(),
  { variant: "text", size: "md", disabled: false, type: "button" },
);

defineEmits<{ click: [MouseEvent] }>();
</script>

<template>
  <button
    :class="['btn', 'btn-' + variant, size === 'lg' ? 'btn-lg' : '']"
    :type="type"
    :disabled="disabled"
    :title="title"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>
