import { useEffect, useState, type MouseEvent as ReactMouseEvent } from "react";

import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { DocInfo, Variant } from "../../../api/types";
import { DocViewer } from "../../components/DocViewer";
import { VariantCard } from "../../components/VariantCard";
import { copyText } from "../../lib/clipboard";
import { Button, Sheet } from "../../ui";
import { CustomCampaignPanel } from "../campaigns/CustomCampaignPanel";
import { variantPath } from "../campaigns/host";
import { ImportDialog } from "../campaigns/ImportDialog";
import type { CustomHost } from "./host";
import "./custom.css";

/**
 * 自制战役页（React 版，从 `views/CustomView.vue` 整块迁过来）。
 *
 * 与「战役」页是**并列的顶层选项**，不是它的一个分组 —— 因为两者根本不是一类：
 * 原版战役是游戏自己驱动的，自制战役要单独挂模组、直接启动游戏进图。
 * 混在一页里会让人以为「自制战役也就是某个战役的改版」。
 *
 * 这一整块（列表 + 导入对话框 + 地图/模组面板 + 说明阅读器）都在同一棵 React 树里，
 * 外壳只递数据（槽位 / 库根目录 / 读取中）和几个动作（见 `host.ts`）。
 */
export function CustomView(host: CustomHost) {
  /** 导入按钮的"该开文件对话框了"信号；每次 +1。 */
  const [importNonce, setImportNonce] = useState(0);
  /** 导入对话框的忙碌状态（用存在外壳的这份来禁用按钮和换文案）。 */
  const [importing, setImporting] = useState(false);
  /** 正在看哪一部自制战役。 */
  const [opened, setOpened] = useState<string | null>(null);
  /** 正在读的说明文档。 */
  const [openedDoc, setOpenedDoc] = useState<DocInfo | null>(null);
  /** 等待确认删除的那个版本 —— 删战役是不可逆的，先问一句。 */
  const [confirming, setConfirming] = useState<Variant | null>(null);

  useEffect(() => {
    host.bootstrap();
    // 只在挂载时跑一次：host 的回调每次渲染都是新的，不该进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const customSlot = host.slots.find((slot) => slot.slug === "custom") ?? null;
  const variants = customSlot?.variants ?? [];
  const openedVariant = variants.find((item) => item.id === opened) ?? null;

  /** 导入完直接打开刚进来的那部。 */
  function onImported(): void {
    const newest = variants[0];
    if (newest) setOpened(newest.id);
  }

  /** 启用 / 停用某部自制战役。 */
  async function toggleActive(variant: Variant): Promise<void> {
    const active = customSlot?.active === variant.id;
    const ok = await host.activate("custom", active ? null : variant.id);
    if (ok) host.notify("success", active ? "已停用" : "已启用");
  }

  /**
   * **版本卡片上的右键菜单**。
   *
   * 卡片自己会回调 menu / drop（删除按钮也在卡片上），这一页两个都得接 ——
   * 漏掉的话「删除」就是假按钮、右键也没菜单。
   */
  function showVariantMenu(event: ReactMouseEvent, variant: Variant): void {
    host.showMenu(
      event,
      [
        { id: "open", label: opened === variant.id ? "已打开" : "打开（看地图和模组）" },
        {
          id: "toggle",
          label: customSlot?.active === variant.id ? "停用" : "启用",
        },
        { id: "export", label: "导出这个包…", separatorBefore: true },
        { id: "copy", label: "复制版本目录路径" },
        { id: "drop", label: "从库中删除", danger: true, separatorBefore: true },
      ],
      (id) => {
        if (id === "open") setOpened(variant.id);
        if (id === "toggle") void toggleActive(variant);
        if (id === "export") void doExport(variant);
        if (id === "copy") void copyVariantPath(variant);
        if (id === "drop") setConfirming(variant);
      },
    );
  }

  /** 把版本目录路径塞进剪贴板，方便自己去翻地图。 */
  async function copyVariantPath(variant: Variant): Promise<void> {
    const path = variantPath(host.libraryRoot, "custom", variant.id);
    const ok = await copyText(path);
    host.notify(ok ? "success" : "info", ok ? "已复制：" + path : path);
  }

  /** 导出成一个包。 */
  async function doExport(variant: Variant): Promise<void> {
    try {
      const destination = await api.pickExportPath(variant.name);
      if (!destination) return;

      const report = await api.exportVariant("custom", variant.id, destination, false);
      host.notify(
        "success",
        "已导出：" + [report.maps + " 张地图", report.mods + " 个模组"].join(" · "),
      );
    } catch (error) {
      host.notify("error", errorText(error));
    }
  }

  /** 用户在确认框里点了「删除」。 */
  async function confirmDrop(): Promise<void> {
    const variant = confirming;
    if (!variant) return;
    setConfirming(null);

    const ok = await host.removeVariant("custom", variant.id);
    if (ok && opened === variant.id) setOpened(null);
  }

  /**
   * 页面空白处的右键菜单。
   *
   * 这一页高频操作就三件：导入、看工具、刷新 —— 都给到右键里，
   * 不用去顶栏找按钮。
   */
  function showPageMenu(event: ReactMouseEvent): void {
    host.showMenu(
      event,
      [
        { id: "import", label: "导入战役包…" },
        { id: "cheats", label: "作弊码查询", separatorBefore: true },
        { id: "reload", label: "重新读取" },
      ],
      (id) => {
        if (id === "import") setImportNonce((value) => value + 1);
        if (id === "cheats") host.openCheats();
        if (id === "reload") host.refresh();
      },
    );
  }

  /** 顶部「小工具」：作弊码这类查询工具挂在这里，不单开标签页。 */
  function openTools(event: ReactMouseEvent): void {
    host.showMenu(
      event,
      [
        { id: "cheats", label: "作弊码查询" },
        { id: "reload", label: "重新读取库", separatorBefore: true },
      ],
      (id) => {
        if (id === "cheats") host.openCheats();
        if (id === "reload") host.refresh();
      },
    );
  }

  return (
    <div className="custom-page">
      <div
        className="page"
        onContextMenu={(event) => event.target === event.currentTarget && showPageMenu(event)}
      >
        {/*
          看板：这一页的内容直接压在壁纸上，没有这块半透明底板的话深色文字根本读不出来。
          样式与战役页的 hero 保持一致（半透明 + 背景模糊）。
        */}
        <section className="hero">
          <div className="hero__body">
            <p className="hero__eyebrow">独立整部战役</p>
            <h2 className="hero__title">自制战役</h2>
            <p className="hero__text">
              <strong>不装进游戏目录</strong> —— 地图留在启动器里，
              直接启动游戏就能玩。
            </p>
          </div>

          <div className="hero__actions">
            <button className="btn btn-text" type="button" disabled={host.loading} onClick={openTools}>
              小工具 ▾
            </button>
            <button
              className="btn btn-primary"
              type="button"
              disabled={importing}
              onClick={() => setImportNonce((value) => value + 1)}
            >
              {importing ? "核对中…" : "＋ 导入战役包"}
            </button>
          </div>
        </section>

        <ImportDialog
          entry="custom"
          importNonce={importNonce}
          slots={host.slots}
          droppedPackage={null}
          consumeDropped={() => undefined}
          notify={host.notify}
          refresh={host.refresh}
          onImported={onImported}
          onBusy={setImporting}
        />

        {customSlot && variants.length > 0 ? (
          <div className="grid">
            {variants.map((item) => (
              <VariantCard
                key={item.id}
                slot={customSlot}
                variant={item}
                active={customSlot.active === item.id}
                selected={opened === item.id}
                onPick={() => setOpened(item.id)}
                onActivate={() => void host.activate(customSlot.slug, item.id)}
                onDrop={() => setConfirming(item)}
                onMenu={(event) => showVariantMenu(event, item)}
              />
            ))}
          </div>
        ) : (
          <section className="empty">
            <p className="empty__title">还没有自制战役</p>
            <p className="empty__text">
              点「＋ 导入战役包」选一个包，启动器会自动认出它属于自制战役。
            </p>
          </section>
        )}

        {/* 地图与模组面板：换版本就整个重挂（key 挂 id） */}
        {openedVariant && customSlot && (
          <CustomCampaignPanel
            key={openedVariant.id}
            slot={customSlot.slug}
            variant={openedVariant}
            active={customSlot.active === openedVariant.id}
            refresh={host.refresh}
            notify={host.notify}
            showMenu={host.showMenu}
            onOpenDoc={setOpenedDoc}
          />
        )}

        {openedDoc && openedVariant && customSlot && (
          <DocViewer
            slot={customSlot.slug}
            variantId={openedVariant.id}
            doc={openedDoc}
            onClose={() => setOpenedDoc(null)}
            notify={host.notify}
          />
        )}
      </div>

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
  );
}
