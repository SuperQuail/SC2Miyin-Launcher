<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

import { api } from "../api/bridge";
import { slotArt } from "../api/art";
import type { BoundPatch, SlotView, Variant } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";
import VariantCard from "../components/VariantCard.vue";

const props = defineProps<{ slot: SlotView }>();
const emit = defineEmits<{ back: [] }>();

const { activate, removeVariant, launch, busy, refresh, notify } = useLauncher();

/** 当前选中的版本；null 表示原版战役。 */
const selected = ref<string | null>(props.slot.active);

const bannerStyle = computed(() => {
  const art = slotArt(props.slot.slug);
  return art ? { backgroundImage: "url(" + art + ")" } : undefined;
});

const dirty = computed(() => selected.value !== props.slot.active);

/** 启用当前选中的版本。 */
async function apply(): Promise<void> {
  const ok = await activate(props.slot.slug, selected.value);
  if (ok) emit("back");
}

/** 启用并立刻启动游戏。 */
async function applyAndPlay(): Promise<void> {
  const ok = await activate(props.slot.slug, selected.value);
  if (ok) await launch();
}

/** 删除一个已导入的版本。 */
async function drop(variant: Variant): Promise<void> {
  const ok = await removeVariant(props.slot.slug, variant.id);
  if (ok && selected.value === variant.id) selected.value = null;
}

/* ---------------- 补丁 ---------------- */

const patches = ref<BoundPatch[]>([]);
const available = ref<BoundPatch[]>([]);
const showAvailable = ref(false);

/** 跑一个会改动补丁的动作，然后刷新列表。 */
async function guard(action: () => Promise<unknown>): Promise<void> {
  try {
    await action();
    await loadPatches();
  } catch (error) {
    notify("error", errorText(error));
  }
}

async function loadPatches(): Promise<void> {
  try {
    patches.value = await api.listBindings(props.slot.slug);
    available.value = await api.listAvailablePatches(props.slot.slug);
  } catch (error) {
    notify("error", errorText(error));
  }
}

onMounted(loadPatches);

function togglePatch(item: BoundPatch): Promise<void> {
  return guard(() => api.configurePatch(props.slot.slug, item.id, !item.enabled, null));
}

function unbindPatch(item: BoundPatch): Promise<void> {
  return guard(() => api.unbindPatch(props.slot.slug, item.id));
}

function bindPatchItem(item: BoundPatch): Promise<void> {
  return guard(() => api.bindPatch(props.slot.slug, item.id));
}

async function setPriority(item: BoundPatch, raw: string): Promise<void> {
  const priority = Number.parseInt(raw, 10);
  if (Number.isNaN(priority)) return;
  await guard(() => api.configurePatch(props.slot.slug, item.id, null, priority));
}

/** 按 requires 自动挂载匹配得上的补丁。 */
async function autoBind(): Promise<void> {
  try {
    const bound = await api.autoBindPatches();
    await loadPatches();
    notify(
      bound.length ? "success" : "info",
      bound.length ? "已自动挂载 " + bound.length + " 个补丁" : "没有能自动匹配的补丁（没有声明依赖的只能手动挂）",
    );
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 导入一个补丁包。 */
async function importPatch(): Promise<void> {
  try {
    const path = await api.pickPackage();
    if (!path) return;
    const patch = await api.importPatch(path);
    await loadPatches();
    notify("success", "补丁「" + patch.name + "」已进库");
  } catch (error) {
    notify("error", errorText(error));
  }
}

/* ---------------- 补丁的编辑与导出 ---------------- */

const editingPatch = ref<BoundPatch | null>(null);
const patchForm = ref({
  name: "",
  author: "",
  registrationId: "",
  description: "",
  priority: 100,
});

function openPatchEdit(item: BoundPatch): void {
  editingPatch.value = item;
  patchForm.value = {
    name: item.name,
    author: item.author ?? "",
    registrationId: item.registration_id ?? "",
    description: item.description ?? "",
    priority: item.priority,
  };
}

async function savePatchEdit(): Promise<void> {
  const target = editingPatch.value;
  if (!target) return;

  try {
    await api.updatePatch(target.id, {
      name: patchForm.value.name,
      author: patchForm.value.author,
      registrationId: patchForm.value.registrationId,
      description: patchForm.value.description,
      priority: patchForm.value.priority,
    });
    editingPatch.value = null;
    await loadPatches();
    notify("success", "已保存");
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 单独导出一个补丁包。 */
async function exportPatchItem(item: BoundPatch): Promise<void> {
  try {
    const destination = await api.pickExportPath(item.name);
    if (!destination) return;

    exporting.value = true;
    const report = await api.exportPatch(item.id, destination);
    const parts = [report.maps + " 张地图", report.mods + " 个模组"];
    notify("success", "补丁已导出：" + parts.join(" · "));
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    exporting.value = false;
  }
}

/* ---------------- 编辑元数据 ---------------- */

const editing = ref<Variant | null>(null);
const form = ref({ name: "", author: "", registrationId: "", description: "" });

function openEdit(variant: Variant): void {
  editing.value = variant;
  form.value = {
    name: variant.name,
    author: variant.author ?? "",
    registrationId: variant.registration_id ?? "",
    description: variant.description ?? "",
  };
}

async function saveEdit(): Promise<void> {
  const target = editing.value;
  if (!target) return;

  try {
    await api.updateVariant(props.slot.slug, target.id, {
      name: form.value.name,
      author: form.value.author,
      registrationId: form.value.registrationId,
      description: form.value.description,
    });
    editing.value = null;
    await refresh();
    notify("success", "已保存");
  } catch (error) {
    notify("error", errorText(error));
  }
}

/* ---------------- 导出 ---------------- */

const exporting = ref(false);

async function doExport(mergePatches: boolean): Promise<void> {
  const variant = props.slot.variants.find((item) => item.id === selected.value);
  if (!variant) {
    notify("info", "先选一个版本再导出");
    return;
  }

  try {
    const destination = await api.pickExportPath(variant.name);
    if (!destination) return;

    exporting.value = true;
    const report = await api.exportVariant(
      props.slot.slug,
      variant.id,
      destination,
      mergePatches,
    );

    const parts = [report.maps + " 张地图", report.mods + " 个模组"];
    if (report.patches.length) parts.push("附带 " + report.patches.length + " 个补丁");
    let message = "已导出：" + parts.join(" · ");
    if (report.expanded.length) {
      message += "（有 " + report.expanded.length + " 项是目录树形态，CCM 可能读不了）";
    }
    notify("success", message);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div class="menu">
    <button class="back" type="button" @click="emit('back')">← 返回战役列表</button>

    <!-- 头条：用该战役的官方美术做背景 -->
    <header class="banner" :style="bannerStyle">
      <div class="banner__scrim"></div>
      <div class="banner__body">
        <h2 class="banner__title">{{ slot.display_name }}</h2>
        <p class="banner__sub">
          选择要游玩的版本 —— 原版战役，或导入的玩家版本
        </p>
      </div>
      <button class="btn btn-primary banner__play" type="button" @click="launch">
        开始游戏
      </button>
    </header>

    <!-- 版本卡片：原版永远排第一 -->
    <div class="section-head">
      <h3 class="section-head__title">
        可选版本
        <span class="section-head__count">{{ slot.variants.length + 1 }}</span>
      </h3>
      <span class="section-head__hint">导入新版本请回到战役列表页</span>
    </div>

    <div class="grid">
      <VariantCard
        :slot="slot"
        :variant="null"
        :active="slot.active === null"
        :selected="selected === null"
        @pick="selected = null"
      />
      <VariantCard
        v-for="item in slot.variants"
        :key="item.id"
        :slot="slot"
        :variant="item"
        :active="slot.active === item.id"
        :selected="selected === item.id"
        @pick="selected = item.id"
        @drop="drop(item)"
      />
    </div>

    <p v-if="!slot.variants.length" class="hint">
      还没有导入任何玩家版本。回到战役列表页点「导入战役包」，
      启动器会自动判断它属于哪部战役。
    </p>

    <p v-if="slot.notice" class="hint">{{ slot.notice }}</p>

    <!-- 补丁：这个战役上叠了哪些覆盖层 -->
    <section class="patches">
      <div class="patches__head">
        <h3 class="patches__title">
          补丁
          <span class="patches__count">{{ patches.length }}</span>
        </h3>
        <div class="patches__actions">
          <button class="btn btn-text" type="button" @click="autoBind">自动匹配</button>
          <button class="btn btn-text" type="button" @click="importPatch">＋ 导入补丁</button>
          <button class="btn btn-tonal" type="button" @click="showAvailable = !showAvailable">
            {{ showAvailable ? "收起可选补丁" : "挂载补丁（" + available.length + "）" }}
          </button>
        </div>
      </div>

      <p class="patches__hint">
        补丁是覆盖层：它不复制战役本体，启用时按优先级叠加。关掉即撤销，随时可恢复。
      </p>

      <ul v-if="patches.length" class="plist">
        <li v-for="item in patches" :key="item.id" class="pitem" :class="{ 'pitem--off': !item.enabled }">
          <label class="pitem__toggle">
            <input type="checkbox" :checked="item.enabled" @change="togglePatch(item)" />
            <span class="pitem__name">{{ item.name }}</span>
          </label>
          <span v-if="item.version" class="tag">v{{ item.version }}</span>
          <span v-if="item.author" class="pitem__author">{{ item.author }}</span>
          <span class="pitem__spacer"></span>
          <label class="pitem__priority">
            优先级
            <input
              type="number"
              class="pitem__input"
              :value="item.priority"
              @change="setPriority(item, ($event.target as HTMLInputElement).value)"
            />
          </label>
          <button class="btn btn-text" type="button" @click="openPatchEdit(item)">编辑</button>
          <button class="btn btn-text" type="button" :disabled="exporting" @click="exportPatchItem(item)">
            导出
          </button>
          <button class="btn btn-text" type="button" @click="unbindPatch(item)">移出</button>
        </li>
      </ul>
      <p v-else class="patches__hint">
        这个战役还没有挂补丁。声明了依赖的补丁可以「自动匹配」；没声明的只能手动挂。
      </p>

      <div v-if="showAvailable" class="pavail">
        <div v-for="item in available" :key="item.id" class="pitem">
          <span class="pitem__name">{{ item.name }}</span>
          <span v-if="item.matched" class="tag">依赖匹配</span>
          <span v-else-if="!item.requires.length" class="tag">通用补丁</span>
          <span class="pitem__spacer"></span>
          <button class="btn btn-tonal" type="button" @click="bindPatchItem(item)">挂上</button>
        </div>
        <p v-if="!available.length" class="patches__hint">
          补丁库是空的，先导入补丁包。
        </p>
      </div>
    </section>

    <!-- 底部操作条 -->
    <footer class="actions">
      <span class="actions__label">
        当前选择：<strong>{{ selected === null ? "原版战役" : selected }}</strong>
      </span>
      <span class="actions__spacer"></span>
      <button
        v-if="selected !== null"
        class="btn btn-outline"
        type="button"
        @click="openEdit(slot.variants.find((v) => v.id === selected)!)"
      >
        编辑信息
      </button>
      <button
        class="btn btn-outline"
        type="button"
        :disabled="exporting || selected === null"
        @click="doExport(false)"
      >
        {{ exporting ? "导出中…" : "导出包" }}
      </button>
      <button
        class="btn btn-outline"
        type="button"
        :disabled="exporting || selected === null"
        @click="doExport(true)"
      >
        导出（含补丁）
      </button>
      <button class="btn btn-outline" type="button" :disabled="busy" @click="applyAndPlay">
        启用并开始游戏
      </button>
      <button class="btn btn-primary" type="button" :disabled="!dirty || busy" @click="apply">
        {{ dirty ? "启用这个版本" : "已是当前版本" }}
      </button>
    </footer>
    <!-- 编辑补丁 -->
    <div v-if="editingPatch" class="sheet">
      <div class="sheet__card">
        <h3 class="sheet__title">编辑补丁「{{ editingPatch.name }}」</h3>
        <p class="sheet__note">
          同样只改启动器记录的元数据，不动包内容，也不影响它已经挂在哪些战役上。
        </p>

        <label class="field">
          <span class="field__label">补丁名</span>
          <input v-model="patchForm.name" class="field__input" type="text" />
        </label>
        <label class="field">
          <span class="field__label">作者</span>
          <input v-model="patchForm.author" class="field__input" type="text" placeholder="未知作者" />
        </label>
        <label class="field">
          <span class="field__label">注册 ID</span>
          <input v-model="patchForm.registrationId" class="field__input" type="text" />
        </label>
        <label class="field">
          <span class="field__label">默认优先级</span>
          <input v-model.number="patchForm.priority" class="field__input" type="number" />
        </label>
        <label class="field">
          <span class="field__label">描述</span>
          <textarea v-model="patchForm.description" class="field__input" rows="3"></textarea>
        </label>

        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="editingPatch = null">取消</button>
          <button class="btn btn-primary" type="button" @click="savePatchEdit">保存</button>
        </div>
      </div>
    </div>

    <!-- 编辑元数据 -->
    <div v-if="editing" class="sheet">
      <div class="sheet__card">
        <h3 class="sheet__title">编辑「{{ editing.name }}」</h3>
        <p class="sheet__note">
          只改启动器记录的元数据，不动包里的原始文件 —— 随时可以改回来。
          改名字不会改版本目录，挂在它上面的补丁不受影响。
        </p>

        <label class="field">
          <span class="field__label">战役名</span>
          <input v-model="form.name" class="field__input" type="text" />
        </label>
        <label class="field">
          <span class="field__label">作者</span>
          <input v-model="form.author" class="field__input" type="text" placeholder="未知作者" />
        </label>
        <label class="field">
          <span class="field__label">注册 ID</span>
          <input v-model="form.registrationId" class="field__input" type="text" placeholder="例如 HTXL.golden-lotv" />
        </label>
        <label class="field">
          <span class="field__label">描述</span>
          <textarea v-model="form.description" class="field__input" rows="3"></textarea>
        </label>

        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="editing = null">取消</button>
          <button class="btn btn-primary" type="button" @click="saveEdit">保存</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 注意：本组件的根是 flex 项，不能再写 margin: 0 auto ——
   自动外边距会吞掉交叉轴的 stretch，整页会缩成一条。 */
.menu {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
  padding-bottom: 4px;
}

.back {
  align-self: flex-start;
  padding: 6px 14px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.16);
  color: #fff;
  font-size: 13px;
  font-weight: 600;
}

.back:hover {
  background: rgba(255, 255, 255, 0.26);
}

/* ---------- 头条 ---------- */

.banner {
  position: relative;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 20px;
  min-height: 168px;
  padding: 22px 26px;
  border-radius: var(--radius-lg);
  background-color: #101a2e;
  background-size: cover;
  background-position: center 32%;
  box-shadow: var(--shadow-2);
  overflow: hidden;
}

.banner__scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    100deg,
    rgba(8, 14, 28, 0.92) 8%,
    rgba(8, 14, 28, 0.55) 52%,
    rgba(8, 14, 28, 0.2) 100%
  );
}

.banner__body {
  position: relative;
  z-index: 1;
}

.banner__title {
  margin: 0;
  font-size: 28px;
  font-weight: 700;
  color: #fff;
  letter-spacing: 0.4px;
  text-shadow: 0 2px 10px rgba(0, 0, 0, 0.55);
}

.banner__sub {
  margin: 5px 0 0;
  font-size: 13px;
  color: rgba(255, 255, 255, 0.82);
}

.banner__play {
  position: relative;
  z-index: 1;
}

/* ---------- 列表 ---------- */

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
}

.section-head__title {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 0;
  font-size: 17px;
  font-weight: 700;
  color: #fff;
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.55);
}

.section-head__count {
  padding: 1px 9px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.22);
  font-size: 12.5px;
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(252px, 1fr));
  gap: 16px;
}

.hint {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.75;
  color: rgba(255, 255, 255, 0.8);
  text-shadow: 0 1px 5px rgba(0, 0, 0, 0.5);
}

.section-head__hint {
  font-size: 12.5px;
  color: rgba(255, 255, 255, 0.75);
  text-shadow: 0 1px 5px rgba(0, 0, 0, 0.5);
}

/* ---------- 补丁面板 ---------- */

.patches {
  padding: 14px 16px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
}

.patches__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.patches__title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 15px;
  font-weight: 700;
}

.patches__count {
  padding: 1px 8px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  font-size: 12px;
}

.patches__actions {
  display: flex;
  gap: 6px;
}

.patches__hint {
  margin: 8px 0 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--on-surface-variant);
}

.plist {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 10px 0 0;
  padding: 0;
  list-style: none;
}

.pitem {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  font-size: 12.5px;
}

.pitem--off {
  opacity: 0.55;
}

.pitem__toggle {
  display: flex;
  align-items: center;
  gap: 7px;
  cursor: pointer;
}

.pitem__name {
  font-weight: 700;
}

.pitem__author {
  color: var(--on-surface-variant);
}

.pitem__spacer {
  flex: 1;
}

.pitem__priority {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--on-surface-variant);
}

.pitem__input {
  width: 68px;
  padding: 3px 6px;
  border-radius: var(--radius-xs);
  border: 1px solid var(--outline);
  background: var(--surface-1);
  font-size: 12.5px;
}

.pavail {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px dashed color-mix(in srgb, var(--outline) 65%, transparent);
}

/* ---------- 编辑对话框 ---------- */

.sheet {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(12, 18, 32, 0.45);
  backdrop-filter: blur(3px);
}

.sheet__card {
  width: min(460px, 92vw);
  padding: 20px 22px;
  border-radius: var(--radius-lg);
  background: var(--surface-1);
  box-shadow: var(--shadow-3);
}

.sheet__title {
  margin: 0 0 6px;
  font-size: 17px;
}

.sheet__note {
  margin: 0 0 14px;
  font-size: 12px;
  line-height: 1.65;
  color: var(--on-surface-variant);
}

.field {
  display: block;
  margin-bottom: 10px;
}

.field__label {
  display: block;
  margin-bottom: 4px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--on-surface-variant);
}

.field__input {
  width: 100%;
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 13px;
  resize: vertical;
}

.sheet__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}

.tag {
  padding: 1px 7px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  color: var(--on-surface-variant);
  font-size: 11px;
  font-weight: 600;
}

/* ---------- 底部操作条 ---------- */

.actions {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--surface-1) 90%, transparent);
  border: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
  box-shadow: var(--shadow-2);
}

.actions__label {
  font-size: 13px;
  color: var(--on-surface-variant);
}

.actions__label strong {
  color: var(--on-surface);
}

.actions__spacer {
  flex: 1;
}
</style>
