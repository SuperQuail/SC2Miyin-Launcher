import type { Preview } from "../../../api/types";
import { Button, Sheet } from "../../ui";
// .plist / .pitem 在 slot-menu.css 里：战役页本来就加载它，抽出来之后自制战役页也共用同一份
import "./slot-menu.css";

/**
 * 铺盘前的**预演**：启用这个版本会把游戏目录改成什么样，先给人看一眼。
 *
 * 从 `SlotMenu` 里抽出来的（行为没变）：自制战役页也要用 —— 那边原来从来没有
 * 渲染过这个对话框，用户的感受就是"点了启用没反应"。
 */
export function ActivationPreviewDialog({
  variantId,
  preview,
  busy,
  onConfirm,
  onCancel,
}: {
  /** 要启用的版本 id（标题里念出来）。 */
  variantId: string;
  preview: Preview;
  busy: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <Sheet
      title={"启用「" + variantId + "」将写入以下内容"}
      onClose={onCancel}
      actions={
        <>
          <Button onClick={onCancel}>取消</Button>
          <Button variant="primary" disabled={busy} onClick={onConfirm}>
            继续写入游戏目录
          </Button>
        </>
      }
    >
      <ul className="plist">
        {preview.add.length > 0 && (
          <li className="pitem">
            <span className="pitem__name">新增</span>
            <span className="tag">{preview.add.length} 项</span>
          </li>
        )}
        {preview.overwrite.length > 0 && (
          <li className="pitem">
            <span className="pitem__name">覆盖（会先备份原文件）</span>
            <span className="tag">{preview.overwrite.length} 项</span>
          </li>
        )}
        {preview.takeover.length > 0 && (
          <li className="pitem">
            <span className="pitem__name">接管别人的文件</span>
            <span className="tag">{preview.takeover.length} 项</span>
          </li>
        )}
        {preview.delete.length > 0 && (
          <li className="pitem">
            <span className="pitem__name">移除上次写入的文件</span>
            <span className="tag">{preview.delete.length} 项</span>
          </li>
        )}
      </ul>
      <p className="sheet__text">
        将向游戏目录写入 {(preview.bytes / 1024 / 1024).toFixed(1)} MB。
        被覆盖的文件会先备份到数据目录，切回原版时自动还原。
      </p>
      {preview.overwrite.length > 0 && (
        <details className="sheet__note">
          <summary>查看将被覆盖的文件（前 20 个）</summary>
          {preview.overwrite.slice(0, 20).map((item) => (
            <p key={item.target} className="hint">
              {item.target}
            </p>
          ))}
        </details>
      )}
    </Sheet>
  );
}
