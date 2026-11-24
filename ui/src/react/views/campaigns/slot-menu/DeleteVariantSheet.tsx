import type { Variant } from "../../../../api/types";
import { Button, Sheet } from "../../../ui";

/** 删版本前的确认（从 `SlotMenu.tsx` 拆出）：删了就得重新导入。 */
export function DeleteVariantSheet({
  variant,
  onClose,
  onConfirm,
}: {
  variant: Variant;
  onClose: () => void;
  onConfirm: () => void;
}) {
  return (
    <Sheet
      title={"删除「" + variant.name + "」？"}
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose}>取消</Button>
          <Button variant="primary" onClick={onConfirm}>
            删除
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        将从战役库中删除此改版，并连同它已写入游戏目录的地图与模组一并移除。
        <strong>删除后需重新导入。</strong>
      </p>
    </Sheet>
  );
}
