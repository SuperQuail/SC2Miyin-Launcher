import { AppStateProvider } from "../state/AppState";
import { AppShell } from "./AppShell";

/** 用户侧入口：外壳与全局状态。 */
export function App() {
  return (
    <AppStateProvider>
      <AppShell />
    </AppStateProvider>
  );
}
