/**
 * 开发者页的设置存在本地：包名、包信息、封面与说明书的路径。
 *
 * 这些都是**手填的**，重开一次页面就没了太亏 —— 下次打开原样回来。
 * （提交历史里也存了一份当时的包信息，可以随时捡回来。）
 */
const SETTINGS_KEY = "miyin.dev.settings";

/** 表单里的包信息。 */
export type DevMeta = {
  author: string;
  version: string;
  description: string;
  tags: string;
  id: string;
  kind: string;
  campaign: string;
  mainMap: string;
  modid: string;
  mods: string;
};

export const EMPTY_META: DevMeta = {
  author: "",
  version: "",
  description: "",
  tags: "",
  id: "",
  kind: "campaign",
  campaign: "",
  mainMap: "",
  modid: "",
  mods: "",
};

/** 表单里的包信息，字段与 `PackageMeta` 对齐，只是多两个"选中的文件路径"。 */
export type DevSettings = {
  pkg: string;
  meta: DevMeta;
  cover: string;
  doc: string;
  /** 勾了哪些文件 —— 重开页面、重启启动器都要还在 */
  picked: string[];
};

export function loadSettings(): DevSettings | null {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    return raw ? (JSON.parse(raw) as DevSettings) : null;
  } catch {
    return null;
  }
}

export function saveSettings(settings: DevSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // 隐私模式下存不了，当次有效就行
  }
}
