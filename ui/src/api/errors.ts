/**
 * 把任意异常转成可展示的文本。
 *
 * 放在 `api/`（框架无关）而不是某个 composable 里：**Vue 侧和 React 侧都要用**，
 * 而现在两边不能互相 import 组件层的文件。
 */
export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
