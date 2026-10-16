import type { LibraryMod, ModComparison, ModPreview, StandaloneMod } from "../../../api/types";
import { formatBytes } from "../../../api/art";
import { Button, Field, Sheet } from "../../ui";

/**
 * 模组页的三个弹窗。
 *
 * 从 `views/ModsView.vue` 里搬出来 —— 原来它们和列表、工具条挤在一个 1312 行的
 * 文件里。现在每个弹窗自己一个组件，页面那边只负责"开哪个"。
 */

/** 同 modid：问一句作为新版本还是独立改版。 */
export function ChooseModeSheet({
  choosing,
  onClose,
  onChoose,
}: {
  choosing: { path: string; preview: ModPreview } | null;
  onClose: () => void;
  onChoose: (mode: "version" | "separate") => void;
}) {
  if (!choosing) return null;
  return (
    <Sheet
      title={`「${choosing.preview.name}」要怎么放？`}
      onClose={onClose}
      actions={
        <>
          <Button onClick={() => onChoose("separate")}>作为独立改版</Button>
          <Button variant="primary" onClick={() => onChoose("version")}>
            作为新版本
          </Button>
        </>
      }
    >
      {choosing.preview.mod_count > 1 && (
        <p className="sheet__text">
          这一包里带了 <strong>{choosing.preview.mod_count}</strong> 个模组，
          下面说的是其中<strong>第一个</strong>（<code>{choosing.preview.folder}</code>）。
          其余会按各自的名字分别导入。
        </p>
      )}
      <p className="sheet__text">
        库里已经有 <strong>{choosing.preview.existing.length}</strong> 个同为{" "}
        <code>{choosing.preview.modid}</code> 的模组：
      </p>
      <ul className="existing">
        {choosing.preview.existing.map((item) => (
          <li key={item.id} className="existing__row">
            <span className="existing__name">{item.name}</span>
            <span className="existing__ver">v{item.version ?? "未标版本"}</span>
          </li>
        ))}
      </ul>
      <p className="sheet__text">
        <strong>作为新版本</strong>：归到同一个模组下面，之后可以自由切换用哪一版。
        <br />
        <strong>作为独立改版</strong>：单独显示一个 —— 适用于「拿别人的模组改的」，
        它跟原版是两码事。
      </p>
    </Sheet>
  );
}

/** 删除确认：自绘的，不用 window.confirm。 */
export function RemoveSheet({
  removing,
  onClose,
  onConfirm,
}: {
  removing: LibraryMod | null;
  onClose: () => void;
  onConfirm: () => void;
}) {
  if (!removing) return null;
  return (
    <Sheet
      title={`删除「${removing.name}」？`}
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose}>取消</Button>
          <Button variant="primary" onClick={onConfirm}>
            删除
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        文件会从库里移除。如果它已经铺进游戏 <code>Mods/</code> 目录，
        <strong>那边也会一起撤掉</strong>；你自己手动放的模组不受影响。
      </p>
    </Sheet>
  );
}

/** 编辑模组信息（独立导入的、以及战役包带来的都能改）。 */
export function EditModSheet({
  editing,
  form,
  onChange,
  onClose,
  onSave,
  saving,
}: {
  editing: LibraryMod | StandaloneMod | null;
  form: { name: string; author: string; version: string; description: string };
  onChange: (next: { name: string; author: string; version: string; description: string }) => void;
  onClose: () => void;
  onSave: () => void;
  saving: boolean;
}) {
  if (!editing) return null;
  return (
    <Sheet
      title={`编辑「${editing.name}」`}
      onClose={onClose}
      actions={
        <>
          <Button onClick={onClose}>取消</Button>
          <Button variant="primary" disabled={saving} onClick={onSave}>
            保存
          </Button>
        </>
      }
    >
      <p className="sheet__text">
        只改启动器记录的信息，<strong>不动模组文件</strong>，随时可以改回来。
      </p>
      <div className="fields">
        <Field label="名称">
          <input value={form.name} type="text" onChange={(e) => onChange({ ...form, name: e.target.value })} />
        </Field>
        <Field label="作者">
          <input
            value={form.author}
            type="text"
            placeholder="可不填"
            onChange={(e) => onChange({ ...form, author: e.target.value })}
          />
        </Field>
        <Field label="版本">
          <input
            value={form.version}
            type="text"
            placeholder="例如 1.4"
            onChange={(e) => onChange({ ...form, version: e.target.value })}
          />
        </Field>
        <Field label="说明">
          <textarea
            value={form.description}
            rows={3}
            onChange={(e) => onChange({ ...form, description: e.target.value })}
          />
        </Field>
      </div>
    </Sheet>
  );
}

/** 对比结果。 */
export function CompareSheet({
  comparing,
  onClose,
}: {
  comparing: ModComparison | null;
  onClose: () => void;
}) {
  if (!comparing) return null;
  const changed = comparing.files.filter((file) => file.status !== "same");
  return (
    <Sheet
      className="sheet__card--wide"
      title={
        <>
          {comparing.before.name} v{comparing.before.version ?? "未标"} → v
          {comparing.after.version ?? "未标"}
        </>
      }
      onClose={onClose}
      actions={<Button onClick={onClose}>关闭</Button>}
    >
      {comparing.identical ? (
        <p className="sheet__text">
          两个版本内容<strong>完全一样</strong>。
        </p>
      ) : (
        <>
          <p className="sheet__text">
            共 <strong>{changed.length}</strong> 个文件有差异（总 {comparing.files.length} 个）。
          </p>
          <ul className="diff">
            {changed.map((file) => (
              <li key={file.path} className={"diff__row diff__row--" + file.status}>
                <span className="diff__tag">{file.status}</span>
                <span className="diff__path">{file.path}</span>
              </li>
            ))}
          </ul>
          {comparing.semantic.map((item) => (
            <details key={item.path} className="semantic">
              <summary className="semantic__head">语义 diff：{item.path}</summary>
              <pre className="semantic__body">{item.text}</pre>
            </details>
          ))}
          {comparing.semantic_note && <p className="sheet__note">{comparing.semantic_note}</p>}
        </>
      )}
    </Sheet>
  );
}

/** 游戏目录里实际放着什么（只读列表）。 */
export function GameModsSection({ entries }: { entries: { name: string; display: string; expanded: boolean; size_bytes: number }[] }) {
  return (
    <section className="game">
      <header className="game__head">
        <h3 className="game__title">游戏目录里的模组</h3>
        <span className="game__count">{entries.length} 个</span>
      </header>
      <p className="game__hint">
        这是 <code>Mods/</code> 里实际存在的文件 —— 包括你自己放的、别的工具留下的。
        上面启用模组后会立刻铺进来。
      </p>
      {entries.length === 0 ? (
        <p className="game__empty">游戏目录里还没有模组。</p>
      ) : (
        <ul className="mods">
          {entries.map((entry) => (
            <li key={entry.name} className="mod mod--ro">
              <div className="mod__info">
                <span className="mod__name">{entry.display}</span>
              </div>
              <div className="mod__actions">
                <span className="mod__path">
                  {entry.name}
                  {entry.expanded ? "（目录）" : ""}
                </span>
                <span className="mod__size">{formatBytes(entry.size_bytes)}</span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
