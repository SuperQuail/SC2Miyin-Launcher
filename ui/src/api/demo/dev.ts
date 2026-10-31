// 演示后端：窗口控制。
import type { LauncherApi } from "../types";

import { delay } from "./fixtures";

export const devDemo = {
  windowMinimize: () => delay(undefined),
  windowToggleMaximize: () => delay(false),
  windowClose: () => delay(undefined),
  windowIsMaximized: () => delay(false),
} satisfies Partial<LauncherApi>;
