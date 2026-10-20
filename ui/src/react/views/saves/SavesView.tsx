import { useEffect } from "react";

import type { SaveBackup, SaveIsolation, SaveSet, SlotView } from "../../../api/types";
import { Button, Field } from "../../ui";
import "./saves.css";

/**
 * 存档页：按战役分开的存档组，**归属可以手动改**。
 *
 * 用户的原话：「可以管理各个战役 mod 对应的存档，并且可以手动选择将存档定位到别的战役，
 * 以防止错误匹配」。所以这一页只干三件事：看组、改归属、切 / 存。
 *
 * 隔离**开关不在这页**（在设置里）—— 两处都放开关只会让人不确定该点哪个。
 * 关着的时候这一页只解释"为什么没有组、去哪打开"。
 */
export function SavesView({
  slots,
  isolation,
  groups,
  banks,
  busy,
  onSwitch,
  onAssign,
  onSaveInto,
  onRefresh,
  onOpenSettings,
}: {
  slots: SlotView[];
  isolation: SaveIsolation;
  /** 存档组（后端把 data/saves 下的每个目录算一组）。 */
  groups: SaveBackup[];
  /** 游戏里现在这份 Banks。 */
  banks: SaveSet | null;
  busy: boolean;
  onSwitch: (name: string) => void;
  /** 手动定位：指给某个战役，或 null = 不算任何战役（原版）。 */
  onAssign: (name: string, slot: string | null) => void;
  /** 把游戏里这份存进这一组。 */
  onSaveInto: (name: string) => void;
  onRefresh: () => void;
  onOpenSettings: () => void;
}) {
  // 进页面读一遍：组是文件夹，别的地方可能刚改过
  useEffect(() => {
    onRefresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const banksText = !banks
    ? "正在读取…"
    : banks.missing
      ? "还没有存档（还没玩过）"
      : banks.files.length + " 个文件 · " + (banks.bytes / 1024).toFixed(0) + " KB";

  return (
    <div className="saves">
      <section className="saves__head">
        <p className="saves__eyebrow">按战役分开的存档</p>
        <h2 className="saves__title">存档</h2>
        <p className="saves__text">
          {isolation.enabled
            ? "每个战役用自己的一份存档，换战役不会把上一部的进度冲掉。关掉之后不再动 Banks，已经存下的组都留着。"
            : "现在关着：启动器不碰 Banks（游戏里就那一份）。要按战役分开管，去设置里打开「按战役隔离存档」。"}
        </p>
        <p className="saves__status">
          游戏里这份：{banksText}
          {isolation.enabled && isolation.active && <> · 正在用「{isolation.active}」</>}
        </p>
        <div className="saves__actions">
          <Button disabled={busy} onClick={onRefresh}>
            重新读取
          </Button>
          {!isolation.enabled && (
            <Button variant="tonal" onClick={onOpenSettings}>
              去设置里打开
            </Button>
          )}
        </div>
      </section>

      {groups.length === 0 ? (
        <section className="saves__empty">
          <p className="saves__empty-title">还没有存档组</p>
          <p className="saves__empty-text">
            {isolation.enabled
              ? "启用一个战役之后会自动建组：官方战役会接着当前这份进度，自制战役从零开始。"
              : "存档隔离关着的时候不分组。去设置里打开它，启用战役时就会自动建组。"}
          </p>
        </section>
      ) : (
        <section className="saves__card">
          <div className="saves__list-head">
            <span className="saves__list-title">存档组</span>
            <span className="saves__list-count">{groups.length} 个</span>
          </div>

          <ul className="saves__list">
            {groups.map((group) => {
              const active = isolation.active === group.name;
              const slot = isolation.assignments[group.name] ?? "";
              return (
                <li key={group.name} className={"saves__row" + (active ? " saves__row--active" : "")}>
                  <div className="saves__row-main">
                    <span className="saves__row-name">
                      {group.label}
                      {active && <span className="tag">当前在用</span>}
                    </span>
                    <span className="saves__row-meta">
                      {group.files} 个文件 · {(group.bytes / 1024).toFixed(0)} KB · {group.name}
                    </span>
                  </div>

                  <Field label="归属战役" row hint={slot ? undefined : "不属于任何战役"}>
                    <select
                      className="saves__select"
                      value={slot}
                      disabled={busy}
                      onChange={(event) => onAssign(group.name, event.target.value || null)}
                      aria-label={"「" + group.label + "」归到哪部战役"}
                    >
                      <option value="">不属于任何战役（原版）</option>
                      {slots.map((item) => (
                        <option key={item.slug} value={item.slug}>
                          {item.display_name}
                        </option>
                      ))}
                    </select>
                  </Field>

                  <div className="saves__row-actions">
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

          <p className="saves__foot">
            「切换到这个」= 游戏里换成这份存档（现在这份会先存回它自己的组）；
            「设为当前」= 把游戏里现在这份存进这个组。归属改了之后，启用那个战役就会用它。
          </p>
        </section>
      )}
    </div>
  );
}
