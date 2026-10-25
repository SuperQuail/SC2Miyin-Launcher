import { useEffect, useMemo, useState } from "react";

import type { SaveInventory } from "../../../api/types";
import { Button, Select } from "../../ui";
import "./saves.css";

/**
 * 存档页：**一张表列出游戏真正的存档**（`*.SC2Save`）。
 *
 * 之前这一页读的是 `Documents\StarCraft II\Banks` —— 那目录在真机上根本不存在（或为空），
 * 所以什么都读不出来。现在改成读 `Accounts/<账号>/<地域>/Saves/**`，与游戏「加载游戏」
 * 里看到的是同一批文件。
 *
 * **按战役分区** —— SC2 的存档天生属于某个战役（`虚空之遗 战役 08` / `虫群之心 战役 04` …），
 * 所以列表要先按战役分组，而不是像"世界列表"那样平铺。
 * 文件名认不出战役的进「未归类」，用户之后可以手动归类（下一轮）。
 */
export function SavesView({
  inventory,
  busy,
  onScan,
}: {
  inventory: SaveInventory | null;
  busy: boolean;
  onScan: () => void;
}) {
  const [account, setAccount] = useState("all");
  const [folder, setFolder] = useState("all");

  useEffect(() => {
    if (inventory === null) onScan();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const saves = inventory?.saves ?? [];
  const folders = useMemo(() => [...new Set(saves.map((item) => item.folder))], [saves]);

  /** 从存档文件名认战役；认不出返回 null（进「未归类」）。 */
  const campaignOf = (name: string): { key: string; name: string } | null => {
    const rules: Array<{ key: string; name: string; patterns: RegExp }> = [
      { key: "wol", name: "自由之翼", patterns: /自由之翼|Liberty|WoL/i },
      { key: "hots", name: "虫群之心", patterns: /虫群之心|Swarm|HotS/i },
      { key: "lotv", name: "虚空之遗", patterns: /虚空之遗|LotV|Void/i },
      { key: "nova", name: "诺娃隐秘行动", patterns: /诺娃|Nova|NCO/i },
    ];
    return rules.find((rule) => rule.patterns.test(name)) ?? null;
  };

  const shown = saves.filter(
    (item) =>
      (account === "all" || item.account === account) &&
      (folder === "all" || item.folder === folder),
  );

  // 分区：官方四大战役（固定顺序）+ 未归类（既有自制战役的，也有认不出的）
  const sections: Array<{ key: string; name: string; saves: typeof shown }> = [];
  for (const item of shown) {
    const campaign = campaignOf(item.name);
    const section = campaign ?? { key: "zz-unknown", name: "未归类 / 自制战役" };
    const found = sections.find((entry) => entry.key === section.key);
    if (found) found.saves.push(item);
    else sections.push({ key: section.key, name: section.name, saves: [item] });
  }
  sections.sort((a, b) => (a.key === "zz-unknown" ? 1 : b.key === "zz-unknown" ? -1 : 0));

  const stamp = (seconds: number) => {
    if (!seconds) return "—";
    const date = new Date(seconds * 1000);
    return date.toLocaleString("zh-CN", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  const kb = (bytes: number) => (bytes / 1024).toFixed(0) + " KB";

  return (
    <div className="page">
      <section className="rounded-lg border border-[color-mix(in_srgb,var(--outline)_45%,transparent)] bg-surface p-5 shadow-sm">
        <div className="mb-3 flex items-center justify-between gap-4">
          <h2 className="m-0 text-base font-semibold text-on-surface">存档</h2>
          <div className="flex gap-1.5">
            <Button disabled={busy} loading={busy} onClick={onScan}>
              重新扫描
            </Button>
          </div>
        </div>

        <p className="mb-3 max-w-[720px] text-[13px] leading-relaxed text-muted">
          这里列的是游戏目录里真正的存档文件（<code>Documents/StarCraft II/Accounts/&lt;账号&gt;/&lt;地域&gt;/Saves</code>
          下的 <code>.SC2Save</code>），和游戏里「加载游戏」看到的是同一批。
        </p>

        {inventory === null ? (
          <p className="m-0 text-[12.5px] text-muted">正在扫描…</p>
        ) : (
          <div className="flex flex-wrap items-center gap-x-6 gap-y-1.5 text-[12.5px]">
            <span>
              <span className="text-muted">存档</span>{" "}
              <span className="font-semibold text-on-surface">{saves.length}</span> 份
            </span>
            <span>
              <span className="text-muted">账号</span>{" "}
              <span className="font-semibold text-on-surface">{inventory.accounts.length}</span> 个
            </span>
            <span>
              <span className="text-muted">Banks</span>{" "}
              <span className="font-semibold text-on-surface">{inventory.banks.length}</span> 份
            </span>
            {folder !== "all" && (
              <span className="text-muted">筛选中：{folder}</span>
            )}
          </div>
        )}
      </section>

      <section className="flex flex-col gap-2">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-baseline gap-2">
            <h3 className="m-0 text-[13.5px] font-semibold text-on-surface">存档列表</h3>
            <span className="text-[11.5px] text-muted">{shown.length} 份</span>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[12px] text-muted">账号</span>
            <span className="inline-block w-[11rem]">
            <Select
              value={account}
              placeholder="全部账号"
              ariaLabel="按账号筛选"
              options={[
                { value: "all", label: "全部账号" },
                ...(inventory?.accounts ?? []).map((id) => ({ value: id, label: id })),
              ]}
              onChange={setAccount}
            />
            </span>
            <span className="text-[12px] text-muted">位置</span>
            <span className="inline-block w-[13rem]">
            <Select
              value={folder}
              placeholder="全部位置"
              ariaLabel="按存档位置筛选"
              options={[
                { value: "all", label: "全部位置" },
                ...folders.map((name) => ({ value: name, label: name })),
              ]}
              onChange={setFolder}
            />
            </span>
          </div>
        </div>

        {shown.length === 0 ? (
          <p className="rounded-lg border border-[color-mix(in_srgb,var(--outline)_45%,transparent)] bg-surface p-5 text-[13px] text-muted">
            {saves.length === 0
              ? "没扫到存档。先在游戏里存一次档，或者确认游戏用的是哪个 Windows 账号的「文档」目录。"
              : "当前筛选下没有存档。"}
          </p>
        ) : (
          sections.map((section) => (
            <div key={section.key} className="flex flex-col gap-2">
              <div className="flex items-baseline gap-2">
                <h4 className="m-0 text-[13px] font-semibold text-on-surface">{section.name}</h4>
                <span className="text-[11.5px] text-muted">{section.saves.length} 份</span>
              </div>
              <ul className="saves__list">
                {section.saves.map((item) => (
                  <li className="saves__row" key={item.path}>
                    <div className="saves__info">
                      <span className="saves__name">
                        <span className="saves__title-text">{item.name}</span>
                      </span>
                      <span className="saves__meta">
                        {stamp(item.modified)} · {kb(item.bytes)} · {item.account}/{item.region} · {item.folder}
                      </span>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          ))
        )}
      </section>


      <p className="saves__foot">
        按战役分组是从存档文件名认出来的（`虚空之遗 …` / `LotV …` 之类）。
        认不出的进「未归类」，之后可以手动归类；备份与切换还在做。
      </p>
    </div>
  );
}
