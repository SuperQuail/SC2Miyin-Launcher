/**
 * 战役流程（战役列表 → 某个战役的菜单 → 导入/更新/补丁）要跟外壳说的话。
 *
 * 视图**不直接 import 全局状态**（原来那个 697 行的 `useLauncher` 已经拆成
 * `react/state/` 下的一组领域 hook）：数据由外壳组装成 `CampaignsHost` 递进来，
 * 动作直接调。右键菜单的本体在外壳里，这边只说「在哪儿弹、有哪些项」。
 */
import type { MouseEvent as ReactMouseEvent } from "react";

import type { Installation, Preview, SlotView } from "../../../api/types";

/** 一条右键菜单项（和外壳那里的形状一致）。 */
export interface CampaignMenuItem {
  id: string;
  label: string;
  danger?: boolean;
  disabled?: boolean;
  separatorBefore?: boolean;
}

/** 待确认的启用：预演已经算好，等用户点头（外壳算的，弹窗画在菜单页里）。 */
export interface CampaignPendingActivation {
  slot: string;
  variantId: string;
  preview: Preview;
}

/** 菜单里点出来的动作 —— 交给外壳的 `useLauncher`（它还要用预演弹窗等外壳的东西）。 */
export interface CampaignActions {
  /** 启动游戏。 */
  launch: () => void;
  /** 启用某个版本；variantId 为 null 表示切回原版。返回是否成功。 */
  activate: (slot: string, variantId: string | null) => Promise<boolean>;
  /** 从库里删掉一个版本。返回是否成功。 */
  removeVariant: (slot: string, variantId: string) => Promise<boolean>;
  /** 重新读一遍战役库（外壳的 `refresh`）。 */
  refresh: () => void;
  /** 待确认的启用预演；没有就是 null。 */
  pendingActivation: CampaignPendingActivation | null;
  /** 用户在预演里点了「继续」。 */
  confirmActivation: () => Promise<boolean>;
  cancelActivation: () => void;
  /** 打开作弊码查询（外壳的弹层）。 */
  openCheats: () => void;
  /** 切到设置页。 */
  openSettings: () => void;
  /** 进「安装整合包」页（拖包与「导入战役包」都走它）。 */
  openInstall: () => void;
  /** 选一个**文件夹**当战役包，然后进安装页（作者直接发的、已解开的包走这条）。 */
  openInstallFolder: () => void;
}

export interface CampaignsHost extends CampaignActions {
  /** 游戏安装信息；null 表示还没找到。 */
  installation: Installation | null;
  slots: SlotView[];
  /** 库根目录（复制版本目录路径用）。 */
  libraryRoot: string;
  /** 外壳正在读战役库。 */
  loading: boolean;
  /** 外壳的忙碌状态 —— 用于禁用几个按钮，和原来 `busy` 的用法一致。 */
  busy: boolean;
  /** 被拖进窗口的包路径；处理完外壳会清掉。 */
  droppedPackage: string | null;
  /** 拖进来的包已经交给导入对话框了。 */
  consumeDropped: () => void;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  reveal: (path: string) => void;
  /** 首次进入时让外壳去探测游戏、读战役库。 */
  bootstrap: () => void;
  /** 还没设置游戏目录时去选目录。 */
  chooseGameDirectory: () => void;
  showMenu: (event: ReactMouseEvent, items: CampaignMenuItem[], onPick: (id: string) => void) => void;
}

/** 当前选中的版本；`__vanilla__` 是「原版战役」那张卡片的哨兵值。 */
export const VANILLA = "__vanilla__";

/** 版本目录的绝对路径（复制到剪贴板用）。 */
export function variantPath(libraryRoot: string, slot: string, variantId: string): string {
  return libraryRoot + "\\campaigns\\" + slot + "\\" + variantId;
}

/** 区域标签：国服 / 其他。 */
export function regionLabel(branch: string | null | undefined): string {
  if (!branch) return "未知区域";
  return branch.toLowerCase() === "cn" ? "国服" : branch.toUpperCase();
}
