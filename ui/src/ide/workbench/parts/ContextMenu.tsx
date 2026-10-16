import Button from "@jetbrains/ring-ui-built/components/button/button";

export type ContextMenuItem = {
  label: string;
  /** 危险操作（删除这种）用红字 */
  danger?: boolean;
  onSelect: () => void;
};

/**
 * 固定定位的右键菜单。
 *
 * 菜单项是 Ring UI 的 Button —— 外观由 .ctx__item 收口。空白处右键、
 * 文件行右键、提交记录右键都走这一份，不再各写一套（AGENTS §18.2）。
 */
export function ContextMenu({ x, y, items }: { x: number; y: number; items: ContextMenuItem[] }) {
  return (
    <div className="ctx" style={{ left: x, top: y }} onClick={(event) => event.stopPropagation()}>
      {items.map((item) => (
        <Button
          key={item.label}
          className={"ctx__item" + (item.danger ? " ctx__item--danger" : "")}
          onClick={item.onSelect}
        >
          {item.label}
        </Button>
      ))}
    </div>
  );
}
