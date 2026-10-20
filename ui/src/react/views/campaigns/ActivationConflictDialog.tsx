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
 * 「保持另一个」= 什么都不做：关掉它，现在生效的还是另一个战役（不是回滚、不是撤销）。
 */
export function ActivationConflictDialog({
  conflicts,
  busy,
  onUseNew,
  onKeepOther,
}: {
  conflicts: ActivationConflict[];
  busy: boolean;
  /** 用新战役的：继续启用（会覆盖对方）。 */
  onUseNew: () => void;
  /** 保持另一个：取消，什么都不改。 */
  onKeepOther: () => void;
}) {
  const shown = conflicts.slice(0, SHOWN);
  const rest = conflicts.length - shown.length;

  return (
    <Sheet
      title="这个战役会覆盖另一个战役的文件"
      onClose={onKeepOther}
      actions={
        <>
          <Button disabled={busy} onClick={onKeepOther}>
            保持另一个
          </Button>
          <Button variant="primary" loading={busy} onClick={onUseNew}>
            用新战役的
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        两个战役都要往游戏目录的同一个位置写东西。继续启用会用这一份覆盖掉另一个 ——
        先确认要哪个。
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
        「保持另一个」什么都不改：现在生效的还是另一个战役，游戏目录一个字节都不动。
      </p>
    </Sheet>
  );
}
