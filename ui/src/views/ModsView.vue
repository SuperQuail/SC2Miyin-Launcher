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
import type { GameModEntry, LibraryMod } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const { installation, loading: bootLoading, reveal, notify, bootstrap } = useLauncher();

const mods = ref<LibraryMod[]>([]);
const gameMods = ref<GameModEntry[]>([]);
const loading = ref(true);
const busy = ref<string | null>(null);
const keyword = ref("");

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

/** 搜模组名或所属战役。 */
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
 * 按「战役 → 版本」分组。
 *
 * 模组不是孤立的 —— 它总是属于某个战役的某个版本，
 * 分开显示用户才知道「关掉它会影响谁」。
 */
const groups = computed(() => {
  const buckets = new Map<string, { key: string; slot: string; slotName: string; variantName: string; mods: LibraryMod[] }>();

  for (const mod of filtered.value) {
    const key = mod.slot + "/" + mod.variant_id;
    const bucket = buckets.get(key);
    if (bucket) bucket.mods.push(mod);
    else
      buckets.set(key, {
        key,
        slot: mod.slot,
        slotName: mod.slot_name,
        variantName: mod.variant_name,
        mods: [mod],
      });
  }

  return [...buckets.values()];
});

const mountedCount = computed(() => mods.value.filter((mod) => mod.mounted).length);
const hasMods = computed(() => mods.value.length > 0);

/** 开关一个模组：把这个版本里其它模组的状态一起算进去，整体写回。 */
async function toggle(mod: LibraryMod): Promise<void> {
  const siblings = mods.value.filter(
    (item) => item.slot === mod.slot && item.variant_id === mod.variant_id,
  );
  const next = siblings
    .map((item) => (item.path === mod.path ? !item.mounted : item.mounted))
    .map((mounted, index) => (mounted ? siblings[index].path : null))
    .filter((path): path is string => path !== null);

  busy.value = mod.path;
  try {
    await api.setMountedMods(mod.slot, mod.variant_id, next);
    mods.value = mods.value.map((item) =>
      siblings.some((sibling) => sibling.path === item.path)
        ? { ...item, mounted: next.includes(item.path) }
        : item,
    );
    notify("success", (next.includes(mod.path) ? "已启用 " : "已关闭 ") + mod.name);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}

/** 整组一起开 / 关。 */
async function toggleGroup(
  group: { slot: string; variantName: string; mods: LibraryMod[] },
  mounted: boolean,
): Promise<void> {
  const next = mounted ? group.mods.map((mod) => mod.path) : [];
  busy.value = group.slot + group.variantName;
  try {
    await api.setMountedMods(group.slot, group.mods[0].variant_id, next);
    const paths = new Set(group.mods.map((mod) => mod.path));
    mods.value = mods.value.map((item) =>
      paths.has(item.path) ? { ...item, mounted } : item,
    );
    notify("success", mounted ? "已全部启用" : "已全部关闭");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = null;
  }
}
</script>

<template>
  <div class="page">
    <section class="hero">
      <div class="hero__body">
        <p class="hero__eyebrow">共 {{ mods.length }} 个，已启用 {{ mountedCount }} 个</p>
        <h2 class="hero__title">模组</h2>
        <p class="hero__text">
          地图可能需要模组才能正常打开 —— 原版战役的改版包也一样。
          这里能看到库里所有战役带进来的模组，一键开关。
        </p>
      </div>
      <div class="hero__actions">
        <button class="btn btn-text" type="button" :disabled="loading" @click="load">
          {{ loading ? "读取中…" : "重新读取" }}
        </button>
        <button
          class="btn btn-text"
          type="button"
          :disabled="!installation"
          @click="installation && reveal(installation.root + '\\Mods')"
        >
          打开游戏 Mods 目录
        </button>
      </div>
    </section>

    <div class="bar">
      <input
        v-model="keyword"
        class="bar__search"
        type="search"
        placeholder="搜索模组或所属战役"
      />
      <span class="bar__count">{{ filtered.length }} / {{ mods.length }}</span>
    </div>

    <p v-if="loading || bootLoading" class="hint">正在读取…</p>
    <p v-else-if="!hasMods" class="hint">
      现在还没有任何模组。导入带 .SC2Mod 的战役包后，它们会出现在这里。
    </p>
    <p v-else-if="!filtered.length" class="hint">没有匹配的模组。</p>

    <section v-for="group in groups" :key="group.key" class="group">
      <header class="group__head">
        <div>
          <span class="group__campaign">{{ group.slotName }}</span>
          <span class="group__variant">{{ group.variantName }}</span>
        </div>
        <div class="group__actions">
          <button
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
          <span class="mod__name">{{ mod.name }}</span>
          <span class="mod__path">{{ mod.path }}</span>
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
        上面启用某个模组后，下次启用战役时才会铺进来。
      </p>

      <p v-if="!gameMods.length" class="game__empty">游戏目录里还没有模组。</p>
      <ul v-else class="mods">
        <li v-for="entry in gameMods" :key="entry.name" class="mod mod--ro">
          <span class="mod__name">{{ entry.display }}</span>
          <span class="mod__path">
            {{ entry.name }}{{ entry.expanded ? "（目录）" : "" }}
          </span>
          <span class="mod__size">{{ formatBytes(entry.size_bytes) }}</span>
        </li>
      </ul>
    </section>
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
