/**
 * 更新终端里的一行。
 *
 * 原来直接读 `new Date()`，测不了；现在时间由调用方给，纯函数可以断言。
 */
export function formatLogLine(line: string, now: Date = new Date()): string {
  return "[" + now.toTimeString().slice(0, 8) + "] " + line;
}

/** 终端最多留多少行（再老的滚掉）。 */
export const MAX_LOG_LINES = 400;

/** 追加一行并裁到上限。 */
export function appendLog(lines: string[], line: string, now?: Date): string[] {
  return [...lines.slice(-(MAX_LOG_LINES - 1)), formatLogLine(line, now)];
}
