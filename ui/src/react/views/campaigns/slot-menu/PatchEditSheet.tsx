import { useState } from "react";

import { api } from "../../../../api/bridge";
import { errorText } from "../../../../api/errors";
import type { BoundPatch } from "../../../../api/types";
import { Button, Field, Sheet } from "../../../ui";

/** 只改显示名；包内容与其它字段都不动。 */
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
  const [name, setName] = useState(patch.name);

  async function save(): Promise<void> {
    try {
      await api.updatePatch(patch.id, { name });
      onClose();
      await onSaved();
      notify("success", "已保存");
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  return (
    <Sheet
      title={"重命名补丁「" + patch.name + "」"}
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
      <Field label="补丁名">
        <input type="text" value={name} onChange={(event) => setName(event.target.value)} />
      </Field>
    </Sheet>
  );
}
