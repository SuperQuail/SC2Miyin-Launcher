import type { ActivationConflict } from "../../../api/bridge";
import { Button, Sheet } from "../../ui";
import "./activation-dialog.css";

/** 最多列几条 —— 再多就"等 N 个"，别让弹窗变成一面墙。 */
const SHOWN = 5;

/**
 * 「这个战役会覆盖另一个战役的文件」。
 *
 * 版式照合并对话框（`views/merge/MergeDialog`）：一个 Sheet + 一段说明 + 一份清单 +
 * 两个按钮。**这里不做任何写盘动作** —— 用户点「用新战役的」之后才轮到启用流程。
 *
 * 两个按钮直接写战役名：用户要的就是「启用 A」还是「启用 B」，不要再转述一遍。
 * 「启用另一个」= 什么都不做：关掉它，现在生效的还是那个战役（不是回滚、不是撤销）。
 */
export function ActivationConflictDialog({
  conflicts,
  newCampaign,
  otherCampaign,
  busy,
  onUseNew,
  onKeepOther,
}: {
  conflicts: ActivationConflict[];
  /** 正在启用的那个（显示在人看得懂的按钮上）。 */
  newCampaign: string;
  /** 已经启用、跟它撞车的那个。 */
  otherCampaign: string;
  busy: boolean;
  /** 启用新战役：继续（会用它的文件覆盖对方）。 */
  onUseNew: () => void;
  /** 启用另一个：什么都不改，关掉对话框。 */
  onKeepOther: () => void;
}) {
  const shown = conflicts.slice(0, SHOWN);
  const rest = conflicts.length - shown.length;

  return (
    <Sheet
      title={newCampaign + " 和 " + otherCampaign + " 冲突"}
      onClose={onKeepOther}
      actions={
        <>
          <Button disabled={busy} onClick={onKeepOther}>
            启用 {otherCampaign}
          </Button>
          <Button variant="primary" loading={busy} onClick={onUseNew}>
            启用 {newCampaign}
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        它们都要往游戏目录写同一批文件（{conflicts.length} 个）。同一时间只能生效一个，选哪个？
      </p>

      <section className="conflict__section">
        <h4 className="conflict__head">
          会撞的位置 · {conflicts.length} 个
        </h4>
        <div className="conflict__scroll">
          {shown.map((item) => (
            <div className="conflict__item" key={item.target}>
              <code className="conflict__path">{item.target}</code>
              <span className="conflict__owner">
                被 {item.slot_name} · {item.variant_name} 占着
              </span>
            </div>
          ))}
        </div>
        {rest > 0 && <p className="conflict__more">等 {rest} 个</p>}
      </section>

      <p className="conflict__foot">
        选 {otherCampaign}：什么都不改。选 {newCampaign}：它的文件会盖掉对方的，对方会有一部分失效。
      </p>
    </Sheet>
  );
}
