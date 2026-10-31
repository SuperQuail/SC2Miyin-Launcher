import { useEffect, useState } from "react";

import { api } from "../../../../api/bridge";
import { errorText } from "../../../../api/errors";
import type { BoundPatch } from "../../../../api/types";
import { usePermissionGate } from "../../../state/usePermission";

import { PatchEditSheet } from "./PatchEditSheet";

/**
 * 补丁面板：这个战役上叠了哪些覆盖层（从 `SlotMenu.tsx` 拆出）。
 *
 * 「导出中」仍由菜单页持有 —— 补丁的「导出」与底部操作条的导出按钮共用一个忙碌标记。
 */
export function PatchPanel({
  slot,
  notify,
  exporting,
  onExporting,
}: {
  slot: string;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  exporting: boolean;
  onExporting: (busy: boolean) => void;
}) {
  /** 提权闸门：导入补丁是写盘动作。 */
  const permission = usePermissionGate();
  const [patches, setPatches] = useState<BoundPatch[]>([]);
  const [available, setAvailable] = useState<BoundPatch[]>([]);
  const [showAvailable, setShowAvailable] = useState(false);
  const [editingPatch, setEditingPatch] = useState<BoundPatch | null>(null);

  /** 跑一个会改动补丁的动作，然后刷新列表。 */
  async function guard(action: () => Promise<unknown>): Promise<void> {
    try {
      await action();
      await loadPatches();
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  async function loadPatches(): Promise<void> {
    try {
      const [bound, free] = await Promise.all([
        api.listBindings(slot),
        api.listAvailablePatches(slot),
      ]);
      setPatches(bound);
      setAvailable(free);
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  useEffect(() => {
    void loadPatches();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slot]);

  function togglePatch(item: BoundPatch): Promise<void> {
    return guard(() => api.configurePatch(slot, item.id, !item.enabled, null));
  }

  function unbindPatch(item: BoundPatch): Promise<void> {
    return guard(() => api.unbindPatch(slot, item.id));
  }

  function bindPatchItem(item: BoundPatch): Promise<void> {
    return guard(() => api.bindPatch(slot, item.id));
  }

  async function setPriority(item: BoundPatch, raw: string): Promise<void> {
    const priority = Number.parseInt(raw, 10);
    if (Number.isNaN(priority)) return;
    await guard(() => api.configurePatch(slot, item.id, null, priority));
  }

  /** 按 requires 自动挂载匹配得上的补丁。 */
  async function autoBind(): Promise<void> {
    try {
      const bound = await api.autoBindPatches();
      await loadPatches();
      notify(
        bound.length ? "success" : "info",
        bound.length
          ? "已自动挂载 " + bound.length + " 个补丁"
          : "没有能自动匹配的补丁（没有声明依赖的只能手动挂）",
      );
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  /** 导入一个补丁包。 */
  async function importPatch(): Promise<void> {
    try {
      const path = await api.pickPackage();
      if (!path) return;
      const patch = await permission.run(() => api.importPatch(path), {
        op: "import_patch",
        args: { path },
      });
      await loadPatches();
      notify("success", patch ? "补丁「" + patch.name + "」已进库" : "补丁已进库");
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  /** 单独导出一个补丁包。 */
  async function exportPatchItem(item: BoundPatch): Promise<void> {
    try {
      const destination = await api.pickExportPath(item.name);
      if (!destination) return;
      onExporting(true);
      const report = await api.exportPatch(item.id, destination);
      notify("success", "补丁已导出：" + report.maps + " 张地图 · " + report.mods + " 个模组");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      onExporting(false);
    }
  }

  return (
    <>
      <section className="patches">
        <div className="patches__head">
          <h3 className="patches__title">
            补丁
            <span className="patches__count">{patches.length}</span>
          </h3>
          <div className="patches__actions">
            <button className="btn btn-text" type="button" onClick={() => void autoBind()}>
              自动匹配
            </button>
            <button className="btn btn-text" type="button" onClick={() => void importPatch()}>
              ＋ 导入补丁
            </button>
            <button className="btn btn-tonal" type="button" onClick={() => setShowAvailable(!showAvailable)}>
              {showAvailable ? "收起可选补丁" : "挂载补丁（" + available.length + "）"}
            </button>
          </div>
        </div>

        <p className="patches__hint">
          补丁是覆盖层：它不复制战役本体，启用时按优先级叠加。关掉即撤销，随时可恢复。
        </p>

        {patches.length > 0 ? (
          <ul className="plist">
            {patches.map((item) => (
              <li key={item.id} className={"pitem" + (item.enabled ? "" : " pitem--off")}>
                <label className="pitem__toggle">
                  <input type="checkbox" checked={item.enabled} onChange={() => void togglePatch(item)} />
                  <span className="pitem__name">{item.name}</span>
                </label>
                {item.version && <span className="tag">v{item.version}</span>}
                {item.author && <span className="pitem__author">{item.author}</span>}
                <span className="pitem__spacer" />
                <label className="pitem__priority">
                  优先级
                  <input
                    type="number"
                    className="pitem__input"
                    value={item.priority}
                    onChange={(event) => void setPriority(item, event.target.value)}
                  />
                </label>
                <button className="btn btn-text" type="button" onClick={() => setEditingPatch(item)}>
                  编辑
                </button>
                <button
                  className="btn btn-text"
                  type="button"
                  disabled={exporting}
                  onClick={() => void exportPatchItem(item)}
                >
                  导出
                </button>
                <button className="btn btn-text" type="button" onClick={() => void unbindPatch(item)}>
                  移出
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="patches__hint">
            这个战役还没有挂补丁。声明了依赖的补丁可以「自动匹配」；没声明的只能手动挂。
          </p>
        )}

        {showAvailable && (
          <div className="pavail">
            {available.map((item) => (
              <div key={item.id} className="pitem">
                <span className="pitem__name">{item.name}</span>
                {item.matched ? (
                  <span className="tag">依赖匹配</span>
                ) : item.requires.length === 0 ? (
                  <span className="tag">通用补丁</span>
                ) : null}
                <span className="pitem__spacer" />
                <button className="btn btn-tonal" type="button" onClick={() => void bindPatchItem(item)}>
                  挂上
                </button>
              </div>
            ))}
            {available.length === 0 && <p className="patches__hint">补丁库是空的，先导入补丁包。</p>}
          </div>
        )}
      </section>

      {editingPatch && (
        <PatchEditSheet
          patch={editingPatch}
          notify={notify}
          onSaved={loadPatches}
          onClose={() => setEditingPatch(null)}
        />
      )}
    </>
  );
}
