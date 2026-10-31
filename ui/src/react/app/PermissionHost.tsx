import type { ReactNode } from "react";

import { PermissionDialog } from "../components/PermissionDialog";
import { PermissionScope, usePermission } from "../state/usePermission";

/**
 * 全局按需提权状态。
 *
 * 包在**所有页面外面**：哪个页面撞上 `[PERMISSION]` 都弹这同一个框，
 * 提权后重放的也只是那次操作。读操作不经过它，启动时也不问任何存储权限。
 */
export function PermissionHost({ children }: { children: ReactNode }) {
  const permission = usePermission();
  return (
    <PermissionScope value={permission}>
      {children}
      <PermissionDialog gate={permission} />
    </PermissionScope>
  );
}
