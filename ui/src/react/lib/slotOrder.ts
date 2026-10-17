/**
 * 战役卡片的拖动排序。
 *
 * 顺序是**界面偏好**，存 localStorage —— 不进库索引、不问后端。
 *
 * **这是叠加在固定顺序之上的覆盖**（AGENTS §14.2：战役顺序由 `CampaignType::MENU` 定死，
 * 不许按名字排）：保存过的 slug 按保存的顺序排前面，**没保存过的接在后面保持原顺序**，
 * 库里已经没有的 slug 直接忽略。所以新版本 / 新战役进来时不会掉队，也不会打乱别人。
 */

/** localStorage 的键。 */
export const SLOT_ORDER_KEY = "miyin.slotOrder";

/** 读保存的顺序。读不出来（隐私模式 / 存坏了）就当没排过。 */
export function readSlotOrder(): string[] {
  try {
    const raw = localStorage.getItem(SLOT_ORDER_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : [];
  } catch {
    return [];
  }
}

/** 存顺序。存不下就算了 —— 顶多下次打开回到默认顺序，不该因此报错。 */
export function writeSlotOrder(slugs: string[]): void {
  try {
    localStorage.setItem(SLOT_ORDER_KEY, JSON.stringify(slugs));
  } catch {
    /* 隐私模式之类写不进去 */
  }
}

/** 按保存的顺序重排；没记过的排在后面，保持传进来的原顺序。 */
export function applyOrder<T extends { slug: string }>(slots: T[], saved: string[]): T[] {
  const rank = new Map(saved.map((slug, index) => [slug, index]));
  // Array.prototype.sort 是稳定的：没记过的都拿 MAX_SAFE_INTEGER，相对顺序原样保留
  return [...slots].sort(
    (a, b) => (rank.get(a.slug) ?? Number.MAX_SAFE_INTEGER) - (rank.get(b.slug) ?? Number.MAX_SAFE_INTEGER),
  );
}

/**
 * 把 `from` 那一项挪到 `to` 这个**落点**上。
 *
 * 落点是"第几格前面"：`0` = 排到最前，`list.length` = 排到最后。
 * 往下拖时 `to` 落在自己后面的话，去掉自己之后它要往前挪一格（否则会跑到目标后面去）。
 */
export function moveSlot(list: string[], from: number, to: number): string[] {
  if (from < 0 || from >= list.length) return list;
  const rest = [...list];
  const [item] = rest.splice(from, 1);
  const target = to > from ? to - 1 : to;
  rest.splice(Math.max(0, Math.min(target, rest.length)), 0, item);
  return rest;
}

/**
 * 键盘挪一格的落点（`moveSlot` 要的那个"第几格前面"）；挪不动返回 null。
 *
 * 往右要跨过**自己那一格**（+2）—— 落点是自己右边界的话等于没动，
 * 这个坑踩过：按 Alt+→ 看着没事，顺序其实没变。
 */
export function stepTarget(list: string[], from: number, step: -1 | 1): number | null {
  if (from < 0 || from >= list.length) return null;
  const to = step < 0 ? from - 1 : from + 2;
  return to < 0 || to > list.length ? null : to;
}

/**
 * 拖动落在哪一格的**前面**：鼠标在某一格中点左边，落点就在它前面。
 * 被拖的那格自己不算 —— 它本来就要被拿走，算了就会"挡在自己前面"。
 */
export function dropIndex(cards: HTMLElement[], moving: string, clientX: number): number {
  let index = 0;
  for (const card of cards) {
    if (card.dataset.slug === moving) continue;
    const box = card.getBoundingClientRect();
    if (box.left + box.width / 2 <= clientX) index += 1;
  }
  return index;
}
