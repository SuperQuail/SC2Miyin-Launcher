import { useEffect, useState } from "react";

import { MIYIN } from "../../api/art";
import { api } from "../../api/bridge";
import { useAppState } from "../state/AppState";
import type { ViewId } from "../state/flags";
import { RESOURCE_SITE } from "../state/useSiteLinks";

const TABS: { id: ViewId; label: string }[] = [
  { id: "campaigns", label: "战役" },
  // 自制战役是**另一个顶层选项**，不是「战役」里的一个分组 ——
  // 两类的玩法根本不同（一个由游戏驱动，一个得用编辑器打开）
  { id: "custom", label: "自制战役" },
  { id: "mods", label: "模组" },
  { id: "settings", label: "设置" },
];

/** 主题三档各自的图标：跟随系统 = 显示器，日间 = 太阳，夜间 = 月亮。 */
function ThemeIcon({ mode, theme }: { mode: string; theme: string }) {
  if (mode === "system") {
    return (
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <rect x="2" y="3" width="12" height="8" rx="1.2" />
        <path d="M5.5 13.5h5" />
      </svg>
    );
  }
  if (theme === "dark") {
    return (
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M13 9.5A5.5 5.5 0 0 1 6.5 3a5.5 5.5 0 1 0 6.5 6.5Z" />
      </svg>
    );
  }
  return (
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <circle cx="8" cy="8" r="3" />
      <path d="M8 1v1.8M8 13.2V15M1 8h1.8M13.2 8H15M3.2 3.2l1.3 1.3M11.5 11.5l1.3 1.3M12.8 3.2l-1.3 1.3M4.5 11.5l-1.3 1.3" />
    </svg>
  );
}

/**
 * 顶栏，同时是标题栏：空白处按住可以拖窗口。
 *
 * 我们关掉了系统边框（`decorations: false`），所以最小化 / 最大化 / 关闭得自己画。
 * 浏览器演示模式下这几条命令是桩，按钮照常显示（和外壳原来一样）。
 */
export function TopBar({ preview }: { preview: boolean }) {
  const app = useAppState();
  const [maximized, setMaximized] = useState(false);

  async function syncMaximized(): Promise<void> {
    try {
      setMaximized(await api.windowIsMaximized());
    } catch {
      // 拿不到就当没最大化
    }
  }

  useEffect(() => {
    if (!app.isDesktop) return;
    void syncMaximized();
    // 双击标题栏 / Win+↑ 是系统改的，只能靠 resize 补上
    const onResize = () => void syncMaximized();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
    // 只在桌面壳里挂；syncMaximized 每次渲染都是新的，不进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [app.isDesktop]);

  async function toggleMaximize(): Promise<void> {
    try {
      setMaximized(await api.windowToggleMaximize());
    } catch {
      // 忽略
    }
  }

  /**
   * 按住顶栏拖动窗口。
   *
   * 本来 `data-tauri-drag-region` 就够了，但它有个坑：**只认事件落在带这个属性的
   * 元素本身**。点了里面带文字的 span、或任何子元素，就不响应 —— 用户的感觉就是
   * 「有时候能拖有时候不能」。所以自己接管：落点不在按钮/链接/输入框/窗口控件上就调
   * 系统的拖动。
   */
  function startWindowDrag(event: React.MouseEvent): void {
    if (event.button !== 0) return;

    const target = event.target as HTMLElement | null;
    if (target?.closest("button, a, input, select, textarea, .winctl")) return;

    void (async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        await getCurrentWindow().startDragging();
      } catch {
        // 浏览器演示模式没有这个能力，忽略
      }
    })();
  }

  const themeTitle =
    app.theme.mode === "system"
      ? "跟随系统（点一下切到日间）"
      : app.theme.mode === "light"
        ? "日间（点一下切到夜间）"
        : "夜间（点一下切回跟随系统）";

  return (
    <header className="topbar" data-tauri-drag-region onMouseDown={startWindowDrag}>
      <div className="brand" data-tauri-drag-region>
        <span className="brand__avatar">
          <img src={MIYIN.chibi} alt="弥音" />
        </span>
        <div className="brand__text">
          <span className="brand__name">弥音启动器</span>
          <span className="brand__sub">MiYin Launcher</span>
        </div>
      </div>

      {preview ? (
        <span className="tab tab--active">预览</span>
      ) : (
        <nav className="tabs" data-tauri-drag-region>
          {TABS.map((tab) => (
            <button
              key={tab.id}
              className={"tab" + (app.nav.view === tab.id ? " tab--active" : "")}
              type="button"
              onClick={() => app.nav.setView(tab.id)}
            >
              {tab.label}
            </button>
          ))}
          {/* 资源网站：**不是视图**，点了也不切页 —— 先问一句，确认了才交给系统浏览器 */}
          <button
            className="tab tab--site"
            type="button"
            title="资源网站（用系统默认浏览器打开）"
            onClick={() => void app.sites.requestSite(RESOURCE_SITE)}
          >
            资源网站
          </button>
          {/* 开发者页是独立入口（整页跳过去），不是这个 SPA 里的视图 */}
          <button
            className="tab tab--dev"
            type="button"
            title="开发者页（实验）：扫游戏目录、看文件、记版本"
            onClick={app.nav.openDevPage}
          >
            开发
            <span className="tab__badge">实验</span>
          </button>
        </nav>
      )}

      <div className="topbar__right">
        <button className="themectl" type="button" title={themeTitle} onClick={app.theme.cycleTheme}>
          <ThemeIcon mode={app.theme.mode} theme={app.theme.theme} />
        </button>
        {app.update.updateAvailable && (
          <button
            className="update-badge"
            type="button"
            title="有新版本可用，点开设置页查看"
            onClick={() => app.nav.setView("settings")}
          >
            {/* 下载图标：向下箭头落进托盘，用户一眼就知道是"可以下载新版本" */}
            <svg className="update-badge__icon" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M8 1.5v7.5" />
              <path d="M5 6.5 8 9.5l3-3" />
              <path d="M2.5 11v1.5A2 2 0 0 0 4.5 14.5h7a2 2 0 0 0 2-2V11" />
            </svg>
            <span>新版本 {app.update.updateAvailable}</span>
          </button>
        )}
        {!app.isDesktop && <span className="tag tag--demo">演示模式</span>}
        {/* 启动器自己的版本：以前这里只显示游戏构建号，很容易被当成启动器版本 */}
        {app.installation.launcherVersion && (
          <span className="tag tag--app" title="弥音启动器版本">
            v{app.installation.launcherVersion}
          </span>
        )}
        {app.installation.installation && (
          <span className="tag" title="星际争霸 II 版本">
            SC2 {app.installation.installation.version}
          </span>
        )}
      </div>

      {/* 自绘的窗口按钮：系统边框已经关掉了 */}
      <div className="winctl">
        <button
          className="winctl__btn"
          type="button"
          title="最小化"
          onClick={() => void api.windowMinimize()}
        >
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M2.5 6h7" />
          </svg>
        </button>
        <button
          className="winctl__btn"
          type="button"
          title={maximized ? "还原" : "最大化"}
          onClick={() => void toggleMaximize()}
        >
          {maximized ? (
            <svg viewBox="0 0 12 12" aria-hidden="true">
              <rect x="2.5" y="4.5" width="5" height="5" rx="1" />
              <path d="M4.5 4.5v-2h5v5h-2" />
            </svg>
          ) : (
            <svg viewBox="0 0 12 12" aria-hidden="true">
              <rect x="2.5" y="2.5" width="7" height="7" rx="1.5" />
            </svg>
          )}
        </button>
        <button
          className="winctl__btn winctl__btn--close"
          type="button"
          title="关闭"
          onClick={() => void api.windowClose()}
        >
          <svg viewBox="0 0 12 12" aria-hidden="true">
            <path d="M3 3l6 6" />
            <path d="M9 3l-6 6" />
          </svg>
        </button>
      </div>
    </header>
  );
}
