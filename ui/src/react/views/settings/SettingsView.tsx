import { useEffect, useState } from "react";

import { MIYIN } from "../../../api/art";
import { api } from "../../../api/bridge";
import type {
  InstalledCampaign,
  Installation,
  SaveBackup,
  SaveIsolation,
  SaveSet,
  SlotView,
  Staged,
  UpdateCheck,
  UpdateProgress,
} from "../../../api/types";
import { useDevMode } from "../../lib/devMode";
import { usePermissionGate } from "../../state/usePermission";
import { Button, Sheet } from "../../ui";
import { ToolsPanel } from "./ToolsPanel";
import { UpdatePanel } from "./UpdatePanel";
import "./settings.css";

/** 外壳递进来的数据与动作。React 侧不直接读外壳的全局状态。 */
export interface SettingsViewProps {
  notify: (kind: "success" | "error", text: string) => void;
  installation: Installation | null;
  libraryRoot: string;
  isDesktop: boolean;
  installedCampaigns: InstalledCampaign[];
  saves: SaveSet | null;
  saveBackups: SaveBackup[];
  saveIsolation: SaveIsolation;
  /** 隔离状态文件读不出来时的提示（后端 `Isolation.warning`），没有就是 null。 */
  saveIsolationWarning?: string | null;
  slots: SlotView[];
  busy: boolean;
  launcherVersion: string;
  update: {
    check: UpdateCheck | null;
    staged: Staged | null;
    progress: UpdateProgress | null;
    logs: string[];
    checking: boolean;
    downloading: boolean;
  };
  actions: {
    chooseGameDirectory: () => void;
    reveal: (path: string) => void;
    collectCampaign: (dir: string) => void;
    toggleSaveIsolation: (enabled: boolean) => void;
    saveCurrentSaves: () => void;
    switchSaveProfile: (name: string) => void;
    assignSaveProfile: (name: string, slot: string | null) => void;
    backupSaves: () => void;
    restoreSaves: (name: string) => void;
    /** 挂载时刷新存档与「游戏目录里已经装着的」。 */
    refresh: () => void;
    checkUpdate: () => void;
    downloadUpdate: () => void;
    applyUpdate: () => void;
    /** 进开发者房间（控件的展示台）。 */
    openRoom: () => void;
    /** 进开发者页（ide.html，独立的 React 入口）。 */
    openDevPage: () => void;
  };
}

/**
 * 设置页（React 版，从 `views/SettingsView.vue` 整块迁过来）。
 *
 * 页面本身几乎全是说明文字与列表，真正的动作都由外壳提供 ——
 * 这里只负责把它们摆出来。原来嵌在里面的两个 Vue 面板
 * （可选工具、更新）也一并迁了过来。
 */
export function SettingsView(props: SettingsViewProps) {
  const { installation, saves, saveIsolation, saveBackups, busy, actions } = props;
  const [devMode, setDevMode] = useDevMode();
  /** 提权闸门：「全部停用」也是写盘动作。 */
  const permission = usePermissionGate();
  /** 开启隔离要二次确认：它会按战役整套替换 Banks，而这块仍是 WIP。 */
  const [confirmIsolation, setConfirmIsolation] = useState(false);
  /** 「全部停用」是不可逆的写盘动作，先问一句。 */
  const [confirmDeactivate, setConfirmDeactivate] = useState(false);

  useEffect(() => {
    actions.refresh();
    // 只在挂载时刷一次：actions 每次渲染都是新对象
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** 一键撤掉启动器写进游戏目录的全部战役文件（不删库里的内容）。 */
  async function deactivateAll(): Promise<void> {
    setConfirmDeactivate(false);
    try {
      const warnings = await permission.run(() => api.deactivateAll(), {
        op: "deactivate_all",
        args: {},
      });
      actions.refresh();
      if (warnings?.length) {
        props.notify("success", "已停用全部战役；另有 " + warnings.length + " 条提示：" + warnings.join("；"));
      } else {
        props.notify("success", "已停用全部战役，游戏目录已恢复为原版。");
      }
    } catch (error) {
      props.notify("error", String(error));
    }
  }

  const rows = [
    { label: "安装根目录", value: installation?.root },
    { label: "游戏启动器", value: installation?.executable },
    { label: "版本切换器", value: installation?.switcher },
    { label: "官方战役目录", value: installation?.campaign_maps_root },
    { label: "自制战役目录", value: installation?.custom_campaigns_root },
    { label: "模组目录", value: installation?.mods_root },
    { label: "战役存档目录", value: installation?.banks_root },
    { label: "用户文档目录", value: installation?.documents_root },
  ].filter((row) => Boolean(row.value));

  return (
    <div className="page">
      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">游戏目录</h3>
          <div className="panel__actions">
            <Button variant="outline" onClick={actions.chooseGameDirectory}>
              重新选择
            </Button>
            {installation && <Button onClick={() => actions.reveal(installation.root)}>打开目录</Button>}
          </div>
        </header>

        {rows.length > 0 ? (
          <dl className="rows">
            {rows.map((row) => (
              <div key={row.label} className="row">
                <dt>{row.label}</dt>
                <dd title={row.value ?? ""}>{row.value}</dd>
              </div>
            ))}
          </dl>
        ) : (
          <p className="hint">尚未设置游戏目录。启动器会先尝试从注册表自动检测。</p>
        )}
      </section>

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">战役库</h3>
          <Button onClick={() => actions.reveal(props.libraryRoot)}>打开库目录</Button>
        </header>
        <p className="path">{props.libraryRoot || "（未初始化）"}</p>
        <p className="hint">
          导入的每个版本都完整保存在战役库中（位于游戏安装目录同级），与游戏目录分离。
          同一个战役可以并存多个版本，切换时只启用选中的那一份。
        </p>
      </section>

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">安装与安全</h3>
        </header>
        <ul className="notes">
          <li>
            <strong>导入前可先预检。</strong>
            打开压缩包时只做检查：确认是否为战役包、是否包含危险路径、解压后的大小。此步骤不写入任何文件。
          </li>
          <li>
            <strong>失败不会留下未完成的数据。</strong>
            内容先解压到临时目录，校验完整后才写入战役库；中途失败会自动清理。
          </li>
          <li>
            <strong>切换只操作启动器写入的文件。</strong>
            启用某个改版时，逐个记录写入的文件；切回原版时只删除这些文件。
            遇到同名原版文件会先备份，切回时还原。不会删除原版目录中的文件。
          </li>
          <li>
            <strong>只写入游戏安装目录。</strong>
            所有写入都限制在游戏安装目录内，不会修改其他位置。
          </li>
          <li>
            <strong>中文编码自动识别。</strong>
            包内说明文字会自动识别编码，简繁中文均可正常显示。
          </li>
        </ul>
      </section>

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">维护</h3>
        </header>
        <p className="hint">
          一键撤掉启动器写进游戏目录的全部战役文件（历史上多装 / 半装的也一起清），
          索引与存档组回到原版。游戏正在运行时会被拒绝；被挪走过的原始文件会按清单还原。
          <strong>不会删除战役库里的内容。</strong>
        </p>
        <Button variant="outline" disabled={busy} onClick={() => setConfirmDeactivate(true)}>
          全部停用并还原游戏目录
        </Button>
      </section>

      {props.installedCampaigns.length > 0 && (
        <section className="card panel">
          <header className="panel__head">
            <h3 className="panel__title">游戏目录中已存在的战役</h3>
            <span className="version">{props.installedCampaigns.length} 个</span>
          </header>
          <p className="hint">
            这些是直接放入 <code>Maps/CustomCampaigns</code> 的战役，尚未加入战役库。加入战役库后即可用启动器管理版本。
            <strong>加入战役库不会删除原目录。</strong>
          </p>
          <ul className="plist">
            {props.installedCampaigns.map((item) => (
              <li key={item.dir} className="pitem">
                <span className="pitem__name">{item.name}</span>
                <span className="tag">
                  {item.files} 个文件 · {(item.bytes / 1024 / 1024).toFixed(1)} MB
                </span>
                <Button onClick={() => actions.collectCampaign(item.dir)}>加入战役库</Button>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">存档</h3>
          <span className="version">我的文档 / StarCraft II / Accounts/&lt;账号&gt;/&lt;地域&gt;/Banks</span>
        </header>
        {saves?.missing ? (
          <p className="hint">暂无存档。完成一关后此处将显示存档。</p>
        ) : saves ? (
          <p className="hint">
            当前有 {saves.files.length} 个存档文件，共 {(saves.bytes / 1024).toFixed(0)} KB。
            {saveIsolation.enabled && saveIsolation.active && <> 正在使用「{saveIsolation.active}」分组。</>}
          </p>
        ) : null}

        {/* WIP：这块还没修好（组与战役的对应关系不对）。警示就摆在开关上方 */}
        <div
          className="flex items-start gap-3 rounded-[var(--radius-md)] border border-[color-mix(in_srgb,var(--warning)_45%,transparent)] bg-[var(--warning-soft)] p-4 shadow-sm"
          role="note"
        >
          <svg
            className="mt-0.5 h-[22px] w-[22px] flex-none fill-none stroke-[var(--warning)] [stroke-linecap:round] [stroke-linejoin:round] [stroke-width:1.8]"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            <path d="M12 9v5M12 17.5v.5" />
            <path d="M10.3 3.9 2.6 17a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
          </svg>
          <div className="min-w-0">
            <p className="m-0 mb-1.5 text-[13.5px] font-bold text-[var(--warning)]">存档管理（WIP）</p>
            <p className="m-0 text-[12.5px] leading-relaxed text-on-surface">
              此功能仍在开发中：启用存档隔离可能导致存档损坏，暂时无法正常工作。在修好之前，请不要依赖它保存进度。
            </p>
          </div>
        </div>

        {/* 状态文件坏了：后端不再静默当「关着」，这里把它原样摆出来（别让用户以为分组凭空没了） */}
        {props.saveIsolationWarning && (
          <p
            className="m-0 rounded-[var(--radius-md)] border border-[color-mix(in_srgb,var(--warning)_45%,transparent)] bg-[var(--warning-soft)] p-4 text-[12.5px] leading-relaxed text-on-surface"
            role="alert"
          >
            <strong className="text-[var(--warning)]">存档隔离状态读取失败：</strong>
            {props.saveIsolationWarning}
          </p>
        )}

        {/* 按战役隔离：**默认关着**，用户主动打开才生效。开关只留在这里，分组列表就在下方。 */}
        <div className="isolation">
          <label className="isolation__switch">
            <input
              type="checkbox"
              checked={saveIsolation.enabled}
              disabled={busy}
              onChange={(event) => {
                // 开启会按战役整套替换 Banks，先弹确认；关闭不写盘，直接执行
                if (event.target.checked) setConfirmIsolation(true);
                else actions.toggleSaveIsolation(false);
              }}
            />
            <span>按战役隔离存档</span>
          </label>
          <p className="hint">
            <strong>默认关闭，且暂不可靠</strong>：开启后会尝试为每个战役准备独立的一套存档，切换战役不再覆盖上一部；
            开启时会将当前存档保存为「原版」分组。关闭后启动器不再修改 Banks，已有分组保留。
            分组管理（修改归属 / 切换 / 还原）就在下面的已保存列表里。
          </p>
        </div>

        <p className="about__actions">
          <Button variant="tonal" disabled={!saves || saves.missing} onClick={actions.saveCurrentSaves}>
            保存当前存档
          </Button>
          <Button disabled={!saves || saves.missing} onClick={actions.backupSaves}>
            另存为备份
          </Button>
        </p>

        {saveBackups.length > 0 && (
          <>
            <p className="hint">
              已保存 {saveBackups.length} 份存档。可切换使用，或将某份存档归属到其他战役；切换时会先保存当前存档。
              <strong>这些操作与上面是同一块 WIP 功能，结果可能不正确，请先自行备份。</strong>
            </p>
            <ul className="plist">
              {saveBackups.map((item) => (
                <li key={item.name} className="pitem pitem--save">
                  <span className="pitem__name">{item.label}</span>
                  <span className="tag">
                    {item.files} 个文件 · {(item.bytes / 1024).toFixed(0)} KB
                  </span>
                  <select
                    className="savepick"
                    value={saveIsolation.assignments[item.name] ?? ""}
                    onChange={(event) => actions.assignSaveProfile(item.name, event.target.value || null)}
                  >
                    <option value="">不归属任何战役</option>
                    {props.slots.map((slot) => (
                      <option key={slot.slug} value={slot.slug}>
                        {slot.display_name}
                      </option>
                    ))}
                  </select>
                  <Button
                    disabled={busy || saveIsolation.active === item.name}
                    onClick={() => actions.switchSaveProfile(item.name)}
                  >
                    {saveIsolation.active === item.name ? "正在使用" : "切换使用"}
                  </Button>
                  <Button
                    disabled={busy || saveIsolation.active === item.name}
                    onClick={() => actions.restoreSaves(item.name)}
                  >
                    还原
                  </Button>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>

      {confirmDeactivate && (
        <Sheet
          title="停用全部战役？"
          onClose={() => setConfirmDeactivate(false)}
          actions={
            <>
              <Button onClick={() => setConfirmDeactivate(false)}>取消</Button>
              <Button variant="primary" disabled={busy} onClick={() => void deactivateAll()}>
                全部停用
              </Button>
            </>
          }
        >
          <p className="sheet__text">
            这会从游戏目录移除启动器写入的所有战役文件，并把索引与存档组切回原版；
            被挪走过的原始文件会按安装清单还原。<strong>战役库里的内容不会被删除。</strong>
          </p>
        </Sheet>
      )}

      {/* 开隔离前问一句：它会改写用户存档，而这块仍是 WIP */}
      {confirmIsolation && (
        <Sheet
          title="开启「按战役隔离存档」？"
          onClose={() => setConfirmIsolation(false)}
          actions={
            <>
              <Button onClick={() => setConfirmIsolation(false)}>取消</Button>
              <Button
                variant="primary"
                disabled={busy}
                onClick={() => {
                  setConfirmIsolation(false);
                  actions.toggleSaveIsolation(true);
                }}
              >
                仍要开启
              </Button>
            </>
          }
        >
          <p className="sheet__text">
            <strong>该功能仍在开发中，可能导致存档损坏。</strong>
            开启后启动器会按战役整套替换游戏目录里的 Banks（先清空再铺）：先把当前存档保存为「原版」分组，
            之后切换战役时换成对应分组。在修好之前，请不要依赖它保存进度；如需开启，请先自行备份存档。
          </p>
        </Sheet>
      )}

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">开发者</h3>
          <span className="version">实验</span>
        </header>

        <div className="isolation">
          <label className="isolation__switch">
            <input
              type="checkbox"
              checked={devMode}
              onChange={(event) => setDevMode(event.target.checked)}
            />
            <span>开发者模式</span>
          </label>
          <p className="hint">
            开启后此处会显示两个入口。该设置只保存在本机，更换设备后需重新开启；不会影响普通用户。
          </p>
        </div>

        {devMode ? (
          <>
            <p className="hint">
              <strong>开发者房间</strong>用于展示所有控件：新控件需先在房间中确认外观与交互，之后才能用于正式界面。
            </p>
            <p className="hint">
              <strong>开发者页</strong>是面向包作者的工作台：扫描游戏目录、勾选要打包的内容、按行读取文本文件。
              地图和模组等二进制文件会提示无法直接打开及可用的打开方式。版本管理尚未接入，对应区域标记为「示例」。
            </p>
            <p className="about__actions">
              <Button variant="primary" onClick={actions.openRoom}>
                进入开发者房间
              </Button>
              <Button variant="tonal" onClick={actions.openDevPage}>
                打开开发者页
              </Button>
            </p>
          </>
        ) : (
          <p className="hint">打开「开发者模式」后，这里会出现「开发者房间」与「开发者页」两个入口。</p>
        )}
      </section>

      <ToolsPanel notify={props.notify} />

      <UpdatePanel
        notify={props.notify}
        launcherVersion={props.launcherVersion}
        check={props.update.check}
        staged={props.update.staged}
        progress={props.update.progress}
        logs={props.update.logs}
        checking={props.update.checking}
        downloading={props.update.downloading}
        checkNow={actions.checkUpdate}
        downloadNow={actions.downloadUpdate}
        applyNow={actions.applyUpdate}
      />

      <section className="card panel about">
        <img className="about__art" src={MIYIN.portrait} alt="弥音" />
        <div className="about__body">
          <header className="panel__head">
            <h3 className="panel__title">关于</h3>
            <span className="version">
              {props.launcherVersion ? "v" + props.launcherVersion : "版本未知"}
            </span>
          </header>
          <p className="hint">
            弥音启动器（MiYin Launcher）：星际争霸 II 战役与 Mod 管理器，使用 Rust 编写。
            界面风格参考 HMCL；包格式兼容 CCM 与弥音标准格式。
          </p>
          <p className="hint">
            当前运行模式：{props.isDesktop ? "桌面版" : "浏览器演示模式（数据为示例）"}。
            {" "}构建时间：
            {typeof __MIYIN_BUILD__ === "string" ? __MIYIN_BUILD__ : "开发模式（未打包）"}。
          </p>
        </div>
      </section>
    </div>
  );
}
