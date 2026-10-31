import type { ReactNode } from "react";

import Button from "@jetbrains/ring-ui-built/components/button/button";
import Dialog from "@jetbrains/ring-ui-built/components/dialog/dialog";
import DropdownMenu from "@jetbrains/ring-ui-built/components/dropdown-menu/dropdown-menu";
import Icon from "@jetbrains/ring-ui-built/components/icon/icon";
import Select from "@jetbrains/ring-ui-built/components/select/select";
import { Tab, Tabs } from "@jetbrains/ring-ui-built/components/tabs/tabs";
import TagsInput from "@jetbrains/ring-ui-built/components/tags-input/tags-input";
import closeIcon from "@jetbrains/icons/close";

import type { DemoState } from "./state";

const MENU_ROWS = [
  { key: "open", label: "打开" },
  { key: "select", label: "勾选（含子目录）" },
  // 危险项：业务侧靠类名上色（见 MenuLayer.tsx 与 ide.css 的 .ctx__item--danger）
  { key: "forget", label: "删掉这条记录", className: "ctx__item--danger" },
];

/** 包信息里的「标签 / 依赖模组」就是它。 */
export function TagsDemo({ state }: { state: DemoState }) {
  return (
    <TagsInput
      size={Select.Size.M}
      tags={state.tags.map((tag) => ({ key: tag, label: tag }))}
      placeholder="打一个字回车"
      allowAddNewTags
      dataSource={() => []}
      onAddTag={(params) => state.setTags([...state.tags, String(params.tag.label ?? "")])}
      onRemoveTag={(params) => state.setTags(state.tags.filter((tag) => tag !== params.tag.label))}
    />
  );
}

/** 文档标签页 —— 每个标签带关闭 ✕，放不下的自动折成「更多」。 */
export function TabsDemo({ state }: { state: DemoState }) {
  return (
    <>
      <div className="gdoctabs">
        <Tabs autoCollapse selected={state.doc} onSelect={state.setDoc}>
          {state.docs.map((name) => (
            <Tab
              key={name}
              id={name}
              title={
                <span className="doctab">
                  {name}
                  {/* 同 DocTabs：标签里不能再套 button，关闭用 span + 图标 */}
                  <span
                    className="doctab__close"
                    title="关闭"
                    onClick={(event) => {
                      event.stopPropagation();
                      state.setDocs((prev) => prev.filter((item) => item !== name));
                    }}
                  >
                    <Icon glyph={closeIcon} />
                  </span>
                </span>
              }
            />
          ))}
        </Tabs>
      </div>
      <Button onClick={() => state.setDocs(["Base.SC2Data", "说明.txt", "UnitData.xml", "README.md", "ComponentList.SC2Components"])}>
        重置标签
      </Button>
    </>
  );
}

/**
 * 右键菜单的菜单项。生产里落点由 `ContextMenuLayer` 给（隐形锚点 + 文档坐标），
 * 锚点元素用谁则随场景 —— 文件行、提交行、空白处都行，这里拿按钮演示。
 */
export function MenuDemo() {
  return (
    <DropdownMenu
      data={MENU_ROWS}
      anchor={(props) => <Button {...(props as object)}>点我展开菜单</Button>}
    />
  );
}

/** 打包没完就想走的那个确认框。 */
export function DialogDemo({ state }: { state: DemoState }) {
  return (
    <>
      <Button onClick={() => state.setGuard(true)}>打开离开确认框</Button>
      {state.guard && (
        <Dialog
          show
          trapFocus
          dense
          label="还在打包"
          closeButtonTitle="不取消，继续打包"
          onCloseClick={() => state.setGuard(false)}
          onOverlayClick={() => state.setGuard(false)}
          onCloseAttempt={() => state.setGuard(false)}
          onEscPress={() => state.setGuard(false)}
        >
          <h3 className="guard__title">还在打包</h3>
          <p className="guard__text">
            正写到 3 / 8。离开就得先取消这次导出 —— <strong>写了一半的包会被删掉</strong>。
          </p>
          <div className="guard__actions">
            <Button onClick={() => state.setGuard(false)}>不取消</Button>
            <Button danger onClick={() => state.setGuard(false)}>取消导出并离开</Button>
          </div>
        </Dialog>
      )}
    </>
  );
}

/** 一行：左边固定宽度的名字，右边摆这个控件的各个变体。 */
export function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grow">
      <span className="glabel">{label}</span>
      {children}
    </div>
  );
}

export function Card({ title, note, children }: { title: string; note: string; children: ReactNode }) {
  return (
    <section className="gcard">
      <h2 className="gcard__title">{title}</h2>
      <p className="gcard__note">{note}</p>
      {children}
    </section>
  );
}
