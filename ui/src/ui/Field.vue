<script setup lang="ts">
/**
 * 表单一行：标签 + 控件。
 *
 * 原先 **49 处**手写 `.field`，而且样式在三个文件里各定义了一遍
 * （`ModsView` / `SlotMenuView` / `UpdatePanel`，连类名都不一致：
 * 有的 `.field__label`、有的裸 `span`）。这里收成一处、样式跟着控件走。
 *
 * 用法：
 *   <Field label="包名"><input v-model="name" /></Field>
 *   <Field label="说明" wide><textarea … /></Field>
 */
withDefaults(
  defineProps<{
    label?: string;
    /** 占满整行（说明、备注这种长字段）。 */
    wide?: boolean;
    /** 出错时标红；配合下面的 error 文案一起用。 */
    invalid?: boolean;
    error?: string;
    hint?: string;
  }>(),
  { wide: false, invalid: false },
);
</script>

<template>
  <label class="field" :class="{ 'field--wide': wide, 'field--bad': invalid }">
    <span v-if="label" class="field__label">{{ label }}</span>
    <slot />
    <slot name="hint">
      <em v-if="error" class="field__error">{{ error }}</em>
      <em v-else-if="hint" class="field__hint">{{ hint }}</em>
    </slot>
  </label>
</template>

<style scoped>
/* 控件自带样式：页面不再各写一份 */
.field {
  display: block;
  margin-bottom: 10px;
}
.field--wide {
  grid-column: 1 / -1;
}
.field__label {
  display: block;
  margin-bottom: 4px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--on-surface-variant);
}
/* 和原来 SlotMenuView 里那份一模一样 —— 搬过来时不改观感 */
.field :deep(input),
.field :deep(select),
.field :deep(textarea) {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--outline);
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 13px;
  resize: vertical;
}
.field--bad :deep(input),
.field--bad :deep(select),
.field--bad :deep(textarea) {
  border-color: var(--danger);
}
.field__error {
  font-size: 11.5px;
  font-style: normal;
  color: var(--danger);
}
.field__hint {
  font-size: 11.5px;
  font-style: normal;
  color: var(--on-surface-variant);
}
</style>
