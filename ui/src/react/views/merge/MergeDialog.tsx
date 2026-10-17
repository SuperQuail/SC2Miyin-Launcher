import type { MergeChoice, MergeStatus } from "../../../api/types";
import { formatBytes } from "../../../api/art";
import { Button, Choice, Sheet } from "../../ui";
import "./merge.css";

const OPTIONS: { value: MergeChoice; label: string; hint: string }[] = [
  { value: "target", label: "保持游戏目录那份", hint: "本地这份不动" },
  { value: "local", label: "用本地这份", hint: "覆盖过去" },
  { value: "both", label: "两份都留", hint: "并排成两个版本" },
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
      title="发现另一份数据"
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
        程序旁边那份数据（<code className="merge__path">{status.local}</code>）和游戏目录这份
        （<code className="merge__path">{status.target}</code>）都想当战役库。不冲突的会并进来，
        下面这几个版本两边都有 —— 你说了算。
      </p>

      <section className="merge__section">
        <h4 className="merge__head">会一起并进来 · {status.additions.length} 个版本（{formatBytes(additionsBytes)}）</h4>
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
        <h4 className="merge__head">两边都有 · {status.conflicts.length} 个，要你选</h4>
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
          这几样和当前安装绑死，一律用游戏目录那份：{status.keep_target.join(" · ")}。
        </p>
      </section>
    </Sheet>
  );
}
