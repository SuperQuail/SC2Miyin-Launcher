import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { useCallback, useEffect, useRef, useState } from "react";

import { formatBytes } from "../../api/art";
import { api } from "../../api/bridge";
import { errorText } from "../../api/errors";
import type { DocInfo } from "../../api/types";
import { Button } from "../ui";
import "./doc-viewer.css";

// Worker 交给 Vite 打包成独立文件，运行时按 URL 加载
pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

/**
 * 说明文档阅读器（React 版，从 `components/DocViewer.vue` 整块迁过来）：
 * 把包里的 PDF 渲染出来。
 *
 * 为什么用 pdfjs 自己画，而不是丢个 `<iframe src="blob:...">` 让 WebView 内置的
 * PDF 查看器去渲染：嵌在 WebView2 里那条路不一定通（还会被 CSP 的 frame-src 挡），
 * 自己画虽然多几行，但行为是确定的。
 */
export function DocViewer({
  slot,
  variantId,
  doc,
  onClose,
  notify,
}: {
  slot: string;
  variantId: string;
  doc: DocInfo;
  onClose: () => void;
  notify: (kind: "error", text: string) => void;
}) {
  const canvas = useRef<HTMLCanvasElement | null>(null);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [total, setTotal] = useState(0);
  const [scale, setScale] = useState(1.25);
  /** pdfjs 的文档对象；它是外部对象，不该被 React 接管，放 ref。 */
  const pdfDoc = useRef<pdfjs.PDFDocumentProxy | null>(null);
  /** 加载任务；销毁要调它（文档对象上只有 cleanup）。 */
  const loadingTask = useRef<pdfjs.PDFDocumentLoadingTask | null>(null);

  /** 画当前页。 */
  const render = useCallback(async (targetPage: number, targetScale: number) => {
    const loaded = pdfDoc.current;
    const element = canvas.current;
    if (!loaded || !element) return;

    const current = await loaded.getPage(targetPage);
    const viewport = current.getViewport({ scale: targetScale });
    const context = element.getContext("2d");
    if (!context) return;

    // 按设备像素比放大画布，避免高分屏上糊成一片
    const ratio = window.devicePixelRatio || 1;
    element.width = Math.floor(viewport.width * ratio);
    element.height = Math.floor(viewport.height * ratio);
    element.style.width = viewport.width + "px";
    element.style.height = viewport.height + "px";

    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, viewport.width, viewport.height);

    await current.render({ canvas: element, canvasContext: context, viewport }).promise;
  }, []);

  // 载入文档
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    void (async () => {
      try {
        const bytes = await api.readDoc(slot, variantId);
        // pdfjs 要 Uint8Array，而且它会「接管」这块内存 —— 传副本更稳妥
        const data = new Uint8Array(bytes.slice(0));
        const task = pdfjs.getDocument({ data });
        loadingTask.current = task;
        const loaded = await task.promise;
        if (cancelled) return;
        pdfDoc.current = loaded;
        setTotal(loaded.numPages);
        setPage(1);
        await render(1, 1.25);
      } catch (error) {
        notify("error", errorText(error));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
      // 销毁要调加载任务 —— 文档对象上只有 cleanup
      void loadingTask.current?.destroy();
    };
  }, [slot, variantId, render, notify]);

  // 翻页 / 缩放后重画
  useEffect(() => {
    if (!loading) void render(page, scale);
    // loading 变 false 时上面那次已经画过首页，不重复画
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, scale, render]);

  function step(delta: number): void {
    const next = page + delta;
    if (next >= 1 && next <= total) setPage(next);
  }

  function zoom(delta: number): void {
    setScale(Math.min(3, Math.max(0.5, Number((scale + delta).toFixed(2)))));
  }

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
      if (event.key === "ArrowRight" || event.key === "PageDown") step(1);
      if (event.key === "ArrowLeft" || event.key === "PageUp") step(-1);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [page, total, onClose]);

  return (
    <div className="doc" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="doc__card">
        <header className="doc__head">
          <div className="doc__title" title={doc.path}>
            {doc.name}
          </div>
          <div className="doc__tools">
            <span className="doc__size">{formatBytes(doc.size)}</span>
            <Button disabled={page <= 1} onClick={() => step(-1)}>
              <span className="btn--tiny">上一页</span>
            </Button>
            <span className="doc__page">
              {page} / {total || "…"}
            </span>
            <Button disabled={page >= total} onClick={() => step(1)}>
              <span className="btn--tiny">下一页</span>
            </Button>
            <Button onClick={() => zoom(-0.25)}>
              <span className="btn--tiny">缩小</span>
            </Button>
            <span className="doc__zoom">{Math.round(scale * 100)}%</span>
            <Button onClick={() => zoom(0.25)}>
              <span className="btn--tiny">放大</span>
            </Button>
            <Button variant="tonal" onClick={onClose}>
              <span className="btn--tiny">关闭</span>
            </Button>
          </div>
        </header>

        <div className="doc__body">
          {loading && <p className="doc__hint">正在读取说明文档…</p>}
          <canvas ref={canvas} className={"doc__canvas" + (loading ? " doc__canvas--hidden" : "")} />
        </div>

        <footer className="doc__foot">
          <kbd>←</kbd> <kbd>→</kbd> 翻页 · <kbd>Esc</kbd> 关闭
        </footer>
      </div>
    </div>
  );
}
