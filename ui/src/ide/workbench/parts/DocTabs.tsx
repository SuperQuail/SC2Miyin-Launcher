import type { DevEntry } from "../../api";

export type DocTabsProps = {
  opened: DevEntry[];
  active: string | null;
  onOpen: (entry: DevEntry) => void;
  onClose: (entry: DevEntry) => void;
};

/**
 * 文档标签页。
 *
 * **故意不用 Ring UI 的 Tabs** —— 那是「切换面板」的组件，在自定义容器里排不出多个
 * 标签（AGENTS §18 的坑表）。这里要的是编辑器那种一个一个的文档标签，仍用自己这套。
 */
export function DocTabs({ opened, active, onOpen, onClose }: DocTabsProps) {
  return (
    <header className="tabsbar">
      {opened.map((entry) => (
        <button
          key={entry.abs}
          type="button"
          className={"doc" + (active === entry.abs ? " doc--on" : "")}
          onClick={() => onOpen(entry)}
        >
          {entry.name}
          <span
            className="docx"
            onClick={(event) => {
              event.stopPropagation();
              onClose(entry);
            }}
          >
            ×
          </span>
        </button>
      ))}
      <span className="tspacer" />
    </header>
  );
}
