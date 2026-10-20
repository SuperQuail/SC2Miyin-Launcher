import { useEffect, useState, type MouseEvent as ReactMouseEvent } from "react";

import { slotArt } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { BoundPatch, DocInfo, ModEntry, SlotView, Variant } from "../../../api/types";
import { copyText } from "../../lib/clipboard";
import { Button, Field, Sheet } from "../../ui";
import { CustomCampaignPanel } from "./CustomCampaignPanel";
import { DocViewer } from "../../components/DocViewer";
import { ImportDialog } from "./ImportDialog";
import { ActivationPreviewDialog } from "./ActivationPreviewDialog";
import { SortableCard } from "./SortableCard";
import { useVariantOrder } from "./useVariantOrder";
import { VariantCard } from "../../components/VariantCard";
import { VANILLA, variantPath, type CampaignMenuItem, type CampaignsHost } from "./host";
import "./slot-menu.css";

/** 补丁编辑表单的初始值。 */
const EMPTY_PATCH_FORM = { name: "", author: "", registrationId: "", description: "", priority: 100 };

/**
 * 某个战役的菜单页（React 版，从 `views/SlotMenuView.vue` 整块迁过来）。
 *
 * 选原版 / 已导入的玩家版本、挂补丁、导入更新导出；自制战役多一块地图与模组面板。
 * 外壳通过 `host` 递数据与动作，右键菜单由外壳的全局菜单承担。
 */
export function SlotMenu({
  slot,
  host,
  onBack,
}: {
  slot: SlotView;
  host: CampaignsHost;
  onBack: () => void;
}) {
  const isCustom = slot.slug === "custom";

  /** 当前选中的版本；null 表示原版战役。 */
  const [selected, setSelected] = useState<string | null>(slot.active);
  /** 自制战役：正在看哪个版本的地图与模组。 */
  const [inspecting, setInspecting] = useState<string | null>(null);
  /** 正在读的说明文档。 */
  const [openedDoc, setOpenedDoc] = useState<DocInfo | null>(null);
  /** 等待确认删除的那个版本 —— 删战役不可逆，先问一句。 */
  const [confirming, setConfirming] = useState<Variant | null>(null);
  /** 补丁面板。 */
  const [patches, setPatches] = useState<BoundPatch[]>([]);
  const [available, setAvailable] = useState<BoundPatch[]>([]);
  const [showAvailable, setShowAvailable] = useState(false);
  const [exporting, setExporting] = useState(false);
  /** 编辑元数据 / 编辑补丁的弹窗。 */
  const [editing, setEditing] = useState<Variant | null>(null);
  const [form, setForm] = useState({ name: "", author: "", registrationId: "", description: "" });
  const [editMods, setEditMods] = useState<ModEntry[]>([]);
  const [editRequired, setEditRequired] = useState<Set<string>>(new Set());
  const [editingPatch, setEditingPatch] = useState<BoundPatch | null>(null);
  const [patchForm, setPatchForm] = useState(EMPTY_PATCH_FORM);
  /** 「更新这一版」的对话框：每次 +1 开一次文件选择器。 */
  const [updateNonce, setUpdateNonce] = useState(0);
  /** 更新这一版的那个对话框在不在跑。 */
  const [updateBusy, setUpdateBusy] = useState(false);

  const inspectedVariant = slot.variants.find((item) => item.id === inspecting) ?? null;
  const dirty = selected !== slot.active;
  /** 版本卡片的顺序：**按槽位分开存**，「原版战役」那张固定卡不进去（它拖不动）。 */
  const order = useVariantOrder(slot.slug, slot.variants);

  /** 跑一个会改动补丁的动作，然后刷新列表。 */
  async function guard(action: () => Promise<unknown>): Promise<void> {
    try {
      await action();
      await loadPatches();
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  async function loadPatches(): Promise<void> {
    try {
      const [bound, free] = await Promise.all([
        api.listBindings(slot.slug),
        api.listAvailablePatches(slot.slug),
      ]);
      setPatches(bound);
      setAvailable(free);
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  useEffect(() => {
    void loadPatches();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slot.slug]);

  /** 选中一个版本；自制战役顺便把地图面板切过去。 */
  function pickVariant(id: string): void {
    const isVanilla = id === VANILLA;
    setSelected(isVanilla ? null : id);
    setInspecting(isCustom && !isVanilla ? id : null);
  }

  /**
   * 启用当前选中的版本。
   *
   * **启用完留在这一页** —— 以前是 `if (ok) onBack()` 退回战役列表，
   * 用户的原话是「启用后别把我弹回去」。启用成功后 `host.activate` 会刷新库，
   * 这一页的卡片与横幅自己就变成"已启用"了，留在原地才能看见结果。
   */
  async function apply(): Promise<void> {
    await host.activate(slot.slug, selected);
  }

  /** 启用并立刻启动游戏。 */
  async function applyAndPlay(): Promise<void> {
    const ok = await host.activate(slot.slug, selected);
    if (ok) host.launch();
  }

  /** 用户在确认框里点了「删除」。 */
  async function confirmDrop(): Promise<void> {
    const variant = confirming;
    if (!variant) return;
    setConfirming(null);
    const ok = await host.removeVariant(slot.slug, variant.id);
    if (ok && selected === variant.id) setSelected(null);
  }

  /**
   * 版本卡片上的右键菜单。
   *
   * 动作和页面底部的操作条一致，只是多给了个更顺手的入口 ——
   * 卡片上直接右键，不用先选中再去找按钮。
   */
  function showVariantMenu(event: ReactMouseEvent, variant: Variant | null): void {
    const entries: CampaignMenuItem[] = variant
      ? [
          { id: "pick", label: "选中这个版本" },
          { id: "activate", label: "启用这个版本" },
          { id: "play", label: "启用并开始游戏" },
          { id: "edit", label: "编辑信息…", separatorBefore: true },
          { id: "export", label: "导出这个包…" },
          { id: "copy", label: "复制版本目录路径", separatorBefore: true },
          { id: "drop", label: "删除这个版本", danger: true },
        ]
      : [
          { id: "pick", label: "选中「原版战役」" },
          { id: "activate", label: "切回原版战役" },
          { id: "play", label: "开始游戏" },
        ];

    host.showMenu(event, entries, (id) => {
      if (id === "pick") {
        pickVariant(variant?.id ?? VANILLA);
        return;
      }
      if (id === "activate") {
        void host.activate(slot.slug, variant?.id ?? null);
        return;
      }
      if (id === "play") {
        void applyAndPlay();
        return;
      }
      if (!variant) return;
      if (id === "edit") openEdit(variant);
      if (id === "export") void doExport(false);
      if (id === "drop") setConfirming(variant);
      if (id === "copy") void copyVariantPath(variant);
    });
  }

  /** 把版本目录路径塞进剪贴板，方便用户自己去翻地图 / 动手改。 */
  async function copyVariantPath(variant: Variant): Promise<void> {
    const path = variantPath(host.libraryRoot, slot.slug, variant.id);
    const ok = await copyText(path);
    host.notify(ok ? "success" : "info", ok ? "已复制：" + path : path);
  }

  /* ---------------- 补丁 ---------------- */

  function togglePatch(item: BoundPatch): Promise<void> {
    return guard(() => api.configurePatch(slot.slug, item.id, !item.enabled, null));
  }

  function unbindPatch(item: BoundPatch): Promise<void> {
    return guard(() => api.unbindPatch(slot.slug, item.id));
  }

  function bindPatchItem(item: BoundPatch): Promise<void> {
    return guard(() => api.bindPatch(slot.slug, item.id));
  }

  async function setPriority(item: BoundPatch, raw: string): Promise<void> {
    const priority = Number.parseInt(raw, 10);
    if (Number.isNaN(priority)) return;
    await guard(() => api.configurePatch(slot.slug, item.id, null, priority));
  }

  /** 按 requires 自动挂载匹配得上的补丁。 */
  async function autoBind(): Promise<void> {
    try {
      const bound = await api.autoBindPatches();
      await loadPatches();
      host.notify(
        bound.length ? "success" : "info",
        bound.length
          ? "已自动挂载 " + bound.length + " 个补丁"
          : "没有能自动匹配的补丁（没有声明依赖的只能手动挂）",
      );
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  /** 导入一个补丁包。 */
  async function importPatch(): Promise<void> {
    try {
      const path = await api.pickPackage();
      if (!path) return;
      const patch = await api.importPatch(path);
      await loadPatches();
      host.notify("success", "补丁「" + patch.name + "」已进库");
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  /** 单独导出一个补丁包。 */
  async function exportPatchItem(item: BoundPatch): Promise<void> {
    try {
      const destination = await api.pickExportPath(item.name);
      if (!destination) return;
      setExporting(true);
      const report = await api.exportPatch(item.id, destination);
      host.notify("success", "补丁已导出：" + report.maps + " 张地图 · " + report.mods + " 个模组");
    } catch (error) {
      host.notify("error", errorText(error));
    } finally {
      setExporting(false);
    }
  }

  function openPatchEdit(item: BoundPatch): void {
    setEditingPatch(item);
    setPatchForm({
      name: item.name,
      author: item.author ?? "",
      registrationId: item.registration_id ?? "",
      description: item.description ?? "",
      priority: item.priority,
    });
  }

  async function savePatchEdit(): Promise<void> {
    const target = editingPatch;
    if (!target) return;
    try {
      await api.updatePatch(target.id, {
        name: patchForm.name,
        author: patchForm.author,
        registrationId: patchForm.registrationId,
        description: patchForm.description,
        priority: patchForm.priority,
      });
      setEditingPatch(null);
      await loadPatches();
      host.notify("success", "已保存");
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  /* ---------------- 编辑元数据 ---------------- */

  function openEdit(variant: Variant): void {
    setEditing(variant);
    setForm({
      name: variant.name,
      author: variant.author ?? "",
      registrationId: variant.registration_id ?? "",
      description: variant.description ?? "",
    });
    setEditMods([]);
    setEditRequired(new Set(variant.declared_mods ?? []));
    void (async () => {
      try {
        setEditMods(await api.variantMods(slot.slug, variant.id));
      } catch {
        setEditMods([]);
      }
    })();
  }

  /** 勾 / 取消一个依赖模组。 */
  function toggleRequired(path: string): void {
    const next = new Set(editRequired);
    if (next.has(path)) next.delete(path);
    else next.add(path);
    setEditRequired(next);
  }

  async function saveEdit(): Promise<void> {
    const target = editing;
    if (!target) return;
    try {
      await api.updateVariant(slot.slug, target.id, {
        name: form.name,
        author: form.author,
        registrationId: form.registrationId,
        description: form.description,
        declaredMods: [...editRequired],
      });
      setEditing(null);
      host.refresh();
      host.notify("success", "已保存");
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  /* ---------------- 导出 ---------------- */

  async function doExport(mergePatches: boolean): Promise<void> {
    const variant = slot.variants.find((item) => item.id === selected);
    if (!variant) {
      host.notify("info", "先选一个版本再导出");
      return;
    }

    try {
      const destination = await api.pickExportPath(variant.name);
      if (!destination) return;
      setExporting(true);
      const report = await api.exportVariant(slot.slug, variant.id, destination, mergePatches);

      const parts = [report.maps + " 张地图", report.mods + " 个模组"];
      if (report.patches.length) parts.push("附带 " + report.patches.length + " 个补丁");
      let message = "已导出：" + parts.join(" · ");
      if (report.expanded.length) {
        message += "（有 " + report.expanded.length + " 项是目录树形态，CCM 可能读不了）";
      }
      host.notify("success", message);
    } catch (error) {
      host.notify("error", errorText(error));
    } finally {
      setExporting(false);
    }
  }

  // 自制战役**不铺底图**：它没有官方美术，槽位兜底出来的通用背景只是噪音（卡片自己带封面）
const art = slot.slug === "custom" ? null : slotArt(slot.slug);

  return (
    <div className="campaigns-menu">
      <div className="menu">
        <button className="back" type="button" onClick={onBack}>
          ← 返回战役列表
        </button>

        {/* 头条：官方战役用该资料片的 key art 做底；**自制战役不要底图** ——
            它没有官方美术，铺一张通用背景图只是噪音，卡片自己带封面就够了。 */}
        <header className="banner" style={art ? { backgroundImage: "url(" + art + ")" } : undefined}>
          <div className="banner__scrim" />
          <div className="banner__body">
            <h2 className="banner__title">{slot.display_name}</h2>
            <p className="banner__sub">
              {isCustom
                ? "挑一部战役，直接启动游戏来玩"
                : "选择要游玩的版本 —— 原版战役，或导入的玩家版本"}
            </p>
          </div>
          <button className="btn btn-primary banner__play" type="button" onClick={() => host.launch()}>
            开始游戏
          </button>
        </header>

        {/* 版本卡片：原版永远排第一 */}
        <div className="section-head">
          <h3 className="section-head__title">
            可选版本
            <span className="section-head__count">
              {slot.variants.length + (isCustom ? 0 : 1)}
            </span>
          </h3>
          <span className="section-head__hint">导入新版本请回到战役列表页</span>
        </div>

        {/* 版本卡片：原版永远排第一（固定卡，不可拖）；玩家版本可拖动排序。
            整块 grid 也是落点：卡片之间的缝里松手同样算数（卡片自己处理过的不重复处理）。 */}
        <div
          className="grid"
          onDragOver={(event) => {
            if (event.target === event.currentTarget) order.dragOver(event, event.currentTarget);
          }}
          onDrop={(event) => {
            if (event.target === event.currentTarget) order.dropAt(event, event.currentTarget);
          }}
        >
          {!isCustom && (
            <VariantCard
              slot={slot}
              variant={null}
              active={slot.active === null}
              selected={selected === null}
              onPick={() => setSelected(null)}
              onActivate={() => void host.activate(slot.slug, null)}
              onDrop={() => undefined}
              onMenu={(event) => showVariantMenu(event, null)}
            />
          )}
          {order.items.map((item) => (
            <SortableCard
              key={item.id}
              id={item.id}
              dragging={order.dragging}
              dropMark={order.dropMark}
              onDragState={order.setDragging}
              onDragOver={order.dragOver}
              onDrop={order.dropAt}
            >
              <VariantCard
                slot={slot}
                variant={item}
                active={slot.active === item.id}
                selected={selected === item.id}
                onPick={() => pickVariant(item.id)}
                onActivate={() => void host.activate(slot.slug, item.id)}
                onDrop={() => setConfirming(item)}
                onMenu={(event) => showVariantMenu(event, item)}
              />
            </SortableCard>
          ))}
        </div>

        {/* 自制战役：地图 + 挂载模组 + 启动 */}
        {isCustom && inspectedVariant && (
          <CustomCampaignPanel
            slot={slot.slug}
            variant={inspectedVariant}
            active={slot.active === inspectedVariant.id}
            refresh={host.refresh}
            notify={host.notify}
            showMenu={host.showMenu}
            onOpenDoc={setOpenedDoc}
          />
        )}

        {isCustom && !inspectedVariant && (
          <p className="hint">点上面任意一部战役，这里会出现它的地图列表与模组挂载。</p>
        )}

        {slot.variants.length === 0 && (
          <p className="hint">
            还没有导入任何玩家版本。回到战役列表页点「导入战役包」，启动器会自动判断它属于哪部战役。
          </p>
        )}

        {slot.notice && <p className="hint">{slot.notice}</p>}

        {/* 补丁：这个战役上叠了哪些覆盖层 */}
        <section className="patches">
          <div className="patches__head">
            <h3 className="patches__title">
              补丁
              <span className="patches__count">{patches.length}</span>
            </h3>
            <div className="patches__actions">
              <button className="btn btn-text" type="button" onClick={() => void autoBind()}>
                自动匹配
              </button>
              <button className="btn btn-text" type="button" onClick={() => void importPatch()}>
                ＋ 导入补丁
              </button>
              <button className="btn btn-tonal" type="button" onClick={() => setShowAvailable(!showAvailable)}>
                {showAvailable ? "收起可选补丁" : "挂载补丁（" + available.length + "）"}
              </button>
            </div>
          </div>

          <p className="patches__hint">
            补丁是覆盖层：它不复制战役本体，启用时按优先级叠加。关掉即撤销，随时可恢复。
          </p>

          {patches.length > 0 ? (
            <ul className="plist">
              {patches.map((item) => (
                <li key={item.id} className={"pitem" + (item.enabled ? "" : " pitem--off")}>
                  <label className="pitem__toggle">
                    <input type="checkbox" checked={item.enabled} onChange={() => void togglePatch(item)} />
                    <span className="pitem__name">{item.name}</span>
                  </label>
                  {item.version && <span className="tag">v{item.version}</span>}
                  {item.author && <span className="pitem__author">{item.author}</span>}
                  <span className="pitem__spacer" />
                  <label className="pitem__priority">
                    优先级
                    <input
                      type="number"
                      className="pitem__input"
                      value={item.priority}
                      onChange={(event) => void setPriority(item, event.target.value)}
                    />
                  </label>
                  <button className="btn btn-text" type="button" onClick={() => openPatchEdit(item)}>
                    编辑
                  </button>
                  <button
                    className="btn btn-text"
                    type="button"
                    disabled={exporting}
                    onClick={() => void exportPatchItem(item)}
                  >
                    导出
                  </button>
                  <button className="btn btn-text" type="button" onClick={() => void unbindPatch(item)}>
                    移出
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="patches__hint">
              这个战役还没有挂补丁。声明了依赖的补丁可以「自动匹配」；没声明的只能手动挂。
            </p>
          )}

          {showAvailable && (
            <div className="pavail">
              {available.map((item) => (
                <div key={item.id} className="pitem">
                  <span className="pitem__name">{item.name}</span>
                  {item.matched ? (
                    <span className="tag">依赖匹配</span>
                  ) : item.requires.length === 0 ? (
                    <span className="tag">通用补丁</span>
                  ) : null}
                  <span className="pitem__spacer" />
                  <button className="btn btn-tonal" type="button" onClick={() => void bindPatchItem(item)}>
                    挂上
                  </button>
                </div>
              ))}
              {available.length === 0 && <p className="patches__hint">补丁库是空的，先导入补丁包。</p>}
            </div>
          )}
        </section>

        {/* 底部操作条 */}
        <footer className="actions">
          <span className="actions__label">
            当前选择：<strong>{selected === null ? "原版战役" : selected}</strong>
          </span>
          <span className="actions__spacer" />
          {selected !== null && (
            <button
              className="btn btn-outline"
              type="button"
              onClick={() => {
                const target = slot.variants.find((item) => item.id === selected);
                if (target) openEdit(target);
              }}
            >
              编辑信息
            </button>
          )}
          <button
            className="btn btn-outline"
            type="button"
            disabled={updateBusy || exporting || host.busy}
            title="用压缩包把这一版换掉（删旧的、装新的；包里没有元数据也能用）"
            onClick={() => setUpdateNonce((value) => value + 1)}
          >
            更新这一版…
          </button>
          <button
            className="btn btn-outline"
            type="button"
            disabled={exporting || selected === null}
            onClick={() => void doExport(false)}
          >
            {exporting ? "导出中…" : "导出包"}
          </button>
          <button
            className="btn btn-outline"
            type="button"
            disabled={exporting || selected === null}
            onClick={() => void doExport(true)}
          >
            导出（含补丁）
          </button>
          <button
            className="btn btn-outline"
            type="button"
            disabled={host.busy || !dirty}
            onClick={() => void applyAndPlay()}
          >
            启用并开始游戏
          </button>
          <button
            className="btn btn-primary"
            type="button"
            disabled={!dirty || host.busy}
            onClick={() => void apply()}
          >
            {dirty ? "启用这个版本" : "已是当前版本"}
          </button>
        </footer>

        <ImportDialog
          mode="update"
          slot={slot.slug}
          variantId={selected ?? ""}
          importNonce={updateNonce}
          slots={host.slots}
          droppedPackage={null}
          consumeDropped={() => undefined}
          notify={host.notify}
          refresh={host.refresh}
          onImported={() => host.refresh()}
          onBusy={(value) => setUpdateBusy(value)}
        />

        {/* 编辑补丁 */}
        {editingPatch && (
          <Sheet
            title={"编辑补丁「" + editingPatch.name + "」"}
            dismissOnBackdrop={false}
            onClose={() => setEditingPatch(null)}
            actions={
              <>
                <Button onClick={() => setEditingPatch(null)}>取消</Button>
                <Button variant="primary" onClick={() => void savePatchEdit()}>
                  保存
                </Button>
              </>
            }
          >
            <p className="sheet__note">
              同样只改启动器记录的元数据，不动包内容，也不影响它已经挂在哪些战役上。
            </p>

            <Field label="补丁名">
              <input
                type="text"
                value={patchForm.name}
                onChange={(event) => setPatchForm({ ...patchForm, name: event.target.value })}
              />
            </Field>
            <Field label="作者">
              <input
                type="text"
                placeholder="未知作者"
                value={patchForm.author}
                onChange={(event) => setPatchForm({ ...patchForm, author: event.target.value })}
              />
            </Field>
            <Field label="注册 ID">
              <input
                type="text"
                value={patchForm.registrationId}
                onChange={(event) => setPatchForm({ ...patchForm, registrationId: event.target.value })}
              />
            </Field>
            <Field label="默认优先级">
              <input
                type="number"
                value={patchForm.priority}
                onChange={(event) =>
                  setPatchForm({ ...patchForm, priority: Number(event.target.value) })
                }
              />
            </Field>
            <Field label="描述">
              <textarea
                rows={3}
                value={patchForm.description}
                onChange={(event) => setPatchForm({ ...patchForm, description: event.target.value })}
              />
            </Field>
          </Sheet>
        )}

        {/* 编辑元数据 */}
        {editing && (
          <Sheet
            title={"编辑「" + editing.name + "」"}
            dismissOnBackdrop={false}
            onClose={() => setEditing(null)}
            actions={
              <>
                <Button onClick={() => setEditing(null)}>取消</Button>
                <Button variant="primary" onClick={() => void saveEdit()}>
                  保存
                </Button>
              </>
            }
          >
            <p className="sheet__note">
              只改启动器记录的元数据，不动包里的原始文件 —— 随时可以改回来。
              改名字不会改版本目录，挂在它上面的补丁不受影响。
            </p>

            <Field label="战役名">
              <input
                type="text"
                value={form.name}
                onChange={(event) => setForm({ ...form, name: event.target.value })}
              />
            </Field>
            <Field label="作者">
              <input
                type="text"
                placeholder="未知作者"
                value={form.author}
                onChange={(event) => setForm({ ...form, author: event.target.value })}
              />
            </Field>
            <Field label="注册 ID">
              <input
                type="text"
                placeholder="例如 HTXL.golden-lotv"
                value={form.registrationId}
                onChange={(event) => setForm({ ...form, registrationId: event.target.value })}
              />
            </Field>
            <Field label="描述">
              <textarea
                rows={3}
                value={form.description}
                onChange={(event) => setForm({ ...form, description: event.target.value })}
              />
            </Field>

            {/* 依赖模组：包作者在这里说清「地图需要哪几个模组」 */}
            {editMods.length > 0 && (
              <div className="field">
                <span className="field__label">
                  依赖模组（{editRequired.size} / {editMods.length}）
                </span>
                <p className="field__hint">
                  勾上的表示<strong>地图需要它才能正常打开</strong>。导出这个包时会一并带上，
                  模组管理页也会给它打「依赖」标。
                </p>
                <ul className="reqmods">
                  {editMods.map((mod) => (
                    <li key={mod.path} className="reqmod">
                      <label className="reqmod__label">
                        <input
                          type="checkbox"
                          checked={editRequired.has(mod.path)}
                          onChange={() => toggleRequired(mod.path)}
                        />
                        <span className="reqmod__name">{mod.name}</span>
                      </label>
                      <span className="reqmod__parts">{mod.parts} 个文件</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </Sheet>
        )}

        {openedDoc && inspectedVariant && (
          <DocViewer
            slot={slot.slug}
            variantId={inspectedVariant.id}
            doc={openedDoc}
            onClose={() => setOpenedDoc(null)}
            notify={() => undefined}
          />
        )}

        {/* 铺盘前的预演：会动哪些文件，先给人看一眼（自制战役页共用这一块） */}
        {host.pendingActivation && host.pendingActivation.slot === slot.slug && (
          <ActivationPreviewDialog
            variantId={host.pendingActivation.variantId}
            preview={host.pendingActivation.preview}
            busy={host.busy}
            onConfirm={() => void host.confirmActivation()}
            onCancel={host.cancelActivation}
          />
        )}

        {/* 删版本：先确认 */}
        {confirming && (
          <Sheet
            title={"删除「" + confirming.name + "」？"}
            onClose={() => setConfirming(null)}
            actions={
              <>
                <Button onClick={() => setConfirming(null)}>取消</Button>
                <Button variant="primary" onClick={() => void confirmDrop()}>
                  删除
                </Button>
              </>
            }
          >
            <p className="sheet__text">
              会把这一版从库里删掉，连带它已经装进游戏目录的地图和模组一起撤回。
              <strong>删了就得重新导入。</strong>
            </p>
          </Sheet>
        )}
      </div>
    </div>
  );
}