import { useState } from "react";

import { api } from "../../../../api/bridge";
import { errorText } from "../../../../api/errors";
import type { BoundPatch } from "../../../../api/types";
import { Button, Field, Sheet } from "../../../ui";

/**
 * 编辑补丁的元数据（从 `SlotMenu.tsx` 的补丁面板拆出）。
 *
 * 只改启动器记录的元数据，不动包内容，也不影响它已经挂在哪些战役上。
 */
export function PatchEditSheet({
  patch,
  notify,
  onSaved,
  onClose,
}: {
  patch: BoundPatch;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  /** 保存成功后刷新补丁列表。 */
  onSaved: () => Promise<void>;
  onClose: () => void;
}) {
  const [form, setForm] = useState({
    name: patch.name,
    author: patch.author ?? "",
    registrationId: patch.registration_id ?? "",
    description: patch.description ?? "",
    priority: patch.priority,
  });

  async function save(): Promise<void> {
    try {
      await api.updatePatch(patch.id, {
        name: form.name,
        author: form.author,
        registrationId: form.registrationId,
        description: form.description,
        priority: form.priority,
      });
      onClose();
      await onSaved();
      notify("success", "已保存");
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  return (
    <Sheet
      title={"编辑补丁「" + patch.name + "」"}
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
        同样只改启动器记录的元数据，不动包内容，也不影响它已经挂在哪些战役上。
      </p>

      <Field label="补丁名">
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
          value={form.registrationId}
          onChange={(event) => setForm({ ...form, registrationId: event.target.value })}
        />
      </Field>
      <Field label="默认优先级">
        <input
          type="number"
          value={form.priority}
          onChange={(event) =>
            setForm({ ...form, priority: Number(event.target.value) })
          }
        />
      </Field>
      <Field label="描述">
        <textarea
          rows={3}
          value={form.description}
          onChange={(event) => setForm({ ...form, description: event.target.value })}
        />
      </Field>
    </Sheet>
  );
}
