import { afterEach, describe, expect, it, vi } from "vitest";

/**
 * 启动兜底：前端挂不上时，用户必须看到「一句人话 + 原始错误 + 重载按钮」，
 * 而不是一片白。这条路径不依赖 React，所以这里直接对着 DOM 断言。
 */

/** 模块里有个「只报第一个错」的开关，每个用例都要一份新模块。 */
async function fresh(): Promise<typeof import("./startup")> {
  vi.resetModules();
  return await import("./startup");
}

function makeApp(): HTMLElement {
  document.body.innerHTML = '<div id="app"><p>正在启动弥音启动器…</p></div>';
  return document.getElementById("app") as HTMLElement;
}

afterEach(() => {
  vi.useRealTimers();
  delete document.documentElement.dataset.miyinMounted;
  document.body.innerHTML = "";
});

describe("启动兜底", () => {
  it("没挂载时出错：把 #app 换成一页可读的失败说明", async () => {
    const { installStartupGuards } = await fresh();
    const app = makeApp();
    installStartupGuards();

    window.dispatchEvent(new ErrorEvent("error", { message: "模块执行失败：boom" }));

    expect(app.querySelector(".boot-fail")).not.toBeNull();
    expect(app.textContent).toContain("启动失败");
    // 原始错误要出现 —— 用户报障时就是靠这段文本
    expect(app.textContent).toContain("boom");
    expect(app.querySelector("button.btn")).not.toBeNull();
    // 静态占位已经被换掉
    expect(app.textContent).not.toContain("正在启动弥音启动器");
  });

  it("未处理的 Promise 拒绝也算启动失败", async () => {
    const { installStartupGuards } = await fresh();
    const app = makeApp();
    installStartupGuards();

    const event = new Event("unhandledrejection") as Event & { reason?: unknown };
    event.reason = new Error("后端没起来");
    window.dispatchEvent(event);

    expect(app.querySelector(".boot-fail")).not.toBeNull();
    expect(app.textContent).toContain("后端没起来");
  });

  it("挂载之后才出错：不抢界面", async () => {
    const { installStartupGuards, markMounted } = await fresh();
    const app = makeApp();
    installStartupGuards();
    markMounted();

    window.dispatchEvent(new ErrorEvent("error", { message: "迟到的错误" }));

    expect(app.querySelector(".boot-fail")).toBeNull();
    expect(app.textContent).toContain("正在启动弥音启动器");
  });

  it("20 秒还没挂载：显示启动超时", async () => {
    vi.useFakeTimers();
    const { installStartupGuards } = await fresh();
    const app = makeApp();
    installStartupGuards();

    vi.advanceTimersByTime(20_000);

    expect(app.textContent).toContain("启动超时");
  });

  it("只报第一个原因（后面的不覆盖）", async () => {
    const { installStartupGuards } = await fresh();
    const app = makeApp();
    installStartupGuards();

    window.dispatchEvent(new ErrorEvent("error", { message: "第一个" }));
    window.dispatchEvent(new ErrorEvent("error", { message: "第二个" }));

    expect(app.textContent).toContain("第一个");
    expect(app.textContent).not.toContain("第二个");
  });
});
