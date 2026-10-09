import { describe, expect, it } from "vitest";

import { isSafeExternalUrl, renderMarkdown } from "./markdown";

/**
 * Release 正文是**从网络来的数据**，这份测试守的是净化那道边界。
 *
 * 为什么必须有：公告正文会以 v-html 插进 DOM，唯一挡在中间的只有
 * DOMPurify。哪天有人图省事把它摘掉，这里要红 —— 而不是等到某段正文
 * 往页面里塞进去一个 onerror=。
 */
describe("Release 正文的净化", () => {
  it("普通 Markdown 照常渲染", () => {
    const html = renderMarkdown("## 更新内容\n\n- 修了个 bug");
    expect(html).toContain("<h2");
    expect(html).toContain("修了个 bug");
  });

  it("script / onerror / javascript: 都进不来", () => {
    const html = renderMarkdown(
      '<script>alert(1)</script>\n\n<img src=x onerror="alert(1)">\n\n[点我](javascript:alert(1))',
    );
    expect(html).not.toContain("<script");
    expect(html).not.toContain("onerror");
    expect(html.toLowerCase()).not.toContain("javascript:");
  });

  it("空正文给空串，界面据此不显示公告", () => {
    expect(renderMarkdown("   ")).toBe("");
    expect(renderMarkdown(null)).toBe("");
  });

  it("只有 http/https 的链接交给系统浏览器", () => {
    expect(isSafeExternalUrl("https://github.com/SuperQuail/SC2Miyin-Launcher")).toBe(true);
    expect(isSafeExternalUrl("http://example.com")).toBe(true);
    expect(isSafeExternalUrl("javascript:alert(1)")).toBe(false);
    expect(isSafeExternalUrl("file:///C:/Windows")).toBe(false);
  });
});
