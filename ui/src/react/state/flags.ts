/**
 * 调试 / 截图开关的解析。
 *
 * 这些开关只决定"从哪个状态开始"，不影响生产路径；抽成纯函数是为了能单测 ——
 * 读 location 的地方越多，越没人验证得过来。
 *
 * - `?view=settings` 直接进某一页
 * - `?theme=light|dark` 强制一次主题（不写进 localStorage）
 * - `?noupdate=1` 跳过启动时的自动检查
 * - `#room` 直接进开发者房间（仍要求开发者模式开着，调试入口不是后门）
 */

/** 顶层视图。"room" 是开发者房间：它不在页签里，从设置页的开发者模式进。 */
export type ViewId = "campaigns" | "custom" | "mods" | "settings" | "room" | "install";

/** 页签里认得的那几页；room 只能从设置页进。 */
const VIEWS: ViewId[] = ["campaigns", "custom", "mods", "settings", "install"];

/** `?view=` 指向的顶层页；认不出来就是战役页。 */
export function viewFromSearch(search: string): ViewId {
  const value = new URLSearchParams(search).get("view");
  return VIEWS.includes(value as ViewId) ? (value as ViewId) : "campaigns";
}

/** `?theme=` 强制的主题；没写 / 写错就是 null（表示用存下来的偏好）。 */
export function forcedThemeFromSearch(search: string): "light" | "dark" | null {
  const value = new URLSearchParams(search).get("theme");
  return value === "light" || value === "dark" ? value : null;
}

/** `?noupdate` 存在就跳过启动时的自动检查（演示模式的新版公告会挡住内容区）。 */
export function skipsAutoUpdate(search: string): boolean {
  return new URLSearchParams(search).has("noupdate");
}

/** `#room` 只在开发者模式开着时生效。 */
export function wantsRoom(hash: string, devMode: boolean): boolean {
  return devMode && hash.includes("room");
}

/** 启动时落在哪一页。 */
export function resolveStartView(options: { search: string; hash: string; devMode: boolean }): ViewId {
  return wantsRoom(options.hash, options.devMode) ? "room" : viewFromSearch(options.search);
}
