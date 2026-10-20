import { useCallback, useEffect, useState } from "react";

import type { SaveBackup, SaveFile, SaveIsolation, SaveSet, SaveTarget } from "../../../api/types";
import { Button } from "../../ui";
import { GroupDetailDialog } from "./GroupDetailDialog";
import "./saves.css";

/**
 * 存档页：按战役分开的存档组，**归属可以手动改**。
 *
 * 只干三件事：看组、改归属、切 / 存。隔离开关不在这页（在设置里）——
 * 两处都放开关只会让人不确定该点哪个。
 *
 * 版式走设置页那套壳（page / card panel / panel__head），不自己造。
 */
export function SavesView({
  targets,
  isolation,
  groups,
  banks,
  busy,
  onSwitch,
  onAssign,
  onSaveInto,
  onRefresh,
  onOpenSettings,
  onGroupFiles,
  onMoveSave,
}: {
  /** 归属下拉的候选：官方战役 + 每部自制战役（键由后端算，界面不拼）。 */
  targets: SaveTarget[];
  isolation: SaveIsolation;
  groups: SaveBackup[];
  banks: SaveSet | null;
  busy: boolean;
  onSwitch: (name: string) => void;
  onAssign: (name: string, slot: string | null) => void;
  onSaveInto: (name: string) => void;
  onRefresh: () => void;
  onOpenSettings: () => void;
  /** 读一个组里的存档文件（打开组详情用）。 */
  onGroupFiles: (name: string) => Promise<SaveFile[]>;
  /** 把一份存档挪到别的组（= 改这份归属的战役）。 */
  onMoveSave: (from: string, file: string, to: string, toLabel?: string) => Promise<void>;
}) {
  useEffect(() => {
    onRefresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // 候选之外、但组身上真有的键（老数据里的 custom）也要能显示，不然下拉是空的；
  // 且**不自动猜**它属于哪部自制战役，让用户自己选。
  const options: SaveTarget[] = [...targets];
  for (const group of groups) {
    const key = isolation.assignments[group.name];
    if (!key || options.some((item) => item.key === key)) continue;
    options.push({ key, label: key === "custom" ? "旧的自制战役（未指定版本）" : key });
  }

  const [detail, setDetail] = useState<{ name: string; label: string } | null>(null);
  const [detailFiles, setDetailFiles] = useState<SaveFile[] | null>(null);

  const loadGroup = useCallback(
    async (name: string) => {
      setDetailFiles(null);
      setDetailFiles(await onGroupFiles(name));
    },
    [onGroupFiles],
  );

  const openGroup = (name: string, label: string) => {
    setDetail({ name, label });
    void loadGroup(name);
  };

  const banksText = !banks
    ? "正在读取…"
    : banks.missing
      ? "还没有存档（还没玩过）"
      : banks.files.length + " 个文件 · " + (banks.bytes / 1024).toFixed(0) + " KB";

  const activeText = !isolation.enabled
    ? "隔离关着"
    : isolation.active
      ? "「" + isolation.active + "」"
      : "还没归过档";

  return (
    <div className="page">
      <section className="card panel">
        <header className="panel__head">
          <h2 className="panel__title">存档</h2>
          <div className="panel__actions">
            <Button disabled={busy} onClick={onRefresh}>
              重新读取
            </Button>
            {!isolation.enabled && (
              <Button variant="tonal" onClick={onOpenSettings}>
                去设置里打开
              </Button>
            )}
          </div>
        </header>

        <p className="saves__lede">
          {isolation.enabled
            ? "每个战役用自己的一份存档，换战役不会把上一部的进度冲掉。关掉之后不再动 Banks，已经存下的组都留着。"
            : "现在关着：启动器不碰 Banks（游戏里就那一份）。要按战役分开管，去设置里打开「按战役隔离存档」。"}
        </p>

        <dl className="saves__facts">
          <div className="saves__fact">
            <dt>游戏里这份</dt>
            <dd>{banksText}</dd>
          </div>
          <div className="saves__fact">
            <dt>当前在用</dt>
            <dd>{activeText}</dd>
          </div>
        </dl>
      </section>

      <section className="card panel">
        <header className="panel__head">
          <h2 className="panel__title">存档组</h2>
          <span className="saves__count">{groups.length} 个</span>
        </header>

        {groups.length === 0 ? (
          <p className="saves__empty">
            还没有存档组
            <span className="saves__empty-sub">
              {isolation.enabled
                ? "启用一个战役之后会自动建组：官方战役会接着当前这份进度，自制战役从零开始。"
                : "存档隔离关着的时候不分组。去设置里打开它，启用战役时就会自动建组。"}
            </span>
          </p>
        ) : (
          <ul className="saves__list">
            {groups.map((group) => {
              const active = isolation.active === group.name;
              const slot = isolation.assignments[group.name] ?? "";
              return (
                <li
                  key={group.name}
                  className={"saves__row" + (active ? " saves__row--active" : "")}
                >
                  <div className="saves__info">
                    <span className="saves__name">
                      <button
                        type="button"
                        className="saves__name-btn"
                        title="打开看看这个组里有哪些存档"
                        onClick={() => openGroup(group.name, group.label)}
                      >
                        {group.label}
                      </button>
                      {active && <span className="tag">当前在用</span>}
                    </span>
                    <span className="saves__meta">
                      {group.files} 个文件 · {(group.bytes / 1024).toFixed(0)} KB
                    </span>
                  </div>

                  <label className="saves__assign">
                    <span className="saves__assign-label">归到</span>
                    <select
                      className="saves__select"
                      value={slot}
                      disabled={busy}
                      onChange={(event) => onAssign(group.name, event.target.value || null)}
                      aria-label={"「" + group.label + "」归到哪部战役"}
                    >
                      <option value="">不属于任何战役（原版）</option>
                      {options.map((item) => (
                        <option key={item.key} value={item.key}>
                          {item.label}
                        </option>
                      ))}
                    </select>
                  </label>

                  <div className="saves__ops">
                    <Button disabled={busy || active} onClick={() => onSwitch(group.name)}>
                      {active ? "正在用" : "切换到这个"}
                    </Button>
                    <Button variant="tonal" disabled={busy} onClick={() => onSaveInto(group.name)}>
                      设为当前
                    </Button>
                  </div>
                </li>
              );
            })}
          </ul>
        )}

        <p className="saves__foot">
          「切换到这个」= 游戏里换成这份存档（现在这份会先存回它自己的组）；「设为当前」= 把游戏里现在这份存进这个组。归属改了之后，启用那个战役就会用它。
        </p>
      </section>

      {detail && (
        <GroupDetailDialog
          groupLabel={detail.label}
          files={detailFiles}
          choices={groups
            .filter((item) => item.name !== detail.name)
            .map((item) => {
              const key = isolation.assignments[item.name];
              const target = key ? options.find((option) => option.key === key) : undefined;
              return {
                name: item.name,
                label: item.label + (target ? " → " + target.label : "（不属于任何战役）"),
              };
            })}
          busy={busy}
          onMove={(file, to) => {
            void (async () => {
              const choice = groups.find((item) => item.name === to);
              await onMoveSave(detail.name, file, to, choice?.label);
              await loadGroup(detail.name);
            })();
          }}
          onReload={() => void loadGroup(detail.name)}
          onClose={() => setDetail(null)}
        />
      )}
    </div>
  );
}
