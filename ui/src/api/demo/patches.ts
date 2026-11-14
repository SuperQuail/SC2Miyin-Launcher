// 演示后端：补丁库、按改版的绑定与合成预演。
import type { BoundPatch, LauncherApi, Patch } from "../types";

import { delay, demoslots, nextDemoCounter } from "./fixtures";

interface DemoBinding {
  enabled: boolean;
  priority: number;
}

type DemoPatch = Patch & { bindings: Record<string, DemoBinding> };

function keyOf(slot: string, variant: string): string {
  return slot + "/" + variant;
}

function firstVariant(slot: string): string {
  return demoslots.find((item) => item.slug === slot)?.variants[0]?.id ?? "";
}

/** 演示补丁：每一条都记录自己在每个改版上的开关与优先级。 */
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
    bindings: {
      [keyOf("hots", firstVariant("hots"))]: { enabled: true, priority: 100 },
    },
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
    bindings: {
      [keyOf("wol", firstVariant("wol"))]: { enabled: false, priority: 200 },
    },
  },
];

function bound(patch: DemoPatch, slot: string, variant: string): BoundPatch {
  const binding = patch.bindings[keyOf(slot, variant)];
  return {
    id: patch.id,
    name: patch.name,
    author: patch.author,
    version: patch.version,
    description: patch.description,
    registration_id: patch.registration_id,
    priority: binding?.priority ?? patch.priority,
    requires: patch.requires,
    payloads: patch.payloads,
    imported_at: patch.imported_at,
    size_bytes: patch.size_bytes,
    mod_count: patch.mod_count,
    enabled: binding?.enabled ?? false,
    matched: false,
  };
}

export const patchesDemo = {
  listPatches: () =>
    delay(
      demopatches.map(({ bindings: _bindings, ...patch }) => patch),
    ),
  listBindings: (slot, variant) =>
    delay(
      demopatches
        .filter((patch) => keyOf(slot, variant) in patch.bindings)
        .map((patch) => bound(patch, slot, variant)),
    ),
  listAvailablePatches: (slot, variant) =>
    delay(
      demopatches
        .filter((patch) => !(keyOf(slot, variant) in patch.bindings))
        .map((patch) => bound(patch, slot, variant)),
    ),
  importPatch: () =>
    delay(
      (() => {
        const id = nextDemoCounter();
        const allVariants = demoslots.flatMap((slot) =>
          slot.variants.map((variant) => keyOf(slot.slug, variant.id)),
        );
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
          bindings: Object.fromEntries(
            allVariants.map((key) => [key, { enabled: false, priority: 100 }]),
          ),
        };
        demopatches.push(created);
        return created;
      })(),
    ),
  autoBindPatches: () => delay([] as string[]),
  bindPatch: (slot, variant, patchId) =>
    delay(
      ((found) => {
        if (found) {
          found.bindings[keyOf(slot, variant)] = {
            enabled: true,
            priority: found.priority,
          };
        }
      })(demopatches.find((item) => item.id === patchId)),
    ),
  unbindPatch: (slot, variant, patchId) =>
    delay(
      ((found) => {
        if (found) delete found.bindings[keyOf(slot, variant)];
      })(demopatches.find((item) => item.id === patchId)),
    ),
  configurePatch: (slot, variant, patchId, enabled, priority) =>
    delay(
      ((found) => {
        if (!found) return found;
        const key = keyOf(slot, variant);
        const binding = found.bindings[key] ?? {
          enabled: true,
          priority: found.priority,
        };
        if (enabled !== null) binding.enabled = enabled;
        if (priority !== null) binding.priority = priority;
        found.bindings[key] = binding;
        return bound(found, slot, variant);
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
