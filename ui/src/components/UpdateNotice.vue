<script setup lang="ts">
/**
 * 启动时的更新公告：发现新版本就弹出来，正文是 GitHub Release 的**渲染后的 Markdown**。
 *
 * 链接不会在 WebView 里跳转（那会把启动器自己导航走），
 * 一律拦下来交给系统浏览器 —— 见 `openLink()`。
 */
import { computed } from "vue";

import { renderMarkdown } from "../api/markdown";
import { api } from "../api/bridge";
import { errorText, useLauncher } from "../composables/useLauncher";

const {
  notify,
  updateCheck,
  updateDownloading,
  updateNoticeVisible,
  updateNoticeMuted,
  dismissUpdateNotice,
  muteThisVersion,
  downloadFromNotice,
} = useLauncher();

const release = computed(() => updateCheck.value?.latest ?? null);

/** 渲染好的公告正文。 */
const body = computed(() => renderMarkdown(release.value?.notes));

/** 点击正文里的链接：不在 WebView 里跳，交给系统浏览器。 */
function onBodyClick(event: MouseEvent): void {
  const target = event.target as HTMLElement | null;
  const anchor = target?.closest("a");
  if (!anchor) return;

  event.preventDefault();
  const href = anchor.getAttribute("href") ?? "";
  if (!/^https?:\/\//i.test(href)) return;
  void api.openUrl(href).catch((error) => notify("error", errorText(error)));
}

/** 关掉公告，并在用户勾了「不再提示」时记下来。 */
function close(): void {
  if (updateNoticeMuted.value) muteThisVersion();
  else dismissUpdateNotice();
}
</script>

<template>
  <div v-if="updateNoticeVisible && release" class="sheet" @click.self="dismissUpdateNotice">
    <div class="sheet__card notice">
      <header class="notice__head">
        <div class="sheet__badge">
          <svg class="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3v10" />
            <path d="M8 9l4 4 4-4" />
            <path d="M4 16v2a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-2" />
          </svg>
        </div>
        <div>
          <h3 class="notice__title">发现新版本 {{ release.version }}</h3>
          <p class="notice__meta">
            <span v-if="release.prerelease" class="tag tag--warn">预发行</span>
            <span>{{ release.published_at.slice(0, 10) }}</span>
          </p>
        </div>
      </header>

      <!-- 公告正文：GitHub Release 的 Markdown，已渲染并净化 -->
      <div class="notice__body">
        <div v-if="body" class="markdown" @click="onBodyClick" v-html="body"></div>
        <p v-else class="notice__empty">这个版本没有写更新说明。</p>
      </div>

      <label class="notice__mute">
        <input v-model="updateNoticeMuted" type="checkbox" />
        <span>不再提示 {{ release.version }}</span>
      </label>

      <div class="sheet__actions">
        <button class="btn btn-text" type="button" @click="close">以后再说</button>
        <button
          class="btn btn-primary"
          type="button"
          :disabled="updateDownloading"
          @click="downloadFromNotice"
        >
          下载并安装
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.notice {
  display: flex;
  flex-direction: column;
  width: min(620px, 94vw);
  /* 公告可能很长，整个卡片别超出屏幕 */
  max-height: min(80vh, 720px);
  padding: 20px 22px 16px;
}

.notice__head {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  flex: none;
}

.notice__title {
  margin: 0 0 4px;
  font-size: 17px;
}

.notice__meta {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 12px;
  color: var(--on-surface-variant);
}

.notice__body {
  flex: 1 1 auto;
  min-height: 90px;
  margin: 12px 0;
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  overflow-y: auto;
}

.notice__empty {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.notice__mute {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: none;
  font-size: 12.5px;
  color: var(--on-surface-variant);
  cursor: pointer;
}

/* ---------- 渲染出来的 Markdown ---------- */

.markdown {
  font-size: 13px;
  line-height: 1.75;
  color: var(--on-surface);
  word-wrap: break-word;
}

.markdown :deep(h1),
.markdown :deep(h2),
.markdown :deep(h3),
.markdown :deep(h4) {
  margin: 14px 0 6px;
  font-size: 14.5px;
  font-weight: 700;
  line-height: 1.5;
}

.markdown :deep(h1:first-child),
.markdown :deep(h2:first-child),
.markdown :deep(h3:first-child) {
  margin-top: 0;
}

.markdown :deep(p) {
  margin: 0 0 9px;
}

.markdown :deep(ul),
.markdown :deep(ol) {
  margin: 0 0 9px;
  padding-left: 20px;
}

.markdown :deep(li) {
  margin-bottom: 3px;
}

.markdown :deep(li > p) {
  margin-bottom: 3px;
}

.markdown :deep(a) {
  color: var(--accent);
  text-decoration: none;
  border-bottom: 1px solid color-mix(in srgb, var(--accent) 40%, transparent);
  cursor: pointer;
}

.markdown :deep(a:hover) {
  border-bottom-color: var(--accent);
}

.markdown :deep(code) {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--surface-3);
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 12px;
}

.markdown :deep(pre) {
  margin: 0 0 10px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  background: #10141c;
  overflow-x: auto;
}

.markdown :deep(pre code) {
  padding: 0;
  background: none;
  color: #c8d2e0;
  font-size: 11.5px;
  line-height: 1.6;
}

.markdown :deep(blockquote) {
  margin: 0 0 10px;
  padding: 2px 0 2px 12px;
  border-left: 3px solid color-mix(in srgb, var(--accent) 45%, transparent);
  color: var(--on-surface-variant);
}

.markdown :deep(hr) {
  margin: 14px 0;
  border: none;
  border-top: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
}

.markdown :deep(table) {
  width: 100%;
  margin: 0 0 10px;
  border-collapse: collapse;
  font-size: 12.5px;
}

.markdown :deep(th),
.markdown :deep(td) {
  padding: 5px 9px;
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
  text-align: left;
}

.markdown :deep(th) {
  background: var(--surface-3);
  font-weight: 700;
}

.markdown :deep(img) {
  max-width: 100%;
  border-radius: var(--radius-sm);
}

.markdown :deep(del) {
  color: var(--on-surface-variant);
}

.markdown :deep(details) {
  margin-bottom: 9px;
}

.markdown :deep(summary) {
  cursor: pointer;
  font-weight: 600;
}

.tag--warn {
  padding: 1px 7px;
  border-radius: var(--radius-pill);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 11px;
  font-weight: 600;
}
</style>
