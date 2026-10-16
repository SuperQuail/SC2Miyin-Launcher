/**
 * 把 Vue 单文件组件里的 <style scoped> 原样搬到 React 侧的 .css。
 *
 * 为什么要脚本：Vue 的 scoped 样式天生隔离，React 那边是全局的 ——
 * 直接把四份样式铺到全局，同名选择器就会互相抢（例如几份里都有 .grid / .tag / .hint）。
 * 做法是给每条规则的**外层选择器**加一个作用域根（组件自己挂在根元素上），
 * 等于把 scoped 那层隔离补回来。
 *
 * 两个不能省的地方：
 * 1. **按花括号配对切规则**，不按行切 —— 扁平的 "^}" 正则会从 @keyframes 中间断开，
 *    postcss 之后就是 "Invalid empty selector"，构建直接红。
 * 2. **先是者优先**：同名选择器只保留第一份（合并多个来源时的既有做法）。
 *
 * 用法：node scripts/scope-vue-styles.mjs
 *
 * 几个来源 .vue 已经删了（迁完就删）—— 要重新生成时先把它们从 git 里取出来，
 * 用 `DSH_VUE_SRC` 指过去：
 *   git show HEAD:ui/src/views/CampaignsView.vue > %TEMP%\CampaignsView.vue   # 其余同理
 *   $env:DSH_VUE_SRC = "$env:TEMP"; node scripts/scope-vue-styles.mjs
 */
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const ui = resolve(here, "../ui");
// 来源 .vue 的根：默认在仓库里；迁完删掉之后用 DSH_VUE_SRC 指到从 git 取出的副本
const sourceRoot = process.env.DSH_VUE_SRC ? resolve(process.env.DSH_VUE_SRC) : ui;

/** 一个来源：从哪个 .vue 抽样式，装进哪个作用域根，写到哪个 .css。 */
const JOBS = [
  {
    from: "src/views/CampaignsView.vue",
    to: "src/react/views/campaigns/campaigns.css",
    root: ".campaigns-page",
  },
  {
    from: "src/views/SlotMenuView.vue",
    to: "src/react/views/campaigns/slot-menu.css",
    root: ".campaigns-menu",
  },
  {
    from: "src/components/ImportDialog.vue",
    to: "src/react/views/campaigns/import-dialog.css",
    root: ".campaigns-import",
  },
  {
    from: "src/components/CustomCampaignPanel.vue",
    to: "src/react/views/campaigns/custom-campaign.css",
    root: ".campaigns-custom",
  },
  {
    from: "src/views/CustomView.vue",
    to: "src/react/views/custom/custom.css",
    root: ".custom-page",
  },
  {
    // 控件自己的样式必须跟着控件走：Vue 那份 <style scoped> 原来是 Field.vue 私有的，
    // base.css 里没有 —— 不搬过来的话 React 的 Field 渲染出来光秃秃的。
    from: "src/ui/Field.vue",
    to: "src/react/ui/field.css",
    root: "",
  },
];

/** 取出 <style scoped>…</style> 的内容。 */
function styleOf(source) {
  const match = /<style scoped>([\s\S]*?)<\/style>/.exec(source);
  if (!match) throw new Error("没找到 <style scoped>");
  return match[1];
}

/**
 * 按花括号配对切出顶层规则。
 *
 * 返回 [{ selector, body, atRule }]，其中 at-rule（@media / @keyframes …）整块作为 body 保留，
 * 里面的嵌套规则**递归**同样处理 —— 这正是扁平正则做不到的地方。
 */
function parseRules(css) {
  // 先去掉注释：规则之间的注释会被切进下一条的选择器里（"… /* 顶部看板 */ .hero"），
  // 生成的 CSS 既难看又容易在下一次解析时出岔子。
  css = css.replace(/\/\*[\s\S]*?\*\//g, "");
  // :deep(...) 是 Vue 编译器专用的，普通 CSS 里 "… :deep(input)" 整条都匹配不上 —— 拆掉壳
  css = css.replace(/:deep\(([^)]*)\)/g, "$1");

  const rules = [];
  let index = 0;

  while (index < css.length) {
    const open = css.indexOf("{", index);
    if (open === -1) break;

    const selector = css.slice(index, open).trim();
    // 找配对的右花括号
    let depth = 1;
    let cursor = open + 1;
    while (cursor < css.length && depth > 0) {
      const ch = css[cursor];
      if (ch === "{") depth += 1;
      else if (ch === "}") depth -= 1;
      cursor += 1;
    }

    const body = css.slice(open + 1, cursor - 1);
    if (selector) rules.push({ selector, body, atRule: selector.startsWith("@") });
    index = cursor;
  }

  return rules;
}

/** 给一条规则的外层选择器加上作用域根；at-rule 递归处理内部。 */
function scope(rule, root, seen, out) {
  const { selector, body, atRule } = rule;

  // root 为空 = 不加作用域（控件自己的样式，类名本来就只属于它）
  if (!root) {
    if (atRule) {
      const inner = [];
      for (const child of parseRules(body)) scope(child, root, seen, inner);
      out.push(selector + " {" + inner.join("") + "}");
      return;
    }
    if (seen.has(selector)) return;
    seen.add(selector);
    out.push(selector + " {" + body + "}");
    return;
  }

  if (atRule) {
    const inner = [];
    for (const child of parseRules(body)) scope(child, root, seen, inner);
    out.push(selector + " {" + inner.join("") + "}");
    return;
  }

  if (seen.has(selector)) return; // 先是者优先
  seen.add(selector);

  const joined = selector
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => (part === root ? root : root + " " + part))
    .join(", ");
  out.push(joined + " {" + body + "}");
}

for (const job of JOBS) {
  const css = styleOf(readFileSync(resolve(sourceRoot, job.from), "utf8"));
  const out = [];
  const seen = new Set();
  for (const rule of parseRules(css)) scope(rule, job.root, seen, out);
  const text =
    "/* 由 scripts/scope-vue-styles.mjs 从 " + job.from + " 抽出，请勿手改 */\n" +
    out.join("\n") + "\n";
  mkdirSync(dirname(resolve(ui, job.to)), { recursive: true });
  writeFileSync(resolve(ui, job.to), text, "utf8");
  console.log(job.to + "  <- " + job.from + " (" + out.length + " 条规则)");
}