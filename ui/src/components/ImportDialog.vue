<script setup lang="ts">
/**
 * 导入战役包：选文件（或拖进来）→ 预检 → 确认 → 进库。
 *
 * 抽成独立组件是因为**两个页面都要用**：战役页与自制战役页。
 * 对外只暴露 `prepare(path)`（拖动进来的包走这条）和 `startImport()`（点按钮），
 * 导入成功后 emit `imported(slot)` 让父页面决定接下来去哪。
 */
import { computed, ref, watch } from "vue";

import { api } from "../api/bridge";
import { formatBytes } from "../api/art";
import type { ImportMode, ImportPreview } from "../api/types";
import { errorText, useLauncher } from "../composables/useLauncher";

const props = withDefaults(
  defineProps<{
    /**
     * 从哪个入口打开的。
     *
     * `custom` 表示用户已经站在「自制战役」页 —— 那就**不要再判断它属于哪部
     * 原版战役**。用户点的是这一页的导入按钮，意图已经够明确了；
     * 没有元数据也不该被识别链带去别的战役。
     */
    entry?: "campaign" | "custom";
    /**
     * `update` 表示这次不是"再装一版"，而是**更新某个战役**：
     * 删掉旧的、装进新的（见 `library::replace_variant`）。
     */
    mode?: "import" | "update";
    /** 更新模式下点明的目标战役 —— 有它就是"强制更新"，不看包认得出谁。 */
    slot?: string;
  }>(),
  { entry: "campaign", mode: "import", slot: "" },
);

const emit = defineEmits<{ imported: [string] }>();

/** 是不是从「自制战役」页进来的。 */
const isCustomEntry = computed(() => props.entry === "custom");
/** 是不是"更新"而不是"导入"。 */
const isUpdate = computed(() => props.mode === "update");

const { slots, notify, refresh, droppedPackage } = useLauncher();

/** 正在处理的导入：选好文件、预检完、等用户确认。 */
const pending = ref<{ preview: ImportPreview; slot: string; mode: ImportMode } | null>(null);
const importing = ref(false);
/** 二次确认弹窗开着没有（高置信度被强改时）。 */
const overrideOpen = ref(false);

/**
 * 要不要把包里的模组一起挂上（默认要）。
 *
 * 自制战役的地图里写死了 `Mods\\xxx.SC2Mod` 依赖，一个都不挂的话
 * 用户点启动必然失败，所以默认勾上 —— 想精简的到战役页面里取消。
 */
const mountMods = ref(true);

const inspection = computed(() => pending.value?.preview.inspection ?? null);
const preview = computed(() => pending.value?.preview ?? null);
const isPatch = computed(() => inspection.value?.kind === "patch");
const packageMods = computed(() => inspection.value?.mod_count ?? 0);

/** 没有自动认出归属时才需要用户选；但**永远允许**改。 */
const needsTarget = computed(
    () => !isCustomEntry.value && pending.value !== null && pending.value.preview.slot === null,
  );

/**
 * 要显示给用户看的预检提示。
 *
 * **自制战役入口要把「归属」相关的几条滤掉** —— 在那一页，归属是用户自己定的，
 * 再说「包内没有声明归属，已按地图文件名判定为自由之翼」纯属误导：
 * 那套判定根本没参与决定，用户已经在自制战役页点了导入。
 */
const shownIssues = computed(() => {
  const all = inspection.value?.issues ?? [];
  if (!isCustomEntry.value) return all;

  const aboutBelonging = new Set([
    "NO_METADATA",
    "CAMPAIGN_IDENTIFIED",
    "CAMPAIGN_UNKNOWN",
  ]);
  return all.filter((issue) => !aboutBelonging.has(issue.code));
});

const pendingName = computed(
  () => inspection.value?.name ?? inspection.value?.suggested_dir_name ?? "未命名战役",
);

/** 信息来源的措辞 —— 自动识别是**尽力而为**，不能说成确定的。 */
const sourceLabel = computed(() => {
  const source = preview.value?.source;
  if (source === "metadata") return "包内声明的资料片";
  if (source === "inferred") return "自动识别";
  return "无法识别";
});

/** 自动识别用到的依据。 */
const evidenceText = computed(() => {
  const found = inspection.value?.identification;
  if (!found) return "";
  return "依据：" + found.detail;
});

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
const overrideWarning = computed(
    // 自制战役入口是用户自己定的，没有「改错了」这回事
    () => !isCustomEntry.value && highConfidence.value && targetChanged.value,
  );

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
    const result = await api.prepareImport(path, props.entry);
    // 自制战役入口：槽位由入口定死，不采信识别结果
    pending.value = {
      preview: result,
      // 更新模式：目标优先用调用方点明的（每个战役页那颗按钮就是这条路），
      // 否则才采信识别结果。自制战役入口同样由入口定死。
      slot: props.slot || (isCustomEntry.value ? "custom" : (result.slot ?? "")),
      mode: "rename",
    };
    mountMods.value = true;
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

/** 确认导入。 */
async function confirmImport(): Promise<void> {
  const current = pending.value;
  if (!current || !current.slot) return;

  // 高置信度却被强改 -> 先问一句，别默默装错。
  // 更新模式不适用：目标是人点明的，不是"改"出来的。
  if (overrideWarning.value && !isUpdate.value) {
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

/** 真正执行导入（或更新）。 */
async function doImport(): Promise<void> {
  const current = pending.value;
  if (!current || !current.slot) return;

  importing.value = true;
  try {
    // 更新：删旧的装新的，该跟着走的记录由后端带过去
    if (isUpdate.value) {
      const updated = await api.updateVariantFromPackage(current.slot, current.preview.path);
      pending.value = null;
      await refresh();
      notify("success", "已更新「" + updated.name + "」");
      emit("imported", current.slot);
      return;
    }

    const created = await api.importPackageWith(
      current.preview.path,
      current.slot,
      current.mode,
    );

    // 用户不想装模组 -> 把导入时默认挂上的清掉
    // （只在索引里改，模组要等激活时才会真的铺进游戏目录）
    if (!mountMods.value && (created.mod_count ?? 0) > 0) {
      await api.setMountedMods(current.slot, created.id, []);
    }

    const slot = current.slot;
    pending.value = null;
    await refresh();
    notify("success", "已导入「" + created.name + "」");
    emit("imported", slot);
  } catch (error) {
    notify("error", errorText(error));
  } finally {
    importing.value = false;
  }
}

/** 取消这次导入。 */
function cancel(): void {
  pending.value = null;
}

defineExpose({ prepare, startImport, busy: importing, open: computed(() => pending.value !== null) });
</script>

<template>
  <!-- 导入预览 -->
  <section v-if="pending" class="import">
    <div class="import__head">
      <div>
        <div class="import__title">
          {{ isUpdate ? "将更新：" : "将导入：" }}{{ pendingName }}
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

      <!--
        来源与依据。
        **自制战役入口不显示这些** —— 用户已经选了「自制战役」，
        再说「自动识别 / 识别依据是地图名像自由之翼」纯属误导：
        那套识别根本没参与决定，归属是用户自己定的。
      -->
      <div v-if="isCustomEntry" class="import__target">
        <div class="import__source">按自制战役导入</div>
        <div class="import__evidence">这一页导入的都算自制战役，不看包属于哪部原版战役</div>
      </div>
      <div v-else class="import__target">
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
      <!-- 自制战役入口：归属已定，不显示选择器，也不显示识别结果 -->
      <div v-if="isCustomEntry || isUpdate" class="import__field">
        <span class="import__label">{{ isUpdate ? "更新：" : "导入到：" }}</span>
        <span class="import__fixed">{{ isUpdate ? chosenName : "自制战役" }}</span>
      </div>

      <!-- 目标：**始终可选**，默认填自动识别的结果 -->
      <div v-else class="import__field">
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

      <!-- 带模组的包：问一句要不要一起装 -->
      <label v-if="packageMods > 0 && !isUpdate" class="check">
        <input v-model="mountMods" type="checkbox" />
        <span>
          一起挂载这 <strong>{{ packageMods }}</strong> 个模组
          <em>（地图需要它们才能打开，建议勾上；之后可以到战役页面里改）</em>
        </span>
      </label>

      <!-- 冲突：覆盖更新 or 重命名后导入（更新模式不问 —— 更新就是换掉旧的） -->
      <div v-if="preview?.conflict && !isUpdate" class="conflict">
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

    <ul v-if="shownIssues.length" class="issues">
      <li
        v-for="issue in shownIssues"
        :key="issue.code"
        class="issue"
        :class="'issue--' + issue.level"
      >
        {{ issue.message }}
      </li>
    </ul>

    <div class="import__actions">
      <p v-if="isUpdate" class="import__note">
        更新会<strong>先删掉这一版、再装进新的</strong>；它正启用着的话，会先切回原版、装完再应用回去。
        存档档案与补丁绑定挂在战役上，不受影响。
      </p>
      <button class="btn btn-text" type="button" @click="cancel">取消</button>
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
        {{ importing ? "处理中…" : isUpdate ? "确认更新" : "确认导入" }}
      </button>
    </div>
  </section>

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
        这个包属于 <strong>{{ identifiedName }}</strong>。
        <br /><br />
        装到别的战役里，地图会落在游戏找不到的位置 ——
        <strong>多半进游戏后找不到关卡，玩不了</strong>。
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
</template>

<style scoped>
.import {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 14px;
  padding: 14px 16px;
  border-radius: var(--radius-md);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
  box-shadow: var(--shadow-2);
}

.import__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.import__title {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 14.5px;
  font-weight: 700;
}

.import__meta {
  margin-top: 3px;
  font-size: 12px;
  color: var(--on-surface-variant);
}

.import__target {
  text-align: right;
}

.import__source {
  font-size: 12.5px;
  font-weight: 600;
}

.import__evidence {
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.import__note {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.import__field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.import__label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--on-surface-variant);
}

.import__fixed {
  padding: 5px 14px;
  border-radius: var(--radius-pill);
  background: var(--primary-container, #d6e4ff);
  color: var(--on-primary-container, #0b3d91);
  font-size: 12.5px;
  font-weight: 600;
}

.import__ask {
  font-size: 12px;
  color: var(--warning);
}

.import__actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.targets {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.target {
  padding: 5px 13px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  color: var(--on-surface-variant);
  font-family: inherit;
  font-size: 12.5px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.target--on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}

.warn {
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  border-left: 3px solid var(--warning);
  background: var(--warning-soft);
  color: var(--warning);
  font-size: 12.5px;
  line-height: 1.7;
}

.check {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  font-size: 12.5px;
  line-height: 1.7;
  cursor: pointer;
}

.check input {
  margin-top: 3px;
}

.check em {
  display: block;
  font-style: normal;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.conflict {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
}

.conflict__text {
  font-size: 12.5px;
  line-height: 1.7;
}

.issues {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.issue {
  font-size: 12px;
  line-height: 1.7;
  color: var(--on-surface-variant);
}

.issue--warning {
  color: var(--warning);
}

.issue--broken {
  color: var(--danger, #c62828);
}

.sheet__badge--warn {
  background: var(--warning-soft);
}

.sheet__badge--warn .sheet__icon {
  stroke: var(--warning);
}
</style>
