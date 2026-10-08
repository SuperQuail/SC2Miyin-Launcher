<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";

import { api } from "../api/bridge";
import { MIYIN, formatBytes } from "../api/art";
import type { ImportMode, ImportPreview } from "../api/types";
import SlotCard from "../components/SlotCard.vue";
import { errorText, useLauncher } from "../composables/useLauncher";
import SlotMenuView from "./SlotMenuView.vue";

const emit = defineEmits<{ "open-settings": [] }>();

const {
  installation,
  slots,
  loading,
  launch,
  reveal,
  refresh,
  chooseGameDirectory,
  bootstrap,
  notify,
  droppedPackage,
} = useLauncher();

/** 当前打开的战役槽位。 */
const opened = ref<string | null>(null);

onMounted(() => {
  void bootstrap();

  // 支持 #slot=wol 直接打开某个战役（调试与截图用）
  const match = /slot=([a-z]+)/.exec(typeof window === "undefined" ? "" : window.location.hash);
  if (match) opened.value = match[1];
});

/** 正在处理的导入：选好文件、预检完、等用户确认。 */
const pending = ref<{ preview: ImportPreview; slot: string; mode: ImportMode } | null>(null);
const importing = ref(false);

const openSlot = computed(() => slots.value.find((slot) => slot.slug === opened.value) ?? null);

const regionLabel = computed(() => {
  const branch = installation.value?.branch;
  if (!branch) return "未知区域";
  return branch.toLowerCase() === "cn" ? "国服" : branch.toUpperCase();
});

const modCount = computed(() =>
  slots.value.reduce((total, slot) => total + slot.variants.length, 0),
);
const activeCount = computed(() => slots.value.filter((slot) => slot.active !== null).length);

/** 原版战役：对官方四大战役的改版，与自制战役不是一类，所以分成两组。 */
const officialSlots = computed(() => slots.value.filter((slot) => slot.slug !== "custom"));

/** 自制战役：独立做的整部战役。 */
const customSlots = computed(() => slots.value.filter((slot) => slot.slug === "custom"));

const inspection = computed(() => pending.value?.preview.inspection ?? null);
const preview = computed(() => pending.value?.preview ?? null);

/** 没有自动认出归属时才需要用户选；但**永远允许**改。 */
const needsTarget = computed(() => pending.value !== null && pending.value.preview.slot === null);

const pendingName = computed(
  () => inspection.value?.name ?? inspection.value?.suggested_dir_name ?? "未命名战役",
);

/** 信息来源的措辞 —— 自动识别是**尽力而为**，不能说成确定的。 */
const sourceLabel = computed(() => {
  const source = preview.value?.source;
  if (source === "metadata") return "包内声明的资料片";
  if (source === "inferred") return "自动识别（尽力而为）";
  return "无法识别";
});

/** 自动识别用到的依据。 */
const evidenceText = computed(() => {
  const found = inspection.value?.identification;
  if (!found) return "";
  const rough = found.evidence === "map_name_prefix" ? "（启发式）" : "";
  return "依据" + rough + "：" + found.detail;
});

const isPatch = computed(() => inspection.value?.kind === "patch");

/**
 * **高置信度**归属：包内明确声明了资料片，或者证据链给的是精确证据。
 *
 * 启发式（地图名前缀）不算 —— 那种本来就只是猜，用户改掉很正常。
 */
const highConfidence = computed(() => {
  const judgement = preview.value;
  if (!judgement || judgement.slot === null) return false;
  if (judgement.source === "metadata") return true;
  const evidence = inspection.value?.identification?.evidence;
  return evidence != null && evidence !== "map_name_prefix";
});

/** 用户把归属改到了别处。 */
const targetChanged = computed(
  () =>
    pending.value !== null &&
    preview.value?.slot != null &&
    pending.value.slot !== preview.value.slot,
);

/**
 * 要不要警告：**高置信度 + 用户强改**。
 *
 * 这种改法多半会让战役装错地方 —— 地图不在游戏期待的子目录里，进去就找不到关卡。
 * 所以拦一下，但**不禁止**：作者有时确实知道自己在干什么。
 */
const overrideWarning = computed(() => highConfidence.value && targetChanged.value);

/** 被识别出来的那个槽位叫什么，用在提示里。 */
const identifiedName = computed(() => {
  const slug = preview.value?.slot;
  if (!slug) return "";
  return slots.value.find((slot) => slot.slug === slug)?.display_name ?? slug;
});

/** 用户当前选中的槽位叫什么。 */
const chosenName = computed(() => {
  const slug = pending.value?.slot;
  if (!slug) return "";
  return slots.value.find((slot) => slot.slug === slug)?.display_name ?? slug;
});

/** 二次确认弹窗开着没有。 */
const overrideOpen = ref(false);

/** 冲突时的新旧版本说法。 */
const conflictText = computed(() => {
  const conflict = preview.value?.conflict;
  if (!conflict) return "";
  const labels: Record<string, string> = {
    newer: "更新的版本",
    same: "相同的版本",
    older: "更旧的版本",
    unknown: "无法比较版本",
  };
  const incoming = conflict.incoming_version ?? "未标版本";
  const existing = conflict.existing_version ?? "未标版本";
  return (
    "库里已有「" + conflict.existing_name + "」" + existing +
    "，本次导入 " + incoming + " —— " + labels[conflict.relation]
  );
});

/** 选择压缩包并预检；能自动判断归属时直接给出目标战役。 */
async function startImport(): Promise<void> {
  try {
    const path = await api.pickPackage();
    if (!path) return;
    await prepare(path);
  } catch (error) {
    notify("error", errorText(error));
  }
}

/** 预检一个包 —— 选文件和拖进来走的是同一条路。 */
async function prepare(path: string): Promise<void> {
  importing.value = true;
  try {
    const result = await api.prepareImport(path);
    pending.value = { preview: result, slot: result.slot ?? "", mode: "rename" };
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}

// 拖进来的包：交给同一套预检流程
watch(droppedPackage, (path) => {
  if (!path) return;
  droppedPackage.value = null;
  void prepare(path);
});

/** 补丁包不进战役：导入补丁库，之后到对应战役里挂载。 */
async function confirmPatchImport(): Promise<void> {
  const current = pending.value;
  if (!current) return;

  importing.value = true;
  try {
    const patch = await api.importPatch(current.preview.path);
    pending.value = null;
    notify("success", "补丁「" + patch.name + "」已进库，到对应战役里挂载即可");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}

/** 确认导入，并直接进入目标战役的菜单。 */
async function confirmImport(): Promise<void> {
  const current = pending.value;
  if (!current || !current.slot) return;

  // 高置信度却被强改 -> 先问一句，别默默装错
  if (overrideWarning.value) {
    overrideOpen.value = true;
    return;
  }

  await doImport();
}

/** 用户看完警告仍然要改。 */
async function importAnyway(): Promise<void> {
  overrideOpen.value = false;
  await doImport();
}

/** 用户被劝回去了：把目标改回识别出来的那个。 */
function revertTarget(): void {
  if (pending.value && preview.value?.slot) {
    pending.value.slot = preview.value.slot;
  }
  overrideOpen.value = false;
}

/** 真正执行导入。 */
async function doImport(): Promise<void> {
  const current = pending.value;
  if (!current || !current.slot) return;

  importing.value = true;
  try {
    const created = await api.importPackageWith(
      current.preview.path,
      current.slot,
      current.mode,
    );
    pending.value = null;
    await refresh();
    opened.value = current.slot;
    notify("success", "已导入「" + created.name + "」");
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}
</script>

<template>
  <div class="page">
    <!-- 尚未设置游戏目录 -->
    <section v-if="!installation" class="empty card">
      <img class="empty__art" :src="MIYIN.cry" alt="" />
      <h2 class="empty__title">还没有找到星际争霸 II</h2>
      <p class="empty__text">
        启动器会先读取注册表里的安装位置。如果没有找到，请手动指定游戏根目录
        （即包含 <code>StarCraft II.exe</code> 的那一层）。
      </p>
      <div class="empty__actions">
        <button class="btn btn-primary btn-lg" type="button" @click="chooseGameDirectory">
          选择游戏目录
        </button>
        <button class="btn btn-outline" type="button" @click="emit('open-settings')">
          前往设置
        </button>
      </div>
    </section>

    <!-- 进入某个战役自己的菜单：选择原版 / 已导入的玩家版本 -->
    <SlotMenuView v-else-if="openSlot" :slot="openSlot" @back="opened = null" />

    <template v-else>
      <!-- 游戏状态 + 看板娘 -->
      <section class="hero card">
        <div class="hero__body">
          <p class="hero__eyebrow">星际争霸 II · {{ regionLabel }}</p>
          <h2 class="hero__title">{{ installation.version ?? "版本未知" }}</h2>
          <p class="hero__path" :title="installation.root">{{ installation.root }}</p>

          <div class="hero__actions">
            <button class="btn btn-primary btn-lg" type="button" @click="launch">开始游戏</button>
            <button class="btn btn-outline" type="button" @click="reveal(installation.root)">
              打开游戏目录
            </button>
          </div>

          <dl class="hero__stats">
            <div class="stat">
              <dt>官方战役</dt>
              <dd>{{ slots.length }}</dd>
            </div>
            <div class="stat">
              <dt>已导入版本</dt>
              <dd>{{ modCount }}</dd>
            </div>
            <div class="stat">
              <dt>已切换</dt>
              <dd>{{ activeCount }}</dd>
            </div>
          </dl>
        </div>

        <div class="hero__kanban" aria-hidden="true">
          <img :src="MIYIN.wink" alt="" />
        </div>
      </section>

      <!-- 导入战役包：放在外面，自动判断属于哪个战役 -->
      <div class="section-head">
        <h3 class="section-head__title">
          战役
          <span class="section-head__count">{{ slots.length }}</span>
        </h3>
        <div class="section-head__actions">
          <button class="btn btn-text" type="button" :disabled="loading" @click="refresh">
            {{ loading ? "读取中…" : "重新读取" }}
          </button>
          <button class="btn btn-primary" type="button" :disabled="importing" @click="startImport">
            {{ importing ? "核对中…" : "＋ 导入战役包" }}
          </button>
        </div>
      </div>

      <p class="section-hint">
        导入时启动器会读取包里的战役信息，自动归入对应的战役；每部战役默认是原版，
        点开可以导入并切换不同玩家制作的版本，多个版本同时保留、互不覆盖。
      </p>

      <!-- 导入预览 -->
      <section v-if="pending" class="import">
        <div class="import__head">
          <div>
            <div class="import__title">
              将导入：{{ pendingName }}
              <span v-if="inspection?.version" class="tag">v{{ inspection.version }}</span>
              <span class="tag" :class="{ 'tag--patch': isPatch }">
                {{ isPatch ? "补丁包" : "战役包" }}
              </span>
            </div>
            <div class="import__meta">
              {{ inspection?.map_count ?? 0 }} 张地图 ·
              {{ formatBytes(inspection?.unpacked_bytes ?? 0) }}
              <span v-if="inspection?.author"> · {{ inspection.author }}</span>
            </div>
          </div>

          <div class="import__target">
            <div class="import__source">{{ sourceLabel }}</div>
            <div v-if="evidenceText" class="import__evidence">{{ evidenceText }}</div>
          </div>
        </div>

        <!-- 补丁包走另一条路 -->
        <p v-if="isPatch" class="import__note">
          这是一个<strong>补丁包</strong>，它不归属任何战役。导入后到对应战役的菜单里挂载即可，
          可以同时挂多个并调整优先级。
        </p>

        <template v-else>
          <!-- 目标战役：**始终可选**，默认填自动识别的结果 -->
          <div class="import__field">
            <span class="import__label">导入到：</span>
            <div class="targets">
              <button
                v-for="item in slots"
                :key="item.slug"
                class="target"
                :class="{ 'target--on': pending.slot === item.slug }"
                type="button"
                @click="pending.slot = item.slug"
              >
                {{ item.display_name }}
              </button>
            </div>
            <span v-if="needsTarget" class="import__ask">自动识别没能判断出归属，请手动选择</span>
          </div>

          <!-- 高置信度却被改到别处：先说清后果，但不禁止 -->
          <div v-if="overrideWarning" class="warn">
            这个包<strong>明确</strong>属于「{{ identifiedName }}」，你把它改到了「{{
              chosenName
            }}」。装错地方通常会让战役里找不到关卡，确认前请想一下。
          </div>

          <!-- 冲突：覆盖更新 or 重命名后导入 -->
          <div v-if="preview?.conflict" class="conflict">
            <div class="conflict__text">{{ conflictText }}</div>
            <div class="targets">
              <button
                class="target"
                :class="{ 'target--on': pending.mode === 'overwrite' }"
                type="button"
                @click="pending.mode = 'overwrite'"
              >
                覆盖更新
              </button>
              <button
                class="target"
                :class="{ 'target--on': pending.mode === 'rename' }"
                type="button"
                @click="pending.mode = 'rename'"
              >
                重命名后导入（两者并存）
              </button>
            </div>
          </div>
        </template>

        <ul v-if="inspection?.issues.length" class="issues">
          <li
            v-for="issue in inspection.issues"
            :key="issue.code"
            class="issue"
            :class="'issue--' + issue.level"
          >
            {{ issue.message }}
          </li>
        </ul>

        <div class="import__actions">
          <button class="btn btn-text" type="button" @click="pending = null">取消</button>
          <button
            v-if="isPatch"
            class="btn btn-primary"
            type="button"
            :disabled="importing || !inspection?.installable"
            @click="confirmPatchImport"
          >
            {{ importing ? "导入中…" : "导入补丁库" }}
          </button>
          <button
            v-else
            class="btn btn-primary"
            type="button"
            :disabled="importing || !pending.slot || !inspection?.installable"
            @click="confirmImport"
          >
            {{ importing ? "导入中…" : "确认导入" }}
          </button>
        </div>
      </section>

      <section class="group">
        <h3 class="group__title">原版战役</h3>
        <p class="group__hint">
          对官方四大战役的改版 —— 重制、换单位、加关卡都算这一类。
        </p>
        <div class="grid">
          <SlotCard
            v-for="slot in officialSlots"
            :key="slot.slug"
            :slot="slot"
            @open="opened = $event"
          />
        </div>
      </section>

      <section class="group">
        <h3 class="group__title">自制战役</h3>
        <p class="group__hint">
          独立做的整部战役，不依附于任何官方战役。导入后在这里按版本管理。
        </p>
        <div class="grid">
          <SlotCard
            v-for="slot in customSlots"
            :key="slot.slug"
            :slot="slot"
            @open="opened = $event"
          />
        </div>
      </section>
    </template>

    <!-- 强改归属的二次确认 -->
    <div v-if="overrideOpen" class="sheet" @click.self="overrideOpen = false">
      <div class="sheet__card">
        <div class="sheet__badge sheet__badge--warn">
          <svg class="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3.5 2.5 20h19L12 3.5Z" />
            <path d="M12 10v4.5" />
            <path d="M12 17.4v.1" />
          </svg>
        </div>
        <h3 class="sheet__title">确定改到「{{ chosenName }}」吗？</h3>
        <p class="sheet__text">
          启动器<strong>确定</strong>这个包属于 <strong>{{ identifiedName }}</strong>，
          依据是<strong>包内自己声明的资料片</strong>或精确的证据链。
          <br /><br />
          强行装到别的战役里，地图会落在游戏不期待的位置 ——
          <strong>多半进游戏后找不到关卡，直接玩不了</strong>。
          装错了可以删掉重导，但白折腾一趟。
        </p>
        <div class="sheet__actions">
          <button class="btn btn-text" type="button" @click="importAnyway">
            我知道，仍然导入
          </button>
          <button class="btn btn-primary" type="button" @click="revertTarget">
            改回 {{ identifiedName }}
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
  gap: 18px;
  max-width: var(--content-max);
  margin: 0 auto;
}

/* ---------- 顶部看板 ---------- */

.hero {
  position: relative;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  overflow: hidden;
  padding: 24px 28px;
  background:
    radial-gradient(360px 280px at 86% 74%, rgba(150, 185, 255, 0.45), transparent 72%),
    linear-gradient(118deg, #ffffff 0%, #fbfdff 58%, #f2f7ff 100%);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.hero__body {
  position: relative;
  z-index: 2;
  max-width: 520px;
}

.hero__eyebrow {
  margin: 0 0 6px;
  font-size: 12.5px;
  letter-spacing: 1.4px;
  text-transform: uppercase;
  color: var(--accent);
  font-weight: 700;
}

.hero__title {
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: var(--on-surface);
}

.hero__path {
  margin: 8px 0 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
  word-break: break-all;
}

.hero__actions {
  display: flex;
  gap: 10px;
  margin-top: 18px;
}

.hero__stats {
  display: flex;
  gap: 26px;
  margin: 18px 0 0;
  padding-top: 14px;
  border-top: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
}

.stat dt {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.stat dd {
  margin: 2px 0 0;
  font-size: 20px;
  font-weight: 700;
  color: var(--accent);
}

/* 看板娘：立绘自带白底，用径向遮罩把方形边缘化开 */
.hero__kanban {
  position: relative;
  z-index: 1;
  flex: 0 0 auto;
  width: 218px;
  margin: -24px -10px -24px 0;
  pointer-events: none;
}

.hero__kanban img {
  display: block;
  width: 100%;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 58%, transparent 78%);
  mask-image: radial-gradient(circle at 50% 52%, #000 58%, transparent 78%);
}

/* ---------- 列表头 ---------- */

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

.section-head__actions {
  display: flex;
  gap: 8px;
}

.section-head__actions .btn-text {
  color: #fff;
  background: rgba(255, 255, 255, 0.16);
}

.section-head__actions .btn-text:not(:disabled):hover {
  background: rgba(255, 255, 255, 0.26);
}

.section-hint {
  margin: -6px 0 0;
  font-size: 12.5px;
  line-height: 1.7;
  color: rgba(255, 255, 255, 0.78);
  text-shadow: 0 1px 5px rgba(0, 0, 0, 0.5);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(268px, 1fr));
  gap: 18px;
}

/* ---------- 导入面板 ---------- */

.import {
  padding: 16px 18px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--accent) 35%, transparent);
  box-shadow: var(--shadow-2);
}

.import__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.import__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 700;
}

.import__meta {
  margin-top: 5px;
  font-size: 12px;
  color: var(--on-surface-variant);
}

.import__target {
  font-size: 12.5px;
  color: var(--on-surface-variant);
  text-align: right;
  max-width: 320px;
}

.import__target strong {
  color: var(--accent);
}

.import__ask {
  color: var(--warning);
}

.targets {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 12px;
}

.target {
  padding: 7px 16px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--outline);
  background: var(--surface-1);
  font-size: 13px;
  font-weight: 600;
}

.target--on {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

.issues {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 12px 0 0;
  padding: 0;
  list-style: none;
}

.issue {
  padding: 7px 10px;
  border-radius: var(--radius-sm);
  font-size: 12.5px;
  line-height: 1.5;
}

.issue--warning {
  background: var(--warning-soft);
  color: var(--warning);
}

.issue--broken {
  background: var(--danger-soft);
  color: var(--danger);
}

.tag--patch {
  background: var(--warning-soft);
  color: var(--warning);
}

.import__source {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--accent);
}

.import__evidence {
  margin-top: 3px;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.import__note {
  margin: 12px 0 0;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 12.5px;
  line-height: 1.6;
}

.import__field {
  margin-top: 14px;
}

.import__label {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--on-surface-variant);
}

.import__field .targets {
  margin-top: 8px;
}

/* 强改归属的警告条 */
.warn {
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  border-left: 3px solid var(--warning);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 12.5px;
  line-height: 1.7;
}

.sheet__badge--warn {
  background: var(--warning-soft);
}

.sheet__badge--warn .sheet__icon {
  stroke: var(--warning);
}

/* 原版战役 / 自制战役 两组 */
.group {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 18px;
}

.group__title {
  margin: 0;
  font-size: 15px;
  font-weight: 700;
}

.group__hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.conflict {
  margin-top: 12px;
  padding: 10px 12px;
  border-radius: var(--radius-sm);
  background: var(--accent-soft);
}

.conflict__text {
  font-size: 12.5px;
  color: var(--on-surface);
  line-height: 1.6;
}

.tag {
  padding: 1px 7px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  color: var(--on-surface-variant);
  font-size: 11px;
  font-weight: 600;
}

.import__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}

/* ---------- 空状态 ---------- */

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 10px;
  padding: 40px 32px 46px;
}

.empty__art {
  width: 176px;
  height: 176px;
  object-fit: contain;
  margin-bottom: -6px;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 62%, transparent 82%);
  mask-image: radial-gradient(circle at 50% 52%, #000 62%, transparent 82%);
}

.empty__title {
  margin: 4px 0 0;
  font-size: 19px;
}

.empty__text {
  margin: 0;
  max-width: 560px;
  color: var(--on-surface-variant);
  font-size: 13.5px;
  line-height: 1.7;
}

.empty__text code {
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  background: var(--surface-3);
  font-size: 12.5px;
}

.empty__actions {
  display: flex;
  gap: 10px;
  margin-top: 12px;
}
</style>
