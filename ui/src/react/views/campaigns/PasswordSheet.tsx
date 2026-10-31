import { useState } from "react";

import { Button, Field, Sheet } from "../../ui";

/**
 * 加密包的密码框。
 *
 * 导入页与「更新战役」对话框共用：预检回 `needs_password` 时弹出来，
 * 用户填完重新预检 —— 密码错就留在这里再试（`wrong`）。
 * 用的是现成的 Sheet / Field / Button，不另造一套样式。
 */
export function PasswordSheet({
  source,
  wrong,
  onCancel,
  onSubmit,
}: {
  /** 哪个包要密码（显示给用户看，别让人对着两三个包猜）。 */
  source: string;
  /** 上一次的密码不对。 */
  wrong: boolean;
  onCancel: () => void;
  /** 提交密码；密码错的话调用方会把 `wrong` 置真、这个框留着。 */
  onSubmit: (password: string) => Promise<void>;
}) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit(): Promise<void> {
    if (!value.trim() || busy) return;
    setBusy(true);
    try {
      await onSubmit(value);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Sheet
      title="这个包有密码"
      dismissOnBackdrop={false}
      onClose={onCancel}
      actions={
        <>
          <Button onClick={onCancel}>取消</Button>
          <Button variant="primary" disabled={busy || !value.trim()} onClick={() => void submit()}>
            {busy ? "正在验证…" : "解锁"}
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        「{source}」是加密的压缩包，需要密码才能读取包内内容。密码由包作者提供。
      </p>
      <Field
        label="压缩包密码"
        invalid={wrong}
        error={wrong ? "密码不对，请再输入一次" : undefined}
        hint="密码区分大小写"
      >
        <input
          type="password"
          value={value}
          autoFocus
          autoComplete="off"
          onChange={(event) => setValue(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void submit();
          }}
        />
      </Field>
    </Sheet>
  );
}
