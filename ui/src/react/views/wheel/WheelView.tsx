import { useEffect, useMemo, useRef, useState } from "react";

import type { Preview, SlotView } from "../../../api/types";
import { Button, Checkbox, Disclosure, Sheet } from "../../ui";
import { ActivationPreviewDialog } from "../campaigns/ActivationPreviewDialog";
import { CampaignCard } from "./CampaignCard";
import {
  DATABASE_CAMPAIGNS,
  databaseWheelItems,
  type DatabaseCampaign,
  type DatabaseWheelItem,
} from "./campaignDatabase";
import { WHEEL_SIZE, WheelDisc } from "./WheelDisc";
import { buildCandidates, type WheelCandidate } from "./wheelCandidates";
import { pickIndex, targetRotation, WHEEL_TEXT_LIMIT } from "./wheelData";
import { readPalette } from "./wheelPalette";
import "./wheel.css";

const SPIN_MS = 4000;
const REST_MS = 60;
const DEFAULT_POINTER = "/wheel/dajiaozhu-pointer.png";
const POINTER_ANGLE_OFFSET = -180;

type OnOff = "on" | "off";
type WheelScope = "owned" | "database";
type WheelItem = WheelCandidate | DatabaseWheelItem;

export interface WheelViewProps {
  theme?: "light" | "dark";
  slots: readonly SlotView[];
  /** 测试注入用；不传就读构建时打包进来的数据区。 */
  database?: readonly DatabaseCampaign[];
  loading?: boolean;
  busy?: boolean;
  activate?: (slot: string, variantId: string | null) => Promise<boolean>;
  notify?: (kind: "info" | "success" | "error", text: string) => void;
  pendingActivation?: { variantId: string; preview: Preview } | null;
  confirmActivation?: () => Promise<boolean>;
  cancelActivation?: () => void;
  random?: () => number;
}

function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

export function WheelView({
  theme = "light",
  slots,
  database,
  loading = false,
  busy = false,
  activate,
  notify,
  pendingActivation,
  confirmActivation,
  cancelActivation,
  random = Math.random,
}: WheelViewProps) {
  const ownedCandidates = useMemo(() => buildCandidates(slots), [slots]);
  const databaseItems = useMemo(
    () => databaseWheelItems(database ?? DATABASE_CAMPAIGNS),
    [database],
  );
  const [scope, setScope] = useState<WheelScope>("owned");
  const candidates = useMemo<WheelItem[]>(
    () => (scope === "owned" ? ownedCandidates : databaseItems),
    [scope, ownedCandidates, databaseItems],
  );
  const [offTypes, setOffTypes] = useState<Set<string>>(() => new Set());
  const [offCampaigns, setOffCampaigns] = useState<Set<string>>(() => new Set());
  const [autoApply, setAutoApply] = useState<OnOff>("on");
  const [excludeDrawn, setExcludeDrawn] = useState<OnOff>("off");
  const [prompt, setPrompt] = useState<WheelItem | null>(null);
  const [campaignCard, setCampaignCard] = useState<DatabaseCampaign | null>(null);
  const [pool, setPool] = useState<WheelItem[]>([]);
  const [spinning, setSpinning] = useState(false);
  const [rotation, setRotation] = useState(0);
  const [drawnVersion, setDrawnVersion] = useState(0);
  const drawn = useRef<Set<string>>(new Set());
  const vanillaInit = useRef(false);
  const stopTimer = useRef<number | null>(null);
  const numbered = pool.length > WHEEL_TEXT_LIMIT;
  const poolIndex = useMemo(
    () => new Map(pool.map((item, index) => [item.id, index + 1] as const)),
    [pool],
  );

  const palette = useMemo(() => readPalette(), [theme]);
  const reducedMotion = prefersReducedMotion();
  const duration = reducedMotion ? 0 : SPIN_MS;

  const typeEntries = useMemo(() => {
    const seen = new Map<string, string>();
    for (const item of candidates) seen.set(item.type, item.typeLabel);
    return [...seen].map(([type, label]) => ({ type, label }));
  }, [candidates]);

  const campaignsForTypes = useMemo(
    () => candidates.filter((item) => !offTypes.has(item.type)),
    [candidates, offTypes],
  );

  const campaignGroups = useMemo(
    () =>
      typeEntries
        .filter((entry) => !offTypes.has(entry.type))
        .map((entry) => ({
          ...entry,
          items: candidates.filter((item) => item.type === entry.type),
        })),
    [typeEntries, candidates, offTypes],
  );

  const visible = useMemo(
    () => candidates.filter((item) => !offTypes.has(item.type) && !offCampaigns.has(item.id)),
    [candidates, offTypes, offCampaigns],
  );

  const available = useMemo(
    () => (excludeDrawn === "on" ? visible.filter((item) => !drawn.current.has(item.id)) : visible),
    [visible, excludeDrawn, drawnVersion],
  );

  // 原版默认剔除；只初始化一次，之后用户手动勾回不会被覆盖。
  useEffect(() => {
    if (vanillaInit.current || ownedCandidates.length === 0) return;
    vanillaInit.current = true;
    setOffCampaigns((current) => {
      const next = new Set(current);
      for (const item of ownedCandidates) {
        if (item.variantId === null) next.add(item.id);
      }
      return next;
    });
  }, [ownedCandidates]);

  useEffect(() => {
    setPrompt(null);
    setCampaignCard(null);
  }, [scope]);

  useEffect(() => {
    setPool(excludeDrawn === "on" ? visible.filter((item) => !drawn.current.has(item.id)) : visible);
    setRotation(0);
  }, [visible, excludeDrawn, scope]);

  function toggleType(type: string, on: boolean): void {
    setOffTypes((current) => {
      const next = new Set(current);
      if (on) next.delete(type);
      else next.add(type);
      return next;
    });
  }

  function toggleCampaign(id: string, on: boolean): void {
    setOffCampaigns((current) => {
      const next = new Set(current);
      if (on) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function selectAllTypes(): void {
    setOffTypes(new Set());
  }

  function clearTypes(): void {
    setOffTypes(new Set(typeEntries.map((entry) => entry.type)));
  }

  function selectAllCampaigns(): void {
    setOffCampaigns((current) => {
      const next = new Set(current);
      for (const item of campaignsForTypes) next.delete(item.id);
      return next;
    });
  }

  function clearCampaigns(): void {
    setOffCampaigns((current) => {
      const next = new Set(current);
      for (const item of campaignsForTypes) next.add(item.id);
      return next;
    });
  }

  useEffect(() => {
    return () => {
      if (stopTimer.current !== null) window.clearTimeout(stopTimer.current);
    };
  }, []);

  const canSpin = available.length >= 2 && !spinning && !loading;

  async function applyChoice(choice: WheelItem): Promise<void> {
    if (!("slot" in choice) || !activate || choice.active) return;
    const ok = await activate(choice.slot, choice.variantId);
    if (ok) notify?.("success", "已启用「" + choice.short + "」");
  }

  function spin(): void {
    if (!canSpin) return;
    const nextPool = available;
    const next = pickIndex(random(), nextPool.length);
    const winner = nextPool[next];
    if (!winner) return;

    setPool(nextPool);
    setPrompt(null);
    setSpinning(true);
    setRotation((current) => targetRotation(current, next, nextPool.length));

    if (stopTimer.current !== null) window.clearTimeout(stopTimer.current);
    stopTimer.current = window.setTimeout(() => {
      setSpinning(false);
      setRotation((current) => ((current % 360) + 360) % 360);
      drawn.current.add(winner.id);
      setDrawnVersion((version) => version + 1);
      if ("campaign" in winner) setCampaignCard(winner.campaign);
      else if (autoApply === "on") void applyChoice(winner);
      else setPrompt(winner);
    }, duration + REST_MS);
  }

  function resetDrawn(): void {
    drawn.current.clear();
    setDrawnVersion((version) => version + 1);
    setPool(visible);
    setRotation(0);
  }

  function confirmPrompt(): void {
    const choice = prompt;
    setPrompt(null);
    if (choice) void applyChoice(choice);
  }

  return (
    <div className="wheel-page">
      <div className="wheel-page__categories">
        <Button variant={scope === "owned" ? "primary" : "text"} onClick={() => setScope("owned")}>
          已有战役
        </Button>
        <Button
          variant={scope === "database" ? "primary" : "text"}
          onClick={() => setScope("database")}
        >
          战役数据库
        </Button>
      </div>

      <div className="wheel-layout">
        <aside className="wheel-panel wheel-panel--filters">
          {scope === "database" && (
            <div className="wheel-database-count">共 {databaseItems.length} 部战役</div>
          )}
          <div className="wheel-filter">
            <div className="wheel-filter__head">
              <span className="wheel-filter__title">战役类型</span>
              <div className="wheel-filter__actions">
                <Button
                  variant="text"
                  onClick={selectAllTypes}
                  disabled={spinning || offTypes.size === 0}
                >
                  全选
                </Button>
                <Button
                  variant="text"
                  onClick={clearTypes}
                  disabled={spinning || offTypes.size === typeEntries.length}
                >
                  清空
                </Button>
              </div>
            </div>
            {typeEntries.map((entry) => (
              <Checkbox
                key={entry.type}
                name={"wheel-type-" + entry.type}
                checked={!offTypes.has(entry.type)}
                onChange={(checked) => toggleType(entry.type, checked)}
                disabled={spinning}
              >
                {entry.label}
              </Checkbox>
            ))}
          </div>
          <div className="wheel-filter">
            <div className="wheel-filter__head">
              <span className="wheel-filter__title">具体战役</span>
              <div className="wheel-filter__actions">
                <Button
                  variant="text"
                  onClick={selectAllCampaigns}
                  disabled={spinning || campaignsForTypes.length === 0}
                >
                  全选
                </Button>
                <Button
                  variant="text"
                  onClick={clearCampaigns}
                  disabled={spinning || campaignsForTypes.length === 0}
                >
                  清空
                </Button>
              </div>
            </div>
            <div className="wheel-filter__list">
              {campaignGroups.map((group) => (
                <Disclosure key={group.type} title={group.label}>
                  {group.items.map((item) => (
                    <Checkbox
                      key={item.id}
                      name={"wheel-campaign-" + item.id}
                      checked={!offCampaigns.has(item.id)}
                      onChange={(checked) => toggleCampaign(item.id, checked)}
                      disabled={spinning}
                    >
                      {numbered && poolIndex.has(item.id) ? (
                        <span className="wheel-index">{poolIndex.get(item.id)}</span>
                      ) : null}
                      {item.label}
                    </Checkbox>
                  ))}
                </Disclosure>
              ))}
            </div>
          </div>
        </aside>

        <section className="wheel-panel wheel-main">
          <h2 className="wheel-page__title">战役转盘</h2>

          <div className="wheel-stage" style={{ width: WHEEL_SIZE, height: WHEEL_SIZE }}>
            <WheelDisc entries={pool} palette={palette} />
            <div
              className="wheel-pointer"
              aria-hidden="true"
              style={{
                transform: "rotate(" + (rotation + POINTER_ANGLE_OFFSET) + "deg)",
                transition: spinning
                  ? "transform " + duration + "ms cubic-bezier(0.15, 0.85, 0.15, 1)"
                  : "none",
              }}
            >
              <img className="wheel-pointer__img" src={DEFAULT_POINTER} alt="" draggable={false} />
            </div>
          </div>

          <div className="wheel-page__actions">
            <Button variant="primary" size="lg" loading={spinning} onClick={spin} disabled={!canSpin}>
              {spinning ? "抽取中…" : "抽取"}
            </Button>
          </div>

          <p className="wheel-page__notice">使用本转盘默认已同意进入战场引导太阳轰炸 --雨樱</p>
        </section>

        <aside className="wheel-panel">
          {scope === "owned" && (
            <Checkbox
              name="wheel-auto-apply"
              checked={autoApply === "on"}
              onChange={(checked) => setAutoApply(checked ? "on" : "off")}
              disabled={spinning}
            >
              自动应用
            </Checkbox>
          )}
          <Checkbox
            name="wheel-exclude-drawn"
            checked={excludeDrawn === "on"}
            onChange={(checked) => setExcludeDrawn(checked ? "on" : "off")}
            disabled={spinning}
          >
            剔除已抽取
          </Checkbox>
          <Button variant="outline" onClick={resetDrawn} disabled={spinning}>
            重置抽取状态
          </Button>
        </aside>
      </div>

      {campaignCard && (
        <CampaignCard campaign={campaignCard} onClose={() => setCampaignCard(null)} />
      )}

      {prompt && (
        <Sheet
          title={"启用「" + prompt.short + "」？"}
          onClose={() => setPrompt(null)}
          actions={
            <>
              <Button onClick={() => setPrompt(null)}>取消</Button>
              <Button variant="primary" onClick={confirmPrompt}>
                启用
              </Button>
            </>
          }
        >
          <span />
        </Sheet>
      )}

      {pendingActivation && confirmActivation && cancelActivation && (
        <ActivationPreviewDialog
          variantId={pendingActivation.variantId}
          preview={pendingActivation.preview}
          busy={busy}
          onConfirm={() => void confirmActivation()}
          onCancel={cancelActivation}
        />
      )}
    </div>
  );
}
