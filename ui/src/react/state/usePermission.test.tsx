import { createElement, useState, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * 按需提权（docs/design/storage-and-elevation.md §3–§4）。
 *
 * 钉住四件事：没有 `[PERMISSION]` 就不打扰、有才弹说明、
 * 「再次提权并同意」先拉助手再用 `privilegedOp` 重放、以及「保持现状」之后界面照常可用。
 */

const requestPrivilegedWorker = vi.fn<() => Promise<boolean>>();
const privilegedOp = vi.fn<(op: string, args: unknown) => Promise<unknown>>();

vi.mock("../../api/bridge", () => ({
  api: {
    requestPrivilegedWorker: () => requestPrivilegedWorker(),
    privilegedOp: (op: string, args: unknown) => privilegedOp(op, args),
  },
}));

import { PermissionHost } from "../app/PermissionHost";
import { usePermissionGate, type PrivilegedRequest } from "./usePermission";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const REQUEST: PrivilegedRequest = { op: "activate", args: { slot: "hots", variantId: "v1" } };
const DENIED = new Error("[PERMISSION] 拒绝访问：D:\\Game\\BLZ\\StarCraft II");

const mounted: Array<() => Promise<void>> = [];
/** 谁先谁后：验证「先 request 再 privilegedOp」 */
const order: string[] = [];

/** 一个"写按钮"：点了就走提权闸门，结果写在按钮文字里。 */
function WriteButton({
  className,
  action,
  request = REQUEST,
}: {
  className: string;
  action: () => Promise<unknown>;
  request?: PrivilegedRequest;
}) {
  const permission = usePermissionGate();
  const [outcome, setOutcome] = useState("idle");
  return (
    <button
      className={className}
      type="button"
      onClick={() => {
        void permission.run(action, request).then(
          (value) => setOutcome("done:" + String(value)),
          (error) =>
            setOutcome("failed:" + (error instanceof Error ? error.message : String(error))),
        );
      }}
    >
      {outcome}
    </button>
  );
}

/** 同一个事件里连做两个写操作（批量安装 / 整组勾选的情况）。 */
function BatchButton({
  first,
  second,
}: {
  first: () => Promise<unknown>;
  second: () => Promise<unknown>;
}) {
  const permission = usePermissionGate();
  const [outcome, setOutcome] = useState("idle");
  return (
    <button
      className="batch"
      type="button"
      onClick={() => {
        void (async () => {
          try {
            await permission.run(first, REQUEST);
            await permission.run(second, {
              op: "delete_variant",
              args: { slot: "hots", variantId: "v1" },
            });
            setOutcome("done");
          } catch (error) {
            setOutcome("failed:" + (error instanceof Error ? error.message : String(error)));
          }
        })();
      }}
    >
      {outcome}
    </button>
  );
}

/** 挂起整套提权状态（内容 + 对话框）。 */
async function mountNode(node: ReactNode): Promise<void> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(createElement(PermissionHost, null, node));
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
    host.remove();
  });
  await flush();
}

function mount(action: () => Promise<unknown>): Promise<void> {
  return mountNode(createElement(WriteButton, { className: "write", action }));
}

async function flush(times = 4): Promise<void> {
  for (let index = 0; index < times; index += 1) {
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });
  }
}

async function click(element: Element | null): Promise<void> {
  if (!element) throw new Error("要点的元素不存在");
  await act(async () => {
    (element as HTMLElement).click();
  });
  await flush();
}

function button(text: string): HTMLButtonElement | undefined {
  return [...document.querySelectorAll("button")].find(
    (item) => item.textContent?.trim() === text,
  ) as HTMLButtonElement | undefined;
}

const writeButton = () => document.querySelector(".write");
const dialog = () => document.querySelector(".sheet");

beforeEach(() => {
  requestPrivilegedWorker.mockReset();
  privilegedOp.mockReset();
  order.length = 0;
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
});

describe("没有权限问题就不打扰", () => {
  it("写操作直接完成：不弹框、不拉助手", async () => {
    const action = vi.fn(async () => "ok");
    await mount(action);

    await click(writeButton());

    expect(writeButton()?.textContent).toBe("done:ok");
    expect(dialog()).toBeNull();
    expect(requestPrivilegedWorker).not.toHaveBeenCalled();
    expect(privilegedOp).not.toHaveBeenCalled();
  });

  it("别的错误照原样抛给调用方，也不弹框", async () => {
    await mount(async () => {
      throw new Error("磁盘已满");
    });

    await click(writeButton());

    expect(writeButton()?.textContent).toBe("failed:磁盘已满");
    expect(dialog()).toBeNull();
    expect(requestPrivilegedWorker).not.toHaveBeenCalled();
  });
});

describe("[PERMISSION] 才弹说明", () => {
  it("弹 Sheet：说清要管理员权限，原因去掉协议前缀", async () => {
    await mount(async () => {
      throw DENIED;
    });

    await click(writeButton());

    expect(dialog()).not.toBeNull();
    expect(document.querySelector(".sheet__title")?.textContent).toBe(
      "需要管理员权限才能写入游戏目录或数据根",
    );
    expect(document.body.textContent).toContain("拒绝访问");
    expect(document.body.textContent).not.toContain("[PERMISSION]");
    expect(button("再次提权并同意")).toBeTruthy();
    expect(button("保持现状")).toBeTruthy();
  });

  it("提权之前不会自己拉助手", async () => {
    await mount(async () => {
      throw DENIED;
    });

    await click(writeButton());

    expect(requestPrivilegedWorker).not.toHaveBeenCalled();
    expect(privilegedOp).not.toHaveBeenCalled();
  });
});

describe("「再次提权并同意」：先拉助手，再重放那一步", () => {
  it("顺序是 requestPrivilegedWorker → privilegedOp(op, args)，本机不再跑一遍", async () => {
    const action = vi.fn(async () => {
      throw DENIED;
    });
    requestPrivilegedWorker.mockImplementation(async () => {
      order.push("request");
      return true;
    });
    privilegedOp.mockImplementation(async (op) => {
      order.push("op:" + op);
      return undefined;
    });
    await mount(action);

    await click(writeButton());
    await click(button("再次提权并同意") ?? null);

    expect(order).toEqual(["request", "op:activate"]);
    expect(privilegedOp).toHaveBeenCalledWith("activate", { slot: "hots", variantId: "v1" });
    // 本机不再重跑一次（那只会再吃一次 [PERMISSION]）
    expect(action).toHaveBeenCalledTimes(1);
    expect(dialog()).toBeNull();
    // 重放的返回值不保证和命令同形：闸门给 undefined，界面接着重新读状态
    expect(writeButton()?.textContent).toBe("done:undefined");
  });

  it("被拒 / 失败：留在框里显示原因，允许再点一次", async () => {
    requestPrivilegedWorker.mockResolvedValueOnce(false);
    requestPrivilegedWorker.mockResolvedValue(true);
    privilegedOp.mockResolvedValue(undefined);
    await mount(async () => {
      throw DENIED;
    });

    await click(writeButton());
    await click(button("再次提权并同意") ?? null);

    // UAC 里点了取消：框不关、说清原因、没有硬把这次操作当成功
    expect(dialog()).not.toBeNull();
    expect(document.body.textContent).toContain("提权没有完成");
    expect(privilegedOp).not.toHaveBeenCalled();
    expect(button("保持现状")).toBeTruthy();

    // 再点一次就重放
    await click(button("再次提权并同意") ?? null);

    expect(requestPrivilegedWorker).toHaveBeenCalledTimes(2);
    expect(privilegedOp).toHaveBeenCalledWith("activate", { slot: "hots", variantId: "v1" });
    expect(dialog()).toBeNull();
  });

  it("助手重放本身失败：把后端的原话显示出来，还能再试", async () => {
    requestPrivilegedWorker.mockResolvedValue(true);
    privilegedOp.mockRejectedValueOnce(new Error("助手进程已经退出"));
    privilegedOp.mockResolvedValue(undefined);
    await mount(async () => {
      throw DENIED;
    });

    await click(writeButton());
    await click(button("再次提权并同意") ?? null);

    expect(document.body.textContent).toContain("助手进程已经退出");
    expect(dialog()).not.toBeNull();

    await click(button("再次提权并同意") ?? null);

    expect(privilegedOp).toHaveBeenCalledTimes(2);
    expect(dialog()).toBeNull();
  });
});

describe("「保持现状」：关掉框，这次算失败，界面继续可用", () => {
  it("不拉助手；再点别的写操作照常完成、不再弹框", async () => {
    const denied = vi.fn(async () => {
      throw DENIED;
    });
    const fine = vi.fn(async () => "ok");
    await mountNode(
      createElement(
        "div",
        null,
        createElement(WriteButton, { className: "first", action: denied }),
        createElement(WriteButton, { className: "second", action: fine }),
      ),
    );

    await click(document.querySelector(".first"));
    await click(button("保持现状") ?? null);

    expect(dialog()).toBeNull();
    expect(requestPrivilegedWorker).not.toHaveBeenCalled();
    expect(privilegedOp).not.toHaveBeenCalled();
    expect(document.querySelector(".first")?.textContent).toContain("failed:需要管理员权限");

    // 界面没有被"锁"成只读：另一个写操作直接做完
    await click(document.querySelector(".second"));

    expect(document.querySelector(".second")?.textContent).toBe("done:ok");
    expect(dialog()).toBeNull();
  });
});

describe("提过一次之后，同一批里不再弹框", () => {
  it("后面的写操作被拒时直接交给助手重放", async () => {
    const first = vi.fn(async () => {
      throw DENIED;
    });
    const second = vi.fn(async () => {
      throw DENIED;
    });
    requestPrivilegedWorker.mockImplementation(async () => {
      order.push("request");
      return true;
    });
    privilegedOp.mockImplementation(async (op) => {
      order.push("op:" + op);
      return undefined;
    });
    await mountNode(createElement(BatchButton, { first, second }));

    await click(document.querySelector(".batch"));
    await click(button("再次提权并同意") ?? null);

    expect(order).toEqual(["request", "op:activate", "op:delete_variant"]);
    expect(privilegedOp).toHaveBeenLastCalledWith("delete_variant", {
      slot: "hots",
      variantId: "v1",
    });
    expect(dialog()).toBeNull();
    expect(document.querySelector(".batch")?.textContent).toBe("done");
  });
});
