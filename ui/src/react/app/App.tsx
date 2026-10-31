import { useEffect } from "react";

import { AppStateProvider } from "../state/AppState";
import { AppShell } from "./AppShell";
import { BootGate } from "./BootGate";
import { ErrorBoundary } from "./ErrorBoundary";
import { PermissionHost } from "./PermissionHost";
import { markMounted } from "./startup";

/** 用户侧入口：错误边界 → 等后端数据 → 外壳与全局状态。 */
export function App() {
  // 挂上了就告诉启动兜底别再抢界面（见 startup.ts 的看门狗）
  useEffect(() => markMounted(), []);

  return (
    <ErrorBoundary>
      <BootGate>
        {/* 要等后端初始化完再挂（在那之前碰后端的命令会 panic）。
            提权对话框挂在所有页面外面 —— 哪个页面撞上 [PERMISSION] 都在这里弹 */}
        <PermissionHost>
          <AppStateProvider>
            <AppShell />
          </AppStateProvider>
        </PermissionHost>
      </BootGate>
    </ErrorBoundary>
  );
}
