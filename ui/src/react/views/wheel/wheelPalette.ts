// 从设计令牌读转盘配色。
//
// 轮盘与指针的颜色都从令牌读出来，集中在这一次读取里。
// 主题变化时调用方用 theme 作依赖重新调用一次即可（见 WheelView）。

import type { WheelPalette } from "./wheelData";

/** 令牌读不到时的兜底（jsdom、首帧样式还没挂）。值抄自 styles/tokens.css。 */
const FALLBACK: WheelPalette = {
  background: ["#dbe6ff", "#e8eefb"],
  text: "#191c22",
  border: "#cdd5e5",
  accent: "#3b6ce0",
  surface: "#ffffff",
};

function token(style: CSSStyleDeclaration, name: string, fallback: string): string {
  const value = style.getPropertyValue(name).trim();
  return value || fallback;
}

/**
 * 读 --accent-soft / --surface-3 / --on-surface / --outline / --accent / --surface-1。
 *
 * 参数可注入：测试传一个假的 CSSStyleDeclaration，不依赖 jsdom 是否支持自定义属性。
 */
export function readPalette(
  style: CSSStyleDeclaration = getComputedStyle(document.documentElement),
): WheelPalette {
  return {
    background: [
      token(style, "--accent-soft", FALLBACK.background[0]),
      token(style, "--surface-3", FALLBACK.background[1]),
    ],
    text: token(style, "--on-surface", FALLBACK.text),
    border: token(style, "--outline", FALLBACK.border),
    accent: token(style, "--accent", FALLBACK.accent),
    surface: token(style, "--surface-1", FALLBACK.surface),
  };
}
