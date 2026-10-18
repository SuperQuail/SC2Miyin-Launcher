import { useEffect, useRef, useState } from "react";

import { slotArt } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { ImportMode, ImportPreview, SlotView } from "../../../api/types";
import { Button, Choice, Field, Progress } from "../../ui";
import "./install.css";

/** 一个待安装的包（一行 = 单包那张卡片 / 清单里的一行）。 */
interface Row {
  /** 行自己的编号 —— 同一个文件被添加两次也算两行，别拿 path 当身份。 */
  id: number;
  path: string;
  /** 预检结果；null = 还在预检（或者预检本身就失败了）。 */
  preview: ImportPreview | null;
  /** 预检失败的原因。 */
  error: string | null;
  /** 装到哪部战役（可改；认不出归属的包就靠它）。 */
  target: string;
  /** 自己这一份的结果 —— 一个失败不影响别的包。 */
  state: "idle" | "ok" | "failed";
  failure: string | null;
}

/** 文件名（路径的最后一段），预检失败时当名字用。 */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** 归属判定的可信度档位（AGENTS §14.7：包内声明 = 确定；镜像路径 / 地图依赖 = 较可靠；文件名前缀 = 猜的）。 */
type Tier = "exact" | "reliable" | "guess" | null;

function tierOf(preview: ImportPreview | null): Tier {
  if (!preview) return null;
  const evidence = preview.inspection.identification?.evidence ?? null;
  if (preview.source === "metadata" || evidence === "metadata") return "exact";
  return evidence === "map_name_prefix" ? "guess" : "reliable";
}

function tierText(tier: Tier): string {
  return tier === "exact"
    ? "确定（包内自己声明）"
    : tier === "reliable"
      ? "较可靠（镜像路径 / 地图依赖）"
      : tier === "guess"
        ? "猜的（文件名前缀）"
        : "";
}

/**
 * 安装整合包（独立一页）。
 *
 * 一次可以拖好几个包，**两种形态共用同一套预检与安装逻辑**（`install()` 一个入口）：
 * - 只有 1 个包：居中那张卡片 —— 封面 + 左标签右值的属性表，和以前一样；
 * - 2 个及以上：清单，每行一个包、各自选归属，底下「全部安装」逐行跑，一个失败不中止整批。
 *
 * **归属判定的可信度**两处都照旧摆明（AGENTS §14.7）—— 不许把推断说成确定的。
 */
export function InstallView({
  entry = "campaign",
  slots,
  droppedPackages,
  consumeDropped,
  notify,
  onBack,
  onInstalled,
}: {
  /** 战役包还是自制战役包 —— 交给后端预检时按哪套规则认归属。 */
  entry?: "campaign" | "custom";
  slots: SlotView[];
  /** 拖进来的包（可能是好几个）。 */
  droppedPackages: string[];
  consumeDropped: () => void;
  notify: (kind: "info" | "success" | "warning" | "error", message: string) => void;
  onBack: () => void;
  onInstalled: (slot: string) => void;
}) {
  const [rows, setRows] = useState<Row[]>([]);
  /**
   * 行的编号来源（同一个文件可以加两次）。
   *
   * 用 ref 而不是 state：同一拍里连加两个包（连点两次「添加包」）时，
   * state 里那个值还没更新，两行会拿到同一个编号 —— 于是 patch() 一次改两行。
   */
  const rowSeq = useRef(0);
  const [mode, setMode] = useState<ImportMode>("overwrite");
  const [checking, setChecking] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [busy, setBusy] = useState(false);

  /** 新的一行。 */
  function makeRow(path: string): Row {
    rowSeq.current += 1;
    return { id: rowSeq.current, path, preview: null, error: null, target: "", state: "idle", failure: null };
  }

  /** 补一行 / 改一行。 */
  function patch(id: number, changes: Partial<Row>): void {
    setRows((current) => current.map((row) => (row.id === id ? { ...row, ...changes } : row)));
  }

  /** 预检一个包（不写任何文件）。 */
  async function inspect(row: Row): Promise<void> {
    try {
      const preview = await api.prepareImport(row.path, entry);
      patch(row.id, { preview, target: preview.slot ?? "" });
    } catch (error) {
      patch(row.id, { error: errorText(error) });
    }
  }

  /** 逐个预检；一个包认不出来不影响别的包。 */
  async function inspectAll(paths: string[]): Promise<void> {
    const fresh = paths.map((path) => makeRow(path));
    setRows(fresh);
    setMode("overwrite");
    setChecking(true);
    try {
      for (const row of fresh) await inspect(row);
    } finally {
      setChecking(false);
    }
  }

  // 拖进来的包：全部预检，然后把它们从外壳的暂存里取走（免得切页又预检一遍）
  useEffect(() => {
    if (droppedPackages.length === 0) return;
    consumeDropped();
    void inspectAll(droppedPackages);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedPackages]);

  /** 再添一个包（文件对话框一次只给一个；拖进来才能一次多个）。 */
  async function add(): Promise<void> {
    const path = await api.pickPackage();
    if (!path) return;
    const row = makeRow(path);
    setRows((current) => [...current, row]);
    setChecking(true);
    try {
      await inspect(row);
    } finally {
      setChecking(false);
    }
  }

  function remove(id: number): void {
    setRows((current) => current.filter((row) => row.id !== id));
  }

  /** 能装的那些行：预检成功、还没装过。 */
  const installable = rows.filter((row) => row.preview !== null && row.state !== "ok");
  const anyConflict = rows.some((row) => row.preview?.conflict);

  /**
   * 安装 —— **两种形态唯一的入口**。
   *
   * **每份独立成败**：一个包失败只记在它自己那一行，后面的照装 ——
   * 拖进来一批却因为第三个包坏了就全停下，是最烦人的那种。
   */
  async function install(): Promise<void> {
    const queue = rows.filter((row) => row.state !== "ok");
    if (queue.length === 0) {
      notify("warning", "先选一个整合包");
      return;
    }

    setBusy(true);
    setProgress({ done: 0, total: queue.length });
    const failures: string[] = [];
    let installed = 0;
    let lastSlot = "";
    let lastName = "";

    try {
      for (let index = 0; index < queue.length; index += 1) {
        const row = queue[index];
        const name = row.preview?.inspection.name ?? fileName(row.path);
        setProgress({ done: index, total: queue.length });

        if (!row.preview) {
          const why = row.error ?? "预检没通过";
          failures.push(name + "：" + why);
          patch(row.id, { state: "failed", failure: why });
          continue;
        }
        if (!row.target) {
          const why = "还没选装到哪部战役";
          failures.push(name + "：" + why);
          patch(row.id, { state: "failed", failure: why });
          continue;
        }

        try {
          await api.importPackageWith(row.path, row.target, mode);
          installed += 1;
          lastSlot = row.target;
          lastName = name;
          patch(row.id, { state: "ok", failure: null });
        } catch (error) {
          const why = errorText(error);
          failures.push(name + "：" + why);
          patch(row.id, { state: "failed", failure: why });
        }
        setProgress({ done: index + 1, total: queue.length });
      }
    } finally {
      setBusy(false);
      setProgress(null);
    }

    if (failures.length === 0) {
      notify("success", installed === 1 ? "已安装：" + lastName : "已安装 " + installed + " 个包");
    } else if (installed === 0) {
      notify("error", "一个都没装上：" + failures.join("；"));
    } else {
      notify("warning", "已安装 " + installed + " 个，失败 " + failures.length + " 个：" + failures.join("；"));
    }

    // 全成功才回来源页（老行为）；有失败就留在这一页，用户能看见哪一份红了再重试
    if (installed > 0 && failures.length === 0) onInstalled(lastSlot);
  }

  const single = rows.length === 1 ? rows[0] : null;

  return (
    <div className="install">
      <header className="install__head">
        <Button variant="outline" onClick={onBack}>
          ← 返回
        </Button>
        <h2 className="install__title">安装整合包</h2>
        <Button variant="text" onClick={onBack}>
          关闭
        </Button>
      </header>

      {rows.length === 0 && (
        <button className="install__drop" type="button" onClick={() => void add()} disabled={busy}>
          <span className="install__drop-title">把整合包拖到这里</span>
          <span className="install__drop-hint">
            可以一次拖好几个，一起装；也可以点这里选文件（zip / 7z / rar 都行）—— 这一步只检查，不写任何文件
          </span>
        </button>
      )}

      {/* 只有一个包：居中属性表（用户选的「A · 对齐 HMCL」那版） */}
      {single && (
        <SingleCard
          row={single}
          slots={slots}
          mode={mode}
          busy={busy}
          checking={checking}
          installable={installable.length > 0}
          onMode={setMode}
          onTarget={(target) => patch(single.id, { target })}
          onReplace={() => remove(single.id)}
          onInstall={() => void install()}
        />
      )}

      {/* 两个及以上：清单 */}
      {rows.length >= 2 && (
        <section className="install__card">
          <div className="install__rows">
            <div className="install__list-head">
              <span className="install__list-title">
                待安装
                <span className="install__tag">{rows.length}</span>
              </span>
              {checking && <span className="install__hint">正在逐个预检…</span>}
            </div>

            {anyConflict && <ConflictChoice mode={mode} onMode={setMode} batch />}

            <div className="install__list">
              {rows.map((row) => (
                <BatchRow
                  key={row.id}
                  row={row}
                  slots={slots}
                  busy={busy}
                  onTarget={(target) => patch(row.id, { target })}
                  onRemove={() => remove(row.id)}
                />
              ))}
            </div>
          </div>

          <footer className="install__foot">
            <div className="install__foot-left">
              <Button variant="text" disabled={busy} onClick={() => void add()}>
                ＋ 添加包
              </Button>
              {progress && (
                <Progress
                  value={progress.done}
                  max={progress.total}
                  label="正在安装"
                  detail={progress.done + " / " + progress.total}
                />
              )}
            </div>
            <Button
              variant="primary"
              size="lg"
              loading={busy}
              disabled={installable.length === 0}
              onClick={() => void install()}
            >
              全部安装（{installable.length}）
            </Button>
          </footer>
        </section>
      )}
    </div>
  );
}

/** 冲突怎么处理（单包那张表里、清单顶上共用同一条）。 */
function ConflictChoice({
  mode,
  onMode,
  batch = false,
}: {
  mode: ImportMode;
  onMode: (mode: ImportMode) => void;
  /** 清单上写「整批」，单包里不用。 */
  batch?: boolean;
}) {
  return (
    <div className="install__conflict">
      <Choice<ImportMode>
        label={batch ? "库里已经有同名的版本，整批按哪种方式处理？" : "库里已经有同名的版本"}
        value={mode}
        onChange={onMode}
        options={[
          { value: "overwrite", label: "覆盖更新", hint: "沿用原目录名，旧版被替换掉" },
          { value: "rename", label: "重命名后导入", hint: "两个版本并存，各自启用" },
        ]}
      />
    </div>
  );
}

/** 归属下拉（两种形态共用）。 */
function TargetSelect({
  row,
  slots,
  busy,
  onTarget,
}: {
  row: Row;
  slots: SlotView[];
  busy: boolean;
  onTarget: (target: string) => void;
}) {
  const name = row.preview?.inspection.name ?? fileName(row.path);
  return (
    <select
      className="install__row-select"
      value={row.target}
      disabled={busy || row.state === "ok"}
      onChange={(event) => onTarget(event.target.value)}
      aria-label={"装到哪部战役：" + name}
    >
      <option value="">（选一部战役）</option>
      {slots.map((slot) => (
        <option key={slot.slug} value={slot.slug}>
          {slot.display_name}
        </option>
      ))}
    </select>
  );
}

/** 只有一个包时那张卡片：封面 + 居中的属性表。 */
function SingleCard({
  row,
  slots,
  mode,
  busy,
  checking,
  installable,
  onMode,
  onTarget,
  onReplace,
  onInstall,
}: {
  row: Row;
  slots: SlotView[];
  mode: ImportMode;
  busy: boolean;
  checking: boolean;
  installable: boolean;
  onMode: (mode: ImportMode) => void;
  onTarget: (target: string) => void;
  onReplace: () => void;
  onInstall: () => void;
}) {
  const inspection = row.preview?.inspection ?? null;
  const tier = tierOf(row.preview);
  const conflict = row.preview?.conflict ?? null;
  const cover = inspection
    ? (inspection.cover ?? slotArt(row.preview?.slot ?? row.target ?? "wol"))
    : null;
  const slotName = row.target
    ? (slots.find((slot) => slot.slug === row.target)?.display_name ?? row.target)
    : null;

  return (
    <section className="install__card">
      {cover && (
        <div className="install__cover" style={{ backgroundImage: "url(" + cover + ")" }}>
          <span className="install__cover-scrim" />
        </div>
      )}

      <div className="install__rows">
        {!inspection ? (
          row.error ? (
            <p className="install__row-bad">✗ 读不了这个包：{row.error}</p>
          ) : (
            <p className="install__hint">{checking ? "正在预检…" : "正在读这个包…"}</p>
          )
        ) : (
          <>
            <Field label="游戏实例名称" row hint="装到哪部战役名下；决定包里的地图落到哪个战役目录">
              <TargetSelect row={row} slots={slots} busy={busy} onTarget={onTarget} />
            </Field>

            <Field label="整合包名称" row>
              <span className="install__value">{inspection.name ?? fileName(row.path)}</span>
            </Field>

            <Field label="版本" row>
              <span className="install__value">
                {inspection.version ?? "（包内没写）"}
                <span className="install__tag">{inspection.kind === "patch" ? "补丁包" : "战役包"}</span>
              </span>
            </Field>

            <Field label="作者" row>
              <span className="install__value">{inspection.author ?? "未知"}</span>
            </Field>

            <Field label="归属战役" row>
              <span className="install__value">
                {slotName ?? "认不出来，上面自己选一个"}
                {tier && <span className="install__tag">{tier === "exact" ? "确定" : "推断"}</span>}
              </span>
            </Field>

            {inspection.identification && (
              <p className="install__why">
                可信度：{tierText(tier)} —— 依据：{inspection.identification.detail}。
                {tier !== "exact" && "这不是确定的，认错了把上面那行改掉就行。"}
              </p>
            )}

            <Field label="包里有什么" row>
              <span className="install__value">
                {inspection.map_count} 张地图 · {inspection.mod_count} 个模组
                {inspection.mod_count > 0 && (
                  <span className="install__hint">（模组是地图的依赖，安装时会一起挂上）</span>
                )}
              </span>
            </Field>

            {conflict && (
              <>
                <p className="install__why">
                  库里已有「{conflict.existing_name}
                  {conflict.existing_version ?? ""}」，这一份是 {conflict.incoming_version ?? "未标版本"}。
                </p>
                <ConflictChoice mode={mode} onMode={onMode} />
              </>
            )}
          </>
        )}

        {row.state === "ok" && <span className="install__row-good">✓ 已安装</span>}
        {row.state === "failed" && <span className="install__row-bad">✗ 失败：{row.failure}</span>}
      </div>

      <footer className="install__foot">
        <Button variant="text" disabled={busy} onClick={onReplace}>
          换一个包
        </Button>
        <Button variant="primary" size="lg" loading={busy} disabled={!installable} onClick={onInstall}>
          安装
        </Button>
      </footer>
    </section>
  );
}

/** 清单里的一行。 */
function BatchRow({
  row,
  slots,
  busy,
  onTarget,
  onRemove,
}: {
  row: Row;
  slots: SlotView[];
  busy: boolean;
  onTarget: (target: string) => void;
  onRemove: () => void;
}) {
  const inspection = row.preview?.inspection ?? null;
  const tier = tierOf(row.preview);
  const tierLabel = tierText(tier);
  const conflict = row.preview?.conflict ?? null;
  const slotName = row.target ? (slots.find((slot) => slot.slug === row.target)?.display_name ?? row.target) : null;

  return (
    <div className={"install__row" + (row.state === "ok" ? " install__row--ok" : "") + (row.state === "failed" ? " install__row--bad" : "")}>
      <div className="install__row-main">
        <span className="install__row-name">
          {inspection?.name ?? fileName(row.path)}
          {inspection && <span className="install__tag">{inspection.kind === "patch" ? "补丁包" : "战役包"}</span>}
          {conflict && <span className="install__tag install__tag--warn">冲突</span>}
        </span>

        {row.error ? (
          <span className="install__row-bad">✗ 读不了这个包：{row.error}</span>
        ) : !row.preview ? (
          <span className="install__row-meta">正在预检…</span>
        ) : (
          <>
            <span className="install__row-meta">
              {inspection?.version ?? "（包内没写版本）"} · {inspection?.author ?? "未知作者"} ·{" "}
              {inspection?.map_count ?? 0} 张地图 · {inspection?.mod_count ?? 0} 个模组
            </span>
            <span className="install__row-meta">
              归属：{slotName ?? "认不出来 —— 右边自己选一部"}
              {tierLabel && <span className="install__tag">{tier === "exact" ? "确定" : "推断"}</span>}
            </span>
            {inspection?.identification && (
              <span className="install__row-why">
                可信度：{tierLabel} —— 依据：{inspection.identification.detail}。
                {tier !== "exact" && "这不是确定的，认错了把右边的归属改掉就行。"}
              </span>
            )}
            {conflict && (
              <span className="install__row-why">
                库里已有「{conflict.existing_name}
                {conflict.existing_version ?? ""}」，这一份是 {conflict.incoming_version ?? "未标版本"}。
              </span>
            )}
          </>
        )}

        {row.state === "ok" && <span className="install__row-good">✓ 已安装</span>}
        {row.state === "failed" && <span className="install__row-bad">✗ 失败：{row.failure}</span>}
      </div>

      <div className="install__row-side">
        <TargetSelect row={row} slots={slots} busy={busy} onTarget={onTarget} />
        {row.state !== "ok" && (
          <Button variant="text" disabled={busy} onClick={onRemove}>
            移除
          </Button>
        )}
      </div>
    </div>
  );
}
