<script setup lang="ts">
/**
 * 说明文档阅读器：把包里的 PDF 渲染出来。
 *
 * 为什么用 pdfjs 自己画，而不是丢个 `<iframe src="blob:...">` 让 WebView 内置的
 * PDF 查看器去渲染：嵌在 WebView2 里那条路不一定通（还会被 CSP 的 frame-src 挡），
 * 自己画虽然多几行，但行为是确定的。
 */
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { onMounted, onUnmounted, ref, shallowRef, watch } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type { DocInfo } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

// Worker 交给 Vite 打包成独立文件，运行时按 URL 加载
pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

const props = defineProps<{ slot: string; variantId: string; doc: DocInfo }>();
const emit = defineEmits<{ (event: "close"): void }>();

const { notify } = useLauncher();

const canvas = ref<HTMLCanvasElement | null>(null);
const loading = ref(true);
const page = ref(1);
const total = ref(0);
const scale = ref(1.25);
/** pdfjs 的文档对象；用 shallowRef —— 它会挂一堆内部状态，不该被 Vue 深度代理 */
const document = shallowRef<pdfjs.PDFDocumentProxy | null>(null);
/** 加载任务；销毁要调它（文档对象上只有 cleanup）。 */
const loadingTask = shallowRef<pdfjs.PDFDocumentLoadingTask | null>(null);

onMounted(load);
onUnmounted(() => {
  // 销毁要调加载任务 —— 文档对象上只有 cleanup
  void loadingTask.value?.destroy();
});

async function load(): Promise<void> {
  loading.value = true;
  try {
    const bytes = await api.readDoc(props.slot, props.variantId);
    // pdfjs 要 Uint8Array，而且它会「接管」这块内存 —— 传副本更稳妥
    const data = new Uint8Array(bytes.slice(0));
    loadingTask.value = pdfjs.getDocument({ data });
    const loaded = await loadingTask.value.promise;
    document.value = loaded;
    total.value = loaded.numPages;
    page.value = 1;
    await render();
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
  }
}

/** 画当前页。 */
async function render(): Promise<void> {
  const loaded = document.value;
  const target = canvas.value;
  if (!loaded || !target) return;

  const current = await loaded.getPage(page.value);
  const viewport = current.getViewport({ scale: scale.value });
  const context = target.getContext("2d");
  if (!context) return;

  // 按设备像素比放大画布，避免高分屏上糊成一片
  const ratio = window.devicePixelRatio || 1;
  target.width = Math.floor(viewport.width * ratio);
  target.height = Math.floor(viewport.height * ratio);
  target.style.width = viewport.width + "px";
  target.style.height = viewport.height + "px";

  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, viewport.width, viewport.height);

  await current.render({ canvas: target, canvasContext: context, viewport }).promise;
}

watch([page, scale], () => void render());

function step(delta: number): void {
  const next = page.value + delta;
  if (next >= 1 && next <= total.value) page.value = next;
}

function zoom(delta: number): void {
  const next = Math.min(3, Math.max(0.5, Number((scale.value + delta).toFixed(2))));
  scale.value = next;
}

function onKey(event: KeyboardEvent): void {
  if (event.key === "Escape") emit("close");
  if (event.key === "ArrowRight" || event.key === "PageDown") step(1);
  if (event.key === "ArrowLeft" || event.key === "PageUp") step(-1);
}

onMounted(() => window.addEventListener("keydown", onKey));
onUnmounted(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="doc" @click.self="emit('close')">
    <div class="doc__card">
      <header class="doc__head">
        <div class="doc__title" :title="doc.path">{{ doc.name }}</div>
        <div class="doc__tools">
          <span class="doc__size">{{ formatBytes(doc.size) }}</span>
          <button class="btn btn-text btn--tiny" type="button" :disabled="page <= 1" @click="step(-1)">
            上一页
          </button>
          <span class="doc__page">{{ page }} / {{ total || "…" }}</span>
          <button
            class="btn btn-text btn--tiny"
            type="button"
            :disabled="page >= total"
            @click="step(1)"
          >
            下一页
          </button>
          <button class="btn btn-text btn--tiny" type="button" @click="zoom(-0.25)">缩小</button>
          <span class="doc__zoom">{{ Math.round(scale * 100) }}%</span>
          <button class="btn btn-text btn--tiny" type="button" @click="zoom(0.25)">放大</button>
          <button class="btn btn-tonal btn--tiny" type="button" @click="emit('close')">关闭</button>
        </div>
      </header>

      <div class="doc__body">
        <p v-if="loading" class="doc__hint">正在读取说明文档…</p>
        <canvas ref="canvas" class="doc__canvas" :class="{ 'doc__canvas--hidden': loading }"></canvas>
      </div>

      <footer class="doc__foot">
        这一版是包自带的说明文档。<kbd>←</kbd> <kbd>→</kbd> 翻页，<kbd>Esc</kbd> 关闭。
      </footer>
    </div>
  </div>
</template>

<style scoped>
.doc {
  position: fixed;
  inset: 0;
  z-index: 70;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: rgba(12, 18, 32, 0.55);
  backdrop-filter: blur(3px);
}

.doc__card {
  display: flex;
  flex-direction: column;
  width: min(1000px, 94vw);
  height: min(90vh, 900px);
  border-radius: var(--radius-lg);
  background: var(--surface-1);
  box-shadow: var(--shadow-3);
  overflow: hidden;
}

.doc__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-bottom: 1px solid color-mix(in srgb, var(--outline) 50%, transparent);
}

.doc__title {
  font-size: 13.5px;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.doc__tools {
  display: flex;
  align-items: center;
  gap: 6px;
  flex: none;
}

.doc__size,
.doc__zoom,
.doc__page {
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.doc__page,
.doc__zoom {
  min-width: 52px;
  text-align: center;
}

.doc__body {
  flex: 1;
  overflow: auto;
  padding: 14px;
  background: var(--surface-3);
  display: flex;
  justify-content: center;
  align-items: flex-start;
}

.doc__hint {
  margin: 24px 0;
  font-size: 13px;
  color: var(--on-surface-variant);
}

.doc__canvas {
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-2);
  background: #fff;
}

.doc__canvas--hidden {
  display: none;
}

.doc__foot {
  padding: 7px 14px;
  border-top: 1px solid color-mix(in srgb, var(--outline) 50%, transparent);
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.btn--tiny {
  padding: 3px 10px;
  font-size: 11.5px;
}

kbd {
  padding: 0 5px;
  border-radius: 4px;
  border: 1px solid var(--outline);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 11px;
}
</style>
