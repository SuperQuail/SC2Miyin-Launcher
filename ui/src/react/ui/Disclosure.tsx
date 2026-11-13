import type { ReactNode } from "react";

import "./disclosure.css";

/** 折叠面板：原生 details/summary，键盘与读屏行为白给。 */
export function Disclosure({ title, children, open }: { title: ReactNode; children: ReactNode; open?: boolean }) {
  return (
    <details className="disclosure" open={open}>
      <summary className="disclosure__summary">{title}</summary>
      <div className="disclosure__body">{children}</div>
    </details>
  );
}
