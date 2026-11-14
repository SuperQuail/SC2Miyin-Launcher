import { useEffect, useState } from "react";

import { formatBytes, formatDate, slotArt } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { MapEntry, ModEntry, SlotView, Variant } from "../../../api/types";
import { Button } from "../../ui";
import { PatchPanel } from "./slot-menu/PatchPanel";
import { variantPath, type CampaignsHost } from "./host";
import "./slot-menu.css";
import "./variant-editor.css";

type Tab = "patches" | "mods" | "maps" | "file";

/** 某个改版的独立编辑页：切改版、启用、开始游戏，下面按页签管理补丁/模组/地图/文件。 */
export function VariantEditorView({
  slot,
  variantId,
  host,
  onBack,
  onSwitch,
}: {
  slot: SlotView;
  variantId: string;
  host: CampaignsHost;
  onBack: () => void;
  onSwitch: (variantId: string) => void;
}) {
  const variant = slot.variants.find((item) => item.id === variantId) ?? null;
  const currentId = variant?.id ?? null;
  const [tab, setTab] = useState<Tab>("patches");
  const [menuOpen, setMenuOpen] = useState(false);
  const [mods, setMods] = useState<ModEntry[]>([]);
  const [maps, setMaps] = useState<MapEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!currentId) return;
    let cancelled = false;
    setLoading(true);
    void Promise.all([api.variantMods(slot.slug, currentId), api.variantMaps(slot.slug, currentId)])
      .then(([nextMods, nextMaps]) => {
        if (cancelled) return;
        setMods(nextMods);
        setMaps(nextMaps);
      })
      .catch((error: unknown) => {
        if (!cancelled) host.notify("error", errorText(error));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
    // host.notify 每次渲染都是新函数，不该进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slot.slug, currentId]);

  if (!variant) return null;

  const targetId = variant.id;
  const enabled = (slot.enabled_variants ?? []).includes(targetId);
  const art = slotArt(slot.slug);
  const tabs: { id: Tab; label: string }[] = [
    { id: "patches", label: "补丁" },
    { id: "mods", label: "模组" },
    { id: "maps", label: "地图" },
    { id: "file", label: "文件" },
  ];

  async function enable(): Promise<void> {
    await host.activate(slot.slug, targetId);
  }

  function selectVariant(next: Variant): void {
    setMenuOpen(false);
    setTab("patches");
    onSwitch(next.id);
  }

  return (
    <div className="ve-page campaigns-menu">
      <div className="ve-shell">
        <header className="ve-head">
          <button className="ve-back" type="button" aria-label="返回改版列表" onClick={onBack}>
            ←
          </button>
          <span className="ve-cover">
            {art && <img src={art} alt="" />}
          </span>
          <div className="ve-headtext">
            <p className="ve-campaign">{slot.display_name}</p>
            <div className="ve-title">
              <div className="ve-select">
                <button className="ve-select__button" type="button" onClick={() => setMenuOpen(!menuOpen)}>
                  <span className="ve-select__label">
                    {variant.name} {variant.version ?? ""}
                  </span>
                  <span className="ve-chevron">▾</span>
                </button>
                {menuOpen && (
                  <div className="ve-menu">
                    {slot.variants.map((item) => (
                      <button
                        key={item.id}
                        className={"ve-menu__item" + (item.id === variant.id ? " is-on" : "")}
                        type="button"
                        onClick={() => selectVariant(item)}
                      >
                        <span>{item.name} {item.version ?? ""}</span>
                        {item.id === variant.id && <span className="ve-check">✓</span>}
                      </button>
                    ))}
                  </div>
                )}
              </div>
              <span className={"ve-status" + (enabled ? " is-on" : "")}>{enabled ? "当前启用" : "未启用"}</span>
            </div>
            <p className="ve-meta">
              {variant.map_count} 张地图 · {variant.mod_count} 个模组 · {formatBytes(variant.size_bytes)} ·{" "}
              {formatDate(variant.imported_at)}
            </p>
          </div>
          <div className="ve-actions">
            {!enabled && (
              <Button variant="primary" disabled={host.busy} onClick={() => void enable()}>
                启用此改版
              </Button>
            )}
            <Button
              variant={enabled ? "primary" : "outline"}
              disabled={!enabled || host.busy}
              onClick={host.launch}
            >
              开始游戏
            </Button>
          </div>
        </header>

        <nav className="ve-tabs">
          {tabs.map((item) => (
            <button
              key={item.id}
              className={"ve-tab" + (tab === item.id ? " is-on" : "")}
              type="button"
              onClick={() => setTab(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>

        <main className="ve-main">
          {tab === "patches" && (
            <div className="ve-panel ve-panel--patches">
              <PatchPanel slot={slot.slug} variant={variant.id} notify={host.notify} />
            </div>
          )}

          {tab === "mods" && (
            <section className="ve-panel">
              <header className="ve-panelhead">
                <h2>模组</h2>
                <span>{mods.length}</span>
              </header>
              {loading ? (
                <p className="ve-state">读取中…</p>
              ) : mods.length === 0 ? (
                <p className="ve-state">暂无模组</p>
              ) : (
                <ul className="ve-list">
                  {mods.map((mod) => (
                    <li key={mod.path} className="ve-list__row">
                      <span className="ve-list__name">{mod.name}</span>
                      <span className="ve-list__meta">{mod.parts} 个部分</span>
                      <span className={"ve-badge" + (mod.in_game ? " is-on" : "")}>
                        {mod.in_game ? "已生效" : mod.mounted ? "已挂载" : "未挂载"}
                      </span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          )}

          {tab === "maps" && (
            <section className="ve-panel">
              <header className="ve-panelhead">
                <h2>地图</h2>
                <span>{maps.length}</span>
              </header>
              {loading ? (
                <p className="ve-state">读取中…</p>
              ) : maps.length === 0 ? (
                <p className="ve-state">暂无地图</p>
              ) : (
                <ul className="ve-list">
                  {maps.map((map) => (
                    <li key={map.path} className="ve-list__row">
                      <span className="ve-list__name">{map.name}</span>
                      <span className="ve-list__meta">{map.chapter ?? "—"}</span>
                      <span className="ve-list__meta">{formatBytes(map.size)}</span>
                      {map.is_main && <span className="ve-badge is-on">主地图</span>}
                    </li>
                  ))}
                </ul>
              )}
            </section>
          )}

          {tab === "file" && (
            <section className="ve-panel">
              <header className="ve-panelhead">
                <h2>文件</h2>
              </header>
              <dl className="ve-info">
                <dt>作者</dt>
                <dd>{variant.author ?? "—"}</dd>
                <dt>版本</dt>
                <dd>{variant.version ?? "—"}</dd>
                <dt>格式</dt>
                <dd>{variant.format}</dd>
                <dt>大小</dt>
                <dd>{formatBytes(variant.size_bytes)}</dd>
                <dt>导入时间</dt>
                <dd>{formatDate(variant.imported_at)}</dd>
                <dt>目录</dt>
                <dd className="ve-path" title={variantPath(host.libraryRoot, slot.slug, variant.id)}>
                  {variantPath(host.libraryRoot, slot.slug, variant.id)}
                </dd>
              </dl>
              <div className="ve-foot">
                <Button
                  variant="outline"
                  onClick={() => host.reveal(variantPath(host.libraryRoot, slot.slug, variant.id))}
                >
                  打开版本目录
                </Button>
              </div>
            </section>
          )}
        </main>
      </div>
    </div>
  );
}
