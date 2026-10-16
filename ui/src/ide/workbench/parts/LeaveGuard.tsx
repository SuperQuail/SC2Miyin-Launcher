import Button from "@jetbrains/ring-ui-built/components/button/button";

/** 打包没完就想走 —— 居中问一句。 */
export function LeaveGuard({
  done,
  total,
  onStay,
  onLeave,
}: {
  done: number;
  total: number;
  onStay: () => void;
  onLeave: () => void;
}) {
  return (
    <div className="guard" onClick={onStay}>
      <div className="guard__card" onClick={(event) => event.stopPropagation()}>
        <h3 className="guard__title">还在打包</h3>
        <p className="guard__text">
          正写到 {done} / {total}。离开就得先取消这次导出 —— <strong>写了一半的包会被删掉</strong>。
        </p>
        <div className="guard__actions">
          <Button onClick={onStay}>不取消</Button>
          {/* 这是"丢掉手上的活"，不是普通的主按钮 —— 用 danger */}
          <Button danger onClick={onLeave}>取消导出并离开</Button>
        </div>
      </div>
    </div>
  );
}
