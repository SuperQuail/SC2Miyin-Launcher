import {
  Fragment,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";

import {
  closeContextMenu,
  contextMenuSnapshot,
  pickContextMenuItem,
  subscribeContextMenu,
} from "../lib/contextMenu";

/**
 * 右键菜单宿主（React 版，从 components/ContextMenu.vue 搬过来）。
 *
 * 由外壳挂一次，全局共用。位置会**夹在视口内** ——
 * 在窗口右下角右键时不至于弹到屏幕外面去。
 */
export function ContextMenu() {
  const { visible, x, y, items } = useSyncExternalStore(subscribeContextMenu, contextMenuSnapshot);
  const box = useRef<HTMLDivElement | null>(null);
  /** 量到真实尺寸之后再定位置，免得先闪一下再跳。 */
  const [position, setPosition] = useState({ x: 0, y: 0 });

  // layout effect 在浏览器绘制前跑，所以夹位置这一步不会被人看见
  useLayoutEffect(() => {
    if (!visible) return;
    const element = box.current;
    if (!element) return;

    const margin = 8;
    const maxX = window.innerWidth - element.offsetWidth - margin;
    const maxY = window.innerHeight - element.offsetHeight - margin;
    setPosition({
      x: Math.max(margin, Math.min(x, maxX)),
      y: Math.max(margin, Math.min(y, maxY)),
    });
  }, [visible, x, y]);

  /** 点别处、滚动、按 Esc 都关掉。 */
  useEffect(() => {
    const onPointerDown = (event: MouseEvent) => {
      const element = box.current;
      if (element && event.target instanceof Node && element.contains(event.target)) return;
      closeContextMenu();
    };
    const onWheel = () => closeContextMenu();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeContextMenu();
    };

    window.addEventListener("mousedown", onPointerDown, true);
    window.addEventListener("wheel", onWheel, { passive: true });
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onPointerDown, true);
      window.removeEventListener("wheel", onWheel);
      window.removeEventListener("keydown", onKey);
    };
  }, []);

  if (!visible) return null;

  return (
    <div
      ref={box}
      className="ctx"
      style={{ left: position.x + "px", top: position.y + "px" }}
      onContextMenu={(event) => event.preventDefault()}
    >
      {items.map((item) => (
        <Fragment key={item.id}>
          {item.separatorBefore && <div className="ctx__sep" />}
          <button
            className={"ctx__item" + (item.danger ? " ctx__item--danger" : "")}
            type="button"
            disabled={item.disabled}
            onClick={() => pickContextMenuItem(item.id)}
          >
            {item.label}
          </button>
        </Fragment>
      ))}
    </div>
  );
}
