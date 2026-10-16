import { useState } from "react";

import { Button } from "../../ui";
import { DevSidePage } from "./DevSidePage";
import { UserSidePage } from "./UserSidePage";
import "./room.css";

/**
 * **开发者房间**：控件的展示台，分两页。
 *
 * - **用户侧**（`index.html`）：react/ui 的控件层与共用组件
 * - **开发者工具侧**（`ide.html`）：Ring UI 那套（用 iframe 嵌，见 DevSidePage 的说明）
 *
 * 规矩写在 AGENTS §18.1：**没有在房间里出现过的控件，不许上生产界面。**
 */

type RoomPage = "user" | "dev";

const PAGES: { id: RoomPage; label: string; sub: string }[] = [
  { id: "user", label: "用户侧", sub: "index.html" },
  { id: "dev", label: "开发者工具侧", sub: "ide.html" },
];

export function RoomView({ onBack }: { onBack: () => void }) {
  // `#room=dev` 直接进第二页（截图与自测用）
  const [page, setPage] = useState<RoomPage>(() =>
    typeof location !== "undefined" && location.hash.includes("dev") ? "dev" : "user",
  );

  return (
    <div className="room">
      <header className="room__head">
        <div>
          <h2 className="room__title">开发者房间</h2>
          <p className="room__lede">
            所有控件的展示台。**新控件必须先进这里**，确认外观与手感之后再拿到生产界面用。
            它分两页：用户侧与开发者工具侧，各自只放自己那一套控件。
          </p>
        </div>
        <Button variant="outline" onClick={onBack}>
          返回设置
        </Button>
      </header>

      <nav className="room__tabs">
        {PAGES.map((item) => (
          <button
            key={item.id}
            className={"room__tab" + (page === item.id ? " room__tab--on" : "")}
            type="button"
            onClick={() => setPage(item.id)}
          >
            {item.label}
            <span className="room__tab-sub">{item.sub}</span>
          </button>
        ))}
      </nav>

      {page === "user" ? <UserSidePage /> : <DevSidePage />}
    </div>
  );
}
