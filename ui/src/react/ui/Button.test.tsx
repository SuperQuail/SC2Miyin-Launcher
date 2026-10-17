import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { renderToStaticMarkup } from "react-dom/server";
import { act } from "react-dom/test-utils";
import { afterEach, describe, expect, it, vi } from "vitest";

import { Button, type ButtonVariant } from "./Button";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/**
 * 按钮的 variant 只是「拼出哪个 class」—— 样式全在全局 base.css 里按这些 class 找。
 * 拼错了界面不会报错，只会长得不对，所以这里按 class 逐个钉住。
 */
const VARIANTS: ButtonVariant[] = ["primary", "tonal", "text", "outline", "danger"];

const mounted: Array<() => Promise<void>> = [];

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

/** 静态渲染出 HTML 再装回 DOM —— 按属性断言，比在字符串里找子串稳。 */
function renderButton(props: Parameters<typeof Button>[0]): HTMLButtonElement {
  const host = document.createElement("div");
  host.innerHTML = renderToStaticMarkup(createElement(Button, props));
  const button = host.querySelector("button");
  if (!button) throw new Error("Button 没有渲染出 <button>");
  return button;
}

describe("Button", () => {
  it("默认是最弱的一档（text），不带 btn-lg", () => {
    const button = renderButton({ children: "取消" });

    expect(button.classList.contains("btn")).toBe(true);
    expect(button.classList.contains("btn-text")).toBe(true);
    expect(button.classList.contains("btn-lg")).toBe(false);
  });

  it("每个 variant 都映射到自己那一档 class", () => {
    for (const variant of VARIANTS) {
      const button = renderButton({ variant, children: "x" });
      expect(button.classList.contains(`btn-${variant}`), `${variant} 那一档的 class 没拼上`).toBe(true);
    }
  });

  it("size=lg 才加 btn-lg", () => {
    expect(renderButton({ size: "lg", children: "x" }).classList.contains("btn-lg")).toBe(true);
    expect(renderButton({ size: "md", children: "x" }).classList.contains("btn-lg")).toBe(false);
  });

  it("disabled 透传到原生按钮（点不动、也不在 tab 链里）", () => {
    expect(renderButton({ disabled: true, children: "保存" }).disabled).toBe(true);
    expect(renderButton({ children: "保存" }).disabled).toBe(false);
  });

  it("type 默认 button（表单里不会意外提交），要提交时显式传", () => {
    expect(renderButton({ children: "x" }).type).toBe("button");
    expect(renderButton({ type: "submit", children: "x" }).type).toBe("submit");
  });

  it("title 透传（图标按钮靠它说明自己是什么）", () => {
    expect(renderButton({ title: "删掉它", children: "x" }).title).toBe("删掉它");
  });

  it("children 渲染出来", () => {
    expect(renderButton({ children: "导入战役包" }).textContent).toBe("导入战役包");
  });

  it("点击真的接上了 onClick", async () => {
    const onClick = vi.fn();
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(createElement(Button, { onClick, children: "点我" }));
    });
    mounted.push(async () => {
      await act(async () => {
        root.unmount();
      });
      host.remove();
    });

    const button = host.querySelector("button");
    if (!button) throw new Error("Button 没有渲染出 <button>");
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("loading：禁用 + aria-busy + 转圈，文案还在", () => {
    const button = renderButton({ loading: true, children: "导出中…" });
    expect(button.disabled).toBe(true);
    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(button.querySelector(".btn__spinner")).not.toBeNull();
    expect(button.textContent).toContain("导出中…");
  });
});
