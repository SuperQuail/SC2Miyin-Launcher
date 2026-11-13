import type { BoundPatch } from "../../../../api/types";
import { Button, Sheet } from "../../../ui";

/** 删补丁前的确认：从库里彻底删掉，并从所有战役上解绑。 */
export function DeletePatchSheet({
  patch,
  onClose,
  onConfirm,
}: {
  patch: BoundPatch;
  onClose: () => void;
  onConfirm: () => void;
}) {
  return (
    <Sheet
      title={"删除补丁「" + patch.name + "」？"}
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose}>取消</Button>
          <Button variant="danger" onClick={onConfirm}>
            删除
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        会把补丁从库里彻底删掉，并从所有战役上解绑。<strong>删了要重新导入。</strong>
      </p>
      <p className="sheet__note">
        已经装进游戏目录的文件不会立刻消失；重新启用一次对应战役后会按新配置重铺。
      </p>
    </Sheet>
  );
}
