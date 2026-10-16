import type { MouseEvent, SVGAttributes } from "react";

import Button from "@jetbrains/ring-ui-built/components/button/button";
import Input, { Size } from "@jetbrains/ring-ui-built/components/input/input";
import Tooltip from "@jetbrains/ring-ui-built/components/tooltip/tooltip";
import addIcon from "@jetbrains/icons/add";
import arrowLeftIcon from "@jetbrains/icons/arrow-left";
import exportIcon from "@jetbrains/icons/export";
import searchIcon from "@jetbrains/icons/search";
import updateIcon from "@jetbrains/icons/update";

/**
 * 按住工具栏拖动窗口。
 *
 * 这一页和启动器共用同一个无边框窗口，所以窗口控件得自己画。
 * `data-tauri-drag-region` 只认事件落在**带这个属性的元素本身** ——
 * 点在里面的文字上就不响应，用户感觉是"有时能拖有时不能"。所以自己接管：
 * 落点不在按钮/输入框/窗口控件上，就调系统拖动。
 */
function startWindowDrag(event: MouseEvent): void {
  if (event.button !== 0) return;
  const target = event.target as HTMLElement | null;
  if (target?.closest("button, a, input, select, textarea, .winctl")) return;
  void (async () => {
    try {
      const { getCurrentWindow } = await import("@tauri-apps/api/window");
      await getCurrentWindow().startDragging();
    } catch {
      // 浏览器演示模式没有这个能力，忽略
    }
  })();
}

/**
 * 窗口按钮的三个图形。Ring UI 没有「最小化 / 还原」这两个形状，所以自己画；
 * 但传给 `icon` 的是**组件类型**，不是 `<svg/>` 元素（AGENTS §18 的坑表）。
 */
function MinimizeIcon(props: SVGAttributes<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 12 12" aria-hidden="true" {...props}>
      <path d="M2.5 6h7" />
    </svg>
  );
}

function MaximizeIcon(props: SVGAttributes<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 12 12" aria-hidden="true" {...props}>
      <rect x="2.5" y="2.5" width="7" height="7" rx="1.5" />
    </svg>
  );
}

function RestoreIcon(props: SVGAttributes<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 12 12" aria-hidden="true" {...props}>
      <rect x="2.5" y="4.5" width="5" height="5" rx="1" />
      <path d="M4.5 4.5v-2h5v5h-2" />
    </svg>
  );
}

function CloseIcon(props: SVGAttributes<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 12 12" aria-hidden="true" {...props}>
      <path d="M3 3l6 6" />
      <path d="M9 3l-6 6" />
    </svg>
  );
}

export type ToolbarProps = {
  pkg: string;
  onPkg: (value: string) => void;
  onHome: () => void;
  onRescan: () => void;
  onAddDirectory: () => void;
  search: string;
  onSearch: (value: string) => void;
  canExport: boolean;
  onExport: () => void;
  maximized: boolean;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onClose: () => void;
};

/** 顶部工具栏：返回 / 包名 / 扫描 / 搜索 / 导出 + 自画的窗口按钮。 */
export function Toolbar({
  pkg,
  onPkg,
  onHome,
  onRescan,
  onAddDirectory,
  search,
  onSearch,
  canExport,
  onExport,
  maximized,
  onMinimize,
  onToggleMaximize,
  onClose,
}: ToolbarProps) {
  return (
    <div className="tb" data-tauri-drag-region onMouseDown={startWindowDrag}>
      <Button icon={arrowLeftIcon} onClick={onHome}>返回启动器</Button>
      <span className="tsep" />
      <Tooltip title="这个包的名字 —— 提交历史和导出都用它">
        <label className="pkgnamectl">
          <span>包名</span>
          <Input
            className="tsearch tsearch--pkg"
            size={Size.S}
            value={pkg}
            onChange={(event) => onPkg(event.target.value)}
            placeholder="给你的包起个名字"
            spellCheck={false}
          />
        </label>
      </Tooltip>
      <span className="tsep" />
      <Button icon={updateIcon} onClick={onRescan}>重新扫描</Button>
      <Button icon={addIcon} onClick={onAddDirectory}>添加目录…</Button>
      <span className="tspacer" />
      <Input
        className="tsearch"
        size={Size.S}
        icon={searchIcon}
        value={search}
        onChange={(event) => onSearch(event.target.value)}
        placeholder="搜索"
      />
      <Button primary icon={exportIcon} disabled={!canExport} onClick={onExport}>导出压缩包</Button>

      {/* 窗口按钮：这页用的是同一个无边框窗口，不画就没有最小化/关闭 */}
      <div className="winctl">
        <Button className="winctl__btn" icon={MinimizeIcon} title="最小化" onClick={onMinimize} />
        <Button
          className="winctl__btn"
          icon={maximized ? RestoreIcon : MaximizeIcon}
          title={maximized ? "还原" : "最大化"}
          onClick={onToggleMaximize}
        />
        <Button className="winctl__btn winctl__btn--close" icon={CloseIcon} title="关闭" onClick={onClose} />
      </div>
    </div>
  );
}
