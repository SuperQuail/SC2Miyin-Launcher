import { useMemo, useState } from "react";

/** 展台里"点一下会变"的那几个控件 —— 一行一个，状态都收在这里。 */
export function useDemoState() {
  const [text, setText] = useState("示例文本");
  const [choice, setChoice] = useState("hots");
  const [checked, setChecked] = useState(true);
  const [on, setOn] = useState(true);
  const [tags, setTags] = useState<string[]>(["重制", "剧情"]);
  const [docs, setDocs] = useState(["Base.SC2Data", "说明.txt", "UnitData.xml", "README.md", "ComponentList.SC2Components"]);
  const [doc, setDoc] = useState("Base.SC2Data");
  const [guard, setGuard] = useState(false);

  return useMemo(
    () => ({
      text,
      setText,
      choice,
      setChoice,
      checked,
      setChecked,
      on,
      setOn,
      tags,
      setTags,
      docs,
      setDocs,
      doc,
      setDoc,
      guard,
      setGuard,
    }),
    [text, choice, checked, on, tags, docs, doc, guard],
  );
}

export type DemoState = ReturnType<typeof useDemoState>;

/** 每个"点一下会变"的控件对应上面哪个状态 —— 展台自己拿它做一致性自检。 */
export const DEMO_KEYS: Record<string, keyof DemoState> = {
  "input-normal": "text",
  "input-icon": "text",
  "input-multiline": "text",
  "select-normal": "choice",
  "toggle-normal": "on",
  "checkbox-normal": "checked",
  "tabs-docs": "docs",
  "tags-input": "tags",
  "dialog": "guard",
};

/** 列在展台上、但确实不需要状态的控件与变体。 */
const STATELESS = [
  "button-variants",
  "button-states",
  "button-icon",
  "button-loader-text",
  "button-toolbar",
  "input-disabled",
  "select-disabled",
  "toggle-disabled",
  "checkbox-partial",
  "checkbox-disabled",
  "tooltip",
  "progress-bar",
  "link",
  "tag",
  "popup-menu",
  "scrollable-section",
];

/** 自检：新控件忘了登记状态就会报出来 —— 免得展台变成"摆着好看"的花架子。 */
export function checkDemoKeys(keys: string[]): string[] {
  return keys.filter((key) => !(key in DEMO_KEYS) && !STATELESS.includes(key));
}
