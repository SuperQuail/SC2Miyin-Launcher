import { useEffect, useState } from "react";

import { slotArt } from "../../../api/art";
import { api } from "../../../api/bridge";
import type { ImportMode, ImportPreview, SlotView } from "../../../api/types";
import { Button, Choice, Field } from "../../ui";
import "./install.css";

/**
 * 安装整合包（独立一页）。
 *
 * 布局照 HMCL 的「安装整合包」：居中一张属性表，左标签右值，底部左描述右主按钮。
 * 另外把**归属判定的可信度**摆明 —— 三档来自包内识别命中哪一层（AGENTS §14.7）：
 * 包内声明 = 确定；镜像路径 / 地图依赖 = 较可靠；文件名前缀 = 猜的。
 *
 * 拖压缩包、点「导入战役包」都走到这一页（见 useShellEffects 与 CampaignsView），
 * 所以人在哪个页签都不影响。
 */
export function InstallView({
  entry = "campaign",
  slots,
  droppedPackage,
  consumeDropped,
  notify,
  onBack,
  onInstalled,
}: {
  /** 战役包还是自制战役包 —— 交给后端预检时按哪套规则认归属。 */
  entry?: "campaign" | "custom";
  slots: SlotView[];
  droppedPackage: string | null;
  consumeDropped: () => void;
  notify: (kind: "info" | "success" | "warning" | "error", message: string) => void;
  onBack: () => void;
  onInstalled: (slot: string) => void;
}) {
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [target, setTarget] = useState<string>("");
  const [mode, setMode] = useState<ImportMode>("overwrite");
  const [busy, setBusy] = useState(false);

  /** 预检一个包（不写任何文件）—— 拖进来的和点选的都是这条路。 */
  async function inspect(path: string): Promise<void> {
    setBusy(true);
    try {
      const result = await api.prepareImport(path, entry);
      setPreview(result);
      setTarget(result.slot ?? "");
      setMode("overwrite");
    } catch (error) {
      notify("error", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  // 拖进来的包：预检一次，然后把它从外壳的暂存里取走（免得切页又预检一遍）
  useEffect(() => {
    if (!droppedPackage) return;
    consumeDropped();
    void inspect(droppedPackage);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [droppedPackage]);

  async function pick(): Promise<void> {
    const path = await api.pickPackage();
    if (path) await inspect(path);
  }

  async function install(): Promise<void> {
    if (!preview || !target) {
      notify("warning", "先选一个整合包，并指定装到哪部战役");
      return;
    }
    setBusy(true);
    try {
      await api.importPackageWith(preview.path, target, mode);
      notify("success", "已安装：" + (preview.inspection.name ?? "这个包"));
      onInstalled(target);
    } catch (error) {
      notify("error", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }

  const inspection = preview?.inspection ?? null;
  const identification = inspection?.identification ?? null;
  const evidence = identification?.evidence ?? null;
  const tier: "exact" | "reliable" | "guess" | null =
    preview === null
      ? null
      : preview.source === "metadata" || evidence === "metadata"
        ? "exact"
        : evidence === "map_name_prefix"
          ? "guess"
          : "reliable";
  const tierText =
    tier === "exact" ? "确定（包内自己声明）" : tier === "reliable" ? "较可靠（镜像路径 / 地图依赖）" : tier === "guess" ? "猜的（文件名前缀）" : "";
  const conflict = preview?.conflict ?? null;
  const cover = inspection ? (inspection.cover ?? slotArt(preview?.slot ?? target ?? "wol")) : null;

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

      {!preview && (
        <button className="install__drop" type="button" onClick={() => void pick()} disabled={busy}>
          <span className="install__drop-title">把整合包拖到这里</span>
          <span className="install__drop-hint">或者点这里选一个文件（zip / 7z / rar 都行）—— 这一步只检查，不写任何文件</span>
        </button>
      )}

      {preview && inspection && (
        <section className="install__card">
          {cover && (
            <div className="install__cover" style={{ backgroundImage: "url(" + cover + ")" }}>
              <span className="install__cover-scrim" />
            </div>
          )}

          <div className="install__rows">
            <Field label="游戏实例名称" row hint="装到哪部战役名下；决定包里的地图落到哪个战役目录">
              <select value={target} onChange={(event) => setTarget(event.target.value)}>
                <option value="">（选一部战役）</option>
                {slots.map((slot) => (
                  <option key={slot.slug} value={slot.slug}>
                    {slot.display_name}
                  </option>
                ))}
              </select>
            </Field>

            <Field label="整合包名称" row>
              <span className="install__value">{inspection.name ?? preview.path.split(/[\\/]/).pop()}</span>
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
                {preview.slot
                  ? slots.find((slot) => slot.slug === preview.slot)?.display_name ?? preview.slot
                  : "认不出来，上面自己选一个"}
                {tierText && <span className="install__tag">{tier === "exact" ? "确定" : "推断"}</span>}
              </span>
            </Field>

            {identification && (
              <p className="install__why">
                可信度：{tierText} —— 依据：{identification.detail}。
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
              <div className="install__conflict">
                <Choice<ImportMode>
                  label={"库里已有「" + conflict.existing_name + "」" + (conflict.existing_version ?? "") + "，这一份是 " + (conflict.incoming_version ?? "未标版本")}
                  value={mode}
                  onChange={setMode}
                  options={[
                    { value: "overwrite", label: "覆盖更新", hint: "沿用原目录名，旧版被替换掉" },
                    { value: "rename", label: "重命名后导入", hint: "两个版本并存，各自启用" },
                  ]}
                />
              </div>
            )}
          </div>

          <footer className="install__foot">
            <Button variant="text" onClick={() => setPreview(null)}>
              换一个包
            </Button>
            <Button variant="primary" size="lg" loading={busy} onClick={() => void install()}>
              安装
            </Button>
          </footer>
        </section>
      )}
    </div>
  );
}