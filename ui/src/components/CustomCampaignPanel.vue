<script setup lang="ts">
/**
 * 自制战役的操作面板：挂载模组 + 地图列表 + 启动。
 *
 * 自制战役**不进游戏目录** —— 地图躺在库里，得用编辑器打开玩。
 * 所以这个面板是整个自制战役的主界面。
 */
import { computed, onMounted, ref, watch } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type { DocInfo, MainMapChoice, MapEntry, ModEntry, Variant } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";
import { useContextMenu } from "../composables/useContextMenu";

const props = defineProps<{ slot: string; variant: Variant }>();
const emit = defineEmits<{ (event: "open-doc", doc: DocInfo): void }>();

const { notify, refresh } = useLauncher();
const menu = useContextMenu();

const maps = ref<MapEntry[]>([]);
const mods = ref<ModEntry[]>([]);
const choice = ref<MainMapChoice | null>(null);
const doc = ref<DocInfo | null>(null);
const loading = ref(true);
const busy = ref(false);
const keyword = ref("");
/** 折叠起来的章节；默认全展开。 */
const collapsed = ref<Set<string>>(new Set());
/** 没挂模组就点启动时的警告。 */
const showMountWarning = ref(false);

onMounted(load);
watch(() => props.variant.id, load);

async function load(): Promise<void> {
  loading.value = true;
  try {
    const [list, modList, picked, found] = await Promise.all([
      api.variantMaps(props.slot, props.variant.id),
      api.variantMods(props.slot, props.variant.id),
      api.mainMapChoice(props.slot, props.variant.id),
      api.variantDoc(props.slot, props.variant.id),
    ]);
    maps.value = list;
    mods.value = modList;
    choice.value = picked;
    doc.value = found;
    collapsed.value = new Set();
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    loading.value = false;
  }
}

/** 搜索过滤：按路径或名字模糊匹配。 */
const filtered = computed(() => {
  const needle = keyword.value.trim().toLowerCase();
  if (!needle) return maps.value;
  return maps.value.filter(
    (map) =>
      map.name.toLowerCase().includes(needle) || map.path.toLowerCase().includes(needle),
  );
});

/**
 * 分组：**跟着包的实际结构走**，不假设「章节」这个概念。
 *
 * 有第一层目录就按它分组（标题直接用目录名，不翻译 —— 作者怎么写就怎么显示）；
 * 全平铺就用一组。搜索时也走同一条路，照样能分组。
 */
const groups = computed(() => {
  const list = filtered.value;
  if (!list.some((map) => map.chapter)) {
    return [{ title: null as string | null, maps: list }];
  }

  const buckets = new Map<string, MapEntry[]>();
  for (const map of list) {
    const key = map.chapter ?? "（未分组）";
    const bucket = buckets.get(key);
    if (bucket) bucket.push(map);
    else buckets.set(key, [map]);
  }

  return [...buckets.entries()].map(([title, entries]) => ({ title, maps: entries }));
});

const mountedCount = computed(() => mods.value.filter((item) => item.mounted).length);
const hasMods = computed(() => mods.value.length > 0);
const mainMap = computed(() => choice.value?.path ?? null);

function toggleGroup(title: string): void {
  const next = new Set(collapsed.value);
  if (next.has(title)) next.delete(title);
  else next.add(title);
  collapsed.value = next;
}

/** 勾 / 取消一个模组。 */
async function toggleMod(entry: ModEntry): Promise<void> {
  const next = mods.value
    .map((item) => (item.path === entry.path ? !item.mounted : item.mounted))
    .map((mounted, index) => (mounted ? mods.value[index].path : null))
    .filter((path): path is string => path !== null);

  busy.value = true;
  try {
    await api.setMountedMods(props.slot, props.variant.id, next);
    mods.value = mods.value.map((item) => ({ ...item, mounted: next.includes(item.path) }));
    notify("success", next.length ? "已挂载 " + next.length + " 个模组" : "已全部取消挂载");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/**
 * 地图行上的右键菜单：不用先点「设为主地图」再点「打开」那么绕。
 */
function showMapMenu(event: MouseEvent, map: MapEntry): void {
  menu.show(
    event,
    [
      { id: "open", label: "用编辑器打开" },
      { id: "main", label: "设为主地图", disabled: map.is_main },
      { id: "copy", label: "复制地图路径", separatorBefore: true },
    ],
    (id) => {
      if (id === "open") void launch(map.path);
      if (id === "main") void markAsMain(map);
      if (id === "copy") void copyMapPath(map);
    },
  );
}

/** 复制地图在库里的绝对路径。 */
async function copyMapPath(map: MapEntry): Promise<void> {
  const text = props.slot + "/" + props.variant.id + "/" + map.path;
  try {
    await navigator.clipboard.writeText(text);
    notify("success", "已复制：" + text);
  } catch {
    notify("info", text);
  }
}

/** 把某张地图设为主地图。 */
async function markAsMain(map: MapEntry): Promise<void> {
  busy.value = true;
  try {
    await api.setMainMap(props.slot, props.variant.id, map.path);
    await load();
    notify("success", "已把「" + map.name + "」设为主地图");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/** 启动：没挂模组先警告，否则用编辑器打开主地图。 */
async function launch(map?: string): Promise<void> {
  const target = map ?? mainMap.value;
  if (!target) {
    notify("info", "还没定主地图 —— 在地图列表里挑一张，点「设为主地图」");
    return;
  }

  // 带了模组却一个都没挂：地图的依赖找不到，打开就是一堆丢失的资源
  if (hasMods.value && mountedCount.value === 0) {
    showMountWarning.value = true;
    return;
  }

  busy.value = true;
  try {
    const result = await api.openMapInEditor(props.slot, props.variant.id, target);
    await refresh();
    notify("success", result.guidance);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}

/** 收到警告后仍然要试。 */
async function launchAnyway(): Promise<void> {
  showMountWarning.value = false;
  const target = mainMap.value;
  if (!target) return;

  busy.value = true;
  try {
    const result = await api.openMapInEditor(props.slot, props.variant.id, target);
    notify("success", result.guidance);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section class="custom">
    <header class="custom__head">
      <div>
        <h3 class="custom__title">{{ variant.name }}</h3>
        <p class="custom__sub">
          自制战役不装进游戏目录 —— 地图就在库里，用编辑器打开玩。
          <span v-if="maps.length">共 {{ maps.length }} 张地图。</span>
        </p>
      </div>
      <div class="custom__actions">
        <button v-if="doc" class="btn btn-text" type="button" @click="emit('open-doc', doc)">
          查看说明
        </button>
        <button class="btn btn-primary" type="button" :disabled="busy || loading" @click="launch()">
          启动
        </button>
      </div>
    </header>

    <p v-if="mainMap" class="custom__main">
      主地图：<code>{{ mainMap }}</code>
      <span v-if="choice?.automatic" class="custom__auto">（这个版本只有一张，自动选的）</span>
    </p>
    <p v-else-if="choice?.warning" class="custom__warn">{{ choice.warning }}</p>
    <p v-else class="custom__warn">
      还没定主地图。挑一张点「设为主地图」，之后「启动」按钮就打开它。
    </p>

    <!-- 挂载模组 -->
    <div v-if="hasMods" class="mount">
      <div class="mount__head">
        <span class="mount__title">挂载模组</span>
        <span class="mount__count">已挂 {{ mountedCount }} / {{ mods.length }}</span>
      </div>
      <p class="mount__hint">
        地图里写死了 <code>Mods\xxx.SC2Mod</code> 这种依赖路径，模组得铺进游戏目录才找得到。
        不同战役的模组会互相打架，所以按需勾选；<strong>一个都不挂的话地图打不开</strong>。
      </p>
      <ul class="mount__list">
        <li v-for="entry in mods" :key="entry.path" class="mod">
          <label class="mod__label">
            <input
              type="checkbox"
              :checked="entry.mounted"
              :disabled="busy"
              @change="toggleMod(entry)"
            />
            <span class="mod__name">{{ entry.name }}</span>
          </label>
          <code class="mod__path">{{ entry.path }}</code>
        </li>
      </ul>
    </div>

    <!-- 地图列表 -->
    <div class="maplist">
      <div class="maplist__head">
        <span class="maplist__title">地图</span>
        <input
          v-model="keyword"
          class="maplist__search"
          type="search"
          placeholder="搜索地图，例如 Terran、Zerg"
        />
        <span class="maplist__count">{{ filtered.length }} / {{ maps.length }}</span>
      </div>

      <p v-if="loading" class="maplist__empty">正在读取库里地图…</p>
      <p v-else-if="!maps.length" class="maplist__empty">这个版本里没有 .SC2Map 地图。</p>
      <p v-else-if="!filtered.length" class="maplist__empty">没有匹配的地图。</p>

      <div v-for="group in groups" :key="group.title ?? '__flat__'" class="group">
        <button
          v-if="group.title"
          class="group__head"
          type="button"
          @click="toggleGroup(group.title)"
        >
          <span class="group__arrow">{{ collapsed.has(group.title) ? "▸" : "▾" }}</span>
          <span class="group__name">{{ group.title }}</span>
          <span class="group__count">{{ group.maps.length }}</span>
        </button>

        <ul v-show="!group.title || !collapsed.has(group.title)" class="group__list">
          <li
            v-for="map in group.maps"
            :key="map.path"
            class="map"
            :class="{ 'map--main': map.is_main }"
            @contextmenu="showMapMenu($event, map)"
          >
            <div class="map__info">
              <span class="map__name">{{ map.name }}</span>
              <span v-if="map.is_main" class="map__badge">主地图</span>
              <span class="map__size">{{ formatBytes(map.size) }}</span>
            </div>
            <div class="map__actions">
              <button
                class="btn btn-text btn--tiny"
                type="button"
                :disabled="busy"
                @click="markAsMain(map)"
              >
                设为主地图
              </button>
              <button
                class="btn btn-tonal btn--tiny"
                type="button"
                :disabled="busy"
                @click="launch(map.path)"
              >
                用编辑器打开
              </button>
            </div>
          </li>
        </ul>
      </div>
    </div>

    <!-- 没挂模组就点启动 -->
    <div v-if="showMountWarning" class="sheet" @click.self="showMountWarning = false">
      <div class="sheet__card">
        <div class="sheet__badge sheet__badge--warn">
          <svg class="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3.5 2.5 20h19L12 3.5Z" />
            <path d="M12 10v4.5" />
            <path d="M12 17.4v.1" />
          </svg>
        </div>
        <h3 class="sheet__title">一个模组都没挂</h3>
        <p class="sheet__text">
          这个战役带了 <strong>{{ mods.length }}</strong> 个模组，而你一个都没勾。
          <br /><br />
          地图里声明了 <code>Mods\xxx.SC2Mod</code> 这样的依赖，
          模组不在游戏目录里，打开地图会是一堆丢失的资源 ——
          <strong>多半进去就报错或者满地紫色方块</strong>。
        </p>
        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="showMountWarning = false">
            我去挂上
          </button>
          <button class="btn btn-primary" type="button" @click="launchAnyway">
            还是打开
          </button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.custom {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 16px 18px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
}

.custom__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.custom__title {
  margin: 0 0 3px;
  font-size: 16px;
}

.custom__sub {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.custom__actions {
  display: flex;
  gap: 8px;
}

.custom__main,
.custom__warn {
  margin: 0;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  font-size: 12.5px;
  line-height: 1.7;
}

.custom__main {
  background: var(--accent-soft);
  color: var(--accent);
}

.custom__warn {
  background: var(--warning-soft);
  color: var(--warning);
}

.custom__auto {
  opacity: 0.75;
}

code {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 12px;
}

/* ---------- 挂载 ---------- */

.mount {
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding: 12px 14px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
}

.mount__head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.mount__title {
  font-size: 13.5px;
  font-weight: 700;
}

.mount__count {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.mount__hint {
  margin: 0;
  font-size: 12px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.mount__list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 4px 0 0;
  padding: 0;
  list-style: none;
}

.mod {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 9px;
  border-radius: var(--radius-sm);
  background: var(--surface-1);
}

.mod__label {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: 1;
  cursor: pointer;
}

.mod__name {
  font-size: 13px;
  font-weight: 600;
}

.mod__path {
  font-size: 11px;
  color: var(--on-surface-variant);
}

/* ---------- 地图列表 ---------- */

.maplist {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.maplist__head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.maplist__title {
  font-size: 13.5px;
  font-weight: 700;
  flex: none;
}

.maplist__search {
  flex: 1;
  padding: 6px 12px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 12.5px;
}

.maplist__count {
  flex: none;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.maplist__empty {
  margin: 6px 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.group {
  display: flex;
  flex-direction: column;
}

.group__head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 9px;
  border: none;
  border-radius: var(--radius-sm);
  background: var(--surface-3);
  color: inherit;
  font-family: inherit;
  font-size: 12.5px;
  font-weight: 700;
  cursor: pointer;
  text-align: left;
}

.group__head:hover {
  background: var(--accent-soft);
}

.group__arrow {
  width: 12px;
  color: var(--on-surface-variant);
}

.group__name {
  flex: 1;
}

.group__count {
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.group__list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 3px 0 8px;
  padding: 0;
  list-style: none;
}

.map {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 5px 9px;
  border-radius: var(--radius-sm);
  transition: background 0.15s ease;
}

.map:hover {
  background: var(--surface-2);
}

.map--main {
  background: var(--accent-soft);
}

.map__info {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.map__name {
  font-size: 12.5px;
  font-family: ui-monospace, Menlo, Consolas, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.map__badge {
  padding: 0 7px;
  border-radius: var(--radius-pill);
  background: var(--accent);
  color: #fff;
  font-size: 10.5px;
  font-weight: 700;
}

.map__size {
  font-size: 11px;
  color: var(--on-surface-variant);
}

.map__actions {
  display: flex;
  gap: 6px;
  flex: none;
  opacity: 0;
  transition: opacity 0.15s ease;
}

.map:hover .map__actions,
.map--main .map__actions {
  opacity: 1;
}

.btn--tiny {
  padding: 3px 10px;
  font-size: 11.5px;
}

.sheet__badge--warn {
  background: var(--warning-soft);
}

.sheet__badge--warn .sheet__icon {
  stroke: var(--warning);
}

.sheet__text code {
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--surface-3);
}
</style>
