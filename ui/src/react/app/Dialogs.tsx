import { Button } from "../ui";
import { useAppState } from "../state/AppState";

/**
 * 全局弹窗宿主。
 *
 * - **要重启了**（更新下载完）：说明为什么必须重启，给「稍后 / 立即重启并安装」
 * - **外部站点**：先问一句再交给系统浏览器，别默默跳走
 *
 * 两个弹窗原来都写在 App.vue 的模板里；搬到 React 后仍然只有一份实现。
 */
export function Dialogs({ preview }: { preview: boolean }) {
  const app = useAppState();
  const site = app.sites.sitePending;

  return (
    <>
      {/* 更新下载完成后提示重启（出方案时不挡内容区） */}
      {!preview && app.update.showRestartPrompt && (
        <div className="sheet">
          <div className="sheet__card">
            <div className="sheet__badge">
              <svg className="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 3v10" />
                <path d="M8 9l4 4 4-4" />
                <path d="M4 16v2a3 3 0 0 0 3 3h10a3 3 0 0 0 3-3v-2" />
              </svg>
            </div>
            <h3 className="sheet__title">更新已下载完成</h3>
            <p className="sheet__text">
              需要重启启动器才能完成安装 —— Windows 上没法覆盖正在运行的程序。
              <br />
              重启后版本会变成 <strong>{app.update.updateStaged?.version}</strong>，
              你导入的战役、补丁和设置都不会受影响。
            </p>
            <div className="sheet__actions">
              <Button onClick={() => app.update.setShowRestartPrompt(false)}>稍后</Button>
              <Button variant="primary" onClick={() => void app.update.applyUpdateNow()}>
                立即重启并安装
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* 外部站点：先问一句再跳 */}
      {site && (
        <div
          className="sheet"
          onClick={(event) => {
            if (event.target === event.currentTarget) app.sites.cancelSite();
          }}
        >
          <div className="sheet__card">
            <h3 className="sheet__title">用浏览器打开{site.name}？</h3>
            <p className="sheet__text">
              会离开启动器，用系统默认浏览器打开 <strong>{site.host}</strong>。
              {app.sites.siteProbing ? (
                "（正在确认能不能走 HTTPS…）"
              ) : (
                <>
                  （走{" "}
                  <strong>{app.sites.siteResolved.startsWith("https") ? "HTTPS" : "HTTP"}</strong>）
                </>
              )}
            </p>
            <div className="sheet__actions">
              <Button onClick={app.sites.cancelSite}>取消</Button>
              <Button variant="primary" disabled={app.sites.siteProbing} onClick={app.sites.confirmSite}>
                打开网站
              </Button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
