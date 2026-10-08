<script setup lang="ts">
/**
 * 作弊码查询页。
 *
 * 目标是「查得到、抄得走」：搜一行、点一下复制，不用手打这些又长又怪的码。
 * 复制优先用 Clipboard API，被权限挡住时退回选中文本框再 execCommand ——
 * WebView 里这两条路哪条通不一定，两条都留着。
 */
import { computed, ref } from "vue";

import { CHEATS, CHEAT_SCOPES, matchCheats, type Cheat, type CheatScope } from "../api/cheats";
import { useLauncher } from "../composables/useLauncher";

const { notify } = useLauncher();

const keyword = ref("");
const scope = ref<CheatScope | "全部">("全部");
/** 刚复制过的码，用来给一格"已复制"的反馈。 */
const copied = ref("");

const list = computed(() => matchCheats(keyword.value, scope.value));

const scopeOptions = computed(() => ["全部", ...CHEAT_SCOPES] as const);

/** 每个适用范围下有多少条，显示在筛选按钮上。 */
function countOf(option: CheatScope | "全部"): number {
  return matchCheats("", option).length;
}

async function copy(cheat: Cheat): Promise<void> {
  const ok = await writeClipboard(cheat.code);
  if (!ok) {
    notify("error", "复制失败，请手动选中");
    return;
  }
  copied.value = cheat.code;
  notify("success", "已复制 " + cheat.code);
  // 过一会儿把"已复制"收回去，避免满屏都是勾
  window.setTimeout(() => {
    if (copied.value === cheat.code) copied.value = "";
  }, 1600);
}

/** 写剪贴板；两条路都试一遍。 */
async function writeClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // 退回老办法：造一个看不见的输入框，选中后复制
    try {
      const holder = document.createElement("textarea");
      holder.value = text;
      holder.setAttribute("readonly", "");
      holder.style.position = "fixed";
      holder.style.opacity = "0";
      document.body.appendChild(holder);
      holder.select();
      const ok = document.execCommand("copy");
      document.body.removeChild(holder);
      return ok;
    } catch {
      return false;
    }
  }
}
</script>

<template>
  <section class="cheats">
    <header class="cheats__head">
      <h2 class="cheats__title">作弊码</h2>
      <p class="cheats__hint">
        在战役里按 <kbd>Enter</kbd> 打开聊天框，粘贴后回车。<strong>单人战役有效，对战时无效。</strong>
      </p>
    </header>

    <div class="cheats__bar">
      <input
        v-model="keyword"
        class="cheats__search"
        type="search"
        placeholder="搜索效果或作弊码，例如「晶矿」「无敌」"
      />
      <div class="cheats__scopes">
        <button
          v-for="option in scopeOptions"
          :key="option"
          class="chip"
          :class="{ 'chip--on': scope === option }"
          type="button"
          @click="scope = option"
        >
          {{ option }}
          <span class="chip__count">{{ countOf(option) }}</span>
        </button>
      </div>
    </div>

    <p v-if="!list.length" class="cheats__empty">
      没有匹配的作弊码。换个关键词试试，或者点「全部」。
    </p>

    <ul v-else class="cheats__list">
      <li v-for="cheat in list" :key="cheat.code" class="cheat">
        <div class="cheat__main">
          <code class="cheat__code">{{ cheat.code }}</code>
          <span class="cheat__effect">{{ cheat.effect }}</span>
        </div>
        <div class="cheat__meta">
          <span v-if="cheat.scope !== '通用'" class="cheat__scope">{{ cheat.scope }}</span>
          <span v-if="cheat.note" class="cheat__note">{{ cheat.note }}</span>
        </div>
        <button
          class="cheat__copy"
          :class="{ 'cheat__copy--done': copied === cheat.code }"
          type="button"
          :title="'复制 ' + cheat.code"
          @click="copy(cheat)"
        >
          {{ copied === cheat.code ? "已复制" : "复制" }}
        </button>
      </li>
    </ul>

    <footer class="cheats__foot">
      共 {{ CHEATS.length }} 条。带战役名的只在对应战役里生效，其余通用。
    </footer>
  </section>
</template>

<style scoped>
.cheats {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 20px 22px;
  border-radius: var(--radius-lg);
  background: var(--surface-1);
  border: 1px solid color-mix(in srgb, var(--outline) 55%, transparent);
  box-shadow: var(--shadow-2);
}

.cheats__head {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cheats__title {
  margin: 0;
  font-size: 18px;
}

.cheats__hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

kbd {
  padding: 1px 6px;
  border-radius: 4px;
  border: 1px solid var(--outline);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 11.5px;
}

.cheats__bar {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.cheats__search {
  width: 100%;
  padding: 9px 13px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  font-family: inherit;
  font-size: 13px;
}

.cheats__search:focus {
  outline: 2px solid color-mix(in srgb, var(--accent) 45%, transparent);
  outline-offset: 1px;
}

.cheats__scopes {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 11px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--outline);
  background: var(--surface-2);
  color: var(--on-surface-variant);
  font-family: inherit;
  font-size: 12px;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease, transform 0.1s ease;
}

.chip:hover {
  transform: translateY(-1px);
}

.chip--on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}

.chip__count {
  font-size: 10.5px;
  opacity: 0.75;
}

.cheats__empty {
  margin: 8px 0;
  font-size: 13px;
  color: var(--on-surface-variant);
}

.cheats__list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.cheat {
  display: grid;
  grid-template-columns: 1fr auto;
  grid-template-areas:
    "main copy"
    "meta copy";
  align-items: center;
  gap: 2px 12px;
  padding: 9px 12px;
  border-radius: var(--radius-sm);
  background: var(--surface-2);
  transition: background 0.15s ease, transform 0.1s ease;
}

.cheat:hover {
  background: var(--accent-soft);
  transform: translateX(2px);
}

.cheat__main {
  grid-area: main;
  display: flex;
  align-items: baseline;
  gap: 10px;
  flex-wrap: wrap;
}

.cheat__code {
  font-family: ui-monospace, Menlo, Consolas, monospace;
  font-size: 12.5px;
  font-weight: 700;
  color: var(--accent);
  user-select: all;
}

.cheat__effect {
  font-size: 13px;
}

.cheat__meta {
  grid-area: meta;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11.5px;
  color: var(--on-surface-variant);
}

.cheat__scope {
  padding: 0 7px;
  border-radius: var(--radius-pill);
  background: var(--surface-3);
  font-weight: 600;
}

.cheat__copy {
  grid-area: copy;
  padding: 5px 14px;
  border-radius: var(--radius-pill);
  border: none;
  background: var(--accent);
  color: #fff;
  font-family: inherit;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: transform 0.1s ease, background 0.15s ease;
}

.cheat__copy:hover {
  transform: scale(1.06);
}

.cheat__copy:active {
  transform: scale(0.94);
}

.cheat__copy--done {
  background: var(--success, #2e7d32);
}

.cheats__foot {
  font-size: 11.5px;
  color: var(--on-surface-variant);
  text-align: right;
}
</style>
