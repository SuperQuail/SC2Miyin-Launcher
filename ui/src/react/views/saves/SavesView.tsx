import { useCallback, useEffect, useState } from "react";

import type { SaveBackup, SaveFile, SaveIsolation, SaveSet, SaveTarget } from "../../../api/types";
import { Button } from "../../ui";
import { GroupDetailDialog } from "./GroupDetailDialog";
import "./saves.css";

/**
 * 存档页。
 *
 * **一行 = 一个战役/版本**（不是"一个文件夹"）：已启用的战役/版本各占一格，
 * 即使它还没有存档、即使它是原版 —— 用户要看到"这个战役的存档在哪"。
 * 没启用但库里还留着的旧组折叠在最后（未分类 / 未启用），按需展开。
 *
 * 归属由系统按战役/版本维护（组名即分类），用户只能改**单份存档**的归属：
 * 点「管理存档」打开该分类，逐份「归到…」。
 */
export function SavesView({
  targets,
  isolation,
  groups,
  banks,
  busy,
  onRefresh,
  onOpenSettings,
  onGroupFiles,
  onMoveToKey,
}: {
  /** 可归到的战役/版本（带 enabled：是不是现在装着）。 */
  targets: SaveTarget[];
  isolation: SaveIsolation;
  groups: SaveBackup[];
  banks: SaveSet | null;
  busy: boolean;
  onRefresh: () => void;
  onOpenSettings: () => void;
  onGroupFiles: (name: string) => Promise<SaveFile[]>;
  onMoveToKey: (from: string, file: string, key: string) => Promise<void>;
}) {
  const [detail, setDetail] = useState<{ name: string; label: string } | null>(null);
  const [detailFiles, setDetailFiles] = useState<SaveFile[] | null>(null);
  const [showOthers, setShowOthers] = useState(false);

  useEffect(() => {
    onRefresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

  // 绑定键 -> 组
  const groupByBinding = new Map<string, SaveBackup>();
  for (const group of groups) {
    const binding = isolation.assignments[group.name];
    if (binding) groupByBinding.set(binding, group);
  }

  const enabledTargets = targets.filter((item) => item.enabled);
  const usedKeys = new Set(enabledTargets.map((item) => item.key));

  const friendly: Record<string, string> = {
    custom: "自制战役",
    wol: "自由之翼",
    hots: "虫群之心",
    hotsevolution: "虫群之心 · 进化",
    lotv: "虚空之遗",
    lotvprologue: "虚空之遗 · 序章",
    nova: "诺娃隐秘行动",
  };

  /** 一行：一个已启用的战役/版本（可能还没有组），或一个旧组。 */
  interface SaveRow {
    key: string;
    label: string;
    group: SaveBackup | null;
    enabled: boolean;
    /** 归属（老组里没归属的显示成未分类）。 */
    loose?: boolean;
  }

  const labelOf = (group: SaveBackup): string => {
    const key = isolation.assignments[group.name];
    const target = key ? targets.find((item) => item.key === key) : undefined;
    if (target) return target.label;
    if (group.legacy) return "未分类（升级前的存档）";
    return group.label;
  };

  const rows: SaveRow[] = enabledTargets.map((item) => ({
    key: item.key,
    label: item.label,
    group: groupByBinding.get(item.key) ?? null,
    enabled: true,
  }));

  // 未启用/未分类的旧组：只留有内容的；标签跟已启用那行重复的丢掉（那是同一战役的空壳目录）；
  // 分区按它们的标签前缀去找对应战役 —— 折在该战役下面，而不是全塞进一个「未分类」。
  const enabledLabels = new Set(rows.map((row) => row.label));
  const otherRows: SaveRow[] = groups
    .filter((group) => {
      const binding = isolation.assignments[group.name];
      if (binding && usedKeys.has(binding)) return false;
      if (group.files === 0) return false;
      return !enabledLabels.has(labelOf(group));
    })
    .map((group) => ({
      key: "group:" + group.name,
      label: labelOf(group),
      group,
      enabled: false,
      loose: !isolation.assignments[group.name],
    }));

  /** 老组属于哪个战役：看它的目录名/标签以哪个战役名开头。 */
  const campaignOfLabel = (label: string): string | null => {
    for (const [slug, name] of Object.entries(friendly)) {
      if (label.startsWith(name)) return slug;
    }
    return null;
  };
  const sectionOf = (row: SaveRow): { key: string; name: string } => {
    if (!row.enabled) {
      const matched = campaignOfLabel(row.label);
      if (matched) return { key: matched, name: friendly[matched] ?? matched };
      return { key: "zz-other", name: "未分类 / 已停用" };
    }
    const slot = row.key.includes("/") ? row.key.split("/")[0] : row.key;
    const base = targets.find((item) => item.key === slot);
    return { key: slot, name: base ? base.label.replace("（原版）", "") : (friendly[slot] ?? slot) };
  };

  const sections: Array<{ key: string; name: string; rows: SaveRow[] }> = [];
  for (const row of [...rows, ...(showOthers ? otherRows : [])]) {
    const section = sectionOf(row);
    const found = sections.find((item) => item.key === section.key);
    if (found) found.rows.push(row);
    else sections.push({ key: section.key, name: section.name, rows: [row] });
  }
  sections.sort((a, b) => (a.key === "zz-other" ? 1 : b.key === "zz-other" ? -1 : 0));

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
      <section className="rounded-lg border border-[color-mix(in_srgb,var(--outline)_45%,transparent)] bg-surface p-5 shadow-sm">
        <div className="mb-3 flex items-center justify-between gap-4">
          <h2 className="m-0 text-base font-semibold text-on-surface">存档</h2>
          <div className="flex gap-1.5">
            <Button disabled={busy} onClick={onRefresh}>
              重新读取
            </Button>
            {!isolation.enabled && (
              <Button variant="tonal" onClick={onOpenSettings}>
                去设置里打开
              </Button>
            )}
          </div>
        </div>

        <p className="mb-3 max-w-[680px] text-[13px] leading-relaxed text-muted">
          {isolation.enabled
            ? "每个战役用自己的一份存档，换战役不会把上一部的进度冲掉。关掉之后不再动 Banks，已经存下的分类都留着。"
            : "现在关着：启动器不碰 Banks（游戏里就那一份）。要按战役分开管，去设置里打开「按战役隔离存档」。"}
        </p>

        <dl className="m-0 flex flex-col gap-1.5">
          <div className="flex gap-3 text-[12.5px]">
            <dt className="w-[84px] shrink-0 text-muted">游戏里这份</dt>
            <dd className="m-0 break-all">{banksText}</dd>
          </div>
          <div className="flex gap-3 text-[12.5px]">
            <dt className="w-[84px] shrink-0 text-muted">当前在用</dt>
            <dd className={isolation.enabled && isolation.active ? "m-0 font-semibold text-accent" : "m-0"}>
              {activeText}
            </dd>
          </div>
        </dl>
      </section>

      {sections.length === 0 && (
        <section className="rounded-lg border border-[color-mix(in_srgb,var(--outline)_45%,transparent)] bg-surface p-5">
          <p className="saves__empty">
            还没有存档分类
            <span className="saves__empty-sub">
              {isolation.enabled
                ? "启用一个战役之后会自动建分类：官方战役会接着当前这份进度，自制战役从零开始。"
                : "存档隔离关着的时候不分类。去设置里打开它，启用战役时就会自动建分类。"}
            </span>
          </p>
        </section>
      )}

      {sections.map((section) => (
        <section key={section.key} className="flex flex-col gap-2">
          <div className="flex items-baseline gap-2">
            <h3 className="m-0 text-[13.5px] font-semibold text-on-surface">{section.name}</h3>
            <span className="text-[11.5px] text-muted">{section.rows.length} 个分类</span>
          </div>
          <ul className="saves__list">
            {section.rows.map((row) => {
              const group = row.group;
              const active = group !== null && isolation.active === group.name;
              const emptyRow = group === null || group.files === 0;
              return (
                <li
                  key={row.key}
                  className={"saves__row" + (active ? " saves__row--active" : "")}
                >
                  <div className="saves__info">
                    <span className="saves__name">
                      <span className="saves__title-text">{row.label}</span>
                      {row.enabled && <span className="tag">已启用</span>}
                      {active && <span className="tag">当前在用</span>}
                      {row.loose && <span className="saves__loose">未指定</span>}
                    </span>
                    <span className="saves__meta">
                      {emptyRow
                        ? "还没有存档"
                        : group.files + " 个文件 · " + (group.bytes / 1024).toFixed(0) + " KB"}
                      {group ? " · " + group.name : ""}
                    </span>
                  </div>

                  <div className="saves__ops">
                    {group && (
                      <span className="saves__name-btn">
                        <Button
                          variant="tonal"
                          title={"打开「" + row.label + "」，看里面的存档、改其中某一份的归属"}
                          onClick={() => openGroup(group.name, row.label)}
                        >
                          管理存档
                        </Button>
                      </span>
                    )}                  </div>
                </li>
              );
            })}
          </ul>
        </section>
      ))}

      {otherRows.length > 0 && (
        <div className="flex flex-col gap-2">
          <Button variant="tonal" onClick={() => setShowOthers((was) => !was)}>
            {showOthers ? "收起未启用 / 未分类" : "显示未启用 / 未分类（" + otherRows.length + "）"}
          </Button>
        </div>
      )}

      <p className="saves__foot">
        这个页面只管「存档归哪个战役」。要让某个战役生效、或切换版本，去「战役 / 自制战役」页操作。
        改某一份存档的归属，点「管理存档」打开它。
      </p>

      {detail && (
        <GroupDetailDialog
          groupLabel={detail.label}
          files={detailFiles}
          choices={targets.map((item) => ({ key: item.key, label: item.label }))}
          busy={busy}
          onMove={(file, key) => {
            void (async () => {
              await onMoveToKey(detail.name, file, key);
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
