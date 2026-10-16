import { createElement, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { renderToStaticMarkup } from "react-dom/server";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Sheet } from "./Sheet";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * 遮罩弹窗。
 *
 * 关键是那两条容易写反的行为：**默认点遮罩关**（以前那些必须显式 false 才不关），
 * 以及**点卡片里面绝不能关** —— 后者弄错的话，用户点一下内容弹窗就没了。
 */
const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

function renderSheet(props: Parameters<typeof Sheet>[0]): HTMLElement {
  const host = document.createElement("div");
  host.innerHTML = renderToStaticMarkup(createElement(Sheet, props));
  return host;
}

async function mountSheet(node: ReactNode) {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(node);
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });

  const pick = (selector: string): Element => {
    const found = host.querySelector(selector);
    if (!found) throw new Error(`弹窗里没有 ${selector}`);
    return found;
  };

  return {
    pick,
    async click(target: Element) {
      await act(async () => {
        target.dispatchEvent(new MouseEvent("click", { bubbles: true }));
      });
    },
    async rerender(node: ReactNode) {
      await act(async () => {
        root.render(node);
      });
    },
  };
}

describe("Sheet 的结构", () => {
  it("遮罩里套一张卡片", () => {
    const host = renderSheet({ children: createElement("p", null, "内容") });

    expect(host.querySelector(".sheet > .sheet__card")).not.toBeNull();
  });

  it("给了 title 才渲染标题", () => {
    const withTitle = renderSheet({ title: "导入战役包", children: createElement("p", null, "x") });
    expect(withTitle.querySelector(".sheet__title")?.textContent).toBe("导入战役包");

    const without = renderSheet({ children: createElement("p", null, "x") });
    expect(without.querySelector(".sheet__title")).toBeNull();
  });

  it("给了 actions 才渲染按钮那一排，且排在内容后面", () => {
    const withActions = renderSheet({
      children: createElement("p", null, "内容"),
      actions: createElement("button", null, "确定"),
    });
    const actions = withActions.querySelector(".sheet__card > .sheet__actions");
    expect(actions?.textContent).toBe("确定");

    const without = renderSheet({ children: createElement("p", null, "内容") });
    expect(without.querySelector(".sheet__actions")).toBeNull();
  });

  it("className 拼在卡片上（宽窄/尺寸靠它）", () => {
    const host = renderSheet({ className: "sheet--wide", children: createElement("p", null, "x") });

    const card = host.querySelector(".sheet__card");
    expect(card?.classList.contains("sheet__card")).toBe(true);
    expect(card?.classList.contains("sheet--wide")).toBe(true);
    expect(host.querySelector(".sheet")?.classList.contains("sheet--wide")).toBe(false);
  });

  it("内容直接渲染在卡片里（不是塞进 actions）", () => {
    const host = renderSheet({ children: createElement("p", { className: "body" }, "正文") });

    expect(host.querySelector(".sheet__card > .body")?.textContent).toBe("正文");
  });
});

describe("Sheet 的关闭行为", () => {
  it("默认点遮罩就关", async () => {
    const onClose = vi.fn();
    const view = await mountSheet(
      createElement(Sheet, { title: "导入", onClose, children: createElement("p", null, "内容") }),
    );

    await view.click(view.pick(".sheet"));

    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("dismissOnBackdrop=false 时点遮罩不关（跑着不能中断的那种弹窗）", async () => {
    const onClose = vi.fn();
    const view = await mountSheet(
      createElement(Sheet, {
        onClose,
        dismissOnBackdrop: false,
        children: createElement("p", null, "正在安装…"),
      }),
    );

    await view.click(view.pick(".sheet"));
    expect(onClose, "有 dismissOnBackdrop=false 就不该关").not.toHaveBeenCalled();

    // 正对照：同一个弹窗换成默认行为就关得掉 —— 证明上面那句"没关"是这道闸拦的，
    // 而不是点击压根没落到处理函数上
    await view.rerender(createElement(Sheet, { onClose, children: createElement("p", null, "正在安装…") }));
    await view.click(view.pick(".sheet"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("点卡片里面不关（不然点一下正文弹窗就没了）", async () => {
    const onClose = vi.fn();
    const view = await mountSheet(
      createElement(Sheet, { onClose, children: createElement("p", { className: "body" }, "内容") }),
    );

    await view.click(view.pick(".body"));
    expect(onClose).not.toHaveBeenCalled();

    // 正对照：遮罩本身是关得掉的
    await view.click(view.pick(".sheet"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("点底部按钮区不会顺手关掉（关不关由按钮自己决定）", async () => {
    const onClose = vi.fn();
    const view = await mountSheet(
      createElement(Sheet, {
        onClose,
        actions: createElement("button", { className: "ok" }, "确定"),
        children: createElement("p", null, "内容"),
      }),
    );

    await view.click(view.pick(".ok"));
    expect(onClose).not.toHaveBeenCalled();

    // 正对照：关得掉的那条路还在
    await view.click(view.pick(".sheet"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
