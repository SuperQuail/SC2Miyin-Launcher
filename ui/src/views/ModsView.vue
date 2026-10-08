<script setup lang="ts">
/**
 * 模组管理：把库里所有战役的模组摊平在一页，一键开关。
 *
 * 为什么单独一页 —— 官方战役的改版包一样可能带模组（换了单位的重制版就是），
 * 以前只能进到某个战役里一个个勾，想跨战役看「我到底挂了哪些模组」很费劲。
 */
import { computed, onMounted, ref } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type {
  GameModEntry,
  LibraryMod,
  ModComparison,
  ModImport,
  ModImportMode,
  ModPreview,
  StandaloneMod,
} from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const { installation, loading: bootLoading, reveal, notify, bootstrap } = useLauncher();

const mods = ref<LibraryMod[]>([]);
const gameMods = ref<GameModEntry[]>([]);
const loading = ref(true);
const busy = ref<string | null>(null);
const keyword = ref("");
const importing = ref(false);

/** 正在编辑的独立模组；null 表示没开编辑框。 */
const editing = ref<StandaloneMod | null>(null);
/** 正在看的对比结果。 */
const comparing = ref<ModComparison | null>(null);
/** 对比时选中的「另一个版本」。 */
const compareWith = ref<Record<string, string>>({});

/** 勾选要一起导出的版本（只对独立模组有意义）。 */
const picked = ref<Set<string>>(new Set());

/** 勾 / 取消一个版本。 */
function togglePick(id: string): void {
  const next = new Set(picked.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  picked.value = next;
}

/**
 * 拿这一版跟同组的另一版比。
 *
 * `compareWith` 记的是「这个 modid 选了哪个版本去比」—— 一组版本里挑一个基准。
 */
async function compare(current: LibraryMod): Promise<void> {
  const baseId = compareWith.value[current.modid ?? ""] ?? "";
  if (!baseId || !current.standalone_id) return;

  busy.value = current.path;
  try {
    comparing.value = await api.compareMods(baseId, current.standalone_id);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/** 把勾中的版本打成一个包。 */
async function exportPicked(): Promise<void> {
  const ids = [...picked.value];
  if (!ids.length) return;

  busy.value = "export-many";
  try {
    const message = await api.exportMods(ids);
    notify("success", "已导出：" + message);
    picked.value = new Set();
  } catch (error) {
    const text = errorText(error);
    if (!text.includes("取消")) notify("error", text);
  } finally {
    busy.value = null;
  }
}

/** 等着用户选「新版本 / 独立改版」的那次导入。 */
const choosing = ref<{ path: string; preview: ModPreview } | null>(null);
/** 等着确认删除的那个模组。 */
const removing = ref<LibraryMod | null>(null);
const form = ref({ name: "", author: "", version: "", description: "" });

onMounted(async () => {
  await bootstrap();
  await load();
});

async function load(): Promise<void> {
  loading.value = true;
  try {
    mods.value = await api.listLibraryMods();
    gameMods.value = await api.listGameMods();
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
  }
}

/** 模组名或所属战役。 */
const filtered = computed(() => {
  const needle = keyword.value.trim().toLowerCase();
  if (!needle) return mods.value;
  return mods.value.filter(
    (mod) =>
      mod.name.toLowerCase().includes(needle) ||
      mod.variant_name.toLowerCase().includes(needle) ||
      mod.slot_name.toLowerCase().includes(needle),
  );
});

/**
 * 按「来源 → 战役 → 版本」分组。
 *
 * 模组不是孤立的：它总是跟着某个包来的，或者单独导入的。
 * 分开显示用户才知道「关掉它会影响谁」。
 */
const groups = computed(() => {
  const buckets = new Map<
    string,
    {
      key: string;
      slot: string;
      slotName: string;
      variantId: string;
      variantName: string;
      origin: LibraryMod["origin"];
      mods: LibraryMod[];
    }
  >();

  for (const mod of filtered.value) {
    // 独立模组按 **modid** 归并 —— 同一个模组的多个版本显示成一组，
    // 用户一眼看得出「这几个是同一个东西的不同时期」。
    // 战役包带的模组按「来源 + 版本」分组，它们本来就是跟着包走的。
    const key =
      mod.origin === "standalone"
        ? "standalone|" + (mod.modid ?? mod.path)
        : mod.origin + "|" + mod.slot + "|" + mod.variant_id;
    const bucket = buckets.get(key);
    if (bucket) bucket.mods.push(mod);
    else
      buckets.set(key, {
        key,
        slot: mod.slot,
        slotName: mod.origin === "standalone" ? (mod.modid ?? mod.name) : mod.slot_name,
        variantId: mod.variant_id,
        variantName: mod.variant_name,
        origin: mod.origin,
        mods: [mod],
      });
  }

  return [...buckets.values()];
});

const mountedCount = computed(() => mods.value.filter((mod) => mod.mounted).length);
const hasMods = computed(() => mods.value.length > 0);
const standaloneCount = computed(
  () => mods.value.filter((mod) => mod.origin === "standalone").length,
);

/** 来源怎么说。 */
function originLabel(origin: LibraryMod["origin"]): string {
  if (origin === "standalone") return "单独导入";
  if (origin === "custom_campaign") return "自制战役包";
  return "原版战役包";
}

/** 开关一个模组。独立模组和战役模组走两条路。 */
async function toggle(mod: LibraryMod): Promise<void> {
  busy.value = mod.path;
  try {
    if (mod.standalone_id) {
      await api.setModEnabled(mod.standalone_id, !mod.mounted);
    } else {
      await toggleCampaignMod(mod);
    }
    await load();
    notify("success", (mod.mounted ? "已关闭 " : "已启用 ") + mod.name);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/** 战役包带的模组：把同版本的模组状态整体写回。 */
async function toggleCampaignMod(mod: LibraryMod): Promise<void> {
  const siblings = mods.value.filter(
    (item) => item.slot === mod.slot && item.variant_id === mod.variant_id,
  );
  const next = siblings
    .map((item) => (item.path === mod.path ? !item.mounted : item.mounted))
    .map((mounted, index) => (mounted ? siblings[index].path : null))
    .filter((path): path is string => path !== null);

  await api.setMountedMods(mod.slot, mod.variant_id, next);
}

/** 整组一起开 / 关。 */
async function toggleGroup(
  group: { slot: string; variantId: string; mods: LibraryMod[] },
  mounted: boolean,
): Promise<void> {
  busy.value = group.slot + group.variantId;
  try {
    const standalone = group.mods.filter((mod) => mod.standalone_id);
    if (standalone.length) {
      for (const mod of standalone) {
        await api.setModEnabled(mod.standalone_id as string, mounted);
      }
    }

    const campaign = group.mods.filter((mod) => !mod.standalone_id);
    if (campaign.length) {
      await api.setMountedMods(
        campaign[0].slot,
        campaign[0].variant_id,
        mounted ? campaign.map((mod) => mod.path) : [],
      );
    }

    await load();
    notify("success", mounted ? "已全部启用" : "已全部关闭");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/**
 * 导入一个模组包。
 *
 * 流程：选包 -> 预检 -> **库里已有同族的就问一句** -> 导入。
 *
 * 为什么要问：同 modid 的可能是「同一个模组的新版本」，也可能是
 * 「有人拿它改了个版本」—— 后者跟原版是两码事，硬并进版本列表会让人
 * 以为它们是同一个东西的不同时期。这个只有用户知道，替他决定太武断。
 */
async function importMod(kind: "file" | "folder"): Promise<void> {
  importing.value = true;
  try {
    const path = await api.pickModSource(kind);
    if (!path) return;

    const found = await api.previewMod(path);

    // 库里已经有一份**完全一样**的：不用问，后端会直接用那份
    if (found.duplicate) {
      const result = await api.importMod(path);
      await load();
      notify("info", result.message);
      return;
    }

    // 有同 modid 的 -> 问一句
    if (found.existing.length) {
      choosing.value = { path, preview: found };
      return;
    }

    const result = await api.importMod(path);
    await load();
    report(result);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}

/** 用户在「新版本 / 独立改版」之间选了。 */
async function chooseMode(mode: ModImportMode): Promise<void> {
  const current = choosing.value;
  if (!current) return;
  choosing.value = null;

  importing.value = true;
  try {
    const result = await api.importMod(current.path, mode);
    await load();
    report(result);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}

/** 把导入结果如实说给用户。 */
function report(result: ModImport): void {
  notify(result.action === "duplicate" ? "info" : "success", result.message);
  if (result.action === "added") {
    notify("info", "到列表里把它启用就会铺进游戏目录");
  }
}

/** 打开编辑框。 */
function editModAction(mod: LibraryMod): void {
  if (!mod.standalone_id) return;
  void (async () => {
    try {
      const all = await api.listStandaloneMods();
      const found = all.find((item) => item.id === mod.standalone_id);
      if (!found) return;
      editing.value = found;
      form.value = {
        name: found.name,
        author: found.author ?? "",
        version: found.version ?? "",
        description: found.description ?? "",
      };
    } catch (error) {
      notify("error", errorText(error));
    }
  })();
}

async function saveEdit(): Promise<void> {
  const current = editing.value;
  if (!current) return;

  busy.value = current.id;
  try {
    await api.updateMod(current.id, {
      name: form.value.name,
      author: form.value.author || null,
      version: form.value.version || null,
      description: form.value.description || null,
    });
    editing.value = null;
    await load();
    notify("success", "已保存");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/** 导出成一个 zip。 */
async function exportMod(mod: LibraryMod): Promise<void> {
  if (!mod.standalone_id) return;
  busy.value = mod.path;
  try {
    const target = await api.exportMod(mod.standalone_id);
    notify("success", "已导出到 " + target);
  } catch (error) {
    const text = errorText(error);
    // 用户点取消不是错误，别弹红字
    if (text.includes("取消")) return;
    notify("error", text);
  } finally {
    busy.value = null;
  }
}

/** 删掉。 */
async function removeMod(mod: LibraryMod): Promise<void> {
  if (!mod.standalone_id) return;
  // 用自绘的确认框而不是 window.confirm —— 浏览器原生弹窗跟整个界面不是一套
  removing.value = mod;
}

/** 确认删除。 */
async function confirmRemove(): Promise<void> {
  const mod = removing.value;
  if (!mod?.standalone_id) return;
  removing.value = null;

  busy.value = mod.path;
  try {
    await api.removeMod(mod.standalone_id);
    await load();
    notify("success", "已删除");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/** 打开游戏 Mods 目录。 */
function openModsDir(): void {
  if (installation.value) reveal(installation.value.root + "\\Mods");
}
</script>

<template>
  <div class="page">
    <section class="hero">
      <div class="hero__body">
        <p class="hero__eyebrow">
          共 {{ mods.length }} 个，已启用 {{ mountedCount }} 个
          <span v-if="standaloneCount"> · 其中单独导入 {{ standaloneCount }} 个</span>
        </p>
        <h2 class="hero__title">模组</h2>
        <p class="hero__text">
          地图可能需要模组才能正常打开 —— 原版战役的改版包也一样。
          这里能看到模组的来源，启停它们，也能单独导入、导出、改信息。
        </p>
      </div>
      <div class="hero__actions">
        <button
          v-if="picked.size"
          class="btn btn-tonal"
          type="button"
          :disabled="busy !== null"
          @click="exportPicked"
        >
          导出选中的 {{ picked.size }} 个版本
        </button>
        <button class="btn btn-text" type="button" :disabled="loading" @click="load">
          {{ loading ? "读取中…" : "重新读取" }}
        </button>
        <button class="btn btn-text" type="button" :disabled="!installation" @click="openModsDir">
          打开游戏 Mods 目录
        </button>
        <button class="btn btn-text" type="button" :disabled="importing" @click="importMod('folder')">
          从文件夹导入
        </button>
        <button class="btn btn-primary" type="button" :disabled="importing" @click="importMod('file')">
          {{ importing ? "导入中…" : "＋ 导入模组包" }}
        </button>
      </div>
    </section>

    <div class="bar">
      <input v-model="keyword" class="bar__search" type="search" placeholder="搜索模组或所属战役" />
      <span class="bar__count">{{ filtered.length }} / {{ mods.length }}</span>
    </div>

    <p v-if="loading || bootLoading" class="hint">正在读取…</p>
    <p v-else-if="!hasMods" class="hint">
      现在还没有任何模组。导入带 .SC2Mod 的战役包，或者直接用上面的按钮单独导一个。
    </p>
    <p v-else-if="!filtered.length" class="hint">没有匹配的模组。</p>

    <section v-for="group in groups" :key="group.key" class="group">
      <header class="group__head">
        <div class="group__title">
          <span class="origin" :class="'origin--' + group.origin">
            {{ originLabel(group.origin) }}
          </span>
          <span class="group__campaign">{{ group.slotName }}</span>
          <span v-if="group.variantName" class="group__variant">{{ group.variantName }}</span>
          <span v-if="group.origin === 'standalone' && group.mods.length > 1" class="badge">
            {{ group.mods.length }} 个版本
          </span>
        </div>
        <div class="group__actions">
          <!--
            独立模组是**版本互斥**的：同一 modid 只能启用一个（它们在游戏目录里
            抢同一个文件夹），所以不提供「全部启用」。
          -->
          <button
            v-if="group.origin !== 'standalone'"
            class="btn btn-text btn--tiny"
            type="button"
            :disabled="busy !== null"
            @click="toggleGroup(group, true)"
          >
            全部启用
          </button>
          <button
            class="btn btn-text btn--tiny"
            type="button"
            :disabled="busy !== null"
            @click="toggleGroup(group, false)"
          >
            全部关闭
          </button>
        </div>
      </header>

      <ul class="mods">
        <li v-for="mod in group.mods" :key="mod.path" class="mod">
          <div class="mod__info">
            <input
              v-if="mod.standalone_id"
              class="mod__pick"
              type="checkbox"
              :checked="picked.has(mod.standalone_id)"
              :title="'勾上：导出时带上这个版本'"
              @change="togglePick(mod.standalone_id as string)"
            />
            <span class="mod__name">{{ mod.name }}</span>
            <span v-if="mod.required" class="badge badge--req" title="包内声明为依赖模组">依赖</span>
            <span v-if="mod.origin === 'standalone' && mod.version" class="badge badge--ver">
              v{{ mod.version }}
            </span>
            <span v-if="mod.parts > 1" class="badge">{{ mod.parts }} 个文件</span>
            <!-- 把「铺进游戏目录时叫什么」显出来：地图里引用的就是这个，用户一眼能核对 -->
            <span v-if="mod.folder" class="mount" :title="'铺进游戏目录后的样子'">
              → Mods/{{ mod.folder }}{{ mod.kind === "folder" ? "/" : "" }}
            </span>
          </div>

          <div class="mod__actions">
            <template v-if="mod.standalone_id">
              <select
                v-if="group.mods.length > 1"
                class="mod__base"
                :value="compareWith[mod.modid ?? ''] ?? ''"
                @change="compareWith[mod.modid ?? ''] = ($event.target as HTMLSelectElement).value"
              >
                <option value="">基准…</option>
                <option
                  v-for="other in group.mods.filter((item) => item.path !== mod.path)"
                  :key="other.path"
                  :value="other.standalone_id ?? ''"
                >
                  与 v{{ other.version ?? "未标" }} 比
                </option>
              </select>
              <button
                v-if="compareWith[mod.modid ?? '']"
                class="btn btn-text btn--tiny"
                type="button"
                :disabled="busy !== null"
                @click="compare(mod)"
              >
                对比
              </button>
              <button
                class="btn btn-text btn--tiny"
                type="button"
                :disabled="busy !== null"
                @click="editModAction(mod)"
              >
                编辑
              </button>
              <button
                class="btn btn-text btn--tiny"
                type="button"
                :disabled="busy !== null"
                @click="exportMod(mod)"
              >
                导出
              </button>
              <button
                class="btn btn-text btn--tiny btn--danger"
                type="button"
                :disabled="busy !== null"
                @click="removeMod(mod)"
              >
                删除
              </button>
            </template>

            <button
              class="switch"
              :class="{ 'switch--on': mod.mounted }"
              type="button"
              :disabled="busy !== null"
              :title="mod.mounted ? '点击关闭' : '点击启用'"
              @click="toggle(mod)"
            >
              <span class="switch__knob"></span>
              <span class="switch__text">{{ mod.mounted ? "已启用" : "已关闭" }}</span>
            </button>
          </div>
        </li>
      </ul>
    </section>

    <!-- 游戏目录里实际放着什么 -->
    <section class="game">
      <header class="game__head">
        <h3 class="game__title">游戏目录里的模组</h3>
        <span class="game__count">{{ gameMods.length }} 个</span>
      </header>
      <p class="game__hint">
        这是 <code>Mods/</code> 里实际存在的文件 —— 包括你自己放的、别的工具留下的。
        上面启用模组后会立刻铺进来。
      </p>

      <p v-if="!gameMods.length" class="game__empty">游戏目录里还没有模组。</p>
      <ul v-else class="mods">
        <li v-for="entry in gameMods" :key="entry.name" class="mod mod--ro">
          <div class="mod__info">
            <span class="mod__name">{{ entry.display }}</span>
          </div>
          <div class="mod__actions">
            <span class="mod__path">{{ entry.name }}{{ entry.expanded ? "（目录）" : "" }}</span>
            <span class="mod__size">{{ formatBytes(entry.size_bytes) }}</span>
          </div>
        </li>
      </ul>
    </section>

    <!-- 对比结果 -->
    <div v-if="comparing" class="sheet" @click.self="comparing = null">
      <div class="sheet__card sheet__card--wide">
        <h3 class="sheet__title">
          {{ comparing.before.name }} v{{ comparing.before.version ?? "未标" }}
          →
          v{{ comparing.after.version ?? "未标" }}
        </h3>

        <p v-if="comparing.identical" class="sheet__text">
          两个版本内容<strong>完全一样</strong>。
        </p>

        <template v-else>
          <p class="sheet__text">
            共 <strong>{{ comparing.files.filter((f) => f.status !== "same").length }}</strong>
            个文件有差异（总 {{ comparing.files.length }} 个）。
          </p>

          <ul class="diff">
            <li
              v-for="file in comparing.files.filter((f) => f.status !== 'same')"
              :key="file.path"
              class="diff__row"
              :class="'diff__row--' + file.status"
            >
              <span class="diff__tag">{{ file.status }}</span>
              <span class="diff__path">{{ file.path }}</span>
            </li>
          </ul>

          <details v-for="item in comparing.semantic" :key="item.path" class="semantic">
            <summary class="semantic__head">语义 diff：{{ item.path }}</summary>
            <pre class="semantic__body">{{ item.text }}</pre>
          </details>

          <p v-if="comparing.semantic_note" class="sheet__note">
            {{ comparing.semantic_note }}
          </p>
        </template>

        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="comparing = null">关闭</button>
        </div>
      </div>
    </div>

    <!-- 同 modid：问一句作为新版本还是独立改版 -->
    <div v-if="choosing" class="sheet" @click.self="choosing = null">
      <div class="sheet__card">
        <h3 class="sheet__title">「{{ choosing.preview.name }}」要怎么放？</h3>
        <p v-if="choosing.preview.mod_count > 1" class="sheet__text">
      这一包里带了 <strong>{{ choosing.preview.mod_count }}</strong> 个模组，
      下面说的是其中<strong>第一个</strong>（<code>{{ choosing.preview.folder }}</code>）。
      其余会按各自的名字分别导入。
    </p>

    <p class="sheet__text">
          库里已经有 <strong>{{ choosing.preview.existing.length }}</strong> 个同为
          <code>{{ choosing.preview.modid }}</code> 的模组：
        </p>

        <ul class="existing">
          <li v-for="item in choosing.preview.existing" :key="item.id" class="existing__row">
            <span class="existing__name">{{ item.name }}</span>
            <span class="existing__ver">v{{ item.version ?? "未标版本" }}</span>
          </li>
        </ul>

        <p class="sheet__text">
          <strong>作为新版本</strong>：归到同一个模组下面，之后可以自由切换用哪一版。<br />
          <strong>作为独立改版</strong>：单独显示一个 —— 适用于「拿别人的模组改的」，
          它跟原版是两码事。
        </p>

        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="chooseMode('separate')">
            作为独立改版
          </button>
          <button class="btn btn-primary" type="button" @click="chooseMode('version')">
            作为新版本
          </button>
        </div>
      </div>
    </div>

    <!-- 删除确认：自绘的，不用 window.confirm -->
    <div v-if="removing" class="sheet" @click.self="removing = null">
      <div class="sheet__card">
        <h3 class="sheet__title">删除「{{ removing.name }}」？</h3>
        <p class="sheet__text">
          文件会从库里移除。如果它已经铺进游戏 <code>Mods/</code> 目录，
          <strong>那边也会一起撤掉</strong>；你自己手动放的模组不受影响。
        </p>
        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="removing = null">取消</button>
          <button class="btn btn-primary" type="button" @click="confirmRemove">删除</button>
        </div>
      </div>
    </div>

    <!-- 编辑独立模组的信息 -->
    <div v-if="editing" class="sheet" @click.self="editing = null">
      <div class="sheet__card">
        <h3 class="sheet__title">编辑「{{ editing.name }}」</h3>
        <p class="sheet__text">
          只改启动器记录的信息，<strong>不动模组文件</strong>，随时可以改回来。
        </p>

        <div class="fields">
          <label class="field">
            <span class="field__label">名称</span>
            <input v-model="form.name" class="field__input" type="text" />
          </label>
          <label class="field">
            <span class="field__label">作者</span>
            <input v-model="form.author" class="field__input" type="text" placeholder="可不填" />
          </label>
          <label class="field">
            <span class="field__label">版本</span>
            <input v-model="form.version" class="field__input" type="text" placeholder="例如 1.4" />
          </label>
          <label class="field">
            <span class="field__label">说明</span>
            <textarea v-model="form.description" class="field__input" rows="3"></textarea>
          </label>
        </div>

        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="editing = null">取消</button>
          <button class="btn btn-primary" type="button" :disabled="busy !== null" @click="saveEdit">
            保存
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: var(--content-max);
  margin: 0 auto;
}

.hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  flex-wrap: wrap;
  padding: 24px 28px;
  border-radius: var(--radius-lg);
  background: color-mix(in srgb, var(--surface-1) 74%, transparent);
  backdrop-filter: blur(14px) saturate(1.15);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.hero__body {
  max-width: 620px;
}

.hero__eyebrow {
  margin: 0 0 6px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--accent);
}

.hero__title {
  margin: 0 0 8px;
  font-size: 26px;
  font-weight: 700;
}

.hero__text {
  margin: 0;
  font-size: 13px;
  line-height: 1.8;
  color: var(--on-surface-variant);
}

.hero__actions {
  display: flex;
  gap: 8px;
  flex: none;
}

.bar {
  display: flex;
  align-items: center;
  gap: 10px;
}

.bar__search {
  flex: 1;
  padding: 7px 14px;
  border-radius: var(--radius-pill);
  border: 1px solid color-mix(in srgb, var(--outline) 60%, transparent);
  background: color-mix(in srgb, var(--surface-1) 78%, transparent);
  backdrop-filter: blur(10px);
  font-family: inherit;
  font-size: 12.5px;
}

.bar__count {
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.hint {
  margin: 0;
  padding: 12px 16px;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--surface-1) 74%, transparent);
  backdrop-filter: blur(14px);
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.group,
.game {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 16px;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--surface-1) 74%, transparent);
  backdrop-filter: blur(14px) saturate(1.15);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.group__head,
.game__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.group__campaign {
  font-size: 13.5px;
  font-weight: 700;
}

.group__variant {
  margin-left: 8px;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.group__actions {
  display: flex;
  gap: 4px;
}

.game__title {
  margin: 0;
  font-size: 13.5px;
  font-weight: 700;
}

.game__count,
.mod__size {
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.game__hint,
.game__empty {
  margin: 0 0 4px;
  font-size: 12px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.mods {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.mod {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--surface-2) 70%, transparent);
}

/* 来源标记：三种来源一眼分得开 */
.origin {
  flex: none;
  padding: 1px 8px;
  border-radius: var(--radius-pill);
  font-size: 11px;
  font-weight: 600;
}

.origin--official_campaign {
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  color: var(--accent);
}

.origin--custom_campaign {
  background: color-mix(in srgb, #7c4dff 18%, transparent);
  color: #6b3fe0;
}

.origin--standalone {
  background: color-mix(in srgb, #00897b 18%, transparent);
  color: #00796b;
}

.badge {
  flex: none;
  padding: 0 7px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  color: var(--on-surface-variant);
  font-size: 10.5px;
}

.mount {
  flex: none;
  padding: 1px 8px;
  border-radius: var(--radius-pill);
  background: color-mix(in srgb, #00897b 12%, transparent);
  color: #00796b;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 10.5px;
}

.badge--ver {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  background: color-mix(in srgb, var(--accent) 12%, transparent);
  color: var(--accent);
}

.badge--req {
  background: var(--accent);
  color: #fff;
  font-weight: 700;
}

.group__title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.mod__info {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
}

.mod__actions {
  display: flex;
  align-items: center;
  gap: 5px;
  flex: none;
}

/* 对比结果 */
.sheet__card--wide {
  width: min(920px, 92vw);
}

.diff {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 10px 0;
  padding: 0;
  list-style: none;
  max-height: 220px;
  overflow: auto;
}

.diff__row {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 4px 9px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
}

.diff__tag {
  flex: none;
  width: 62px;
  font-size: 10.5px;
  font-weight: 700;
  text-transform: uppercase;
}

.diff__row--changed .diff__tag {
  color: var(--warning, #b26a00);
}

.diff__row--added .diff__tag {
  color: #00796b;
}

.diff__row--removed .diff__tag {
  color: var(--danger, #c62828);
}

.diff__path {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.semantic {
  margin: 8px 0;
}

.semantic__head {
  cursor: pointer;
  font-size: 12.5px;
  font-weight: 600;
}

.semantic__body {
  margin: 6px 0 0;
  padding: 10px 12px;
  max-height: 260px;
  overflow: auto;
  border-radius: var(--radius-sm);
  background: #101725;
  color: #cfe3ff;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11.5px;
  line-height: 1.6;
  white-space: pre-wrap;
}

.mod__base {
  padding: 2px 6px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  color: inherit;
  font-family: inherit;
  font-size: 11px;
}

/* 询问对话框里的已有版本列表 */
.existing {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin: 10px 0;
  padding: 0;
  list-style: none;
  max-height: 160px;
  overflow: auto;
}

.existing__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 5px 10px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
}

.existing__name {
  font-size: 12.5px;
  font-weight: 600;
}

.existing__ver {
  font-size: 11.5px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  color: var(--on-surface-variant);
}

/* 编辑框 */
.fields {
  display: flex;
  flex-direction: column;
  gap: 9px;
  margin: 14px 0;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field__label {
  font-size: 12px;
  font-weight: 600;
  color: var(--on-surface-variant);
}

.field__input {
  padding: 7px 11px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  color: var(--on-surface);
  font-family: inherit;
  font-size: 12.5px;
  resize: vertical;
}

.btn--danger {
  color: var(--danger, #c62828);
}

.mod__pick {
  flex: none;
  cursor: pointer;
}

.mod__name {
  font-size: 12.5px;
  font-weight: 600;
  min-width: 150px;
}

.mod__path {
  flex: 1;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11px;
  color: var(--on-surface-variant);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mod--ro {
  background: transparent;
}

/* 开关 */
.switch {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: none;
  padding: 3px 10px 3px 4px;
  border-radius: var(--radius-pill);
  border: 1px solid color-mix(in srgb, var(--outline) 60%, transparent);
  background: var(--surface-1);
  color: var(--on-surface-variant);
  font-family: inherit;
  font-size: 11.5px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, border-color 0.15s ease;
}

.switch__knob {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--outline);
  transition: background 0.15s ease, transform 0.15s ease;
}

.switch--on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.switch--on .switch__knob {
  background: #fff;
  transform: translateX(2px);
}

.btn--tiny {
  padding: 3px 10px;
  font-size: 11.5px;
}

code {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--surface-2);
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 11.5px;
}
</style>
