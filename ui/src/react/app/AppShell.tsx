import { Suspense, lazy, type ComponentType } from "react";

import { BACKDROP } from "../../api/art";
import { showContextMenu } from "../lib/contextMenu";
import { useAppState } from "../state/AppState";
import { Toast } from "../ui";
import { ActivationConflictDialog } from "../views/campaigns/ActivationConflictDialog";
import { CampaignsView } from "../views/campaigns/CampaignsView";
import type { CampaignsHost } from "../views/campaigns/host";
import { CheatsView } from "../views/cheats/CheatsView";
import { CustomView } from "../views/custom/CustomView";
import { InstallView } from "../views/install/InstallView";
import { MergeDialog } from "../views/merge/MergeDialog";
import type { CustomHost } from "../views/custom/host";
import { ModsView } from "../views/mods/ModsView";
import type { ModsHost } from "../views/mods/useMods";
import { RoomView } from "../views/room/RoomView";
import { SavesView } from "../views/saves/SavesView";
import { SettingsView, type SettingsViewProps } from "../views/settings/SettingsView";
import { UpdateNotice } from "../views/update/UpdateNotice";
import { ContextMenu } from "./ContextMenu";
import { Dialogs } from "./Dialogs";
import { TopBar } from "./TopBar";
import { useShellEffects } from "./useShellEffects";

/**
 * 界面方案预览（AGENTS §17）。
 *
 * `?preview=<名字>` 时**内容区**换成 `src/preview/<名字>.tsx`，外壳（蓝色顶栏、
 * 背景、窗口按钮）全都是真的 —— 出方案要在真界面里看。
 *
 * 预览件不入库（.gitignore 里有），所以用 glob 而不是 import：别人克隆下来
 * 没有那个目录，匹配到空，一切照旧；生产构建里这段也是死的。
 */
const previewModules = import.meta.env.DEV
  ? (import.meta.glob("../../preview/*.tsx") as Record<string, () => Promise<{ default: ComponentType }>>)
  : {};
const previewName = import.meta.env.DEV
  ? new URLSearchParams(location.search).get("preview")
  : null;
const previewLoader = previewName
  ? previewModules["../../preview/" + previewName + ".tsx"]
  : undefined;
const PreviewView = previewLoader ? lazy(previewLoader) : null;

/**
 * 外壳：顶栏 / 页签 / 窗口按钮 / 内容区 / 全局弹窗。
 *
 * 六个 React 视图**直接渲染**（原来各挂一座 `ReactIsland`）—— 外壳自己就是 React，
 * 数据从 `AppState` 拿，动作直接调，不必再绕 emit / 自定义事件那条回执通道。
 */
export function AppShell() {
  const app = useAppState();
  const { dragging } = useShellEffects();
  const hasPreview = PreviewView !== null;

  const campaignsHost: CampaignsHost = {
    installation: app.installation.installation,
    slots: app.library.slots,
    libraryRoot: app.installation.libraryRoot,
    loading: app.loading,
    busy: app.busy,
    // 战役页那个导入对话框还是单包（views/campaigns/host.ts 不在本次范围内）：拖进来的话给它第一个
    droppedPackage: app.library.droppedPackages[0] ?? null,
    consumeDropped: () => app.library.setDroppedPackages([]),
    notify: app.notify,
    reveal: app.installation.reveal,
    bootstrap: () => void app.installation.bootstrap(),
    chooseGameDirectory: () => void app.installation.chooseGameDirectory(),
    showMenu: showContextMenu,
    launch: () => void app.library.launch(),
    activate: app.library.requestActivation,
    removeVariant: app.library.removeVariant,
    refresh: () => void app.library.refresh(),
    pendingActivation: app.library.pendingActivation,
    confirmActivation: app.library.confirmActivation,
    cancelActivation: app.library.cancelActivation,
    openCheats: app.nav.openCheats,
    openSettings: () => app.nav.setView("settings"),
    openInstall: () => app.nav.openInstall("campaign"),
  };

  const customHost: CustomHost = {
    openInstall: () => app.nav.openInstall("custom"),
    slots: app.library.slots,
    libraryRoot: app.installation.libraryRoot,
    loading: app.loading,
    busy: app.busy,
    notify: app.notify,
    bootstrap: () => void app.installation.bootstrap(),
    refresh: () => void app.library.refresh(),
    activate: app.library.requestActivation,
    // 自制战役页也要那两道确认（预演 / 冲突）—— 以前这里没有，点了启用像没反应
    pendingActivation: app.library.pendingActivation,
    confirmActivation: app.library.confirmActivation,
    cancelActivation: app.library.cancelActivation,
    removeVariant: app.library.removeVariant,
    showMenu: showContextMenu,
    openCheats: app.nav.openCheats,
  };

  const modsHost: ModsHost = {
    installationRoot: app.installation.installation?.root ?? null,
    notify: app.notify,
    reveal: app.installation.reveal,
    bootstrap: () => void app.installation.bootstrap(),
    showMenu: showContextMenu,
  };

  const settingsProps: SettingsViewProps = {
    notify: app.notify,
    installation: app.installation.installation,
    libraryRoot: app.installation.libraryRoot,
    isDesktop: app.isDesktop,
    installedCampaigns: app.saves.installedCampaigns,
    saves: app.saves.saves,
    saveBackups: app.saves.saveBackups,
    saveIsolation: app.saves.saveIsolation,
    slots: app.library.slots,
    busy: app.busy,
    launcherVersion: app.installation.launcherVersion,
    update: {
      check: app.update.updateCheck,
      staged: app.update.updateStaged,
      progress: app.update.updateProgress,
      logs: app.update.updateLogs,
      checking: app.update.updateChecking,
      downloading: app.update.updateDownloading,
    },
    actions: {
      chooseGameDirectory: () => void app.installation.chooseGameDirectory(),
      reveal: app.installation.reveal,
      collectCampaign: (dir) => void app.saves.collectCampaign(dir),
      toggleSaveIsolation: (enabled) => void app.saves.toggleSaveIsolation(enabled),
      saveCurrentSaves: () => void app.saves.saveCurrentSaves(),
      switchSaveProfile: (name) => void app.saves.switchSaveProfile(name),
      assignSaveProfile: (name, slot) => void app.saves.assignSaveProfile(name, slot),
      // 备份一份存档：外壳的实现要一个"标签"，这里给默认的那个
      backupSaves: () => void app.saves.backupSaves("手动"),
      restoreSaves: (name) => void app.saves.restoreSaves(name),
      refresh: () => {
        void app.saves.refreshSaves();
        void app.saves.refreshInstalled();
      },
      checkUpdate: () => void app.update.checkUpdateNow(false),
      downloadUpdate: () => void app.update.downloadUpdateNow(),
      applyUpdate: () => void app.update.applyUpdateNow(),
      openRoom: app.nav.openRoom,
      openDevPage: app.nav.openDevPage,
    },
  };

  const view = app.nav.view;

  return (
    <div className="app">
      <div className="app-backdrop" />
      <div className="app-wallpaper" style={{ backgroundImage: "url(" + BACKDROP + ")" }} />

      <TopBar preview={hasPreview} />

      {/* 拖拽导入 */}
      {dragging && (
        <div className="dropzone">
          <div className="dropzone__card">
            <div className="dropzone__title">松手即可导入</div>
            <div className="dropzone__text">
              支持 zip / 7z / rar / tar —— 启动器会自动判断它属于哪部战役，
              认不出来会让你选，不会瞎猜。
            </div>
          </div>
        </div>
      )}

      <main className="content">
        {PreviewView ? (
          <Suspense fallback={null}>
            <PreviewView />
          </Suspense>
        ) : (
          // 切页时淡入上移，免得页面像被"啪"地换掉
          <div className="view" key={view}>
            {view === "campaigns" && <CampaignsView {...campaignsHost} />}
            {view === "install" && (
              <InstallView
                entry={app.nav.installEntry}
                slots={app.library.slots}
                droppedPackages={app.library.droppedPackages}
                consumeDropped={() => app.library.setDroppedPackages([])}
                notify={app.notify}
                onBack={() => app.nav.back()}
                onInstalled={(slug) => {
                  void app.library.refresh();
                  // 装完回到进来之前那一处 —— back() 就够了，安装页不需要知道从哪来
                  app.nav.back();
                  void slug;
                }}
              />
            )}
            {view === "custom" && <CustomView {...customHost} />}
            {view === "mods" && <ModsView {...modsHost} />}
            {view === "saves" && (
              <SavesView
                inventory={app.saves.inventory}
                busy={app.busy}
                onScan={() => void app.saves.scanSaves()}
              />
            )}
            {view === "settings" && <SettingsView {...settingsProps} />}
            {view === "room" && <RoomView onBack={() => app.nav.setView("settings")} />}
          </div>
        )}
      </main>

      {/* 提示：挂在 .view **外面** —— 里面那个 view-in 动画会让 .view 变成
          position: fixed 的包含块，提示会掉到屏幕外；切页时也不该丢 */}
      <Toast
        toasts={app.toasts}
        onClose={app.dismiss}
        onPause={app.pauseToasts}
        onResume={app.resumeToasts}
      />

      {/* 启动时发现另一份 data：不冲突的后端直接并，冲突的在这里让用户选 */}
      {app.merge.status && (
        <MergeDialog
          status={app.merge.status}
          choices={app.merge.choices}
          applying={app.merge.applying}
          onChoose={app.merge.setChoice}
          onApply={() => void app.merge.apply()}
          onLater={app.merge.dismiss}
        />
      )}

      {/* 启用时会覆盖另一个已启用战役：先问一句。**这个框出现时一个字节都还没写盘** */}
      {app.library.pendingConflict &&
        (() => {
          const pending = app.library.pendingConflict;
          const slot = app.library.slots.find((item) => item.slug === pending.slot);
          const variant = slot?.variants.find((item) => item.id === pending.variantId);
          const other = pending.conflicts[0];
          return (
            <ActivationConflictDialog
              conflicts={pending.conflicts}
              newCampaign={variant?.name ?? slot?.display_name ?? pending.slot}
              otherCampaign={other ? other.slot_name + " · " + other.variant_name : ""}
              busy={app.busy}
              onUseNew={() => void app.library.confirmConflict()}
              onKeepOther={app.library.cancelConflict}
            />
          );
        })()}

      {/* 作弊码：战役页 / 自制战役页的「小工具」里弹出来 */}
      {app.nav.cheatsOpen && <CheatsView notify={app.notify} onClose={app.nav.closeCheats} />}

      {/* 全局右键菜单：任何地方调 showContextMenu() 就能弹 */}
      <ContextMenu />

      {/* 启动时的更新公告（渲染 Release 正文的 Markdown）；出方案时别挡住内容区 */}
      {!hasPreview && (
        <UpdateNotice
          visible={app.update.updateNoticeVisible}
          downloading={app.update.updateDownloading}
          muted={app.update.updateNoticeMuted}
          release={app.update.updateCheck?.latest ?? null}
          onDismiss={app.update.dismissUpdateNotice}
          onMute={app.update.muteThisVersion}
          onMutedChange={app.update.setUpdateNoticeMuted}
          onDownload={() => void app.update.downloadFromNotice()}
          onNotify={app.notify}
        />
      )}

      <Dialogs preview={hasPreview} />
    </div>
  );
}
