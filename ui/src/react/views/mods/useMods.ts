import { useCallback, useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";

import { api } from "../../../api/bridge";
import type {
  GameModEntry,
  ModComparison,
  LibraryMod,
  ModImport,
  ModImportMode,
  ModPreview,
  StandaloneMod,
} from "../../../api/types";
import { copyText } from "../../lib/clipboard";
import { useCommand } from "../../lib/useCommand";
import { usePermissionGate } from "../../state/usePermission";
import { filterMods, groupMods } from "./data";

/** 页面要跟外壳说的话。 */
export interface ModsHost {
  notify: (kind: "info" | "success" | "error", text: string) => void;
  reveal: (path: string) => void;
  /** 游戏安装目录；没设置时是 null。 */
  installationRoot: string | null;
  /** 菜单（全局右键菜单在外壳里）。 */
  showMenu: (event: ReactMouseEvent, items: { id: string; label: string; danger?: boolean; disabled?: boolean }[], onPick: (id: string) => void) => void;
  /** 让外壳去 bootstrap（首次进入时探测游戏）。 */
  bootstrap: () => void;
}

/**
 * 模组页的状态与动作。
 *
 * **不再用那个 697 行的 `useLauncher`** —— 这个页面要什么就只拿什么：
 * 列表数据、分组、开关、导入导出、编辑弹窗的状态。外壳只负责递 `installationRoot`
 * 与两个回调（notify / reveal）。
 */
export function useMods(host: ModsHost) {
  /** 提权闸门：挂载 / 启停模组要往游戏目录写东西。 */
  const permission = usePermissionGate();
  const [mods, setMods] = useState<LibraryMod[]>([]);
  const [gameMods, setGameMods] = useState<GameModEntry[]>([]);
  const [keyword, setKeyword] = useState("");
  const [picked, setPicked] = useState<Set<string>>(new Set());
  const [comparing, setComparing] = useState<ModComparison | null>(null);
  const [compareWith, setCompareWith] = useState<Record<string, string>>({});

  /**
   * 读取。
   *
   * 这是本页唯一"读"的动作 —— 用 useCommand 拿统一的 loading/error，
   * 不再手抄 `try/catch + notify`（原来那一套在这个文件里写了十几遍）。
   */
  const load = useCommand(
    useCallback(async () => {
      const [library, game] = await Promise.all([api.listLibraryMods(), api.listGameMods()]);
      setMods(library);
      setGameMods(game);
      return library;
    }, []),
    { onError: (text) => host.notify("error", text) },
  );

  useEffect(() => {
    host.bootstrap();
    void load.run();
    // 只在挂载时跑一次：host 的回调每次渲染都是新的，不该进依赖
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const filtered = useMemo(() => filterMods(mods, keyword), [mods, keyword]);
  const groups = useMemo(() => groupMods(filtered), [filtered]);
  const mountedCount = mods.filter((mod) => mod.mounted).length;
  const standaloneCount = mods.filter((mod) => mod.origin === "standalone").length;

  /** 一个动作的"在跑"的键：同时只允许一个写操作（原来就是拿 busy 当锁）。 */
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const guard = useCallback(async (key: string, action: () => Promise<void>) => {
    setBusyKey(key);
    try {
      await action();
    } finally {
      setBusyKey(null);
    }
  }, []);

  /** 开关一个模组。独立模组和战役模组走两条路。 */
  const toggle = useCallback(
    (mod: LibraryMod) =>
      guard(mod.path, async () => {
        try {
          if (mod.standalone_id) {
            const id = mod.standalone_id;
            await permission.run(() => api.setModEnabled(id, !mod.mounted), {
              op: "mods_set_enabled",
              args: { id, enabled: !mod.mounted },
            });
          } else {
            // 战役包带的模组：把同版本的模组状态整体写回
            const siblings = mods.filter(
              (item) => item.slot === mod.slot && item.variant_id === mod.variant_id,
            );
            const next = siblings
              .map((item) => (item.path === mod.path ? !item.mounted : item.mounted))
              .map((mounted, index) => (mounted ? siblings[index].path : null))
              .filter((path): path is string => path !== null);
            await api.setMountedMods(mod.slot, mod.variant_id, next);
          }
          await load.run();
          host.notify("success", (mod.mounted ? "已取消勾选 " : "已勾选 ") + mod.name);
        } catch (error) {
          host.notify("error", String(error));
        }
      }),
    [guard, mods, load, host, permission],
  );

  /** 整组一起开 / 关。 */
  const toggleGroup = useCallback(
    (group: { slot: string; variantId: string; mods: LibraryMod[] }, mounted: boolean) =>
      guard(group.slot + group.variantId, async () => {
        try {
          const standalone = group.mods.filter((mod) => mod.standalone_id);
          for (const mod of standalone) {
            const id = mod.standalone_id as string;
            await permission.run(() => api.setModEnabled(id, mounted), {
              op: "mods_set_enabled",
              args: { id, enabled: mounted },
            });
          }
          const campaign = group.mods.filter((mod) => !mod.standalone_id);
          if (campaign.length) {
            await api.setMountedMods(
              campaign[0].slot,
              campaign[0].variant_id,
              mounted ? campaign.map((mod) => mod.path) : [],
            );
          }
          await load.run();
          host.notify("success", mounted ? "已勾选全部模组" : "已取消勾选全部模组");
        } catch (error) {
          host.notify("error", String(error));
        }
      }),
    [guard, load, host, permission],
  );

  /** 复制模组在游戏目录里的名字。 */
  const copyMount = useCallback(
    async (mod: LibraryMod) => {
      const text = "Mods/" + (mod.folder ?? mod.name);
      const ok = await copyText(text);
      host.notify(ok ? "success" : "info", ok ? "已复制：" + text : text);
    },
    [host],
  );

  /** 拿这一版跟同组的另一版比。 */
  const compare = useCallback(
    (current: LibraryMod) =>
      guard(current.path, async () => {
        const baseId = compareWith[current.modid ?? ""] ?? "";
        if (!baseId || !current.standalone_id) return;
        try {
          setComparing(await api.compareMods(baseId, current.standalone_id));
        } catch (error) {
          host.notify("error", String(error));
        }
      }),
    [guard, compareWith, host],
  );

  /** 把勾中的版本打成一个包。 */
  const exportPicked = useCallback(
    () =>
      guard("export-many", async () => {
        const ids = [...picked];
        if (!ids.length) return;
        try {
          const message = await api.exportMods(ids);
          host.notify("success", "已导出：" + message);
          setPicked(new Set());
        } catch (error) {
          const text = String(error);
          if (!text.includes("取消")) host.notify("error", text);
        }
      }),
    [guard, picked, host],
  );

  /** 导出单个模组。 */
  const exportMod = useCallback(
    (mod: LibraryMod) =>
      guard(mod.path, async () => {
        if (!mod.standalone_id) return;
        try {
          const target = await api.exportMod(mod.standalone_id);
          host.notify("success", "已导出到 " + target);
        } catch (error) {
          const text = String(error);
          // 用户点取消不是错误，别弹红字
          if (!text.includes("取消")) host.notify("error", text);
        }
      }),
    [guard, host],
  );

  const togglePick = useCallback((id: string) => {
    setPicked((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  return {
    mods,
    gameMods,
    keyword,
    setKeyword,
    picked,
    togglePick,
    comparing,
    setComparing,
    compareWith,
    setCompareWith,
    filtered,
    groups,
    mountedCount,
    standaloneCount,
    loading: load.loading,
    busyKey,
    reload: () => void load.run(),
    toggle,
    toggleGroup,
    copyMount,
    compare,
    exportPicked,
    exportMod,
  };
}

/** 导入那一段单独拿出来：它有"库里已有同族"这一步要问用户。 */
export function useModImport(
  host: ModsHost,
  reload: () => void | Promise<void>,
  setChoosing: (value: { path: string; preview: ModPreview } | null) => void,
) {
  const permission = usePermissionGate();
  const [importing, setImporting] = useState(false);

  const report = useCallback(
    (result: ModImport) => {
      host.notify(result.action === "duplicate" ? "info" : "success", result.message);
      if (result.action === "added") host.notify("info", "在列表中勾选后，启动器会把它写入游戏目录");
    },
    [host],
  );

  const run = useCallback(
    async (kind: "file" | "folder", mode?: ModImportMode, path?: string) => {
      setImporting(true);
      try {
        const source = path ?? (await api.pickModSource(kind));
        if (!source) return;

        const found = await api.previewMod(source);
        // 库里已经有一份完全一样的：不用问，后端会直接用那份
        if (found.duplicate) {
          const result = await permission.run(() => api.importMod(source), {
            op: "mods_import",
            args: { path: source, mode: null },
          });
          await reload();
          host.notify("info", result ? result.message : "已导入模组");
          return;
        }
        // 有同 modid 的 -> 问一句
        if (found.existing.length && !mode) {
          setChoosing({ path: source, preview: found });
          return;
        }
        const result = await permission.run(() => api.importMod(source, mode), {
          op: "mods_import",
          args: { path: source, mode: mode ?? null },
        });
        await reload();
        if (result) report(result);
        else host.notify("success", "已导入模组");
      } catch (error) {
        host.notify("error", String(error));
      } finally {
        setImporting(false);
      }
    },
    [host, permission, reload, report, setChoosing],
  );

  return { importing, importMod: run };
}

/** 编辑弹窗的状态与保存。 */
export function useModEdit(host: ModsHost, reload: () => void | Promise<void>) {
  const permission = usePermissionGate();
  const [editing, setEditing] = useState<LibraryMod | null>(null);
  const [editTarget, setEditTarget] = useState<StandaloneMod | null>(null);
  const [form, setForm] = useState({ name: "", author: "", version: "", description: "" });

  /**
   * 打开编辑框 —— **所有模组都能改**，不只是独立导入的。
   *
   * 跟着战役包来的模组也是一条模组记录：第一次编辑时后端会先建一条
   * （只存元数据），改的是记录、不动包里的原始文件。
   */
  const openEdit = useCallback(
    async (mod: LibraryMod) => {
      setEditing(mod);
      setEditTarget(null);
      try {
        if (mod.mod_record_id) {
          const all = await api.listStandaloneMods();
          const found = all.find((item) => item.id === mod.mod_record_id);
          if (found) {
            setEditTarget(found);
            setForm({
              name: found.name,
              author: found.author ?? "",
              version: found.version ?? "",
              description: found.description ?? "",
            });
            return;
          }
        }
        setForm({ name: mod.name, author: "", version: mod.version ?? "", description: "" });
      } catch (error) {
        host.notify("error", String(error));
      }
    },
    [host],
  );

  const [saving, setSaving] = useState(false);
  const save = useCallback(async () => {
    const current = editing;
    if (!current) return;
    setSaving(true);
    try {
      // 战役包带来的、还没记录的：带上定位信息，后端会先建一条记录
      const identity =
        !editTarget && "mod_record_id" in current
          ? {
              slot: current.slot,
              variant: current.variant_id,
              path: current.path,
              folder: current.folder ?? current.name,
              name: current.name,
              version: current.version,
              kind: current.kind ?? "folder",
              parts: current.parts,
            }
          : null;

      const recordId = editTarget?.id ?? null;
      const changes = {
        name: form.name,
        author: form.author || null,
        version: form.version || null,
        description: form.description || null,
      };
      await permission.run(() => api.editMod(recordId, identity, changes), {
        op: "mods_edit",
        args: { recordId, identity, changes },
      });

      setEditing(null);
      setEditTarget(null);
      await reload();
      host.notify("success", "已保存");
    } catch (error) {
      host.notify("error", String(error));
    } finally {
      setSaving(false);
    }
  }, [editing, editTarget, form, host, permission, reload]);

  return { editing, setEditing, form, setForm, openEdit, save, saving };
}

/** 删除确认。 */
export function useModRemove(host: ModsHost, reload: () => void | Promise<void>) {
  const permission = usePermissionGate();
  const [removing, setRemoving] = useState<LibraryMod | null>(null);
  const [removingBusy, setBusy] = useState(false);

  const confirm = useCallback(async () => {
    const mod = removing;
    if (!mod?.standalone_id) return;
    const id = mod.standalone_id;
    setRemoving(null);
    setBusy(true);
    try {
      await permission.run(() => api.removeMod(id), { op: "mods_remove", args: { id } });
      await reload();
      host.notify("success", "已删除");
    } catch (error) {
      host.notify("error", String(error));
    } finally {
      setBusy(false);
    }
  }, [removing, host, permission, reload]);

  return { removing, setRemoving, confirm, removingBusy };
}
