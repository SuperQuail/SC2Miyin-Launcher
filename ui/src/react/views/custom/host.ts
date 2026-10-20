/**
 * 自制战役页要跟外壳说的话。
 *
 * 和战役流程（campaigns/host.ts）一个路数：视图不直接碰全局状态，数据由外壳
 * 组装成 `CustomHost` 递进来，动作直接调。
 */
import type { MouseEvent as ReactMouseEvent } from "react";

import type { SlotView } from "../../../api/types";
import type { CampaignMenuItem, CampaignPendingActivation } from "../campaigns/host";

export interface CustomHost {
  /** 进「安装整合包」页（自制战役版）。 */
  openInstall: () => void;
  /** 全部槽位；这一页只取 slug 为 `custom` 的那个。 */
  slots: SlotView[];
  /** 库根目录（复制版本目录路径用）。 */
  libraryRoot: string;
  /** 外壳正在读战役库。 */
  loading: boolean;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  /** 首次进入时让外壳去探测游戏、读战役库。 */
  bootstrap: () => void;
  /** 重新读一遍战役库。 */
  refresh: () => void;
  /** 启用 / 停用某个版本；variantId 为 null 表示停用。返回是否成功。 */
  activate: (slot: string, variantId: string | null) => Promise<boolean>;
  /** 有没有会写盘的动作在跑（预演对话框的按钮要禁用）。 */
  busy: boolean;
  /**
   * 待用户点头的启用预演。
   *
   * 以前这里没有这个字段 —— 于是撤销预演的状态在自制战役页**从来没被渲染过**，
   * 用户第一次点「启用」的感觉就是"点了没反应"（真 bug，已补）。
   */
  pendingActivation: CampaignPendingActivation | null;
  confirmActivation: () => Promise<boolean>;
  cancelActivation: () => void;
  /** 从库里删掉一个版本。返回是否成功。 */
  removeVariant: (slot: string, variantId: string) => Promise<boolean>;
  showMenu: (event: ReactMouseEvent, items: CampaignMenuItem[], onPick: (id: string) => void) => void;
  /** 打开作弊码查询（外壳的弹层）。 */
  openCheats: () => void;
}
