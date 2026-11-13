import { createRoot } from "react-dom/client";
import { act } from "react-dom/test-utils";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SlotView, Variant } from "../../../api/types";
import { WheelView, type WheelViewProps } from "./WheelView";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const mounted: Array<() => Promise<void>> = [];
const originalMatchMedia = Object.getOwnPropertyDescriptor(window, "matchMedia");

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(async () => {
  for (const unmount of mounted.splice(0)) await unmount();
  document.body.innerHTML = "";
  vi.useRealTimers();
  if (originalMatchMedia) Object.defineProperty(window, "matchMedia", originalMatchMedia);
  else delete (window as unknown as { matchMedia?: unknown }).matchMedia;
});

function variant(id: string, name: string): Variant {
  return {
    id,
    name,
    author: null,
    version: null,
    description: null,
    format: "miyin",
    imported_at: 0,
    source: null,
    map_count: 0,
    mod_count: 0,
    size_bytes: 0,
    target_sub: null,
    cover: null,
    tags: [],
    registration_id: null,
    declared_mods: [],
    doc: null,
  };
}

function slot(
  slug: string,
  displayName: string,
  variants: Variant[],
  active: string | null = null,
): SlotView {
  return {
    slug,
    display_name: displayName,
    sub_directory: null,
    variants,
    active,
    active_name: null,
    vanilla_enabled: false,
    notice: null,
  };
}

const TWO = [slot("wol", "自由之翼", [variant("a", "甲"), variant("b", "乙")])];

async function renderView(props: Partial<WheelViewProps> = {}): Promise<HTMLDivElement> {
  const host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<WheelView slots={TWO} {...props} />);
  });
  mounted.push(async () => {
    await act(async () => {
      root.unmount();
    });
  });
  return host;
}

function spinButton(host: HTMLElement): HTMLButtonElement {
  const button = host.querySelector<HTMLButtonElement>(".wheel-page__actions button");
  if (!button) throw new Error("没有抽取按钮");
  return button;
}

function pointer(host: HTMLElement): HTMLElement {
  const element = host.querySelector<HTMLElement>(".wheel-pointer");
  if (!element) throw new Error("没有指针");
  return element;
}

describe("WheelView", () => {
  it("分类在轮盘区域之外，轮盘和抽取按钮保持静止", async () => {
    const host = await renderView({ random: () => 0.99 });
    expect(
      [...host.querySelectorAll(".wheel-page__categories button")].map((item) => item.textContent),
    ).toEqual(["已有战役", "战役数据库"]);
    expect(host.querySelector("h2")?.textContent).toBe("战役转盘");
    expect(host.querySelector("img.wheel-pointer__img")?.getAttribute("src")).toBe(
      "/wheel/dajiaozhu-pointer.png",
    );
    expect(spinButton(host).textContent).toBe("抽取");
  });

  it("抽取后指针指向中奖扇区，并自动启用中奖版本", async () => {
    const activate = vi.fn(async () => true);
    const host = await renderView({ random: () => 0.99, activate });

    await act(async () => {
      spinButton(host).click();
    });

    // 2 个候选，winner=1 的中心在 270°；4 圈后 1710°，减指针图朝向的 180°
    expect(pointer(host).style.transform).toBe("rotate(1530deg)");

    await act(async () => {
      vi.advanceTimersByTime(4100);
      await Promise.resolve();
    });

    expect(pointer(host).style.transform).toBe("rotate(90deg)");
    expect(activate).toHaveBeenCalledWith("wol", "b");
  });

  it("关闭自动应用后弹出启用询问", async () => {
    const activate = vi.fn(async () => true);
    const host = await renderView({ random: () => 0.99, activate });

    const off = host.querySelector<HTMLInputElement>('input[name="wheel-auto-apply"]');
    await act(async () => {
      off?.click();
    });
    await act(async () => {
      spinButton(host).click();
    });
    await act(async () => {
      vi.advanceTimersByTime(4100);
    });

    expect(document.querySelector(".sheet__title")?.textContent).toBe("启用「乙」？");

    const confirm = [...document.querySelectorAll<HTMLButtonElement>(".sheet__actions button")].find(
      (item) => item.textContent === "启用",
    );
    await act(async () => {
      confirm?.click();
      await Promise.resolve();
    });

    expect(activate).toHaveBeenCalledWith("wol", "b");
  });

  it("剔除已抽取后候选不足两个时禁用抽取", async () => {
    const host = await renderView({ random: () => 0.99 });
    const on = host.querySelector<HTMLInputElement>('input[name="wheel-exclude-drawn"]');
    await act(async () => {
      on?.click();
    });
    await act(async () => {
      spinButton(host).click();
    });
    await act(async () => {
      vi.advanceTimersByTime(4100);
    });

    expect(spinButton(host).disabled).toBe(true);
  });

  it("减少动态效果时很快恢复按钮", async () => {
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      writable: true,
      value: () => ({ matches: true }),
    });

    const host = await renderView({ random: () => 0 });
    await act(async () => {
      spinButton(host).click();
    });
    await act(async () => {
      vi.advanceTimersByTime(70);
    });

    expect(spinButton(host).disabled).toBe(false);
  });

  it("默认剔除原版战役", async () => {
    const withVanilla = {
      ...slot("wol", "自由之翼", [variant("a", "甲"), variant("b", "乙")]),
      vanilla_enabled: true,
    };
    const host = await renderView({ slots: [withVanilla] });
    await act(async () => {});

    const vanilla = host.querySelector<HTMLInputElement>(
      'input[name="wheel-campaign-wol:__vanilla__"]',
    );
    expect(vanilla?.checked).toBe(false);
    expect(host.querySelectorAll(".wheel-disc path")).toHaveLength(2);
  });

  it("数据库抽到后显示资料卡，下载源为可点击链接", async () => {
    const database = [
      {
        id: "db-a",
        title: "数据战役甲",
        type: "custom" as const,
        typeLabel: "自制战役",
        authors: ["作者甲"],
        description: "描述甲",
        download_sources: ["https://pan.example.com/s/abc"],
        author_links: ["https://example.com/author"],
      },
      {
        id: "db-b",
        title: "数据战役乙",
        type: "custom" as const,
        typeLabel: "自制战役",
        authors: [],
        description: "",
        download_sources: [],
        author_links: [],
      },
    ];
    const host = await renderView({ database, random: () => 0 });
    const tab = [...host.querySelectorAll<HTMLButtonElement>(".wheel-page__categories button")].find(
      (item) => item.textContent === "战役数据库",
    );

    await act(async () => {
      tab?.click();
    });
    await act(async () => {});

    expect(host.querySelector(".wheel-disc path")).not.toBeNull();
    expect(host.querySelector<HTMLInputElement>('input[name="wheel-auto-apply"]')).toBeNull();

    await act(async () => {
      spinButton(host).click();
    });
    await act(async () => {
      vi.advanceTimersByTime(4100);
    });

    expect(document.querySelector(".sheet__title")?.textContent).toBe("数据战役甲");
    const link = document.querySelector<HTMLAnchorElement>(".campaign-card__link");
    expect(link?.getAttribute("href")).toBe("https://pan.example.com/s/abc");
  });

  it("少于两个候选时按钮禁用", async () => {
    const host = await renderView({
      slots: [slot("wol", "自由之翼", [variant("a", "甲")])],
    });

    expect(spinButton(host).disabled).toBe(true);
  });

  it("超过 10 个候选时轮盘与左栏使用同一套编号", async () => {
    const variants = Array.from({ length: 11 }, (_, index) => variant("v" + index, "战役" + index));
    const host = await renderView({ slots: [slot("wol", "自由之翼", variants)] });

    const wheelLabels = [...host.querySelectorAll(".wheel-disc text")].map((item) =>
      [...item.childNodes]
        .filter((node) => node.nodeType === 3)
        .map((node) => node.textContent ?? "")
        .join(""),
    );
    const indices = [...host.querySelectorAll(".wheel-index")].map((item) => item.textContent);

    expect(wheelLabels).toEqual(Array.from({ length: 11 }, (_, index) => String(index + 1)));
    expect(indices).toEqual(Array.from({ length: 11 }, (_, index) => String(index + 1)));
  });

  it("具体战役分组默认折叠", async () => {
    const host = await renderView();

    const groups = host.querySelectorAll<HTMLDetailsElement>(".wheel-filter__list details");
    expect(groups).toHaveLength(1);
    expect(groups[0].open).toBe(false);
  });

  it("取消战役类型筛选后按钮禁用", async () => {
    const host = await renderView();
    const type = host.querySelector<HTMLInputElement>('input[name="wheel-type-wol"]');

    await act(async () => {
      type?.click();
    });

    expect(spinButton(host).disabled).toBe(true);
  });
});
