import { useSyncExternalStore } from "react";

import { viewFromSearch, wantsRoom, type ViewId } from "../state/flags";
import { isDevMode } from "./devMode";

/**
 * 导航：**URL 即状态**（见 docs/navigation.md）。
 *
 * 只有「位置」进 URL —— 哪一页、哪个战役、安装页按哪套规则预检；它们都能从库里重建。
 * 输入框草稿、滚动位置、拖到一半的状态**不进**（丢了不构成 bug）。本地路径也绝不进。
 *
 * 于是几件老毛病不用逐页修：
 * - 装完回原处：`back()`，安装页压根不需要知道"从哪来"（所以 installReturn 删了）
 * - 哪个战役开着：`?slot=wol`，战役页与自制战役页共用同一个字段
 * - 刷新 / 深链：URL 还在，读一遍就还原
 * - 浏览器后退：页面级跳转走 push，页内改动走 replace
 */
export type Route = { view: ViewId } & Record<string, string>;

/** 解析 location。`devMode` 传进来而不是现读 —— 纯函数才好测。 */
export function parseRoute(search: string, hash: string, devMode: boolean): Route {
  const params = new URLSearchParams(search);
  const route: Route = { view: "campaigns" };
  for (const [key, value] of params) {
    if (key !== "view" && value) route[key] = value;
  }
  // 进房间的两条入口都要开发者模式：?view=room 是新写法，#room 留着兼容（截图脚本在用）
  const room = params.get("view") === "room" || wantsRoom(hash, devMode);
  route.view = room && devMode ? "room" : viewFromSearch(search);
  return route;
}

/** Route -> 查询串。view 放最前，空值不写进 URL。 */
export function routeSearch(route: Route): string {
  const params = new URLSearchParams();
  params.set("view", route.view);
  for (const [key, value] of Object.entries(route)) {
    if (key !== "view" && value) params.set(key, value);
  }
  return "?" + params.toString();
}

/**
 * 合并一次改动。
 *
 * 是**合并**不是替换：从战役页进安装页时 `slot=wol` 留在 URL 里，回来才知道开的是哪一部。
 * 空串 = 把这个字段去掉（别留 `slot=` 这种半截参数）。
 */
export function mergeRoute(route: Route, patch: Record<string, string | undefined>): Route {
  const next: Route = { ...route };
  for (const [key, value] of Object.entries(patch)) {
    if (value === undefined || value === "") delete next[key];
    else next[key] = value;
  }
  if (!next.view) next.view = "campaigns";
  return next;
}

const listeners = new Set<() => void>();
let cached: Route | null = null;
let cachedKey = "";

function read(): Route {
  return parseRoute(location.search, location.hash, isDevMode());
}

/** 现在的 route。没变就返回**同一个对象** —— useSyncExternalStore 靠它判断要不要重渲染。 */
export function currentRoute(): Route {
  const next = read();
  const key = routeSearch(next) + "|" + location.hash;
  if (!cached || key !== cachedKey) {
    cached = next;
    cachedKey = key;
  }
  return cached;
}

function emit(): void {
  cached = null;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

if (typeof window !== "undefined") {
  // 浏览器后退/前进，以及手改 hash（#room）
  window.addEventListener("popstate", emit);
  window.addEventListener("hashchange", emit);
}

/** 页面级跳转：进安装页、进某个战役 —— **push**，能 back 回来。 */
export function navigate(patch: Record<string, string | undefined>): void {
  const next = mergeRoute(currentRoute(), patch);
  history.pushState(next, "", routeSearch(next));
  emit();
}

/** 页内参数：改选中 / 展开之类 —— **replace**，不攒历史（否则后退要点很多次）。 */
export function setParams(patch: Record<string, string | undefined>): void {
  const next = mergeRoute(currentRoute(), patch);
  history.replaceState(next, "", routeSearch(next));
  emit();
}

/** 回上一处（装完 / 返回）。 */
export function back(): void {
  history.back();
}

/** 订阅 route：popstate / hashchange / navigate / setParams 都会让组件重渲染。 */
export function useRoute(): Route {
  return useSyncExternalStore(subscribe, currentRoute, currentRoute);
}
