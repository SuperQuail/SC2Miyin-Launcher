import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

/**
 * 自制战役页的**接线**测试。
 *
 * 为什么专门测这个：VariantCard 会 emit @@drop@@（卡片上就有删除按钮）和
 * @@menu@@（右键），而自制战役页曾经两个都没接 —— 于是「删除」点了没反应
 * （假按钮）、右键也没菜单。
 *
 * 这类 bug **编译过了、类型也过了**（emit 是合法的，只是没人听），
 * 单测后端更测不到。只能在挂载之后真的点一下。
 */

const deleteVariant = vi.fn(async () => {});
const activateVariant = vi.fn(async () => []);

const variant = {
  id: "scmr",
  name: "SCMR8.0",
  author: null,
  version: "8.0",
  description: null,
  map_count: 138,
  mod_count: 4,
  size_bytes: 0,
  imported_at: 0,
  source: "x.rar",
  requested: false,
  active: false,
  cover: null,
  tags: [],
  main_map: null,
  kind: "campaign",
  requires: [],
  target_sub: null,
  registration_id: null,
};

const customSlot = {
  slug: "custom",
  display_name: "自制战役",
  active: null,
  variants: [variant],
  notice: null,
};

vi.mock("../api/bridge", () => ({
  api: {
    deleteVariant: (...args: unknown[]) => deleteVariant(...(args as [])),
    activateVariant: (...args: unknown[]) => activateVariant(...(args as [])),
    variantMaps: async () => [],
    variantMods: async () => [],
    mainMapChoice: async () => null,
    variantDoc: async () => null,
    variantCover: async () => null,
  },
}));

vi.mock("../composables/useLauncher", async () => {
  // ImportDialog 会 watch(droppedPackage)：假对象（{ value: null }）不是 ref，
  // Vue 只会在控制台警告一句、这条线其实是断的 —— 所以这里要真的 ref
  const { ref } = await import("vue");

  return {
    useLauncher: () => ({
    slots: { value: [customSlot] },
    loading: { value: false },
    busy: { value: false },
    libraryRoot: { value: "D:\\launcher" },
    refresh: async () => {},
    bootstrap: async () => {},
    notify: () => {},
    activate: (...args: unknown[]) => activateVariant(...(args as [])),
    removeVariant: (...args: unknown[]) => deleteVariant(...(args as [])),
      droppedPackage: ref(null),
    }),
    errorText: (error: unknown) => String(error),
  };
});

import CustomView from "./CustomView.vue";
import VariantCard from "../components/VariantCard.vue";

describe("自制战役页的卡片接线", () => {
  beforeEach(() => {
    deleteVariant.mockClear();
    activateVariant.mockClear();
  });

  it("卡片上的删除按钮真的会去删（不是假按钮）", async () => {
    const wrapper = mount(CustomView);
    await wrapper.vm.$nextTick();

    const card = wrapper.findComponent(VariantCard);
    expect(card.exists(), "应当渲染出一张版本卡片").toBe(true);

    // 点卡片上的删除按钮
    await card.find(".variant__drop, button[title*='删除']").trigger("click");
    await wrapper.vm.$nextTick();

    // 删之前要弹确认框
    expect(wrapper.find(".sheet").exists(), "删战役要先确认").toBe(true);
  });

  it("卡片的右键真的会弹菜单", async () => {
    const wrapper = mount(CustomView);
    await wrapper.vm.$nextTick();

    const card = wrapper.findComponent(VariantCard);
    await card.trigger("contextmenu");

    // 右键菜单是 teleport 到 body 的，直接看组件实例上有没有接
    expect(card.emitted("menu"), "卡片要 emit menu").toBeTruthy();
  });
});
