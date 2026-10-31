import { useCallback, useEffect, useState } from "react";

import { formatBytes } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { ToolRelease, ToolStatus } from "../../../api/types";
import { Button } from "../../ui";

/**
 * 可选外部工具（React 版，从 `components/ToolsPanel.vue` 整块迁过来）。
 *
 * 这些不是启动器的必需件 —— 不装照样能用，只是少了某项能力。所以装不装、装哪个版本
 * 都由用户定，**但安装本身要静默**：点了就下、下完就位，不弹浏览器、也不让用户自己解压。
 */
export function ToolsPanel({ notify }: { notify: (kind: "success" | "error", text: string) => void }) {
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState<string | null>(null);
  /** 展开版本列表的那个工具 id。 */
  const [picking, setPicking] = useState<string | null>(null);
  const [releases, setReleases] = useState<Record<string, ToolRelease[]>>({});

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setTools(await api.listTools());
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setLoading(false);
    }
  }, [notify]);

  useEffect(() => {
    void load();
  }, [load]);

  /** 装最新版（或指定版本）。 */
  async function install(id: string, version: string | null): Promise<void> {
    setBusy(id);
    try {
      const updated = await api.installTool(id, version);
      setTools((current) => current.map((item) => (item.id === id ? updated : item)));
      setPicking(null);
      notify("success", updated.name + " " + (updated.version ?? "") + " 已安装");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(null);
    }
  }

  async function openSevenZip(): Promise<void> {
    try {
      await api.openSevenZip();
    } catch (error) {
      notify("error", errorText(error));
    }
  }

  async function uninstall(id: string): Promise<void> {
    setBusy(id);
    try {
      await api.uninstallTool(id);
      await load();
      notify("success", "已卸载");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(null);
    }
  }

  /** 展开版本列表（第一次展开才去查）。 */
  async function toggleVersions(id: string): Promise<void> {
    if (picking === id) {
      setPicking(null);
      return;
    }
    setPicking(id);
    if (releases[id]) return;

    setBusy(id);
    try {
      setReleases((current) => ({ ...current, [id]: [] }));
      const found = await api.toolReleases(id);
      setReleases((current) => ({ ...current, [id]: found }));
    } catch (error) {
      notify("error", errorText(error));
      setPicking(null);
    } finally {
      setBusy(null);
    }
  }

  function versionLabel(release: ToolRelease): string {
    const date = release.published_at.slice(0, 10);
    return release.version + (release.prerelease ? "（预发行）" : "") + " · " + date + " · " + formatBytes(release.size);
  }

  return (
    <section className="card panel">
      <header className="panel__head">
        <h3 className="panel__title">可选工具</h3>
        <span className="panel__hint">不安装也可正常使用启动器</span>
      </header>

      <article className="tool">
        <div className="tool__head">
          <div className="tool__title">
            <span className="tool__name">7-Zip 解压工具</span>
            <span className="tag tag--ok">已内置</span>
          </div>
          <div className="tool__actions">
            <Button variant="primary" onClick={() => void openSevenZip()}>
              <span className="btn--tiny">打开 7-Zip</span>
            </Button>
          </div>
        </div>
        <p className="tool__about">
          导入 7z / rar 等压缩包时启动器会自动用它解压；也可以从这里打开 7-Zip 的完整界面，做解压、压缩、测试等操作。
        </p>
      </article>

      {loading && <p className="notes__empty">正在读取…</p>}

      {tools.map((tool) => (
        <article key={tool.id} className="tool">
          <div className="tool__head">
            <div className="tool__title">
              <span className="tool__name">{tool.name}</span>
              {tool.installed ? <span className="tag tag--ok">已安装 {tool.version}</span> : <span className="tag">未安装</span>}
            </div>
            <div className="tool__actions">
              <Button onClick={() => void api.openToolRepo(tool.id)}>
                <span className="btn--tiny">仓库</span>
              </Button>
              <Button disabled={busy !== null} onClick={() => void toggleVersions(tool.id)}>
                <span className="btn--tiny">选版本 ▾</span>
              </Button>
              <Button variant="primary" disabled={busy !== null} onClick={() => void install(tool.id, null)}>
                <span className="btn--tiny">
                  {busy === tool.id ? "处理中…" : tool.installed ? "更新" : "安装"}
                </span>
              </Button>
              {tool.installed && (
                <Button variant="danger" disabled={busy !== null} onClick={() => void uninstall(tool.id)}>
                  <span className="btn--tiny">卸载</span>
                </Button>
              )}
            </div>
          </div>

          <p className="tool__about">{tool.about}</p>

          {/* 自动装失败过：说清为什么之后不再自动重试 */}
          {tool.auto_failed && (
            <p className="tool__failed">
              上次自动安装失败{tool.auto_error ? "：" + tool.auto_error : ""}。
              <strong>后续启动不会再自动尝试</strong>；如需安装，请点上方「安装」。
            </p>
          )}
          {tool.installed && tool.path && (
            <p className="tool__path" title={tool.path}>
              {tool.path} · {formatBytes(tool.size_bytes)}
            </p>
          )}

          {/* 版本列表 */}
          {picking === tool.id && (
            <ul className="versions">
              {busy === tool.id ? (
                <li className="versions__empty">正在查询…</li>
              ) : !(releases[tool.id] ?? []).length ? (
                <li className="versions__empty">此工具暂无可安装版本</li>
              ) : (
                (releases[tool.id] ?? []).map((release) => (
                  <li key={release.tag} className="version">
                    <span className="version__label">{versionLabel(release)}</span>
                    <Button
                      disabled={busy !== null || !release.has_asset}
                      onClick={() => void install(tool.id, release.version)}
                    >
                      <span className="btn--tiny">{release.has_asset ? "安装此版本" : "此版本无附件"}</span>
                    </Button>
                  </li>
                ))
              )}
            </ul>
          )}
        </article>
      ))}
    </section>
  );
}
