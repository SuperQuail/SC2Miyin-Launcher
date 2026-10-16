import { useCallback, useEffect, useRef, useState } from "react";

import { formatBytes } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { NetworkSettings, Staged, UpdateCheck, UpdateProgress } from "../../../api/types";
import { Button } from "../../ui";

/**
 * 更新面板（React 版，从 `components/UpdatePanel.vue` 整块迁过来）：
 * 检查 / 下载 / 安装，外加一个**内嵌终端**告诉用户正在发生什么。
 *
 * 状态仍归外壳（顶栏的更新角标也要读同一份），这里通过 props 拿、通过 emit 触发动作。
 */
export interface UpdatePanelProps {
  notify: (kind: "success" | "error", text: string) => void;
  launcherVersion: string;
  check: UpdateCheck | null;
  staged: Staged | null;
  progress: UpdateProgress | null;
  logs: string[];
  checking: boolean;
  downloading: boolean;
  checkNow: () => void;
  downloadNow: () => void;
  applyNow: () => void;
}

export function UpdatePanel(props: UpdatePanelProps) {
  const { notify, check, staged, progress, logs, checking, downloading } = props;

  const [network, setNetwork] = useState<NetworkSettings>({
    proxy_mode: "auto",
    proxy_url: null,
    use_mirrors: true,
    include_prerelease: true,
  });
  const [proxy, setProxy] = useState<{ url: string; source: string } | null>(null);
  const [savingNetwork, setSavingNetwork] = useState(false);
  const [showNotes, setShowNotes] = useState(false);
  const logBox = useRef<HTMLDivElement | null>(null);
  const panelBox = useRef<HTMLElement | null>(null);

  useEffect(() => {
    void (async () => {
      try {
        setNetwork(await api.networkSettings());
        setProxy(await api.detectedProxy());
      } catch (error) {
        notify("error", errorText(error));
      }
    })();
  }, [notify]);

  // 从更新公告跳过来时自动开始下载 —— 把面板滚到视线里，
  // 否则用户只听到"在下载"却看不到进度条与终端
  useEffect(() => {
    if (!downloading) return;
    panelBox.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [downloading]);

  // 终端自动滚到底：用户进来看到的是最新一行，而不是最老的
  useEffect(() => {
    if (logBox.current) logBox.current.scrollTop = logBox.current.scrollHeight;
  }, [logs.length]);

  const latest = check?.latest ?? null;
  const notes = latest?.notes?.trim() ?? "";
  const percent = progress?.percent ?? 0;
  const hasPercent = progress?.percent != null;

  const networkLabel = check?.via ?? (proxy ? proxy.url + "（" + proxy.source + "）" : "直连");

  /** 进度文本；总长度未知时只说已下载多少。 */
  const progressText = progress
    ? progress.total
      ? formatBytes(progress.done) + " / " + formatBytes(progress.total)
      : formatBytes(progress.done)
    : "";

  const saveNetwork = useCallback(async () => {
    setSavingNetwork(true);
    try {
      await api.setNetworkSettings(network);
      setProxy(await api.detectedProxy());
      notify("success", "网络设置已保存");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setSavingNetwork(false);
    }
  }, [network, notify]);

  function openRelease(): void {
    const url = latest?.html_url || "https://github.com/SuperQuail/SC2Miyin-Launcher/releases";
    void api.openUrl(url).catch((error) => notify("error", errorText(error)));
  }

  return (
    <section ref={panelBox} className="panel">
      <header className="panel__head">
        <h4 className="panel__title">更新</h4>
        <span className="panel__version">当前 {props.launcherVersion || "…"}</span>
      </header>

      <p className="panel__note">
        启动时会<strong>自动检查</strong>一次更新，有新版本会在右上角显示下载标记。
        国内直连较慢时会自动并发试多个镜像；代理被限流时会自动改直连重试。
      </p>

      <div className="row">
        <Button variant="tonal" disabled={checking} onClick={props.checkNow}>
          {checking ? "检查中…" : "重新检查"}
        </Button>
        <Button onClick={openRelease}>打开发行页面</Button>
        <span className="row__hint">网络：{networkLabel}</span>
      </div>

      {latest && check?.available ? (
        <div className="release">
          <div className="release__head">
            <span className="release__version">{latest.version}</span>
            {latest.prerelease && <span className="tag tag--warn">预发行</span>}
            <span className="release__date">{latest.published_at.slice(0, 10)}</span>
          </div>

          {notes && (
            <div className={"release__notes" + (showNotes ? "" : " release__notes--folded")}>{notes}</div>
          )}
          {notes.length > 160 && (
            <Button onClick={() => setShowNotes((value) => !value)}>
              {showNotes ? "收起说明" : "展开说明"}
            </Button>
          )}

          {/* 进度条 */}
          {(downloading || progress) && (
            <div className="progress">
              <div className="progress__track">
                <div
                  className={"progress__fill" + (hasPercent ? "" : " progress__fill--unknown")}
                  style={hasPercent ? { width: percent + "%" } : undefined}
                />
              </div>
              <div className="progress__meta">
                {hasPercent && <span className="progress__percent">{Math.round(percent)}%</span>}
                <span className="progress__bytes">{progressText}</span>
              </div>
            </div>
          )}

          <div className="row">
            <Button variant="primary" disabled={downloading} onClick={props.downloadNow}>
              {downloading ? "下载中…" : staged ? "重新下载" : "下载"}
            </Button>
            <Button variant="primary" disabled={!staged} onClick={props.applyNow}>
              安装并重启
            </Button>
            {staged && <span className="row__hint">已就绪：{staged.version}</span>}
          </div>
        </div>
      ) : check && !check.error && !check.available ? (
        <p className="panel__ok">已经是最新版本。</p>
      ) : check?.error ? (
        <p className="panel__warn">{check.error}</p>
      ) : null}

      {/* 内嵌终端 */}
      <div className="console">
        <div className="console__head">
          <span className="console__title">更新日志</span>
          <span className="console__hint">这里能看到刚才到底做了什么</span>
        </div>
        <div ref={logBox} className="console__body">
          {logs.length === 0 ? (
            <p className="console__empty">还没有日志。启动时的自动检查跑完就会有内容。</p>
          ) : (
            logs.map((line, index) => (
              <p key={index} className="console__line">
                {line}
              </p>
            ))
          )}
        </div>
      </div>

      <hr className="divider" />

      <h5 className="panel__sub">网络与更新通道</h5>

      <label className="field">
        <span className="field__label">代理</span>
        <select
          className="field__input"
          value={network.proxy_mode}
          onChange={(event) =>
            setNetwork({ ...network, proxy_mode: event.target.value as NetworkSettings["proxy_mode"] })
          }
        >
          <option value="auto">自动（环境变量 → 系统代理 → 直连）</option>
          <option value="off">不使用代理</option>
          <option value="manual">手动指定</option>
        </select>
      </label>

      {network.proxy_mode === "manual" && (
        <label className="field">
          <span className="field__label">代理地址</span>
          <input
            className="field__input"
            type="text"
            placeholder="http://127.0.0.1:7897"
            value={network.proxy_url ?? ""}
            onChange={(event) => setNetwork({ ...network, proxy_url: event.target.value })}
          />
        </label>
      )}

      <label className="check">
        <input
          type="checkbox"
          checked={network.use_mirrors}
          onChange={(event) => setNetwork({ ...network, use_mirrors: event.target.checked })}
        />
        <span>下载时自动尝试 GitHub 镜像加速（国内推荐）</span>
      </label>

      <label className="check">
        <input
          type="checkbox"
          checked={network.include_prerelease}
          onChange={(event) => setNetwork({ ...network, include_prerelease: event.target.checked })}
        />
        <span>接收预发行版本（alpha / beta）</span>
      </label>

      <div className="row">
        <Button variant="tonal" disabled={savingNetwork} onClick={() => void saveNetwork()}>
          {savingNetwork ? "保存中…" : "保存网络设置"}
        </Button>
      </div>
    </section>
  );
}
