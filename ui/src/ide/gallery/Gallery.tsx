import { useMemo } from "react";

import { GallerySections, DEMO_CONTROLS } from "./sections";
import { checkDemoKeys, useDemoState } from "./state";

/**
 * 控件展台：`ide.html#gallery` 时**只**渲染这一页。
 *
 * 用户侧的「开发者房间」（AGENTS §18.1）用 iframe 套它，所以这里：
 * 不带工具栏、不碰任何 Tauri 能力 —— 浏览器演示模式下也照样能看。
 * 每个控件都配一行小字说明本项目在哪用它；卡片内容按控件分成 ./sections.tsx 与 ./demos.tsx。
 */
export function Gallery() {
  const state = useDemoState();

  // 自检：新加的控件忘了登记状态（或不登记为无状态）就在这里报出来
  const unregistered = import.meta.env.DEV ? checkDemoKeys(DEMO_CONTROLS) : [];
  if (unregistered.length > 0) {
    console.warn("[gallery] 这些控件还没登记状态：", unregistered.join(", "));
  }

  const head = useMemo(
    () => (
      <header className="gallery__head">
        <h1 className="gallery__title">Ring UI 控件展台</h1>
        <p className="gallery__note">
          开发者页（<code>ide.html</code>）用到的每个 Ring UI 控件都在这里摆一份，含变体与禁用态。
          只在地址带 <code>#gallery</code> 时出现，是给用户侧「开发者房间」用 iframe 嵌的那一页。
        </p>
      </header>
    ),
    [],
  );

  return (
    <div className="gallery">
      {head}
      <GallerySections state={state} />
    </div>
  );
}
