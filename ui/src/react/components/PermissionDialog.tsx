import { permissionReason, type PermissionApi } from "../state/usePermission";
import { Button, Sheet } from "../ui";

/**
 * 按需提权说明（写游戏目录 / 数据根被系统拒绝时弹）。
 *
 * 两个出口：「再次提权并同意」让管理员助手重放刚才那一步（UAC 弹一次），
 * 「保持现状」关掉框 —— 这次操作失败，其余功能照常可用。
 * **必须二选一**：点遮罩不算数 —— 这背后是一次没做的写操作，不该被误点吞掉。
 */
export function PermissionDialog({ gate }: { gate: PermissionApi }) {
  const ask = gate.ask;
  if (!ask) return null;

  return (
    <Sheet
      title="需要管理员权限才能写入游戏目录或数据根"
      dismissOnBackdrop={false}
      actions={
        <>
          <Button onClick={gate.keepAsIs}>保持现状</Button>
          <Button variant="primary" loading={ask.busy} onClick={() => void gate.elevate()}>
            再次提权并同意
          </Button>
        </>
      }
    >
      <p className="sheet__text">系统拒绝了这次写入：{permissionReason(ask.reason)}</p>
      <p className="sheet__text">
        可以让启动器临时拉起一个管理员助手进程来完成（Windows 会弹一次 UAC）。
        同意后启动器会立刻重放刚才那一步；本次运行里再被拒的写操作都直接交给它。
      </p>
      {ask.failure && (
        <>
          <p className="sheet__text permission__failure">提权失败：{ask.failure}</p>
          <p className="sheet__note">可以再点一次「再次提权并同意」，或者先保持现状，其余功能不受影响。</p>
        </>
      )}
    </Sheet>
  );
}
