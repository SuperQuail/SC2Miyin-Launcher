import { useEffect } from "react";

import { api } from "../../../api/bridge";
import { renderMarkdown } from "../../../api/markdown";
import { Button, Sheet } from "../../ui";
import "./update-notice.css";

/** 发现的新版本（来自 GitHub Release）。字段与 `updateCheck.latest` 对齐。 */
export interface NoticeRelease {
  version: string;
  notes?: string | null;
  prerelease?: boolean;
  published_at: string;
}

export interface UpdateNoticeProps {
  visible: boolean;
  downloading: boolean;
  /** 用户勾了「不再提示这个版本」。 */
  muted: boolean;
  release: NoticeRelease | null;
  onDismiss: () => void;
  /** 记下「不再提示这个版本」。 */
  onMute: () => void;
  onMutedChange: (muted: boolean) => void;
  onDownload: () => void;
  onNotify: (kind: "error", text: string) => void;
}

/**
 * 启动时的更新公告（React 版，从 `components/UpdateNotice.vue` 整块迁过来）。
 *
 * 正文是 GitHub Release 的 **渲染并净化过的** Markdown；
 * 里面的链接**不在 WebView 里跳**（那会把启动器自己导航走），一律交给系统浏览器。
 *
 * 以前是 React 岛，靠 `emit("dismiss")` 这类事件跟 Vue 外壳说话；现在外壳也是
 * React，直接收回调。
 */
export function UpdateNotice({
  visible,
  downloading,
  muted,
  release,
  onDismiss,
  onMute,
  onMutedChange,
  onDownload,
  onNotify,
}: UpdateNoticeProps) {
  const body = renderMarkdown(release?.notes);

  /** 链接点击：拦下来交给系统浏览器。 */
  const onBodyClick = (event: React.MouseEvent<HTMLDivElement>) => {
    const anchor = (event.target as HTMLElement | null)?.closest("a");
    if (!anchor) return;
    event.preventDefault();
    const href = anchor.getAttribute("href") ?? "";
    if (!/^https?:\/\//i.test(href)) return;
    void api.openUrl(href).catch((error) => onNotify("error", String(error)));
  };

  // 公告开着的时候按 Esc 关掉（原版没有，但这属于"覆盖层要有退路"的既有约定）
  useEffect(() => {
    if (!visible) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onDismiss();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [visible, onDismiss]);

  if (!visible || !release) return null;

  return (
    <Sheet
      className="notice"
      onClose={onDismiss}
      actions={
        <>
          <Button onClick={muted ? onMute : onDismiss}>以后再说</Button>
          <Button variant="primary" disabled={downloading} onClick={onDownload}>
            下载并安装
          </Button>
        </>
      }
    >
      <header className="notice__head">
        <div className="sheet__badge">
          <svg className="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3v10" />
            <path d="M8 9l4 4 4-4" />
            <path d="M4 16v2a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-2" />
          </svg>
        </div>
        <div>
          <h3 className="notice__title">发现新版本 {release.version}</h3>
          <p className="notice__meta">
            {release.prerelease && <span className="tag tag--warn">预发行</span>}
            <span>{release.published_at.slice(0, 10)}</span>
          </p>
        </div>
      </header>

      {/* 公告正文：GitHub Release 的 Markdown，已渲染并净化 */}
      <div className="notice__body">
        {body ? (
          <div className="markdown" onClick={onBodyClick} dangerouslySetInnerHTML={{ __html: body }} />
        ) : (
          <p className="notice__empty">这个版本没有写更新说明。</p>
        )}
      </div>

      <label className="notice__mute">
        <input
          type="checkbox"
          checked={muted}
          onChange={(event) => onMutedChange(event.target.checked)}
        />
        <span>不再提示 {release.version}</span>
      </label>
    </Sheet>
  );
}
