import { createContext, useCallback, useContext, useMemo, useRef, useState } from "react";

import PopupMenu from "@jetbrains/ring-ui-built/components/popup-menu/popup-menu";

export type ContextMenuItem = {
  label: string;
  /** 危险操作（删除这种）用红字 */
  danger?: boolean;
  onSelect: () => void;
};

type MenuState = { x: number; y: number; items: ContextMenuItem[] };
type MenuApi = {
  menu: MenuState | null;
  open: (event: { clientX: number; clientY: number }, items: ContextMenuItem[]) => void;
  close: () => void;
};

const MenuContext = createContext<MenuApi | null>(null);

/**
 * 右键菜单的状态与开关。菜单本体是同文件的 `ContextMenuLayer` ——
 * 两者共用一份 context，拆开就得多导一个类型出去。
 */
export function useContextMenu(): MenuApi {
  const [menu, setMenu] = useState<MenuState | null>(null);
  // 回调要稳定：Workbench 的空白右键监听挂在 window 上，menu 每次渲染都换新的话
  // 那个监听会跟着反复解绑重绑。读最新值靠 setMenu 的函数式更新。
  const open = useCallback(
    (event: { clientX: number; clientY: number }, items: ContextMenuItem[]) =>
      // 没有菜单项就别开（业务把菜单清空时不该弹一个空框）
      setMenu(items.length > 0 ? { x: event.clientX, y: event.clientY, items } : null),
    [],
  );
  const close = useCallback(() => setMenu(null), []);
  return useMemo(() => ({ menu, open, close }), [menu, open, close]);
}

export const MenuProvider = MenuContext.Provider;

/**
 * 右键菜单本体：Ring UI 的 `PopupMenu`。
 *
 * 定位、贴边翻转、点外关闭、Esc、上下键 + 回车都交给库；我们只提供**落点** ——
 * 一个 1×1 的隐形锚点。坐标用**文档坐标**（clientX/Y 加滚动量），因为 Popup 算的是
 * 「相对 document.body 的绝对定位 + 文档滚动量」，只给视口坐标会在页面滚动后偏掉。
 *
 * 菜单项走 `rows`（数据）而不是 children：这样才有单选高亮与键盘可达，
 * 手搓的一排按钮两样都没有。
 */
export function ContextMenuLayer() {
  const api = useContext(MenuContext);
  const [anchor, setAnchor] = useState<HTMLDivElement | null>(null);
  const opened = useRef(0);
  if (!api?.menu) return null;

  const { x, y, items } = api.menu;
  return (
    <>
      <div
        className="ctxanchor"
        style={{ left: x + scrollX, top: y + scrollY }}
        // 锚点自己也在"空白处"：菜单开着的时候别让它再触发一次兜底菜单
        onContextMenu={(event) => event.preventDefault()}
        onMouseDown={(event) => {
          // 菜单还开着时再按右键：关掉重开，让新位置的菜单跟着鼠标走
          if (event.button === 2) api.close();
        }}
        onMouseUp={(event) => {
          // 打开菜单的那次右键，up 会落在刚弹出来的菜单上把菜单项点掉 ——
          // 这次 up 一律吞掉。鼠标离开过之后（下一次右键）恢复正常。
          if (opened.current === 0) opened.current = 1;
          else event.preventDefault();
          event.stopPropagation();
        }}
        ref={setAnchor}
      />
      <PopupMenu
        anchorElement={anchor}
        // 锚点就落在光标准星上，偏移只用来给菜单与光标之间留一点缝
        offset={4}
        className="ctx"
        data={items}
        onSelect={(item) => {
          // 动作自己会关菜单（items 里的 close），所以这里别让库抢先自动关。
          // 返回 false = "这次别自动关"。
          item.onSelect();
          return false;
        }}
        onMouseOut={() => {
          opened.current = 0;
        }}
      />
    </>
  );
}
