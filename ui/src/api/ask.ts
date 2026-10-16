/**
 * 向外壳要一个"是 / 否"的动作（启用版本、删除版本）。
 *
 * 为什么单独一条通道：`ReactIsland` 的 emit 是**单向**的 —— 外壳收到就完了，
 * 没有回执。而这两个动作的调用方要按结果决定下一步（成功了才返回列表 / 清空选中）。
 *
 * 这个文件**框架无关**（不 import vue / react），两边都能用：
 * React 侧 `askShell()` 发问，Vue 外壳在 App.vue 里监听、处理完调 `reply`。
 */
export const ASK_ACTIVATE = "campaign:activate";
export const ASK_REMOVE_VARIANT = "campaign:remove-variant";

/** 问一次；外壳不回话就当没成功（不阻塞界面）。 */
export interface AskDetail {
  slot: string;
  variantId: string | null;
  reply: (ok: boolean) => void;
}

export function askShell(event: string, slot: string, variantId: string | null): Promise<boolean> {
  return new Promise<boolean>((resolve) => {
    const detail: AskDetail = { slot, variantId, reply: resolve };
    window.dispatchEvent(new CustomEvent<AskDetail>(event, { detail }));
  });
}

/** 外壳那一侧：挂上监听。返回卸载函数。 */
export function onShellAsk(
  event: string,
  handler: (slot: string, variantId: string | null) => Promise<boolean>,
): () => void {
  const listener = (raw: Event) => {
    const detail = (raw as CustomEvent<AskDetail>).detail;
    void handler(detail.slot, detail.variantId).then((ok) => detail.reply(ok));
  };
  window.addEventListener(event, listener);
  return () => window.removeEventListener(event, listener);
}
