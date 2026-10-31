import { useEffect, useState } from "react";

import { api } from "../../../../api/bridge";
import { errorText } from "../../../../api/errors";
import type { ExportFormat, ExportPreview, Variant } from "../../../../api/types";
import { Button, Sheet } from "../../../ui";
import "./scnexus.css";

/**
 * 导出向导（替换原来 SlotMenu 上直接点「导出」的做法）。
 *
 * 第一步选格式：弥音包（完整保真）/ 星际枢纽兼容包（先预检、把会丢的内容逐条列出来），
 * 再选「包含补丁」，最后才选保存位置 —— 不在用户没看见代价之前写文件。
 */
export function ExportSheet({
  slot,
  variant,
  notify,
  onBusy,
  onClose,
}: {
  slot: string;
  variant: Variant;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  /** 导出 / 预检在跑时告诉父级，让别的导出按钮避开。 */
  onBusy?: (busy: boolean) => void;
  onClose: () => void;
}) {
  const [format, setFormat] = useState<ExportFormat>("miyin");
  const [mergePatches, setMergePatches] = useState(true);
  const [preview, setPreview] = useState<ExportPreview | null>(null);
  const [previewing, setPreviewing] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);

  // 只有星际枢纽格式需要先看「会丢什么」；弥音包是完整格式，不用预检
  useEffect(() => {
    let cancelled = false;
    setPreview(null);
    setPreviewError(null);
    if (format !== "scnexus") {
      setPreviewing(false);
      return;
    }
    setPreviewing(true);
    void (async () => {
      try {
        const result = await api.previewExportVariant(slot, variant.id, mergePatches, format);
        if (!cancelled) setPreview(result);
      } catch (error) {
        if (!cancelled) setPreviewError(errorText(error));
      } finally {
        if (!cancelled) setPreviewing(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [format, mergePatches, slot, variant.id]);

  useEffect(() => {
    onBusy?.(exporting || previewing);
    return () => onBusy?.(false);
  }, [exporting, previewing, onBusy]);

  // 预检失败时也不能放行：用户还没看到会丢什么，不能悄悄写文件
  const blocked =
    format === "scnexus" &&
    (previewing || previewError !== null || (preview !== null && !preview.supported));
  const losses = preview?.losses ?? [];

  async function runExport(): Promise<void> {
    try {
      const destination = await api.pickExportPath(variant.name);
      if (!destination) return;
      setExporting(true);
      const report = await api.exportVariant(slot, variant.id, destination, mergePatches, format);
      const lost = report.losses?.length ?? 0;
      let message =
        "已导出「" + variant.name + "」：" + report.maps + " 张地图 · " + report.mods + " 个模组";
      if (lost > 0) message += "（有 " + lost + " 项内容没有进入星际枢纽包）";
      notify("success", message);
      onClose();
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setExporting(false);
    }
  }

  return (
    <Sheet
      title={"导出「" + variant.name + "」"}
      onClose={exporting ? undefined : onClose}
      actions={
        <>
          <Button onClick={onClose} disabled={exporting}>
            取消
          </Button>
          <Button
            variant="primary"
            loading={exporting}
            disabled={blocked}
            onClick={() => void runExport()}
          >
            {exporting ? "导出中…" : "选择位置并导出"}
          </Button>
        </>
      }
    >
      <p className="sheet__text">选择导出格式。两种格式都能重新导入启动器，区别在于保真程度。</p>

      <div className="export-formats">
        <button
          type="button"
          className={"export-format" + (format === "miyin" ? " export-format--on" : "")}
          aria-pressed={format === "miyin"}
          onClick={() => setFormat("miyin")}
        >
          <span className="export-format__head">
            <span className="export-format__title">弥音包格式</span>
            <span className="export-format__tag">推荐</span>
          </span>
          <span className="export-format__desc">
            完整保留地图、模组、封面、说明和补丁绑定，适合备份与分发。
          </span>
          <span className="export-format__arrow" aria-hidden="true">
            →
          </span>
        </button>

        <button
          type="button"
          className={"export-format" + (format === "scnexus" ? " export-format--on" : "")}
          aria-pressed={format === "scnexus"}
          onClick={() => setFormat("scnexus")}
        >
          <span className="export-format__head">
            <span className="export-format__title">星际枢纽兼容包</span>
          </span>
          <span className="export-format__desc">
            星际枢纽可以直接导入；不支持的内容会先逐条列出，不会悄悄丢掉。
          </span>
          <span className="export-format__arrow" aria-hidden="true">
            →
          </span>
        </button>
      </div>

      <label className="hub-check">
        <input
          type="checkbox"
          checked={mergePatches}
          onChange={(event) => setMergePatches(event.target.checked)}
        />
        <span>
          包含补丁
          <em>把已启用的补丁合并进导出包；不合并时补丁不会跟着走。</em>
        </span>
      </label>

      {format === "scnexus" && (
        <div className="export-losses">
          <p className="export-losses__title">以下内容不会进入星际枢纽包：</p>
          {previewing ? (
            <p className="export-losses__note">正在检查…</p>
          ) : previewError ? (
            <p className="export-losses__warn">{previewError}</p>
          ) : losses.length === 0 ? (
            <p className="export-losses__note">没有会丢失的内容。</p>
          ) : (
            <ul className="export-losses__list">
              {losses.map((loss) => (
                <li key={loss.kind + loss.label} className="export-losses__item">
                  <strong>{loss.label}</strong>
                  <span>{loss.detail}</span>
                </li>
              ))}
            </ul>
          )}
          {preview !== null && !preview.supported && (
            <p className="export-losses__warn">
              这个版本不能导出为星际枢纽兼容包，请换「弥音包格式」。
            </p>
          )}
        </div>
      )}
    </Sheet>
  );
}
