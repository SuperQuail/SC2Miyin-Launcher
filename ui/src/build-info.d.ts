// 构建时间戳由 vite.config.ts 的 define 注入；tsc / vitest 下不存在，代码里用 typeof 兜底。
declare const __MIYIN_BUILD__: string;
