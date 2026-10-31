import Button from "@jetbrains/ring-ui-built/components/button/button";
import Dialog from "@jetbrains/ring-ui-built/components/dialog/dialog";

/**
 * 打包没完就想走 —— 居中问一句。
 *
 * 用 Ring UI 的 `Dialog`：遮罩、焦点陷阱（`trapFocus`）、Esc 关闭都是它的默认行为。
 * 之前那个手搓遮罩键盘用户根本够不着 —— 焦点一直留在背后的工具栏上。
 */
export function LeaveGuard({
  done,
  total,
  onStay,
  onLeave,
}: {
  done: number;
  total: number;
  onStay: () => void;
  onLeave: () => void;
}) {
  return (
    <Dialog
      show
      trapFocus
      dense
      /* 手搓那版遮罩键盘够不着（焦点留在背后的工具栏上）；Dialog 自带焦点陷阱与 Esc。
         宽度也交给它 —— 自画的 380px 卡片在窄窗口里会被裁掉 */
      label="还在打包"
      closeButtonTitle="不取消，继续打包"
      onCloseClick={onStay}
      onOverlayClick={onStay}
      onCloseAttempt={onStay}
      onEscPress={onStay}
    >
      <h3 className="guard__title">还在打包</h3>
      <p className="guard__text">
        正写到 {done} / {total}。离开就得先取消这次导出 —— <strong>写了一半的包会被删掉</strong>。
      </p>
      <div className="guard__actions">
        <Button onClick={onStay}>不取消</Button>
        {/* 这是"丢掉手上的活"，不是普通的主按钮 —— 用 danger */}
        <Button danger onClick={onLeave}>取消导出并离开</Button>
      </div>
    </Dialog>
  );
}
