import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Field } from "./Field";

/**
 * 表单一行：标签 + 控件 + 提示。
 *
 * 两个分支值得钉住：**error 压过 hint**（有错时不该还显示"随便填"），
 * 以及 label 没给时不许渲染一个空标签（那是会占位的）。
 */
function renderField(props: Parameters<typeof Field>[0]): HTMLLabelElement {
  const host = document.createElement("div");
  host.innerHTML = renderToStaticMarkup(createElement(Field, props));
  const label = host.querySelector("label");
  if (!label) throw new Error("Field 没有渲染出 <label>");
  return label;
}

const input = () => createElement("input", { className: "input" });

describe("Field", () => {
  it("基础 class 恒为 field，没修饰时不加多余的", () => {
    const label = renderField({ label: "版本号", children: input() });

    expect(label.classList.contains("field")).toBe(true);
    expect(label.classList.contains("field--wide")).toBe(false);
    expect(label.classList.contains("field--bad")).toBe(false);
  });

  it("wide / invalid 各加自己的修饰 class", () => {
    expect(renderField({ wide: true, children: input() }).classList.contains("field--wide")).toBe(true);
    expect(renderField({ invalid: true, children: input() }).classList.contains("field--bad")).toBe(true);

    const both = renderField({ wide: true, invalid: true, children: input() });
    expect(both.classList.contains("field--wide")).toBe(true);
    expect(both.classList.contains("field--bad")).toBe(true);
  });

  it("给了 label 才渲染标签，没给就不留空占位", () => {
    expect(renderField({ label: "游戏目录", children: input() }).querySelector(".field__label")?.textContent).toBe(
      "游戏目录",
    );
    expect(renderField({ children: input() }).querySelector(".field__label")).toBeNull();
  });

  it("控件本身渲染在里面（整行可点，因为外层是 label）", () => {
    const label = renderField({ label: "x", children: input() });

    expect(label.querySelector("input")).not.toBeNull();
  });

  it("只给 hint 时显示提示", () => {
    const label = renderField({ hint: "留空就用自动发现的", children: input() });

    expect(label.querySelector(".field__hint")?.textContent).toBe("留空就用自动发现的");
    expect(label.querySelector(".field__error")).toBeNull();
  });

  it("有 error 时显示错误、并压过 hint", () => {
    const label = renderField({ error: "路径不存在", hint: "留空就用自动发现的", children: input() });

    expect(label.querySelector(".field__error")?.textContent).toBe("路径不存在");
    expect(label.querySelector(".field__hint"), "有错的时候不该还显示提示").toBeNull();
  });

  it("什么都没有时就只剩控件本身", () => {
    const label = renderField({ children: input() });

    expect(label.children).toHaveLength(1);
  });

  it("error 是空串时退回 hint（调用方可能填了个空字符串）", () => {
    const label = renderField({ error: "", hint: "留空就用自动发现的", children: input() });

    expect(label.querySelector(".field__error")).toBeNull();
    expect(label.querySelector(".field__hint")?.textContent).toBe("留空就用自动发现的");
  });
});
