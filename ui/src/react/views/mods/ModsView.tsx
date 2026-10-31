import { useState, type MouseEvent as ReactMouseEvent } from "react";

import type { LibraryMod, ModPreview } from "../../../api/types";
import { Button } from "../../ui";
import { originLabel } from "./data";
import { CompareSheet, ChooseModeSheet, EditModSheet, GameModsSection, RemoveSheet } from "./ModSheets";
import { useModEdit, useModImport, useModRemove, useMods, type ModsHost } from "./useMods";
import "./mods.css";

/**
 * 模组管理页（React 版，从 `views/ModsView.vue` 整块迁过来）。
 *
 * 把库里所有战役的模组摊平在一页、一键开关 —— 官方战役的改版包一样可能带模组
 * （换了单位的重制版就是），以前只能进某个战役里一个个勾。
 *
 * 拆成了四块：状态与动作在 `useMods`（外加导入/编辑/删除三个小 hook），
 * 弹窗在 `ModSheets`，纯逻辑在 `data`，这里只管画。
 */
export function ModsView(host: ModsHost) {
  const mods = useMods(host);
  const [choosing, setChoosing] = useState<{ path: string; preview: ModPreview } | null>(null);
  const importer = useModImport(host, mods.reload, setChoosing);
  const editor = useModEdit(host, mods.reload);
  const remover = useModRemove(host, mods.reload);

  const busy = mods.busyKey !== null;

  /** 行上的右键菜单。和行内按钮是同一批功能，只是不用先去找那几个小按钮。 */
  function showModMenu(event: ReactMouseEvent, mod: LibraryMod): void {
    const items = [
      { id: "toggle", label: mod.mounted ? "取消勾选" : "勾选" },
      { id: "edit", label: "编辑信息…" },
      { id: "copy", label: "复制模组名" },
    ];
    // 只有内容在独立库里的才能导出 / 删除 —— 战役包带来的模组，内容归那个包
    if (mod.source_kind === "library") {
      items.push({ id: "export", label: "导出成一个包…" });
      items.push({ id: "remove", label: "从战役库删除" });
    }
    host.showMenu(event, items, (id) => {
      if (id === "toggle") void mods.toggle(mod);
      if (id === "edit") void editor.openEdit(mod);
      if (id === "copy") void mods.copyMount(mod);
      if (id === "export") void mods.exportMod(mod);
      if (id === "remove") remover.setRemoving(mod);
    });
  }

  /** 页面空白处的右键菜单：导入和刷新。 */
  function showPageMenu(event: ReactMouseEvent): void {
    host.showMenu(
      event,
      [
        { id: "import-file", label: "导入模组包…" },
        { id: "import-folder", label: "从文件夹导入…" },
        { id: "reload", label: "重新读取" },
        { id: "open-mods", label: "打开游戏 Mods 目录", disabled: !host.installationRoot },
      ],
      (id) => {
        if (id === "import-file") void importer.importMod("file");
        if (id === "import-folder") void importer.importMod("folder");
        if (id === "reload") mods.reload();
        if (id === "open-mods" && host.installationRoot) host.reveal(host.installationRoot + "\\Mods");
      },
    );
  }

  return (
    <div className="page" onContextMenu={(event) => event.target === event.currentTarget && showPageMenu(event)}>
      <section className="hero">
        <div className="hero__body">
          <p className="hero__eyebrow">
            共 {mods.mods.length} 个，已勾选 {mods.mountedCount} 个
            {mods.standaloneCount > 0 && <> · 其中独立模组 {mods.standaloneCount} 个</>}
          </p>
          <h2 className="hero__title">模组</h2>
          <p className="hero__text">
            地图可能需要模组才能正常打开 —— 原版战役的改版包也一样。
            这里可以查看模组来源、勾选或取消勾选模组，也可以单独导入、导出和编辑信息。
          </p>
        </div>
        <div className="hero__actions">
          {mods.picked.size > 0 && (
            <Button variant="tonal" disabled={busy} onClick={() => void mods.exportPicked()}>
              导出选中的 {mods.picked.size} 个版本
            </Button>
          )}
          <Button disabled={mods.loading} onClick={mods.reload}>
            {mods.loading ? "读取中…" : "重新读取"}
          </Button>
          <Button
            disabled={!host.installationRoot}
            onClick={() => host.installationRoot && host.reveal(host.installationRoot + "\\Mods")}
          >
            打开游戏 Mods 目录
          </Button>
          <Button disabled={importer.importing} onClick={() => void importer.importMod("folder")}>
            从文件夹导入
          </Button>
          <Button variant="primary" disabled={importer.importing} onClick={() => void importer.importMod("file")}>
            {importer.importing ? "导入中…" : "＋ 导入模组包"}
          </Button>
        </div>
      </section>

      {/* 启动器启动时会自己挂载模组，手动改状态容易和自动挂载打架 */}
      <div className="mods-warn" role="note">
        <svg className="mods-warn__icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M12 9v5M12 17.5v.5" />
          <path d="M10.3 3.9 2.6 17a2 2 0 0 0 1.7 3h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0Z" />
        </svg>
        <div className="mods-warn__body">
          <p className="mods-warn__title">模组由启动器管理</p>
          <p className="mods-warn__text">
            启动器会在启动时自动同步已勾选的模组；请勿手动改动游戏目录中的模组文件，否则可能导致资源缺失或战役无法启动。
          </p>
          <p className="mods-warn__foot">
            确有需要时，可在下方列表勾选或取消勾选；开关仅表示勾选状态，已生效状态以游戏目录同步结果为准。
          </p>
        </div>
      </div>

      <div className="bar">
        <input
          className="bar__search"
          type="search"
          value={mods.keyword}
          placeholder="搜索模组或所属战役"
          onChange={(event) => mods.setKeyword(event.target.value)}
        />
        <span className="bar__count">
          {mods.filtered.length} / {mods.mods.length}
        </span>
      </div>

      {mods.loading ? (
        <p className="hint">正在读取…</p>
      ) : mods.mods.length === 0 ? (
        <p className="hint">
          暂无模组。可导入包含 .SC2Mod 的战役包，或使用上方按钮单独导入模组。
        </p>
      ) : mods.filtered.length === 0 ? (
        <p className="hint">没有匹配的模组。</p>
      ) : null}

      {mods.groups.map((group) => (
        <section key={group.key} className="group">
          <header className="group__head">
            <div className="group__title">
              <span className={"origin origin--" + group.origin}>{originLabel(group.origin)}</span>
              <span className="group__campaign">{group.slotName}</span>
              {group.variantName && <span className="group__variant">{group.variantName}</span>}
              {group.origin === "standalone" && group.mods.length > 1 && (
                <span className="badge">{group.mods.length} 个版本</span>
              )}
            </div>
            <div className="group__actions">
              {/* 独立模组是版本互斥的：同一 modid 只能勾选一个（它们在游戏目录里抢同一个文件夹） */}
              {group.origin !== "standalone" && (
                <Button disabled={busy} onClick={() => void mods.toggleGroup(group, true)}>
                  <span className="btn--tiny">全部勾选</span>
                </Button>
              )}
              <Button disabled={busy} onClick={() => void mods.toggleGroup(group, false)}>
                <span className="btn--tiny">全部取消勾选</span>
              </Button>
            </div>
          </header>

          <ul className="mods">
            {group.mods.map((mod) => (
              <li key={mod.path} className="mod" onContextMenu={(event) => showModMenu(event, mod)}>
                <div className="mod__info">
                  {mod.standalone_id && (
                    <input
                      className="mod__pick"
                      type="checkbox"
                      title="勾上：导出时带上这个版本"
                      checked={mods.picked.has(mod.standalone_id)}
                      onChange={() => mods.togglePick(mod.standalone_id as string)}
                    />
                  )}
                  <span className="mod__name">{mod.name}</span>
                  {/* 勾选 ≠ 已生效：in_game 才是真的铺进了 <游戏>/Mods/ */}
                  {mod.mounted && (
                    <span className={"badge " + (mod.in_game ? "badge--live" : "badge--pending")}>
                      {mod.in_game ? "已生效" : "待生效"}
                    </span>
                  )}
                  {mod.required && (
                    <span className="badge badge--req" title="包内声明为作者必需的模组">
                      依赖
                    </span>
                  )}
                  {mod.origin === "standalone" && mod.version && (
                    <span className="badge badge--ver">v{mod.version}</span>
                  )}
                  {mod.parts > 1 && <span className="badge">{mod.parts} 个文件</span>}
                  {/* 把「写入游戏目录时叫什么」显出来：地图里引用的就是这个 */}
                  {mod.folder && (
                    <span className="mount" title="写入游戏目录后的路径">
                      → Mods/{mod.folder}
                      {mod.kind === "folder" ? "/" : ""}
                    </span>
                  )}
                </div>

                <div className="mod__actions">
                  {group.mods.length > 1 && (
                    <select
                      className="mod__base"
                      value={mods.compareWith[mod.modid ?? ""] ?? ""}
                      onChange={(event) =>
                        mods.setCompareWith({ ...mods.compareWith, [mod.modid ?? ""]: event.target.value })
                      }
                    >
                      <option value="">对比基准…</option>
                      {group.mods
                        .filter((item) => item.path !== mod.path)
                        .map((other) => (
                          <option key={other.path} value={other.standalone_id ?? ""}>
                            与 v{other.version ?? "未标注"} 对比
                          </option>
                        ))}
                    </select>
                  )}
                  {mods.compareWith[mod.modid ?? ""] && (
                    <Button disabled={busy} onClick={() => void mods.compare(mod)}>
                      <span className="btn--tiny">对比</span>
                    </Button>
                  )}
                  <Button disabled={busy} onClick={() => void editor.openEdit(mod)}>
                    <span className="btn--tiny">编辑</span>
                  </Button>
                  {mod.source_kind === "library" && (
                    <>
                      <Button disabled={busy} onClick={() => void mods.exportMod(mod)}>
                        <span className="btn--tiny">导出</span>
                      </Button>
                      <Button variant="danger" disabled={busy} onClick={() => remover.setRemoving(mod)}>
                        <span className="btn--tiny">删除</span>
                      </Button>
                    </>
                  )}
                  <button
                    className={"switch" + (mod.mounted ? " switch--on" : "")}
                    type="button"
                    disabled={busy}
                    title={mod.mounted ? "点击取消勾选" : "点击勾选"}
                    onClick={() => void mods.toggle(mod)}
                  >
                    <span className="switch__knob" />
                    <span className="switch__text">{mod.mounted ? "已勾选" : "未勾选"}</span>
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </section>
      ))}

      <GameModsSection entries={mods.gameMods} />

      <CompareSheet comparing={mods.comparing} onClose={() => mods.setComparing(null)} />
      <ChooseModeSheet
        choosing={choosing}
        onClose={() => setChoosing(null)}
        onChoose={(mode) => {
          const current = choosing;
          setChoosing(null);
          if (current) void importer.importMod("file", mode, current.path);
        }}
      />
      <RemoveSheet
        removing={remover.removing}
        onClose={() => remover.setRemoving(null)}
        onConfirm={() => void remover.confirm()}
      />
      <EditModSheet
        editing={editor.editing}
        form={editor.form}
        onChange={editor.setForm}
        onClose={() => editor.setEditing(null)}
        onSave={() => void editor.save()}
        saving={editor.saving}
      />
    </div>
  );
}
