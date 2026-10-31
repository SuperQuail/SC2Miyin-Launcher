import { useEffect, useState } from "react";

import { api } from "../../../../api/bridge";
import { errorText } from "../../../../api/errors";
import type { ModEntry, Variant } from "../../../../api/types";
import { usePermissionGate } from "../../../state/usePermission";
import { Button, Field, Sheet } from "../../../ui";

/**
 * 编辑改版（玩家版本）的元数据。
 *
 * 从 `SlotMenu.tsx` 拆出：只改启动器记录的元数据，不动包里的原始文件。
 */
export function VariantEditSheet({
  slot,
  variant,
  notify,
  refresh,
  onClose,
}: {
  slot: string;
  variant: Variant;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  /** 库变了，让外壳重新读一遍槽位。 */
  refresh: () => void;
  onClose: () => void;
}) {
  /** 提权闸门：保存改的是库里的记录（写盘）。 */
  const permission = usePermissionGate();
  const [form, setForm] = useState({
    name: variant.name,
    author: variant.author ?? "",
    registrationId: variant.registration_id ?? "",
    description: variant.description ?? "",
  });
  const [mods, setMods] = useState<ModEntry[]>([]);
  const [required, setRequired] = useState<Set<string>>(new Set(variant.declared_mods ?? []));

  useEffect(() => {
    void (async () => {
      try {
        setMods(await api.variantMods(slot, variant.id));
      } catch {
        setMods([]);
      }
    })();
  }, [slot, variant.id]);

  /** 勾 / 取消一个依赖模组。 */
  function toggleRequired(path: string): void {
    const next = new Set(required);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setRequired(next);
  }

  async function save(): Promise<void> {
    const changes = {
      name: form.name,
      author: form.author,
      registrationId: form.registrationId,
      description: form.description,
      declaredMods: [...required],
    };
    try {
      await permission.run(() => api.updateVariant(slot, variant.id, changes), {
        op: "update_variant",
        args: { slot, variantId: variant.id, changes },
      });
      onClose();
      refresh();
      notify("success", "已保存");
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  return (
    <Sheet
      title={"编辑「" + variant.name + "」"}
      dismissOnBackdrop={false}
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose}>取消</Button>
          <Button variant="primary" onClick={() => void save()}>
            保存
          </Button>
        </>
      }
    >
      <p className="sheet__note">
        只改启动器记录的元数据，不动包里的原始文件 —— 随时可以改回来。
        改名字不会改版本目录，挂在它上面的补丁不受影响。
      </p>

      <Field label="战役名">
        <input
          type="text"
          value={form.name}
          onChange={(event) => setForm({ ...form, name: event.target.value })}
        />
      </Field>
      <Field label="作者">
        <input
          type="text"
          placeholder="未知作者"
          value={form.author}
          onChange={(event) => setForm({ ...form, author: event.target.value })}
        />
      </Field>
      <Field label="注册 ID">
        <input
          type="text"
          placeholder="例如 HTXL.golden-lotv"
          value={form.registrationId}
          onChange={(event) => setForm({ ...form, registrationId: event.target.value })}
        />
      </Field>
      <Field label="描述">
        <textarea
          rows={3}
          value={form.description}
          onChange={(event) => setForm({ ...form, description: event.target.value })}
        />
      </Field>

      {/* 依赖模组：包作者在这里说清「地图需要哪几个模组」 */}
      {mods.length > 0 && (
        <div className="field">
          <span className="field__label">
            依赖模组（{required.size} / {mods.length}）
          </span>
          <p className="field__hint">
            勾上的表示<strong>地图需要它才能正常打开</strong>。导出这个包时会一并带上，
            模组管理页也会给它打「依赖」标。
          </p>
          <ul className="reqmods">
            {mods.map((mod) => (
              <li key={mod.path} className="reqmod">
                <label className="reqmod__label">
                  <input
                    type="checkbox"
                    checked={required.has(mod.path)}
                    onChange={() => toggleRequired(mod.path)}
                  />
                  <span className="reqmod__name">{mod.name}</span>
                </label>
                <span className="reqmod__parts">{mod.parts} 个文件</span>
              </li>
            ))}
          </ul>
        </div>
      )}
    </Sheet>
  );
}
