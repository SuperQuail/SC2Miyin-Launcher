// 演示后端：按需提权助手（演示里永远"成功"，能走通重放路径）。
import type { LauncherApi } from "../types";

import { delay } from "./fixtures";

export const permissionDemo = {
  requestPrivilegedWorker: () => delay(true),
  privilegedOp: () => delay(undefined),
} satisfies Partial<LauncherApi>;
