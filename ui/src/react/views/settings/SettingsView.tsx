import { useEffect } from "react";

import { MIYIN } from "../../../api/art";
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
import { Button } from "../../ui";
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

  useEffect(() => {
    actions.refresh();
    // 只在挂载时刷一次：actions 每次渲染都是新对象
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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
          <p className="hint">尚未设置游戏目录。启动器会先尝试从注册表自动发现。</p>
        )}
      </section>

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">战役库</h3>
          <Button onClick={() => actions.reveal(props.libraryRoot)}>打开库目录</Button>
        </header>
        <p className="path">{props.libraryRoot || "（未初始化）"}</p>
        <p className="hint">
          导入的每个玩家版本都完整保存在这里（软件同级的 data 目录），与游戏目录解耦。
          同一个战役可以并存多个版本，切换时只启用选中的那一份。
        </p>
      </section>

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">安装与安全</h3>
        </header>
        <ul className="notes">
          <li>
            <strong>导入前先看一遍。</strong>
            打开压缩包时只做检查 —— 确认里面是不是战役包、有没有危险的文件路径、
            解开后有多大。这一步不写任何文件。
          </li>
          <li>
            <strong>出错不会留下半成品。</strong>
            内容先解到临时目录，确认完整了才正式入库；中途失败会自动清理干净。
          </li>
          <li>
            <strong>切换只动自己放的文件。</strong>
            启用某个版本时，放进去的每个文件都会记账；切回原版时只删这些文件。
            遇到同名的官方文件会先备份，切回时原样还原 —— 不会去删官方目录。
          </li>
          <li>
            <strong>只往游戏目录里写。</strong>
            所有写入都限制在游戏的 Maps / Mods 等目录之内，不会碰别的地方。
          </li>
          <li>
            <strong>中文不会乱码。</strong>
            包里的说明文字会自动识别编码，简繁中文的战役包都能正常显示。
          </li>
        </ul>
      </section>

      {props.installedCampaigns.length > 0 && (
        <section className="card panel">
          <header className="panel__head">
            <h3 className="panel__title">游戏目录里已经装着的</h3>
            <span className="version">{props.installedCampaigns.length} 个</span>
          </header>
          <p className="hint">
            这些是直接扔进 <code>Maps/CustomCampaigns</code> 的战役，还没进库 —— 收编之后就能用启动器管版本了。
            <strong>收编不会删原目录。</strong>
          </p>
          <ul className="plist">
            {props.installedCampaigns.map((item) => (
              <li key={item.dir} className="pitem">
                <span className="pitem__name">{item.name}</span>
                <span className="tag">
                  {item.files} 个文件 · {(item.bytes / 1024 / 1024).toFixed(1)} MB
                </span>
                <Button onClick={() => actions.collectCampaign(item.dir)}>收进库</Button>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">存档</h3>
          <span className="version">我的文档 / StarCraft II / Banks</span>
        </header>
        {saves?.missing ? (
          <p className="hint">还没有存档 —— 玩过一关之后这里才会有东西。</p>
        ) : saves ? (
          <p className="hint">
            现在有 {saves.files.length} 个存档文件，共 {(saves.bytes / 1024).toFixed(0)} KB。
            {saveIsolation.enabled && saveIsolation.active && <> 正在用「{saveIsolation.active}」这一份。</>}
          </p>
        ) : null}

        {/* 按战役隔离：默认关着，打开时会把当前进度收成「原版」 */}
        <div className="isolation">
          <label className="isolation__switch">
            <input
              type="checkbox"
              checked={saveIsolation.enabled}
              disabled={busy}
              onChange={(event) => actions.toggleSaveIsolation(event.target.checked)}
            />
            <span>按战役隔离存档</span>
          </label>
          <p className="hint">
            打开之后，每个战役用自己的一份存档 —— 换战役不会把上一部的进度冲掉。
            <strong>打开这一刻，现在这份会存成「原版」</strong>，不会被谁顶掉。
            关掉只是不再自动切，已经存下的组都留着。
          </p>
        </div>

        <p className="about__actions">
          <Button variant="tonal" disabled={!saves || saves.missing} onClick={actions.saveCurrentSaves}>
            把现在这份存起来
          </Button>
          <Button disabled={!saves || saves.missing} onClick={actions.backupSaves}>
            另外备份一份
          </Button>
        </p>

        {saveBackups.length > 0 && (
          <>
            <p className="hint">
              已存档 {saveBackups.length} 份。切过去、或者把某一份<strong>指给</strong>别的战役。
              切换时会先把现在这份存回去。
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
                    <option value="">不算任何战役</option>
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
                    {saveIsolation.active === item.name ? "正在用" : "切过去"}
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

      <section className="card panel">
        <header className="panel__head">
          <h3 className="panel__title">开发者页</h3>
          <span className="version">实验</span>
        </header>
        <p className="hint">
          给包作者的工作台：扫游戏目录、勾选要打进包的内容、按行读文本文件。
          地图和模组这类二进制会明确告诉你打不开、可以用什么打开。
        </p>
        <p className="hint">版本管理（提交 / 回滚 / 日志）还没接，界面里那一块标着「示例」。</p>
        <p className="about__actions">
          <Button variant="tonal" onClick={actions.openDevPage}>
            打开开发者页
          </Button>
        </p>
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
            <span className="version">v0.1.0</span>
          </header>
          <p className="hint">
            弥音启动器（MiYin Launcher）—— 用 Rust 编写的星际争霸 II 战役与 Mod 管理器。
            美术风格参考 HMCL；战役格式兼容 CCM 与弥音标准包。
          </p>
          <p className="hint">
            当前运行模式：{props.isDesktop ? "桌面版（Tauri）" : "浏览器演示模式（数据为示例）"}。
          </p>
        </div>
      </section>
    </div>
  );
}
