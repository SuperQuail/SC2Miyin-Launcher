import { Button } from "../../ui";

/**
 * 开发者房间 · **开发者工具侧**。
 *
 * 这一页展示开发者页（`ide.html`）用的 Ring UI 控件。
 *
 * **为什么用 iframe 而不是直接 import**：Ring UI 是开发者页的组件库，与用户侧的控件层
 * 是两套视觉语言；而且两个入口本来就是**分开构建**的（AGENTS §18）—— 把 Ring UI 引进
 * 用户侧的包只会让它白白变大。所以这里嵌进来，两边各自的包都不受影响。
 */
export function DevSidePage() {
  return (
    <section className="room__card room__card--frame">
      <header className="room__frame-head">
        <div>
          <h3 className="room__card-title">开发者工具侧 · Ring UI</h3>
          <p className="room__card-note">
            下面是开发者页里的控件展台（`ide.html#gallery`）。它是**用 iframe 嵌进来的**：
            开发者页用的是 JetBrains 的 Ring UI，与用户侧的控件层是两套视觉语言，
            而且两个入口分开构建 —— 把 Ring UI 引进用户侧的包只会让它白白变大。
          </p>
        </div>
        <Button
          variant="tonal"
          onClick={() => {
            // 整页跳过去（开发者页自带「返回启动器」）
            location.href = "/ide.html#gallery";
          }}
        >
          在新页面打开
        </Button>
      </header>

      <div className="room__frame">
        <iframe className="room__frame-inner" src="/ide.html#gallery" title="开发者页控件展台" />
      </div>
    </section>
  );
}
