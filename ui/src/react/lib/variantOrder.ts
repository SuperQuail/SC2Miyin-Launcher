/**
 * 战役**版本卡片**的拖动排序（战役菜单页 / 自制战役页）。
 *
 * 顺序是**界面偏好**，存 localStorage —— 不进库索引、不问后端。
 *
 * 两条规矩：
 * 1. **按槽位分开记**（`{ "wol": [...], "hots": [...] }`）：一个战役一个顺序，互不影响。
 * 2. 是**叠加在固定顺序之上的覆盖**：记过的 id 按记的顺序排前面，没记过的接在后面保持
 *    原顺序，库里已经没有的 id 直接忽略 —— 新导入的版本不会掉队，也不会打乱别人。
 *
 * 官方四大战役**那一层**的顺序不归它管：那是固定菜单（AGENTS §14.2，由 CampaignType::MENU 定）。
 */

/** localStorage 的键。 */
export const VARIANT_ORDER_KEY = "miyin.variantOrder";

/** 全部槽位的顺序表。读不出来（隐私模式 / 存坏了）就当没排过。 */
export function readVariantOrder(): Record<string, string[]> {
  try {
    const raw = localStorage.getItem(VARIANT_ORDER_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const out: Record<string, string[]> = {};
    for (const [slot, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (Array.isArray(value)) out[slot] = value.filter((item): item is string => typeof item === "string");
    }
    return out;
  } catch {
    return {};
  }
}

/** 存整张表。存不下就算了 —— 顶多下次打开回到默认顺序，不该因此报错。 */
export function writeVariantOrder(map: Record<string, string[]>): void {
  try {
    localStorage.setItem(VARIANT_ORDER_KEY, JSON.stringify(map));
  } catch {
    /* 隐私模式之类写不进去 */
  }
}

/** 某个槽位记下的顺序；没记过就是空的。 */
export function orderFor(map: Record<string, string[]>, slot: string): string[] {
  return map[slot] ?? [];
}

/** 换掉某个槽位的顺序，别的槽位一个字都不动。 */
export function withOrder(
  map: Record<string, string[]>,
  slot: string,
  ids: string[],
): Record<string, string[]> {
  return { ...map, [slot]: ids };
}

/** 按记下的顺序重排；没记过的排在后面，保持传进来的原顺序。 */
export function applyOrder<T extends { id: string }>(items: T[], saved: string[]): T[] {
  const rank = new Map(saved.map((id, index) => [id, index]));
  // Array.prototype.sort 是稳定的：没记过的都拿 MAX_SAFE_INTEGER，相对顺序原样保留
  return [...items].sort(
    (a, b) => (rank.get(a.id) ?? Number.MAX_SAFE_INTEGER) - (rank.get(b.id) ?? Number.MAX_SAFE_INTEGER),
  );
}

/**
 * 把 `from` 那一项挪到 `to`：**去掉自己之后**它插在第几格（0..list.length-1）。
 *
 * 这个下标口径和 `dropIndex()` 一致 —— 两处都是"剩下那几张里的位置"，
 * 所以"拖到某一格右边"就直接等于 `moveSlot(list, from, dropIndex(...))`。
 * （反过来把 to 当"去掉自己之前"的槽位来减一，会整整差一格：落点线画在 nova 左边、
 * 卡片却插到了 lotv 左边，踩过一次。）
 */
export function moveSlot(list: string[], from: number, to: number): string[] {
  if (from < 0 || from >= list.length) return list;
  const rest = [...list];
  const [item] = rest.splice(from, 1);
  rest.splice(Math.max(0, Math.min(to, rest.length)), 0, item);
  return rest;
}

/**
 * 键盘挪一格的落点；到边了返回 null（挪不动）。
 *
 * 口径同 `moveSlot`：往右 = 插到下一格**后面**（在剩下那几张里就是 `from + 1`）。
 */
export function stepTarget(list: string[], from: number, step: -1 | 1): number | null {
  if (from < 0 || from >= list.length) return null;
  const to = from + step;
  return to < 0 || to > list.length - 1 ? null : to;
}

/**
 * 拖动落在哪一格：数一数鼠标左边还剩几张卡，数出来就是**去掉被拖那张之后**的落点下标。
 *
 * 被拖的那格自己不算 —— 它本来就要被拿走，算了就会"挡在自己前面"。
 * 只数 `data-order-id` 的卡片，所以「原版战役」那张固定卡（没这个属性）不会把落点顶偏。
 */
export function dropIndex(cards: HTMLElement[], moving: string, clientX: number): number {
  let index = 0;
  for (const card of cards) {
    if (card.dataset.orderId === moving) continue;
    const box = card.getBoundingClientRect();
    if (box.left + box.width / 2 <= clientX) index += 1;
  }
  return index;
}
