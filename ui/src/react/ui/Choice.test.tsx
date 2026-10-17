import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Choice } from "./Choice";

/**
 * Choice 的结构必须是**原生 radio**（无障碍与键盘行为白给），样式才做成段落按钮。
 * 这里钉住三件事：有几个 radio、选中的是哪个、禁用是否传下去。
 */
function render(props: Parameters<typeof Choice<string>>[0]): HTMLFieldSetElement {
  const host = document.createElement("div");
  host.innerHTML = renderToStaticMarkup(createElement(Choice, props));
  const fieldset = host.querySelector("fieldset");
  if (!fieldset) throw new Error("Choice 没有渲染出 <fieldset>");
  return fieldset;
}

describe("Choice", () => {
  const options = [
    { value: "overwrite", label: "覆盖更新", hint: "沿用原目录名" },
    { value: "rename", label: "重命名后导入", hint: "两个版本并存" },
  ];

  it("渲染成一排原生 radio，选中的那个带 checked", () => {
    const fieldset = render({ value: "rename", options, onChange: () => {} });
    const radios = fieldset.querySelectorAll("input[type=radio]");
    expect(radios).toHaveLength(2);
    expect((radios[0] as HTMLInputElement).checked).toBe(false);
    expect((radios[1] as HTMLInputElement).checked).toBe(true);
  });

  it("选中的那个带高亮类，另一个没有", () => {
    const fieldset = render({ value: "overwrite", options, onChange: () => {} });
    const labels = fieldset.querySelectorAll("label.choice__option");
    expect(labels[0].className).toContain("choice__option--on");
    expect(labels[1].className).not.toContain("choice__option--on");
  });

  it("禁用会同时传给 fieldset 和每个 radio", () => {
    const fieldset = render({ value: "overwrite", options, onChange: () => {}, disabled: true });
    expect(fieldset.disabled).toBe(true);
    for (const radio of fieldset.querySelectorAll("input[type=radio]")) {
      expect((radio as HTMLInputElement).disabled).toBe(true);
    }
  });

  it("给了 hint 就渲染在下面", () => {
    const fieldset = render({ value: "overwrite", options, onChange: () => {}, label: "版本冲突", hint: "两个版本各自启用互不影响" });
    expect(fieldset.querySelector("legend")?.textContent).toBe("版本冲突");
    expect(fieldset.querySelector(".choice__foot")?.textContent).toBe("两个版本各自启用互不影响");
  });
});