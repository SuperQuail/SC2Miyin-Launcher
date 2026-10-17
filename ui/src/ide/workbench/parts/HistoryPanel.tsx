import Button from "@jetbrains/ring-ui-built/components/button/button";
import Input, { Size } from "@jetbrains/ring-ui-built/components/input/input";
import ScrollableSection from "@jetbrains/ring-ui-built/components/scrollable-section/scrollable-section";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";

import type { CommitRecord, HistoryDiff } from "../../api";
import { formatBytes, when } from "../../lib/format";

const PALETTE = ["#629755", "#B3893A", "#7A3E9D", "#4A86E8"];

/** IJ 的图形道：HEAD 画环，其余实心点。 */
function Graph({ color, head, last }: { color: string; head?: boolean; last?: boolean }) {
  return (
    <svg className="graph" width="38" height="26" viewBox="0 0 38 26">
      <line x1="11" y1="0" x2="11" y2={last ? 13 : 26} stroke={color} strokeWidth="2" />
      {head ? (
        <>
          <circle cx="11" cy="13" r="5.5" fill="var(--surface-1)" stroke={color} strokeWidth="2.5" />
          <circle cx="11" cy="13" r="2" fill={color} />
        </>
      ) : (
        <circle cx="11" cy="13" r="4.5" fill={color} />
      )}
    </svg>
  );
}

export type HistoryPanelProps = {
  pkg: string;
  commits: CommitRecord[];
  picked: string[];
  pickedBytes: number;
  message: string;
  onMessage: (value: string) => void;
  onSubmit: () => void;
  /** 提交是 IPC 往返 —— 按钮转 loader */
  submitting: boolean;
  logPicked: number | null;
  pendingDiff: HistoryDiff | null;
  onPickWorktree: () => void;
  onPick: (item: CommitRecord, previous: CommitRecord | undefined) => void;
  onContextMenu: (item: CommitRecord, event: { clientX: number; clientY: number }) => void;
  onApply: (item: CommitRecord) => void;
};

/**
 * 底部「日志」（版本管理）面板。
 *
 * 现在被 `SHOW_VERSION_CONTROL` 关着 —— 这套只记路径 + 大小 + 修改时间，
 * 给不了"内容回退"这种真正有用的能力，摆在那儿只会误导人。
 * 代码留着，等接上 SC2Diff 再打开。
 */
export function HistoryPanel({
  pkg,
  commits,
  picked,
  pickedBytes,
  message,
  onMessage,
  onSubmit,
  submitting,
  logPicked,
  pendingDiff,
  onPickWorktree,
  onPick,
  onContextMenu,
  onApply,
}: HistoryPanelProps) {
  return (
    <section className="pane pane--log">
      <header className="pbar">
        <span className="ptitle">日志</span>
        <span className="psub">
          {pkg.trim() ? (commits.length ? commits.length + " 次提交" : "还没有提交") : "先起个包名"}
        </span>
        <span className="ptools">
          <Input
            className="commitmsg"
            size={Size.S}
            value={message}
            onChange={(event) => onMessage(event.target.value)}
            placeholder="这次改了什么"
          />
          <Button
            loader={submitting}
            disabled={!pkg.trim() || picked.length === 0 || !message.trim()}
            onClick={onSubmit}
          >
            {submitting ? "提交中…" : "提交"}
          </Button>
        </span>
      </header>
      {/* 日志区两边都能滚动：ScrollableSection 在滚出边界的那一侧加阴影线 */}
      <ScrollableSection className="pbody pbody--flat logwrap">
        <ul className="tree">
          <li>
            <Button className="trow" onClick={onPickWorktree}>
              <Graph color={PALETTE[0]} head />
              <span className="tver">未提交</span>
              <span className="tmsg">工作区里勾选的 {picked.length} 个文件</span>
              <span className="refs">{picked.length > 0 ? <Tag>未提交</Tag> : null}</span>
              <span className="twhen">现在</span>
            </Button>
          </li>
          {[...commits].reverse().map((item, index) => (
            <li
              key={item.id}
              className={"trow" + (logPicked === item.id ? " is-picked" : "")}
              onContextMenu={(event) => {
                event.preventDefault();
                event.stopPropagation();
                onContextMenu(item, event);
              }}
            >
              <Button
                className={"trow" + (logPicked === item.id ? " is-picked" : "")}
                onClick={() => onPick(item, [...commits].reverse()[index + 1])}
              >
                <Graph color={PALETTE[index % PALETTE.length]} last={index === commits.length - 1} />
                <span className="tver">{item.label ?? "#" + item.id}</span>
                <span className="tmsg">{item.message || "（没写说明）"}</span>
                <span className="refs">{index === 0 ? <Tag>最新</Tag> : null}</span>
                <span className="twhen">{when(item.at)}</span>
              </Button>
            </li>
          ))}
        </ul>
        <aside className="ldetail">
          {logPicked === null ? (
            <>
              <p className="ldtitle">工作区</p>
              <p className="ldmsg">
                勾选 {picked.length} 个文件，共 {formatBytes(pickedBytes) || "0 B"}。
              </p>
              <p className="ldmsg">
                提交记的是「这一版包含哪些文件」（路径 + 大小 + 修改时间），
                用来回答"跟上一版差在哪"。内容哈希要 SC2Diff。
              </p>
            </>
          ) : (
            (() => {
              const item = commits.find((entry) => entry.id === logPicked);
              if (!item) return null;
              return (
                <>
                  <p className="ldtitle">{item.label ?? "#" + item.id}</p>
                  <p className="ldmsg">
                    {item.message || "（没写说明）"} · {when(item.at)}
                  </p>
                  {pendingDiff && (
                    <p className="ldmsg">
                      比上一版：新增 {pendingDiff.added.length} · 改动 {pendingDiff.changed.length} · 删掉{" "}
                      {pendingDiff.removed.length}
                    </p>
                  )}
                  <p className="ldactions">
                    <Button onClick={() => onApply(item)}>应用这一版</Button>
                    <span className="ldhint">把勾选和包信息都恢复成这一版的样子</span>
                  </p>
                  <p className="ldmsg">这一版有 {item.files.length} 个文件：</p>
                  <ul className="mini">
                    {item.files.slice(0, 8).map((file) => (
                      <li key={file.path}>
                        <span>{formatBytes(file.bytes)}</span>
                        {file.path}
                      </li>
                    ))}
                  </ul>
                </>
              );
            })()
          )}
        </aside>
      </ScrollableSection>
    </section>
  );
}
