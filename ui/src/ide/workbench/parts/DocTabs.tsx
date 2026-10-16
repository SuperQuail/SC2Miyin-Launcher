import Button from "@jetbrains/ring-ui-built/components/button/button";
import { Tab, Tabs } from "@jetbrains/ring-ui-built/components/tabs/tabs";
import closeIcon from "@jetbrains/icons/close";

import type { DevEntry } from "../../api";

export type DocTabsProps = {
  opened: DevEntry[];
  active: string | null;
  onOpen: (entry: DevEntry) => void;
  onClose: (entry: DevEntry) => void;
};

/**
 * 文档标签页。
 *
 * 用 Ring UI 的 `Tabs`：`children` 就是标签本身，不需要再配一套面板；
 * `autoCollapse` 让放不下的标签折成「更多」，所以标签多了也不会横向堆出去。
 * （AGENTS §18 那句「排不出多个标签」实测不成立。）
 */
export function DocTabs({ opened, active, onOpen, onClose }: DocTabsProps) {
  return (
    <header className="doctabs">
      <Tabs
        autoCollapse
        selected={active ?? undefined}
        onSelect={(key) => {
          const entry = opened.find((item) => item.abs === key);
          if (entry) onOpen(entry);
        }}
      >
        {opened.map((entry) => (
          <Tab
            key={entry.abs}
            id={entry.abs}
            title={
              <span className="doctab">
                {entry.name}
                <Button
                  className="doctab__close"
                  icon={closeIcon}
                  title="关闭"
                  onClick={(event) => {
                    // 关闭不是切换：别让点击冒泡到标签本身
                    event.stopPropagation();
                    onClose(entry);
                  }}
                />
              </span>
            }
          />
        ))}
      </Tabs>
    </header>
  );
}
