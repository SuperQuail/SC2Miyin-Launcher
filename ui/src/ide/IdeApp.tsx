import { useEffect, useState } from "react";

import { Gallery } from "./gallery/Gallery";
import { Workbench } from "./workbench/Workbench";

/**
 * 开发者页的入口：`#gallery` 只渲染控件展台，其余渲染工作台。
 *
 * **展台必须只有它自己** —— 用户侧的开发者房间会把它 iframe 嵌进去，
 * 带上文件树 / 工具栏就不干净了。同时工作台不挂载，它的扫描等 Tauri 初始化
 * 也不会跑，浏览器演示模式里照样能看。
 */
export function IdeApp() {
  const [gallery, setGallery] = useState(() => location.hash === "#gallery");

  useEffect(() => {
    const sync = () => setGallery(location.hash === "#gallery");
    addEventListener("hashchange", sync);
    return () => removeEventListener("hashchange", sync);
  }, []);

  return gallery ? <Gallery /> : <Workbench />;
}
