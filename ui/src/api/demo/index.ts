/**
 * 浏览器演示后端。
 *
 * 没有 Tauri 运行时（例如直接用浏览器打开 `vite dev`）时，`bridge.ts` 用它替代真实后端，
 * 界面可以脱离桌面壳独立开发与截图验证。夹具与实现按域拆在同目录：
 * campaign / patches / saves / mods / install / update / dev / permission。
 */
import type { LauncherApi } from "../types";

import { campaignDemo } from "./campaign";
import { devDemo } from "./dev";
import { installDemo } from "./install";
import { modDemo } from "./mods";
import { patchesDemo } from "./patches";
import { permissionDemo } from "./permission";
import { savesDemo } from "./saves";
import { updateDemo } from "./update";

/** 各域拼回一张和真实桥同形的表；漏一个方法这里就编译不过。 */
export const demo: LauncherApi = {
  ...campaignDemo,
  ...patchesDemo,
  ...savesDemo,
  ...modDemo,
  ...installDemo,
  ...updateDemo,
  ...devDemo,
  ...permissionDemo,
};

export { demoActivationConflicts } from "./campaign";
export { delay } from "./fixtures";
