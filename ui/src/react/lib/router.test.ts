import { describe, expect, it } from "vitest";

import { mergeRoute, parseRoute, routeSearch, type Route } from "./router";

/**
 * 路由的纯函数部分：解析、合并、回落。
 *
 * 这几条是"URL 即状态"的地基 —— 解析错了整页跑偏（比如把 ?view=room 当战役页），
 * 合并错了就会把上一步的位置丢掉（比如进安装页时把 slot 冲掉，装完就回不到那个战役）。
 * DOM 那边（push/replace/back）只做接线，逻辑都在这里。
 */

describe("parseRoute", () => {
  it("认得出页面与页内参数", () => {
    expect(parseRoute("?view=campaigns&slot=wol", "", false)).toEqual({ view: "campaigns", slot: "wol" });
    expect(parseRoute("?view=install&entry=custom", "", false)).toEqual({ view: "install", entry: "custom" });
  });

  it("存档页也是能进 URL 的页面", () => {
    expect(parseRoute("?view=saves", "", false)).toEqual({ view: "saves" });
    expect(routeSearch({ view: "saves" })).toBe("?view=saves");
  });

  it("没写 view 就是战役页；认不出来的 view 也回落（不许猜）", () => {
    expect(parseRoute("", "", false).view).toBe("campaigns");
    expect(parseRoute("?view=whatever", "", false).view).toBe("campaigns");
    expect(parseRoute("?view=room", "", false).view).toBe("campaigns");
  });

  it("调试开关原样留着（截图脚本在用）", () => {
    expect(parseRoute("?noupdate=1&theme=dark", "", false)).toMatchObject({ noupdate: "1", theme: "dark" });
  });

  it("开发者房间两条入口都要开发者模式", () => {
    // 新写法
    expect(parseRoute("?view=room", "", true).view).toBe("room");
    expect(parseRoute("?view=room", "", false).view).toBe("campaigns");
    // 迁移期兼容的 hash 写法
    expect(parseRoute("", "#room", true).view).toBe("room");
    expect(parseRoute("", "#room", false).view).toBe("campaigns");
  });

  it("空值的参数不进 route（别把 slot= 这种半截参数当位置）", () => {
    expect(parseRoute("?view=campaigns&slot=", "", false).slot).toBeUndefined();
  });
});

describe("mergeRoute", () => {
  const route: Route = { view: "campaigns", slot: "wol" };

  it("改一个字段不碰别的（进安装页时 slot 得留着，回来才知道开的是哪一部）", () => {
    expect(mergeRoute(route, { view: "install", entry: "campaign" })).toEqual({
      view: "install",
      slot: "wol",
      entry: "campaign",
    });
  });

  it("空串 = 去掉这个字段", () => {
    expect(mergeRoute(route, { slot: "" })).toEqual({ view: "campaigns" });
  });

  it("view 不会被抹成空的", () => {
    expect(mergeRoute(route, { view: "" }).view).toBe("campaigns");
    expect(mergeRoute(route, { view: undefined }).view).toBe("campaigns");
  });
});

describe("routeSearch", () => {
  it("view 放最前，空值不写", () => {
    expect(routeSearch({ view: "campaigns", slot: "wol" })).toBe("?view=campaigns&slot=wol");
    expect(routeSearch({ view: "custom", slot: "" })).toBe("?view=custom");
  });

  it("解析 -> 拼回来是同一份（往返不丢参数）", () => {
    const route = parseRoute("?view=campaigns&slot=hots&noupdate=1", "", false);
    expect(parseRoute(routeSearch(route), "", false)).toEqual(route);
  });
});
