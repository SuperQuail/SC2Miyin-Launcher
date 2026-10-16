import { createRoot } from "react-dom/client";

import { RoomView } from "./RoomView";

/**
 * 给 Vue 外壳用的入口。房间不依赖外壳数据，只有"返回"一个回调。
 */
export function mountRoom(host: HTMLElement, emit: (event: string, payload?: unknown) => void) {
  const root = createRoot(host);
  const render = () => root.render(<RoomView onBack={() => emit("back")} />);
  render();
  return {
    update: render,
    teardown: () => root.unmount(),
  };
}
