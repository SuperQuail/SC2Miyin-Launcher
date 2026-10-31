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
        会把这一版从库里删掉，连带它已经装进游戏目录的地图和模组一起撤回。
        <strong>删了就得重新导入。</strong>
      </p>
    </Sheet>
  );
}
