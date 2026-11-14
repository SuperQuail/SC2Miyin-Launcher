import { useEffect, useState } from "react";

import { api } from "../../../../api/bridge";
import { errorText } from "../../../../api/errors";
import type { BoundPatch } from "../../../../api/types";
import { usePermissionGate } from "../../../state/usePermission";

import { DeletePatchSheet } from "./DeletePatchSheet";
import { PatchEditSheet } from "./PatchEditSheet";

/** 补丁面板：只列当前改版的补丁，控件只有启用、优先级、重命名、删除。 */
export function PatchPanel({
  slot,
  variant,
  notify,
}: {
  slot: string;
  variant: string;
  notify: (kind: "info" | "success" | "error", text: string) => void;
}) {
  const permission = usePermissionGate();
  const [patches, setPatches] = useState<BoundPatch[]>([]);
  const [editingPatch, setEditingPatch] = useState<BoundPatch | null>(null);
  const [deletingPatch, setDeletingPatch] = useState<BoundPatch | null>(null);

  async function loadPatches(): Promise<void> {
    try {
      setPatches(await api.listBindings(slot, variant));
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  useEffect(() => {
    void loadPatches();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slot, variant]);

  async function guard(action: () => Promise<unknown>): Promise<void> {
    try {
      await action();
      await loadPatches();
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  function setEnabled(item: BoundPatch, enabled: boolean): Promise<void> {
    return guard(() => api.configurePatch(slot, variant, item.id, enabled, null));
  }

  async function setPriority(item: BoundPatch, raw: string): Promise<void> {
    const priority = Number.parseInt(raw, 10);
    if (Number.isNaN(priority)) return;
    await guard(() => api.configurePatch(slot, variant, item.id, null, priority));
  }

  async function confirmDelete(): Promise<void> {
    const item = deletingPatch;
    if (!item) return;
    setDeletingPatch(null);
    try {
      await permission.run(() => api.deletePatch(item.id), {
        op: "delete_patch",
        args: { patchId: item.id },
      });
      await loadPatches();
      notify("success", "补丁「" + item.name + "」已删除");
    } catch (error) {
      notify("error", errorText(error));
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
        </div>

        {patches.length > 0 ? (
          <ul className="plist">
            {patches.map((item) => (
              <li key={item.id} className={"pitem" + (item.enabled ? "" : " pitem--off")}>
                <input
                  className="pitem__check"
                  type="checkbox"
                  checked={item.enabled}
                  aria-label={"启用 " + item.name}
                  onChange={(event) => void setEnabled(item, event.target.checked)}
                />
                <span className="pitem__name">{item.name}</span>
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
                  重命名
                </button>
                <button className="btn btn-text" type="button" onClick={() => setDeletingPatch(item)}>
                  删除
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <p className="patches__empty">暂无补丁</p>
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

      {deletingPatch && (
        <DeletePatchSheet
          patch={deletingPatch}
          onClose={() => setDeletingPatch(null)}
          onConfirm={() => void confirmDelete()}
        />
      )}
    </>
  );
}
