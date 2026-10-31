import type { SaveFile } from "../../../api/types";
import { Button, Select, Sheet } from "../../ui";
import "./saves.css";

/**
 * 一个存档组的详情：**组里有什么**，以及把某一份挪到别的组。
 *
 * 为什么需要它：归属是整组级别的，一个组里可能混着几个战役的存档。
 * 整组改归属修不了其中某几份 —— 得能打开、逐份挪。
 * 挪一份 = 这一份归到的战役变了（组 ↔ 战役）。
 */
export function GroupDetailDialog({
  groupLabel,
  files,
  error,
  choices,
  busy,
  onMove,
  onReload,
  onClose,
}: {
  groupLabel: string;
  /** null = 正在读。 */
  files: SaveFile[] | null;
  error?: string | null;
  /** 可归到的战役/版本（键 + 人看的名字）。组的绑定由系统管，用户只挑归属。 */
  choices: Array<{ key: string; label: string }>;
  busy: boolean;
  onMove: (file: string, key: string) => void;
  onReload: () => void;
  onClose: () => void;
}) {
  return (
    <Sheet
      title={"「" + groupLabel + "」的内容"}
      onClose={onClose}
      actions={
        <>
          <Button disabled={busy} onClick={onReload}>
            重新读取
          </Button>
          <Button variant="primary" onClick={onClose}>
            关闭
          </Button>
        </>
      }
    >
      <p className="savedetail__lede">
        此组是一份 Banks 快照。若某份存档实际属于其他战役，可将其归入对应战役；操作只影响该存档，组名与其他存档不受影响。
      </p>

      {error ? (
        <p className="savedetail__error">{error}</p>
      ) : files === null ? (
        <p className="savedetail__empty">正在读取…</p>
      ) : files.length === 0 ? (
        <p className="savedetail__empty">此组暂无存档。</p>
      ) : (
        <ul className="savedetail__list">
          {files.map((file) => (
            <li className="savedetail__row" key={file.name}>
              <span className="savedetail__name">{file.name}</span>
              <span className="savedetail__size">{(file.bytes / 1024).toFixed(0)} KB</span>
              <span className="savedetail__move">
                <Select
                  value=""
                  placeholder="归入战役…"
                  ariaLabel={"将「" + file.name + "」归入哪个战役"}
                  options={choices.map((choice) => ({ value: choice.key, label: choice.label }))}
                  disabled={busy || choices.length === 0}
                  onChange={(key) => {
                    if (key) onMove(file.name, key);
                  }}
                />
              </span>
            </li>
          ))}
        </ul>
      )}

      <p className="savedetail__foot">
        仅修改存档库中的该文件，不会修改游戏目录的 Banks。目标位置已有同名存档时不会覆盖，并会给出提示。
      </p>
    </Sheet>
  );
}
