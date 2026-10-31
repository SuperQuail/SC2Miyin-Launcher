import ProgressBar from "@jetbrains/ring-ui-built/components/progress-bar/progress-bar";
import ScrollableSection from "@jetbrains/ring-ui-built/components/scrollable-section/scrollable-section";

import type { DevEntry, ExportProgress, FilePreview } from "../../api";
import { formatBytes } from "../../lib/format";
import { DocTabs } from "./DocTabs";

export type EditorPaneProps = {
  opened: DevEntry[];
  active: string | null;
  preview: FilePreview | null;
  status: string;
  exportProgress: ExportProgress | null;
  extras: string[];
  onOpen: (entry: DevEntry) => void;
  onClose: (entry: DevEntry) => void;
};

/** 中间：文档标签 + 预览（文本 / 二进制 / 图片）+ 底部状态栏。 */
export function EditorPane({
  opened,
  active,
  preview,
  status,
  exportProgress,
  extras,
  onOpen,
  onClose,
}: EditorPaneProps) {
  return (
    <section className="pane pane--editor">
      <DocTabs opened={opened} active={active} onOpen={onOpen} onClose={onClose} />

      <div className="pbody pbody--flat">
        {!preview && (
          /* 空状态用 ScrollableSection 兜住：窗口矮的时候这两行字得能滚到 */
          <ScrollableSection className="fill">
            <div className="unsupported">
              <p className="utitle">左边点一个文件</p>
              <p className="utext">
                文本直接按行打开；图片直接显示；<br />
                二进制（地图、模组这类）会告诉你打不开、可以怎么打开。
              </p>
            </div>
          </ScrollableSection>
        )}

        {preview?.kind === "text" && (
          <div className="code">
            {preview.text.split("\n").map((line, index) => (
              <div className="cline" key={index}>
                <span className="cno">{index + 1}</span>
                <span className="ctext">{line}</span>
              </div>
            ))}
            {preview.truncated && <p className="empty">文件太大，只显示了前面一段。</p>}
          </div>
        )}

        {preview?.kind === "binary" && (
          <ScrollableSection className="fill">
            <div className="unsupported">
              <svg className="i" width="44" height="44" viewBox="0 0 16 16">
                <path d="M8 3.5h6v9H8zM3 6.5h4v6H3z" />
              </svg>
              <p className="utitle">二进制文件</p>
              <p className="utext">
                这个文件不是文本，编辑器没法按行显示它。<br />
                地图和模组是 MPQ 归档，要改内容请在游戏编辑器里打开。
              </p>
              <p className="uhint">大小 {formatBytes(preview.bytes)}</p>
            </div>
          </ScrollableSection>
        )}

        {preview?.kind === "image" && (
          <div className="viewer">
            <div className="stage">
              <div className="shot">{opened.find((item) => item.abs === active)?.name ?? "图片"}</div>
            </div>
            <p className="vbar">
              <span>{formatBytes(preview.bytes)}</span>
              <span className="vzoom">适应窗口 · 100% · － ＋</span>
            </p>
          </div>
        )}
      </div>

      <footer className="status">
        <span>{status}</span>
        {exportProgress && (
          <span className="progress" title="正在打包">
            <ProgressBar
              className="pxbar"
              value={exportProgress.done}
              max={exportProgress.total || 1}
              label="导出进度"
            />
            {exportProgress.done} / {exportProgress.total}
          </span>
        )}
        <span className="sspacer" />
        <span>{extras.length > 0 ? extras.length + " 个自定义目录" : ""}</span>
      </footer>
    </section>
  );
}
