import type { MergeChoice, MergeStatus } from "../../../api/types";
import { formatBytes } from "../../../api/art";
import { Button, Choice, Sheet } from "../../ui";
import "./merge.css";

const OPTIONS: { value: MergeChoice; label: string; hint: string }[] = [
  { value: "target", label: "保留游戏目录中的数据", hint: "本地数据不修改" },
  { value: "local", label: "使用本地数据", hint: "覆盖游戏目录数据" },
  { value: "both", label: "两份都保留", hint: "两份并存" },
];

export interface MergeDialogProps {
  status: MergeStatus;
  choices: Record<string, MergeChoice>;
  applying: boolean;
  onChoose: (key: string, choice: MergeChoice) => void;
  onApply: () => void;
  /** 稍后 —— 关掉，下次启动再问。 */
  onLater: () => void;
}

/**
 * 「发现另一份数据」。
 *
 * 只有**冲突**才值得问 —— 不冲突的那部分后端已经直接并了，这里列出来只是让用户
 * 知道待会儿会多出什么。所以 conflicts 为空时后端返回 null，这个弹窗根本不出现。
 */
export function MergeDialog({ status, choices, applying, onChoose, onApply, onLater }: MergeDialogProps) {
  const additionsBytes = status.additions.reduce((sum, item) => sum + item.bytes, 0);
  const additionKeys = status.additions.map((item) => item.slot + "/" + item.variant);

  return (
    <Sheet
      title="发现另一份战役库数据"
      dismissOnBackdrop={false}
      onClose={onLater}
      actions={
        <>
          <Button onClick={onLater}>稍后</Button>
          <Button variant="primary" loading={applying} onClick={onApply}>
            开始合并
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        程序旁的旧数据（<code className="merge__path">{status.local}</code>）与游戏目录旁的战役库
        （<code className="merge__path">{status.target}</code>）都希望作为战役库。无冲突的部分会合并，
        以下版本两端都存在，请选择保留哪一份。
      </p>

      <section className="merge__section">
        <h4 className="merge__head">将合并 · {status.additions.length} 项（{formatBytes(additionsBytes)}）</h4>
        {status.additions.length === 0 ? (
          <p className="merge__empty">没有额外的版本。</p>
        ) : (
          <ul className="merge__list">
            {status.additions.map((item, index) => (
              <li className="merge__row" key={additionKeys[index]}>
                <span className="merge__row-text">{item.variant_name}</span>
                <span className="merge__size">{formatBytes(item.bytes)}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="merge__section">
        <h4 className="merge__head">两端均存在 · {status.conflicts.length} 项，需要选择</h4>
        <div className="merge__scroll">
          {status.conflicts.map((conflict) => {
            const key = conflict.slot + "/" + conflict.variant;
            return (
              <div className="merge__conflict" key={key}>
                <p className="merge__name">{conflict.variant_name}</p>
                <p className="merge__sizes">
                  本地 {formatBytes(conflict.local_bytes)} · 游戏目录 {conflict.target_bytes > 0 ? formatBytes(conflict.target_bytes) : "未知"}
                </p>
                <Choice
                  name={"merge-" + key}
                  value={choices[key] ?? "target"}
                  options={OPTIONS}
                  onChange={(choice) => onChoose(key, choice)}
                />
              </div>
            );
          })}
        </div>
        <p className="merge__foot">
          以下内容与当前安装绑定，将统一使用游戏目录中的数据：{status.keep_target.join(" · ")}。
        </p>
      </section>
    </Sheet>
  );
}
