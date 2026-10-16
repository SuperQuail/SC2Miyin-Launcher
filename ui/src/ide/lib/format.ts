/** 开发者页用到的纯格式化函数 —— 不含任何框架代码。 */

/** 把字节数写成人看的大小；0 或负数返回空串（调用方自己决定显示什么）。 */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "";
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(0) + " KB";
  if (bytes < 1024 * 1024 * 1024) return (bytes / 1024 / 1024).toFixed(1) + " MB";
  return (bytes / 1024 / 1024 / 1024).toFixed(2) + " GB";
}

/** 相对时间（提交记录用）；at 是秒级时间戳。 */
export function when(at: number): string {
  if (!at) return "";
  const delta = Date.now() / 1000 - at;
  if (delta < 60) return "刚刚";
  if (delta < 3600) return Math.floor(delta / 60) + " 分钟前";
  if (delta < 86400) return Math.floor(delta / 3600) + " 小时前";
  if (delta < 172800) return "昨天";
  return new Date(at * 1000).toLocaleDateString("zh-CN");
}
