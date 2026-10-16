import { useEffect, useState, type ReactNode } from "react";

import { formatBytes } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { ImportMode, ImportPreview, PackageInspection } from "../../../api/types";
import { Button } from "../../ui";
import "./import-dialog.css";

/**
 * 导入战役包（React 版，从 `components/ImportDialog.vue` 整块迁过来）。
 *
 * 选文件（或拖进来）→ 预检 → 确认 → 进库。战役列表页与某个战役的菜单页都用它：
 * 后者带 `mode="update"` + `slot` + `variantId`，是"把这一版换掉"。
 *
 * 外壳只从 `importNonce` 递一次"该开始了"的信号（对应原来的
 * `importer.value.startImport()`）—— 对话框自己不留任何对外方法。
 */
export function ImportDialog({
  entry = "campaign",
  mode = "import",
  slot = "",
  variantId = "",
  importNonce = 0,
  slots,
  droppedPackage,
  consumeDropped,
  notify,
  refresh,
  onImported,
  onBusy,
}: {
  /** 从哪个入口打开的；`custom` 表示站在「自制战役」页 —— 不再判断它属于哪部原版战役。 */
  entry?: "campaign" | "custom";
  /** `update` 表示这次是"更新某个战役"：删掉旧的、装进新的。 */
  mode?: "import" | "update";
  /** 更新模式下点明的目标战役 —— 有它就是"强制更新"，不看包认得出谁。 */
  slot?: string;
  /** 更新模式下点明的那一版已安装的包；空串表示让后端按包的身份去找。 */
  variantId?: string;
  /** 每次 +1 就打开一次文件选择器（外壳按钮的入口）。 */
  importNonce?: number;
  slots: { slug: string; display_name: string }[];
  /** 被拖进窗口的包路径。 */
  droppedPackage: string | null;
  consumeDropped: () => void;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  /** 库变了，让外壳重新读一遍槽位。 */
  refresh: () => void;
  /** 导入/更新完成，去这个槽位的菜单页。 */
  onImported: (slot: string) => void;
  /** 有没有在跑 —— 外壳借此禁用「导入战役包」按钮、换文案。 */
  onBusy?: (busy: boolean) => void;
}) {
  const isCustomEntry = entry === "custom";
  const isUpdate = mode === "update";

  /** 正在处理的导入：选好文件、预检完、等用户确认。 */
  const [pending, setPending] = useState<{ preview: ImportPreview; slot: string; mode: ImportMode } | null>(null);
  const [importing, setImporting] = useState(false);
  // "有没有在跑"要告诉外壳（它要禁用「导入战役包」按钮）—— 放在副作用里报，
  // 免得在渲染过程中通知父组件。
  useEffect(() => {
    onBusy?.(importing);
  }, [importing, onBusy]);

  /** 二次确认弹窗开着没有（高置信度被强改时）。 */
  const [overrideOpen, setOverrideOpen] = useState(false);
  /** 更新完的追问：新包里多出来的模组（后端默认已经挂上）。 */
  const [addedMods, setAddedMods] = useState<string[]>([]);
  const [keepAdded, setKeepAdded] = useState(true);
  const [addedTarget, setAddedTarget] = useState<{ slot: string; variantId: string } | null>(null);
  /** 要不要把包里的模组一起挂上（默认要）。 */
  const [mountMods, setMountMods] = useState(true);

  const inspection: PackageInspection | null = pending?.preview.inspection ?? null;
  const preview: ImportPreview | null = pending?.preview ?? null;
  const isPatch = inspection?.kind === "patch";
  const packageMods = inspection?.mod_count ?? 0;

  /** 没有自动认出归属时才需要用户选；但**永远允许**改。 */
  const needsTarget = !isCustomEntry && pending !== null && pending.preview.slot === null;

  /**
   * 要显示给用户看的预检提示。
   *
   * **自制战役入口要把「归属」相关的几条滤掉** —— 在那一页，归属是用户自己定的，
   * 再说「包内没有声明归属，已按地图文件名判定为自由之翼」纯属误导。
   */
  const aboutBelonging = new Set(["NO_METADATA", "CAMPAIGN_IDENTIFIED", "CAMPAIGN_UNKNOWN"]);
  const shownIssues = (inspection?.issues ?? []).filter(
    (issue) => !isCustomEntry || !aboutBelonging.has(issue.code),
  );

  const pendingName = inspection?.name ?? inspection?.suggested_dir_name ?? "未命名战役";

  /** 信息来源的措辞 —— 自动识别是**尽力而为**，不能说成确定的。 */
  const sourceLabel = (() => {
    if (preview?.source === "metadata") return "包内声明的资料片";
    if (preview?.source === "inferred") return "自动识别";
    return "无法识别";
  })();

  const identification = inspection?.identification ?? null;
  const evidenceText = identification ? "依据：" + identification.detail : "";

  /**
   * **高置信度**归属：包内明确声明了资料片，或者证据链给的是精确证据。
   * 启发式（地图名前缀）不算 —— 那种本来就只是猜，用户改掉很正常。
   */
  const highConfidence = (() => {
    if (!preview || preview.slot === null) return false;
    if (preview.source === "metadata") return true;
    return identification != null && identification.evidence !== "map_name_prefix";
  })();

  /** 用户把归属改到了别处。 */
  const targetChanged =
    pending !== null && preview?.slot != null && pending.slot !== preview.slot;

  /**
   * 要不要警告：**高置信度 + 用户强改**。拦一下但不禁止 —— 作者有时确实知道自己在干什么。
   */
  const overrideWarning = !isCustomEntry && highConfidence && targetChanged;

  const nameOf = (slug: string | null | undefined): string => {
    if (!slug) return "";
    return slots.find((item) => item.slug === slug)?.display_name ?? slug;
  };
  const identifiedName = nameOf(preview?.slot);
  const chosenName = nameOf(pending?.slot);

  /** 冲突时的新旧版本说法。 */
  const conflictText = (() => {
    const conflict = preview?.conflict;
    if (!conflict) return "";
    const labels: Record<string, string> = {
      newer: "更新的版本",
      same: "相同的版本",
      older: "更旧的版本",
      unknown: "无法比较版本",
    };
    const incoming = conflict.incoming_version ?? "未标版本";
    const existing = conflict.existing_version ?? "未标版本";
    return (
      "库里已有「" + conflict.existing_name + "」" + existing +
      "，本次导入 " + incoming + " —— " + labels[conflict.relation]
    );
  })();

  /** 预检一个包 —— 选文件和拖进来走的是同一条路。 */
  async function prepare(path: string): Promise<void> {
    setImporting(true);
    try {
      const result = await api.prepareImport(path, entry);
      setPending({
        preview: result,
        // 更新模式：目标优先用调用方点明的；否则采信识别结果。自制战役入口由入口定死。
        slot: slot || (isCustomEntry ? "custom" : (result.slot ?? "")),
        mode: "rename",
      });
      setMountMods(true);
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setImporting(false);
    }
  }

  /** 选择压缩包并预检。 */
  async function pickAndPrepare(): Promise<void> {
    try {
      const path = await api.pickPackage();
      if (!path) return;
      await prepare(path);
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  // 外壳的按钮点了：拿一次文件。用 ref 记住上次的 nonce，首渲染不算。
  useEffect(() => {
    if (importNonce > 0) void pickAndPrepare();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [importNonce]);

  // 拖进来的包：交给同一套预检流程，处理完通知外壳清掉
  useEffect(() => {
    if (!droppedPackage) return;
    consumeDropped();
    void prepare(droppedPackage);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedPackage]);

  /** 补丁包不进战役：导入补丁库，之后到对应战役里挂载。 */
  async function confirmPatchImport(): Promise<void> {
    const current = pending;
    if (!current) return;
    setImporting(true);
    try {
      const patch = await api.importPatch(current.preview.path);
      setPending(null);
      notify("success", "补丁「" + patch.name + "」已进库，到对应战役里挂载即可");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setImporting(false);
    }
  }

  /** 真正执行导入（或更新）。 */
  async function doImport(): Promise<void> {
    const current = pending;
    if (!current || !current.slot) return;

    setImporting(true);
    try {
      // 更新：删旧的装新的，该跟着走的记录由后端带过去
      if (isUpdate) {
        const result = await api.updateVariantFromPackage(
          current.slot,
          variantId || null,
          current.preview.path,
        );
        setPending(null);
        refresh();
        notify("success", "已更新「" + result.variant.name + "」");

        // 新包带了新模组：默认已经挂上，问一句要不要留
        if (result.addedMods.length > 0) {
          setAddedMods(result.addedMods);
          setKeepAdded(true);
          setAddedTarget({ slot: current.slot, variantId: result.variant.id });
          return;
        }
        onImported(current.slot);
        return;
      }

      const created = await api.importPackageWith(current.preview.path, current.slot, current.mode);

      // 用户不想装模组 -> 把导入时默认挂上的清掉（只在索引里改）
      if (!mountMods && (created.mod_count ?? 0) > 0) {
        await api.setMountedMods(current.slot, created.id, []);
      }

      const target = current.slot;
      setPending(null);
      refresh();
      notify("success", "已导入「" + created.name + "」");
      onImported(target);
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setImporting(false);
    }
  }

  /** 确认导入。 */
  async function confirmImport(): Promise<void> {
    if (!pending || !pending.slot) return;
    // 高置信度却被强改 -> 先问一句，别默默装错。更新模式不适用。
    if (overrideWarning && !isUpdate) {
      setOverrideOpen(true);
      return;
    }
    await doImport();
  }

  /** 新模组那问的"确定"。 */
  async function confirmAddedMods(): Promise<void> {
    const target = addedTarget;
    const drop = !keepAdded;
    const list = addedMods;
    setAddedMods([]);
    setAddedTarget(null);
    if (!target || !drop) return;

    // 取消勾选 = 把这几个从挂载名单里摘掉（其余的保持原样）
    try {
      const rows = await api.variantMods(target.slot, target.variantId);
      const keep = rows.filter((row) => row.mounted && !list.includes(row.path)).map((row) => row.path);
      await api.setMountedMods(target.slot, target.variantId, keep);
      refresh();
      notify("success", "已取消这几个新模组的挂载");
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  return (
    <div className="campaigns-import">
      {/* 导入预览 */}
      {pending && (
        <section className="import">
          <div className="import__head">
            <div>
              <div className="import__title">
                {isUpdate ? "将更新：" : "将导入："}
                {pendingName}
                {inspection?.version && <span className="tag">v{inspection.version}</span>}
                <span className={"tag" + (isPatch ? " tag--patch" : "")}>
                  {isPatch ? "补丁包" : "战役包"}
                </span>
              </div>
              <div className="import__meta">
                {inspection?.map_count ?? 0} 张地图 · {formatBytes(inspection?.unpacked_bytes ?? 0)}
                {inspection?.author && <> · {inspection.author}</>}
              </div>
            </div>

            {/* 来源与依据。**自制战役入口不显示这些** —— 归属是用户自己定的。 */}
            {isCustomEntry ? (
              <div className="import__target">
                <div className="import__source">按自制战役导入</div>
                <div className="import__evidence">这一页导入的都算自制战役，不看包属于哪部原版战役</div>
              </div>
            ) : (
              <div className="import__target">
                <div className="import__source">{sourceLabel}</div>
                {evidenceText && <div className="import__evidence">{evidenceText}</div>}
              </div>
            )}
          </div>

          {/* 补丁包走另一条路 */}
          {isPatch ? (
            <p className="import__note">
              这是一个<strong>补丁包</strong>，它不归属任何战役。导入后到对应战役的菜单里挂载即可，
              可以同时挂多个并调整优先级。
            </p>
          ) : (
            <>
              {/* 自制战役入口 / 更新：归属已定，不显示选择器 */}
              {isCustomEntry || isUpdate ? (
                <div className="import__field">
                  <span className="import__label">{isUpdate ? "更新：" : "导入到："}</span>
                  <span className="import__fixed">{isUpdate ? chosenName : "自制战役"}</span>
                </div>
              ) : (
                <div className="import__field">
                  <span className="import__label">导入到：</span>
                  <div className="targets">
                    {slots.map((item) => (
                      <button
                        key={item.slug}
                        className={"target" + (pending.slot === item.slug ? " target--on" : "")}
                        type="button"
                        onClick={() => setPending({ ...pending, slot: item.slug })}
                      >
                        {item.display_name}
                      </button>
                    ))}
                  </div>
                  {needsTarget && (
                    <span className="import__ask">自动识别没能判断出归属，请手动选择</span>
                  )}
                </div>
              )}

              {/* 高置信度却被改到别处：先说清后果，但不禁止 */}
              {overrideWarning && (
                <div className="warn">
                  这个包<strong>明确</strong>属于「{identifiedName}」，你把它改到了「{chosenName}」。
                  装错地方通常会让战役里找不到关卡，确认前请想一下。
                </div>
              )}

              {/* 带模组的包：问一句要不要一起装 */}
              {packageMods > 0 && !isUpdate && (
                <label className="check">
                  <input
                    type="checkbox"
                    checked={mountMods}
                    onChange={(event) => setMountMods(event.target.checked)}
                  />
                  <span>
                    一起挂载这 <strong>{packageMods}</strong> 个模组
                    <em>（地图需要它们才能打开，建议勾上；之后可以到战役页面里改）</em>
                  </span>
                </label>
              )}

              {/* 冲突：覆盖更新 or 重命名后导入（更新模式不问） */}
              {preview?.conflict && !isUpdate && (
                <div className="conflict">
                  <div className="conflict__text">{conflictText}</div>
                  <div className="targets">
                    <button
                      className={"target" + (pending.mode === "overwrite" ? " target--on" : "")}
                      type="button"
                      onClick={() => setPending({ ...pending, mode: "overwrite" })}
                    >
                      覆盖更新
                    </button>
                    <button
                      className={"target" + (pending.mode === "rename" ? " target--on" : "")}
                      type="button"
                      onClick={() => setPending({ ...pending, mode: "rename" })}
                    >
                      重命名后导入（两者并存）
                    </button>
                  </div>
                </div>
              )}
            </>
          )}

          {shownIssues.length > 0 && (
            <ul className="issues">
              {shownIssues.map((issue) => (
                <li key={issue.code} className={"issue issue--" + issue.level}>
                  {issue.message}
                </li>
              ))}
            </ul>
          )}

          <div className="import__actions">
            {isUpdate && (
              <p className="import__note">
                更新会<strong>先删掉这一版、再装进新的</strong>；它正启用着的话，会先切回原版、装完再应用回去。
                存档档案与补丁绑定挂在战役上，不受影响。
              </p>
            )}
            <Button onClick={() => setPending(null)}>取消</Button>
            {isPatch ? (
              <Button
                variant="primary"
                disabled={importing || !inspection?.installable}
                onClick={() => void confirmPatchImport()}
              >
                {importing ? "导入中…" : "导入补丁库"}
              </Button>
            ) : (
              <Button
                variant="primary"
                disabled={importing || !pending.slot || !inspection?.installable}
                onClick={() => void confirmImport()}
              >
                {importing ? "处理中…" : isUpdate ? "确认更新" : "确认导入"}
              </Button>
            )}
          </div>
        </section>
      )}

      {/* 更新完：新包里多出来的模组 */}
      {addedMods.length > 0 && (
        <SheetFrame title={"新包里多了 " + addedMods.length + " 个模组"} onClose={() => void confirmAddedMods()}>
          <p className="sheet__text">
            已经按默认<strong>一起启用</strong>了。不想要的取消勾选即可 ——
            取消只影响这几个，其余保持原样。
          </p>
          <ul className="mini">
            {addedMods.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
          <label className="check">
            <input
              type="checkbox"
              checked={keepAdded}
              onChange={(event) => setKeepAdded(event.target.checked)}
            />
            <span>一起启用这几个新模组</span>
          </label>
          <div className="sheet__actions">
            <Button variant="primary" onClick={() => void confirmAddedMods()}>
              知道了
            </Button>
          </div>
        </SheetFrame>
      )}

      {/* 强改归属的二次确认 */}
      {overrideOpen && (
        <SheetFrame title={"确定改到「" + chosenName + "」吗？"} onClose={() => setOverrideOpen(false)}>
          <div className="sheet__badge sheet__badge--warn">
            <svg className="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3.5 2.5 20h19L12 3.5Z" />
              <path d="M12 10v4.5" />
              <path d="M12 17.4v.1" />
            </svg>
          </div>
          <p className="sheet__text">
            这个包属于 <strong>{identifiedName}</strong>。
            <br />
            <br />
            装到别的战役里，地图会落在游戏找不到的位置 ——
            <strong>多半进游戏后找不到关卡，玩不了</strong>。
          </p>
          <div className="sheet__actions">
            <Button
              onClick={() => {
                setOverrideOpen(false);
                void doImport();
              }}
            >
              我知道，仍然导入
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                if (pending && preview?.slot) setPending({ ...pending, slot: preview.slot });
                setOverrideOpen(false);
              }}
            >
              改回 {identifiedName}
            </Button>
          </div>
        </SheetFrame>
      )}
    </div>
  );
}

/**
 * 这两处弹窗原来的结构与 base.css 的 `.sheet` 一样，但壳里还有预警徽标，
 * 用 `Sheet` 控件会把它挤到标题下面 —— 结构照旧，只把标记收在一处。
 */
function SheetFrame({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div className="sheet" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="sheet__card">
        <h3 className="sheet__title">{title}</h3>
        {children}
      </div>
    </div>
  );
}