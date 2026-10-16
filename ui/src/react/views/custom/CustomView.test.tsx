import { createElement, type MouseEvent as ReactMouseEvent } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SlotView, Variant } from "../../../api/types";

/**
 * 自制战役页的**接线**测试。
 *
 * 为什么专门测这个：版本卡片会回调 drop（卡片上就有删除按钮）和 menu（右键），
 * 而自制战役页曾经两个都没接 —— 于是「删除」点了没反应（假按钮）、右键也没菜单。
 *
 * 这类 bug **编译过了、类型也过了**（回调是合法的，只是没人听），
 * 单测后端更测不到。只能在挂载之后真的点一下。
 *
 * 没有 @testing-library/react（也不许装），所以自己用 createRoot + act 挂。
 */

const removeVariant = vi.fn(async (_slot: string, _variantId: string) => true);
const activate = vi.fn(async (_slot: string, _variantId: string | null) => true);

/** 最近一次弹出来的菜单项与回调 —— 省得去翻 mock.calls（那里是 any）。 */
let lastItems: CampaignMenuItem[] | null = null;
let lastPick: ((id: string) => void) | null = null;
const showMenu = vi.fn(
  (_event: ReactMouseEvent, items: CampaignMenuItem[], onPick: (id: string) => void) => {
    lastItems = items;
    lastPick = onPick;
  },
);

vi.mock("../../../api/bridge", () => ({
  api: {
    variantCover: async () => null,
  },
}));

import type { CampaignMenuItem } from "../campaigns/host";
import { CustomView } from "./CustomView";
import type { CustomHost } from "./host";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const variant = {
  id: "scmr",
  name: "SCMR8.0",
  author: null,
  version: "8.0",
  description: null,
  format: "ccm",
  imported_at: 0,
  source: "x.rar",
  map_count: 138,
  mod_count: 4,
  size_bytes: 0,
  target_sub: null,
  cover: null,
  tags: [],
  registration_id: null,
  declared_mods: [],
  doc: null,
} satisfies Variant;

const customSlot = {
  slug: "custom",
  display_name: "自制战役",
  sub_directory: null,
  active: null,
  active_name: null,
  variants: [variant],
  notice: null,
} satisfies SlotView;

function makeHost(overrides?: Partial<CustomHost>): CustomHost {
  return {
    slots: [customSlot],
    libraryRoot: "D:\\launcher",
    loading: false,
    notify: () => undefined,
    bootstrap: () => undefined,
    refresh: () => undefined,
    activate,
    removeVariant,
    showMenu,
    openCheats: () => undefined,
    ...overrides,
  };
}

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

beforeEach(() => {
  removeVariant.mockClear();
  activate.mockClear();
  showMenu.mockClear();
  lastItems = null;
  lastPick = null;
});

/** 挂一页出来，返回它所在的容器。 */
async function mountView(host: CustomHost): Promise<HTMLElement> {
  const element = document.createElement("div");
  document.body.appendChild(element);
  const root = createRoot(element);
  await act(async () => {
    root.render(createElement(CustomView, host));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    element.remove();
  });
  return element;
}

/** 在 act 里派发一个鼠标事件。 */
async function fire(target: Element, type: string): Promise<void> {
  await act(async () => {
    target.dispatchEvent(new MouseEvent(type, { bubbles: true }));
  });
}

describe("自制战役页的卡片接线", () => {
  it("卡片上的删除按钮真的会去删（不是假按钮）", async () => {
    const view = await mountView(makeHost());

    const drop = view.querySelector(".variant__drop");
    expect(drop, "应当渲染出一张版本卡片").not.toBeNull();
    await fire(drop as Element, "click");

    // 删之前要弹确认框
    const sheet = document.querySelector(".sheet");
    expect(sheet, "删战役要先确认").not.toBeNull();

    const confirm = [...(sheet as Element).querySelectorAll(".sheet__actions button")].find(
      (button) => button.textContent === "删除",
    );
    expect(confirm, "确认框里要有「删除」按钮").not.toBeUndefined();
    await fire(confirm as Element, "click");

    expect(removeVariant).toHaveBeenCalledWith("custom", "scmr");
  });

  it("卡片的右键真的会弹菜单，菜单里的删除也走确认框", async () => {
    const view = await mountView(makeHost());

    const card = view.querySelector("article.variant");
    expect(card, "应当渲染出一张版本卡片").not.toBeNull();
    await fire(card as Element, "contextmenu");

    expect(showMenu).toHaveBeenCalledTimes(1);
    expect(lastItems, "右键要给出菜单项").not.toBeNull();
    expect((lastItems as CampaignMenuItem[]).some((item) => item.id === "drop")).toBe(true);

    await act(async () => {
      lastPick?.("drop");
    });
    expect(document.querySelector(".sheet"), "从菜单删除同样要先确认").not.toBeNull();
  });
});
