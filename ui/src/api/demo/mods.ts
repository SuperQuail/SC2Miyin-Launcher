// 演示后端：独立模组库。
import type { LauncherApi, ModComparison, ModImport, ModPreview, StandaloneMod } from "../types";

import { delay } from "./fixtures";

export const modDemo = {
  listStandaloneMods: () => delay([]),
  pickModSource: () => delay(null),
  previewMod: () =>
    delay({ name: "演示模组", folder: "演示模组", mod_count: 1, modid: "demo", fingerprint: "", existing: [], duplicate: false, suggested_version: "1.0" } as ModPreview),
  importMod: () =>
    delay({
      record: {
        id: "demo",
        name: "演示模组",
        modid: "demo",
        fingerprint: "",
        folder: "演示模组",
        kind: "folder",
        author: null,
        version: "1.0",
        description: null,
        enabled: false,
        imported_at: 0,
        size_bytes: 0,
        parts: 1,
      } as StandaloneMod,
      records: [],
      action: "added",
      existing: null,
      message: "已导入「演示模组」",
    } as ModImport),
  editMod: () => delay(
    { id: "demo", name: "演示模组", modid: null, fingerprint: "", folder: "演示模组", kind: "folder", source: { kind: "library" }, author: null, version: null, description: null, enabled: false, imported_at: 0, size_bytes: 0, parts: 1 } as StandaloneMod,
  ),
  updateMod: () => delay({ id: "demo", name: "演示模组", modid: null, fingerprint: "", folder: "演示模组", kind: "folder", author: null, version: null, description: null, enabled: false, imported_at: 0, size_bytes: 0, parts: 1 } as StandaloneMod),
  removeMod: () => delay(undefined),
  exportMod: () => delay("D:\\demo.zip"),
  exportMods: () => delay("3 个文件 -> D:\\demo.zip"),
  compareMods: () =>
    delay({
      before: {} as never,
      after: {} as never,
      identical: false,
      files: [],
      semantic: [],
      semantic_note: "演示模式不做对比",
    } as ModComparison),
  setModEnabled: () => delay({ id: "demo", name: "演示模组", modid: null, fingerprint: "", folder: "演示模组", kind: "folder", author: null, version: null, description: null, enabled: true, imported_at: 0, size_bytes: 0, parts: 1 } as StandaloneMod),
} satisfies Partial<LauncherApi>;
