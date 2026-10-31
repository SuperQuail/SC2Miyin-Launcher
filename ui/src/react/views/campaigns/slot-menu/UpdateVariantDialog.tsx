import { ImportDialog } from "../ImportDialog";

/**
 * 「更新这一版」的对话框（从 `SlotMenu.tsx` 拆出）。
 *
 * 就是 `ImportDialog` 的更新模式：每次 `nonce` +1 开一次文件选择器。
 */
export function UpdateVariantDialog({
  slot,
  variantId,
  nonce,
  slots,
  notify,
  refresh,
  onBusy,
}: {
  slot: string;
  /** 已安装的那一版；空串表示让后端按包的身份去找。 */
  variantId: string;
  nonce: number;
  slots: { slug: string; display_name: string }[];
  notify: (kind: "info" | "success" | "error", text: string) => void;
  refresh: () => void;
  onBusy: (busy: boolean) => void;
}) {
  return (
    <ImportDialog
      mode="update"
      slot={slot}
      variantId={variantId}
      importNonce={nonce}
      slots={slots}
      droppedPackage={null}
      consumeDropped={() => undefined}
      notify={notify}
      refresh={refresh}
      onImported={() => refresh()}
      onBusy={onBusy}
    />
  );
}
