import { useEffect, useState, type ReactNode } from "react";

import { formatBytes } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { ImportMode, ImportPreview, PackageInspection } from "../../../api/types";
import { usePermissionGate } from "../../state/usePermission";
import { Button } from "../../ui";
import { PasswordSheet } from "./PasswordSheet";
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
  /** 提权闸门：导入 / 更新是写盘动作（预检那一步不拦）。 */
  const permission = usePermissionGate();

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

  /** 加密包用过的密码（没加密就是 null）。 */
  const [password, setPassword] = useState<string | null>(null);
  /** 密码框开着没有；`passwordWrong` 表示上一次填的不对。 */
  const [asking, setAsking] = useState(false);
  const [passwordWrong, setPasswordWrong] = useState(false);

  const inspection: PackageInspection | null = pending?.preview.inspection ?? null;
  const preview: ImportPreview | null = pending?.preview ?? null;
  const isPatch = inspection?.kind === "patch";

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
    return "未能识别";
  })();

  const identification = inspection?.identification ?? null;
  const evidenceText = identification?.detail ? "依据：" + identification.detail : "";

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
      newer: "较新版本",
      same: "相同版本",
      older: "较旧版本",
      unknown: "无法比较",
    };
    const incoming = conflict.incoming_version ?? "未标注版本";
    const existing = conflict.existing_version ?? "未标注版本";
    return (
      "战役库已有「" + conflict.existing_name + "」" + existing +
      "，本次导入 " + incoming + "（" + labels[conflict.relation] + "）"
    );
  })();

  /**
   * 预检一个包 —— 选文件和拖进来走的是同一条路。
   *
   * 加密包会带着已有密码问一次；后端说还缺（或密码错）就把密码框弹出来。
   */
  async function prepare(path: string, secret: string | null = password): Promise<void> {
    setImporting(true);
    try {
      const result = await api.prepareImport(path, entry, secret);
      setPending({
        preview: result,
        // 更新模式：目标优先用调用方点明的；否则采信识别结果。自制战役入口由入口定死。
        slot: slot || (isCustomEntry ? "custom" : (result.slot ?? "")),
        mode: "rename",
      });
      if (result.inspection.needs_password) {
        setPasswordWrong(secret !== null);
        setAsking(true);
      } else {
        setAsking(false);
      }
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setImporting(false);
    }
  }

  /** 用户填完了密码：记下来，带着它重新预检。 */
  async function applyPassword(secret: string): Promise<void> {
    const path = pending?.preview.path;
    if (!path) return;
    setPassword(secret);
    await prepare(path, secret);
  }

  /** 选择压缩包并预检。 */
  async function pickAndPrepare(): Promise<void> {
    try {
      const path = await api.pickPackage();
      if (!path) return;
      // 换了包就忘掉上一个包留下的密码：密码是跟着包走的
      setPassword(null);
      await prepare(path, null);
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
    setPassword(null);
    void prepare(droppedPackage, null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedPackage]);

  /** 补丁包不进战役：导入补丁库，之后到对应战役里挂载。 */
  async function confirmPatchImport(): Promise<void> {
    const current = pending;
    if (!current) return;
    setImporting(true);
    try {
      const patch = await permission.run(
        () => api.importPatch(current.preview.path, password),
        { op: "import_patch", args: { path: current.preview.path, password } },
      );
      setPending(null);
      notify(
        "success",
        patch
          ? "补丁「" + patch.name + "」已导入补丁库，到对应战役中启用即可"
          : "补丁已导入补丁库，到对应战役中启用即可",
      );
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
    const target = current.slot;

    setImporting(true);
    try {
      // 更新：删旧的装新的，该跟着走的记录由后端带过去
      if (isUpdate) {
        const result = await permission.run(
          () =>
            api.updateVariantFromPackage(
              target,
              variantId || null,
              current.preview.path,
              password,
            ),
          {
            op: "update_variant_from_package",
            args: {
              slot: target,
              variantId: variantId || null,
              path: current.preview.path,
              password,
            },
          },
        );
        setPending(null);
        refresh();
        // 走助手重放时拿不到结果，就退回一句不带名字的话
        notify("success", result ? "已更新「" + result.variant.name + "」" : "已更新");

        // 模组一律由启动器自动管理，这里只提一句新包多出来的
        if (result && result.addedMods.length > 0) {
          notify("info", "新包包含 " + result.addedMods.length + " 个新模组，启动器会自动管理");
        }
        onImported(target);
        return;
      }

      const created = await permission.run(
        () => api.importPackageWith(current.preview.path, target, current.mode, password),
        {
          op: "import_package",
          args: {
            path: current.preview.path,
            slot: target,
            mode: current.mode,
            password,
          },
        },
      );

      setPending(null);
      refresh();
      notify("success", created ? "已导入「" + created.name + "」" : "已导入");
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
                <div className="import__evidence">此页导入的包都会归为自制战役</div>
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
              这是<strong>补丁包</strong>，不归属任何战役。导入后到对应战役菜单中启用；
              可同时启用多个并调整优先级。
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
                    <span className="import__ask">自动识别未能判断归属，请手动选择</span>
                  )}
                </div>
              )}

              {/* 高置信度却被改到别处：先说清后果，但不禁止 */}
              {overrideWarning && (
                <div className="warn">
                  此包<strong>明确</strong>属于「{identifiedName}」，当前选择为「{chosenName}」。
                  选择错误可能导致战役内找不到关卡。
                </div>
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
                      重命名后导入（两个版本并存）
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
                更新将<strong>先删除此版本、再安装新版本</strong>；若该版本正在启用，会先切回原版，
                安装完成后再启用新版本。存档与补丁绑定挂在战役上，不受影响。
              </p>
            )}
            <Button onClick={() => setPending(null)}>取消</Button>
            {/* 关了密码框之后还能再打开 —— 不然这一版就只能取消重来 */}
            {inspection?.needs_password && (
              <Button onClick={() => setAsking(true)}>输入密码</Button>
            )}
            {isPatch ? (
              <Button
                variant="primary"
                disabled={importing || !inspection?.installable}
                onClick={() => void confirmPatchImport()}
              >
                {importing ? "导入中…" : "导入到补丁库"}
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

      {/* 加密包：预检回 needs_password 时弹这里，填完重新预检 */}
      {asking && pending && (
        <PasswordSheet
          source={pending.preview.path.split(/[\\/]/).pop() ?? "这个包"}
          wrong={passwordWrong}
          onCancel={() => setAsking(false)}
          onSubmit={applyPassword}
        />
      )}

      {/* 强改归属的二次确认 */}
      {overrideOpen && (
        <SheetFrame title={"确认改为「" + chosenName + "」？"} onClose={() => setOverrideOpen(false)}>
          <div className="sheet__badge sheet__badge--warn">
            <svg className="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
              <path d="M12 3.5 2.5 20h19L12 3.5Z" />
              <path d="M12 10v4.5" />
              <path d="M12 17.4v.1" />
            </svg>
          </div>
          <p className="sheet__text">
            此包属于 <strong>{identifiedName}</strong>。
            <br />
            <br />
            导入到其他战役会使地图落在游戏无法读取的位置，
            <strong>进入游戏后可能找不到地图</strong>。
          </p>
          <div className="sheet__actions">
            <Button
              onClick={() => {
                setOverrideOpen(false);
                void doImport();
              }}
            >
              仍然导入
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