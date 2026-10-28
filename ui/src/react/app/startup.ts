/**
 * 启动兜底。
 *
 * 前端挂不上的时候，用户至少该看到一句人话和一个「重新加载」按钮，而不是一片白。
 * 走这条路时 **React 已经不可用**（模块执行出错 / 根部渲染直接抛 / 启动超时），
 * 所以这里全是原生 DOM；样式用 base.css 的 .boot-fail（CSS 是另一个请求，JS 挂了它通常还在）。
 */

/** React 挂上之后往 <html> 打这个标记，见 App.tsx。 */
const MOUNTED = "miyinMounted";

/** 启动超时：这段时间还没挂上就认为前端没起来。 */
const STARTUP_TIMEOUT_MS = 20_000;

/** 已经报过一次就别再刷 —— 留第一个原因，后面的一律只进控制台。 */
let reported = false;

/** 挂上了没有（启动兜底只在挂上之前动手）。 */
export function markMounted(): void {
  document.documentElement.dataset[MOUNTED] = "1";
}

function isMounted(): boolean {
  return document.documentElement.dataset[MOUNTED] === "1";
}

/** 把任意错误变成一段可复制的文本（给用户看，也给开发者定位）。 */
function describe(reason: unknown): string {
  if (reason instanceof Error) return reason.stack || reason.message || String(reason);
  if (typeof reason === "string") return reason;
  try {
    return JSON.stringify(reason);
  } catch {
    return String(reason);
  }
}

/**
 * 把 #app 换成一页「启动失败」。
 *
 * 只在 React 没挂上之前调；挂上之后界面是活的，抢过来反而更糟（见 installStartupGuards）。
 */
export function showStartupFailure(reason: unknown): void {
  console.error("[miyin] 启动失败", reason);
  if (reported) return;
  reported = true;

  const container = document.getElementById("app");
  if (!container) return;

  // 清掉 index.html 里的静态占位；半死的 React 树一并清掉
  container.textContent = "";

  const wrap = document.createElement("div");
  wrap.className = "boot-fail";

  const card = document.createElement("div");
  card.className = "boot-fail__card";

  const title = document.createElement("h1");
  title.className = "boot-fail__title";
  title.textContent = "启动失败";

  const hint = document.createElement("p");
  hint.className = "boot-fail__text";
  hint.textContent = "界面没能加载出来。可以点下面重新加载；还不行就把这段错误发给开发者。";

  const detail = document.createElement("pre");
  detail.className = "boot-fail__reason";
  detail.textContent = describe(reason);

  const reload = document.createElement("button");
  reload.type = "button";
  reload.className = "btn btn-primary";
  reload.textContent = "重新加载";
  reload.addEventListener("click", () => location.reload());

  card.append(title, hint, detail, reload);
  wrap.append(card);
  container.append(wrap);
}

/**
 * 在 render 之前装上：window 上的未捕获错误 + 未处理 Promise + 启动看门狗。
 *
 * **挂载之后不再抢界面**：那时有 ErrorBoundary / toast 在管，把能用的界面换成
 * 一页「启动失败」只会更糟。这三种情况都只负责「React 还没挂上」这一段。
 */
export function installStartupGuards(): void {
  window.addEventListener("error", (event) => {
    if (isMounted()) return;
    showStartupFailure(event.error ?? event.message);
  });

  window.addEventListener("unhandledrejection", (event) => {
    if (isMounted()) return;
    showStartupFailure(event.reason);
  });

  window.setTimeout(() => {
    if (isMounted()) return;
    showStartupFailure("启动超时：" + STARTUP_TIMEOUT_MS / 1000 + " 秒内界面没有挂载出来。");
  }, STARTUP_TIMEOUT_MS);
}
