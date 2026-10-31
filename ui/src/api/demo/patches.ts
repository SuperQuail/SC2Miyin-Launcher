// 演示后端：补丁库、补丁绑定与合成预演。
import type { BoundPatch, LauncherApi } from "../types";

import { delay, nextDemoCounter } from "./fixtures";

/** 演示用的补丁：多一个 bound_to 记录它挂在谁身上。 */
type DemoPatch = BoundPatch & { bound_to: string | null };

/** 演示补丁库。 */
const demopatches: DemoPatch[] = [
  {
    id: "幼儿园补丁",
    name: "肉鸽之心 幼儿园补丁 V3",
    author: "小白的心之所念",
    version: "3.0",
    description: "给大部分单位加了幼儿园标签以进行强化",
    registration_id: "xiaoBai.kindergarten-v3",
    priority: 100,
    requires: ["KerriganRogue"],
    payloads: [],
    imported_at: 1790000000,
    size_bytes: 146_800,
    mod_count: 1,
    enabled: true,
    matched: false,
    bound_to: "hots",
  },
  {
    id: "优化覆盖补丁",
    name: "肉鸽之翼之遗 优化覆盖补丁",
    author: "未知作者",
    version: "1.0",
    description: "天赋优化覆盖",
    registration_id: null,
    priority: 200,
    requires: [],
    payloads: [],
    imported_at: 1790000100,
    size_bytes: 534_000,
    mod_count: 1,
    enabled: false,
    matched: false,
    bound_to: null,
  },
];

export const patchesDemo = {
  listPatches: () => delay(demopatches.slice()),
  listBindings: (slot) => delay(demopatches.filter((item) => item.bound_to === slot)),
  listAvailablePatches: (slot) => delay(demopatches.filter((item) => item.bound_to !== slot)),
  importPatch: () =>
    delay(
      (() => {
        const id = nextDemoCounter();
        const created: DemoPatch = {
          id: "补丁-" + id,
          name: "导入的补丁 v" + id,
          author: "未知作者",
          version: "1.0",
          description: null,
          registration_id: null,
          priority: 100,
          requires: [],
          payloads: [],
          imported_at: 1790000200,
          size_bytes: 200_000,
          mod_count: 1,
          enabled: false,
          matched: false,
          bound_to: null,
        };
        demopatches.push(created);
        return created;
      })(),
    ),
  autoBindPatches: () => delay([] as string[]),
  bindPatch: (slot, patchId) =>
    delay(
      ((found) => {
        if (found) {
          found.bound_to = slot;
          found.enabled = true;
        }
      })(demopatches.find((item) => item.id === patchId)),
    ),
  unbindPatch: (_slot, patchId) =>
    delay(
      ((found) => {
        if (found) found.bound_to = null;
      })(demopatches.find((item) => item.id === patchId)),
    ),
  configurePatch: (_slot, patchId, enabled, priority) =>
    delay(
      ((found) => {
        if (found) {
          if (enabled !== null) found.enabled = enabled;
          if (priority !== null) found.priority = priority;
        }
        return found;
      })(demopatches.find((item) => item.id === patchId)),
    ),
  updatePatch: (patchId, changes) =>
    delay(
      ((found) => {
        if (!found) throw new Error("找不到这个补丁");
        if (changes.name !== undefined) found.name = changes.name;
        if (changes.author !== undefined) found.author = changes.author;
        if (changes.description !== undefined) found.description = changes.description;
        if (changes.priority !== undefined) found.priority = changes.priority;
        return found;
      })(demopatches.find((item) => item.id === patchId)),
    ),
  exportPatch: (patchId, destination) =>
    delay({
      path: destination,
      files: 3,
      maps: 0,
      mods: 1,
      expanded: [],
      patches: [demopatches.find((item) => item.id === patchId)?.name ?? "补丁"],
    }),
  deletePatch: (patchId) =>
    delay(
      (() => {
        const index = demopatches.findIndex((item) => item.id === patchId);
        if (index >= 0) demopatches.splice(index, 1);
      })(),
    ),
  previewComposition: () =>
    delay({
      files: [
        {
          target: "Maps/Campaign/paiur01.SC2Map",
          layer: { kind: "campaign" as const },
          expanded: false,
        },
        {
          target: "Mods/Extra.SC2Mod",
          layer: {
            kind: "patch" as const,
            id: "幼儿园补丁",
            name: "幼儿园补丁 V3",
            priority: 100,
          },
          expanded: true,
        },
      ],
      overridden: [],
    }),
} satisfies Partial<LauncherApi>;
