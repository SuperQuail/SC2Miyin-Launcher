import { useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";

import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { DocInfo, Variant } from "../../../api/types";
import { DocViewer } from "../../components/DocViewer";
import { VariantCard } from "../../components/VariantCard";
import { copyText } from "../../lib/clipboard";
import { Button, Sheet } from "../../ui";
import { setParams, useRoute } from "../../lib/router";
import { ActivationPreviewDialog } from "../campaigns/ActivationPreviewDialog";
import { CustomCampaignPanel } from "../campaigns/CustomCampaignPanel";
import { SortableCard } from "../campaigns/SortableCard";
import { useVariantOrder } from "../campaigns/useVariantOrder";
import { variantPath } from "../campaigns/host";
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
  const route = useRoute();
  /** 正在看哪一部自制战役（页内选择，这轮不进 URL）。 */
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

  /**
   * 这一页在 URL 里的位置就是 `slot=custom` —— 和战役页共用同一个字段
   * （语义都是"开着哪一部"）。页内的变体选择这一轮仍留在页面里。
   */
  useEffect(() => {
    if (route.slot !== "custom") setParams({ slot: "custom" });
  }, [route.slot]);

  const customSlot = host.slots.find((slot) => slot.slug === "custom") ?? null;
  // 保住数组身份：顺序 hook 的依赖里有它，每次新建数组会让键盘监听反复重挂
  const variants = useMemo(() => customSlot?.variants ?? [], [customSlot]);
  const openedVariant = variants.find((item) => item.id === opened) ?? null;
  /** 自制战役的版本卡片顺序（和各个官方战役各记各的）。 */
  const order = useVariantOrder("custom", variants);

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
        { id: "open", label: opened === variant.id ? "已打开" : "打开（查看地图与模组）" },
        {
          id: "toggle",
          label: customSlot?.active === variant.id ? "停用" : "启用",
        },
        { id: "export", label: "导出这个包…", separatorBefore: true },
        { id: "copy", label: "复制版本目录路径" },
        { id: "drop", label: "从战役库删除", danger: true, separatorBefore: true },
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
    host.notify(ok ? "success" : "info", ok ? "已复制路径：" + path : path);
  }

  /** 导出成一个包。 */
  async function doExport(variant: Variant): Promise<void> {
    try {
      const destination = await api.pickExportPath(variant.name);
      if (!destination) return;

      const report = await api.exportVariant("custom", variant.id, destination, false);
      host.notify(
        "success",
        "已导出：" + [report.maps + " 张地图", report.mods + " 个模组"].join("、"),
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
        { id: "importFolder", label: "从文件夹导入…" },
        { id: "cheats", label: "作弊码查询", separatorBefore: true },
        { id: "reload", label: "重新读取战役库" },
      ],
      (id) => {
        if (id === "import") host.openInstall();
        if (id === "importFolder") host.openInstallFolder();
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
        { id: "reload", label: "重新读取战役库", separatorBefore: true },
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
            <p className="hero__eyebrow">独立的完整战役</p>
            <h2 className="hero__title">自制战役</h2>
            <p className="hero__text">
              <strong>启用后会写入游戏目录</strong>（Maps/CustomCampaigns），
              由启动器直接启动游戏。
            </p>
          </div>

          <div className="hero__actions">
            <button className="btn btn-text" type="button" disabled={host.loading} onClick={openTools}>
              工具 ▾
            </button>
            <button
              className="btn btn-outline"
              type="button"
              onClick={() => host.openInstallFolder()}
            >
              从文件夹导入
            </button>
            <button
              className="btn btn-primary"
              type="button"
              onClick={() => host.openInstall()}
            >
              ＋ 导入战役包
            </button>
          </div>
        </section>


        {customSlot && variants.length > 0 ? (
          // 卡片可以拖动排序；整块 grid 也是落点（缝隙里松手同样算数）
          <div
            className="grid"
            onDragOver={(event) => {
              if (event.target === event.currentTarget) order.dragOver(event, event.currentTarget);
            }}
            onDrop={(event) => {
              if (event.target === event.currentTarget) order.dropAt(event, event.currentTarget);
            }}
          >
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
                  slot={customSlot}
                  variant={item}
                  active={customSlot.active === item.id}
                  selected={opened === item.id}
                  onPick={() => setOpened(item.id)}
                  onActivate={() => void host.activate(customSlot.slug, item.id)}
                  onDrop={() => setConfirming(item)}
                  onMenu={(event) => showVariantMenu(event, item)}
                />
              </SortableCard>
            ))}
          </div>
        ) : (
          <section className="empty">
            <p className="empty__title">暂无自制战役</p>
            <p className="empty__text">
              点「＋ 导入战役包」选择压缩包；在此页导入的包都会归为自制战役。
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
            activate={host.activate}
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

      {/* 铺盘前的预演：和战役页共用同一个对话框（以前这里从来没渲染，点了像没反应） */}
      {host.pendingActivation && host.pendingActivation.slot === "custom" && (
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
            将从战役库删除此版本，并从游戏目录移除它写入的地图与模组。
            <strong>删除后需重新导入。</strong>
          </p>
        </Sheet>
      )}
    </div>
  );
}
