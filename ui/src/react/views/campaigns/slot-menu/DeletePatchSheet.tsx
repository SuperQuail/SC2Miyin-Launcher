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
        将从战役库中永久删除该补丁，并从所有战役上解除绑定。<strong>删除后需重新导入。</strong>
      </p>
      <p className="sheet__note">
        已写入游戏目录的文件不会立即移除；重新启用对应战役后，将按新配置重新写入。
      </p>
    </Sheet>
  );
}
