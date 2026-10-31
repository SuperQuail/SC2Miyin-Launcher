import { useState } from "react";

import type { ScnexusCampaign } from "../../../../api/types";
import { Button, Sheet } from "../../../ui";

/**
 * 「转为启动器托管」的确认框。
 *
 * 转换会把 SCNexus 里的内容复制进我们的战役库；删除源副本是**可选项**、默认不勾，
 * 而且它正在星际枢纽里启用时不允许删 —— 这和后端 convert 的约束一致。
 */
export function ConvertScnexusSheet({
  campaign,
  onClose,
  onConfirm,
}: {
  campaign: ScnexusCampaign;
  onClose: () => void;
  /** 返回 true 表示转换成功（由调用方负责刷新与关闭）。 */
  onConfirm: (removeSource: boolean) => Promise<boolean>;
}) {
  const [removeSource, setRemoveSource] = useState(false);
  const [busy, setBusy] = useState(false);
  const canRemove = !campaign.active;

  async function submit(): Promise<void> {
    setBusy(true);
    try {
      await onConfirm(removeSource && canRemove);
    } finally {
      setBusy(false);
    }
  }

  return (
    <Sheet
      title={"将「" + campaign.name + "」转为启动器托管？"}
      onClose={busy ? undefined : onClose}
      actions={
        <>
          <Button onClick={onClose} disabled={busy}>
            取消
          </Button>
          <Button variant="primary" loading={busy} onClick={() => void submit()}>
            {busy ? "转换中…" : "转为启动器托管"}
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        会把「{campaign.name}」复制进启动器战役库，转换后就能像普通战役一样启用、导出。
        星际枢纽里的安装不会被改动。
      </p>
      <p className="sheet__text">
        来源：<code>{campaign.path}</code>
        （{campaign.source === "ccm" ? "CCM 目录" : "星际枢纽标准库"}）
      </p>

      <label className="hub-check">
        <input
          type="checkbox"
          checked={removeSource}
          disabled={!canRemove || busy}
          onChange={(event) => setRemoveSource(event.target.checked)}
        />
        <span>
          转换后删除星际枢纽里的副本
          <em>不勾选时只复制，星际枢纽里那份保持原样。</em>
        </span>
      </label>

      {!canRemove ? (
        <p className="hub-note">这个战役在星际枢纽里正在启用，转换后不会删除源副本。</p>
      ) : removeSource ? (
        <p className="hub-warn">
          转换成功后会删除 <code>{campaign.path}</code>，这一步不可恢复。
        </p>
      ) : null}
    </Sheet>
  );
}
