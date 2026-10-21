import type { SaveFile } from "../../../api/types";
import { Button, Sheet } from "../../ui";
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
      title={"「" + groupLabel + "」里有什么"}
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
        这个组是一份 Banks 快照。如果某一份其实属于别的战役，把它归过去 —— 只动这一份，组名和别的存档都不受影响。
      </p>

      {error ? (
        <p className="savedetail__error">{error}</p>
      ) : files === null ? (
        <p className="savedetail__empty">正在读取…</p>
      ) : files.length === 0 ? (
        <p className="savedetail__empty">这个组里还没有存档（空组）。</p>
      ) : (
        <ul className="savedetail__list">
          {files.map((file) => (
            <li className="savedetail__row" key={file.name}>
              <span className="savedetail__name">{file.name}</span>
              <span className="savedetail__size">{(file.bytes / 1024).toFixed(0)} KB</span>
              <select
                className="savedetail__move"
                value=""
                disabled={busy || choices.length === 0}
                onChange={(event) => {
                  const key = event.target.value;
                  if (key) onMove(file.name, key);
                  event.target.value = "";
                }}
                aria-label={"把「" + file.name + "」归到哪个战役"}
              >
                <option value="">{choices.length === 0 ? "没有可选的目的地" : "归到…"}</option>
                {choices.map((choice) => (
                  <option key={choice.key} value={choice.key}>
                    {choice.label}
                  </option>
                ))}
              </select>
            </li>
          ))}
        </ul>
      )}

      <p className="savedetail__foot">
        只动存档库（data/saves）里的这一份，不会动游戏目录的 Banks。目标里已有同名存档时不会覆盖，会提示你。
      </p>
    </Sheet>
  );
}
