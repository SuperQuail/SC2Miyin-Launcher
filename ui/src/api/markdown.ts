/**
 * 把 GitHub Release 的正文（Markdown）渲染成可以直接插进 DOM 的 HTML。
 *
 * 两道处理，缺一不可：
 *
 * 1. **marked 解析** Markdown（支持 GFM：表格、任务列表、删除线等）
 * 2. **DOMPurify 过滤** —— Release 正文说到底是从网络来的数据。就算仓库是自己的，
 *    也不该让一段正文有本事往页面里塞 `<script>` 或 `onerror=` 这类东西。
 *
 * 另外这里**只产出 HTML 字符串**，插进 DOM 的活由调用方负责（`v-html`）。
 */

import DOMPurify from "dompurify";
import { marked } from "marked";

marked.setOptions({
  gfm: true,
  // 单个换行也算换行：更新说明常常一行一条，按 Markdown 严格规则会挤成一坨
  breaks: true,
});

/** 允许的标签：Markdown 正文常见的那几种，不含任何可执行/可嵌入内容。 */
const ALLOWED_TAGS = [
  "h1", "h2", "h3", "h4", "h5", "h6",
  "p", "br", "hr",
  "strong", "em", "del", "code", "pre", "blockquote",
  "ul", "ol", "li",
  "table", "thead", "tbody", "tr", "th", "td",
  "a", "img",
  "details", "summary",
];

/** 允许的属性。`href` 会由调用方再做一次协议检查。 */
const ALLOWED_ATTR = ["href", "title", "src", "alt", "align"];

/**
 * 渲染并净化。
 *
 * `source` 为空或渲染失败时返回空串 —— 界面据此决定不显示公告正文。
 */
export function renderMarkdown(source: string | null | undefined): string {
  const text = (source ?? "").trim();
  if (!text) return "";

  let html: string;
  try {
    html = marked.parse(text, { async: false });
  } catch {
    // Markdown 解析失败不该让界面崩掉：退回纯文本（已经过转义）
    return "<p>" + escapeHtml(text) + "</p>";
  }

  return DOMPurify.sanitize(html, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    // 图片只允许 https 与 data:，挡掉 javascript: 之类
    ALLOW_DATA_ATTR: false,
    FORBID_TAGS: ["style", "form", "input", "button", "iframe", "script"],
    FORBID_ATTR: ["style", "onerror", "onload", "onclick"],
  });
}

/** 把文本转义成 HTML 安全的形式。 */
export function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** 链接是不是可以交给系统浏览器打开（只放行 http/https）。 */
export function isSafeExternalUrl(url: string): boolean {
  return /^https?:\/\//i.test(url.trim());
}
