import { type MouseEvent as ReactMouseEvent } from "react";

import { MIYIN } from "../../../api/art";
import { SlotMenu } from "./SlotMenu";
import { DraggableSlotCard } from "./DraggableSlotCard";
import { findSlot, officialSlots, useCampaigns } from "./useCampaigns";
import { useSlotOrder } from "./useSlotOrder";
import { regionLabel, type CampaignsHost } from "./host";
import "./campaigns.css";
import "./sortable.css";

/**
 * 战役列表页（React 版，从 `views/CampaignsView.vue` 整块迁过来）。
 *
 * 一整块都在这棵 React 树里：列表 → 某个战役的菜单 → 导入 / 更新 / 补丁。
 * 外壳只递数据（安装信息、槽位、库根目录）和几个动作（见 `host.ts`）。
 */
export function CampaignsView(host: CampaignsHost) {
  const view = useCampaigns(host);
  const openSlot = findSlot(host.slots, view.opened);

  /** 页面空白处的右键菜单。 */
  function showPageMenu(event: ReactMouseEvent): void {
    host.showMenu(
      event,
      [
        { id: "import", label: "导入战役包…" },
        { id: "reload", label: "重新读取战役库" },
        { id: "cheats", label: "作弊码查询", separatorBefore: true },
        { id: "settings", label: "设置" },
      ],
      (id) => {
        if (id === "import") host.openInstall();
        if (id === "reload") host.refresh();
        if (id === "cheats") host.openCheats();
        if (id === "settings") host.openSettings();
      },
    );
  }

  /** 顶部「小工具」：作弊码这类查询工具挂这里，不单开标签页。 */
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

  const official = officialSlots(host.slots);
  // 卡片顺序：固定顺序打底，用户拖过的按存的顺序排前面（见 lib/slotOrder.ts）
  const order = useSlotOrder(official);
  const modCount = host.slots.reduce((total, slot) => total + slot.variants.length, 0);
  const activeCount = host.slots.filter((slot) => slot.active !== null).length;

  // 点进某个战役就是它自己的菜单页（不是弹窗）
  if (openSlot) {
    return (
      <div className="campaigns-page" onContextMenu={(event) => event.target === event.currentTarget && showPageMenu(event)}>
        <SlotMenu key={openSlot.slug} slot={openSlot} host={host} onBack={() => view.setOpened(null)} />
      </div>
    );
  }

  return (
    <div className="campaigns-page" onContextMenu={(event) => event.target === event.currentTarget && showPageMenu(event)}>
      <div className="page">
        {/* 尚未设置游戏目录 */}
        {!host.installation ? (
          <section className="empty card">
            <img className="empty__art" src={MIYIN.cry} alt="" />
            <h2 className="empty__title">还没有找到星际争霸 II</h2>
            <p className="empty__text">
              启动器会先读取注册表里的安装位置。如果没有找到，请手动指定游戏根目录
              （即包含 <code>StarCraft II.exe</code> 的那一层）。
            </p>
            <div className="empty__actions">
              <button className="btn btn-primary btn-lg" type="button" onClick={host.chooseGameDirectory}>
                选择游戏目录
              </button>
              <button className="btn btn-outline" type="button" onClick={host.openSettings}>
                前往设置
              </button>
            </div>
          </section>
        ) : (
          <>
            {/* 游戏状态 + 看板娘 */}
            <section className="hero card">
              <div className="hero__body">
                <p className="hero__eyebrow">星际争霸 II · {regionLabel(host.installation.branch)}</p>
                <h2 className="hero__title">{host.installation.version ?? "版本未知"}</h2>
                <p className="hero__path" title={host.installation.root}>
                  {host.installation.root}
                </p>

                <div className="hero__actions">
                  <button className="btn btn-primary btn-lg" type="button" onClick={host.launch}>
                    开始游戏
                  </button>
                  <button
                    className="btn btn-outline"
                    type="button"
                    onClick={() => host.reveal(host.installation?.root ?? "")}
                  >
                    打开游戏目录
                  </button>
                </div>

                <dl className="hero__stats">
                  <div className="stat">
                    <dt>官方战役</dt>
                    {/* 自制战役不算"官方" —— slots 里含它，所以这里要滤掉 */}
                    <dd>{official.length}</dd>
                  </div>
                  <div className="stat">
                    <dt>已导入版本</dt>
                    <dd>{modCount}</dd>
                  </div>
                  <div className="stat">
                    <dt>已切换</dt>
                    <dd>{activeCount}</dd>
                  </div>
                </dl>
              </div>

              <div className="hero__kanban" aria-hidden="true">
                <img src={MIYIN.wink} alt="" />
              </div>
            </section>

            {/* 导入战役包：放在外面，自动判断属于哪个战役 */}
            <div className="section-head">
              <h3 className="section-head__title">
                战役
                <span className="section-head__count">{host.slots.length}</span>
              </h3>
              <div className="section-head__actions">
                <button className="btn btn-text" type="button" disabled={host.loading} onClick={host.refresh}>
                  {host.loading ? "读取中…" : "重新读取"}
                </button>
                <button className="btn btn-text" type="button" onClick={openTools}>
                  小工具 ▾
                </button>
                <button
                  className="btn btn-outline"
                  type="button"
                  onClick={() => host.openInstall()}
                >
                  更新战役包…
                </button>
                <button
                  className="btn btn-primary"
                  type="button"
                  disabled={view.busy !== null}
                  onClick={() => host.openInstall()}
                >
                  ＋ 导入战役包
                </button>
              </div>
            </div>

            <p className="section-hint">
              导入时启动器会读取包里的战役信息，自动归入对应的战役；每部战役默认是原版，
              点开可以导入并切换不同玩家制作的版本，多个版本同时保留、互不覆盖。
            </p>


            {/* 整块 grid 也是落点：卡片之间的缝里松手同样算数（卡片自己处理过的不重复处理） */}
            <div
              className="grid"
              onDragOver={(event) => {
                if (event.target === event.currentTarget) order.dragOver(event, event.currentTarget);
              }}
              onDrop={(event) => {
                if (event.target === event.currentTarget) order.dropAt(event, event.currentTarget);
              }}
            >
              {order.slots.map((slot) => (
                <DraggableSlotCard
                  key={slot.slug}
                  slot={slot}
                  dragging={order.dragging}
                  dropMark={order.dropMark}
                  onDragState={order.setDragging}
                  onDragOver={order.dragOver}
                  onDrop={order.dropAt}
                  onOpen={view.setOpened}
                />
              ))}
            </div>
          </>
        )}
      </div>
    </div>
  );
}