import { useEffect, useMemo, useState, type MouseEvent as ReactMouseEvent } from "react";

import { formatBytes } from "../../../api/art";
import { api } from "../../../api/bridge";
import { errorText } from "../../../api/errors";
import type { DocInfo, MainMapChoice, MapEntry, ModEntry, Variant } from "../../../api/types";
import { copyText } from "../../lib/clipboard";
import type { CampaignMenuItem } from "./host";
import "./custom-campaign.css";

/**
 * 自制战役的操作面板（React 版，从 `components/CustomCampaignPanel.vue` 整块迁过来）。
 *
 * 自制战役会整包装进 `Maps/CustomCampaigns/<战役目录>/`，模组进 `Mods/`。
 * **想卸下来就点「停用」** —— 只撤我们记过账的文件。
 */
export function CustomCampaignPanel({
  slot,
  variant,
  active,
  refresh,
  notify,
  showMenu,
  onOpenDoc,
}: {
  slot: string;
  variant: Variant;
  /** 这个版本是不是当前启用的那个。 */
  active: boolean;
  /** 动过库之后让外壳重新读一遍。 */
  refresh: () => void;
  notify: (kind: "info" | "success" | "error", text: string) => void;
  showMenu: (event: ReactMouseEvent, items: CampaignMenuItem[], onPick: (id: string) => void) => void;
  onOpenDoc: (doc: DocInfo) => void;
}) {
  const [maps, setMaps] = useState<MapEntry[]>([]);
  const [mods, setMods] = useState<ModEntry[]>([]);
  const [choice, setChoice] = useState<MainMapChoice | null>(null);
  const [doc, setDoc] = useState<DocInfo | null>(null);
  /** 包自带的封面；没有就退回渐变，不硬塞一张官方美术（那会张冠李戴）。 */
  const [cover, setCover] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [keyword, setKeyword] = useState("");
  /** 折叠起来的章节；默认全展开。 */
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  /** 没挂模组就点启动时的警告。 */
  const [showMountWarning, setShowMountWarning] = useState(false);

  /** 读这个版本的地图 / 模组 / 主地图 / 说明。 */
  async function load(): Promise<void> {
    setLoading(true);
    try {
      const [list, modList, picked, found] = await Promise.all([
        api.variantMaps(slot, variant.id),
        api.variantMods(slot, variant.id),
        api.mainMapChoice(slot, variant.id),
        api.variantDoc(slot, variant.id),
      ]);
      setMaps(list);
      setMods(modList);
      setChoice(picked);
      setDoc(found);
      setCollapsed(new Set());

      // 封面单独取：取不到不算错误，界面会退回渐变
      try {
        setCover(await api.variantCover(slot, variant.id));
      } catch {
        setCover(null);
      }
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // 换版本就重读（原来是 watch(() => props.variant.id, load)）
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slot, variant.id]);

  /** 搜索过滤：按路径或名字模糊匹配。 */
  const filtered = useMemo(() => {
    const needle = keyword.trim().toLowerCase();
    if (!needle) return maps;
    return maps.filter(
      (map) => map.name.toLowerCase().includes(needle) || map.path.toLowerCase().includes(needle),
    );
  }, [maps, keyword]);

  /**
   * 分组：**跟着包的实际结构走**，不假设「章节」这个概念。
   * 有第一层目录就按它分组（标题直接用目录名，不翻译）；全平铺就用一组。
   */
  const groups = useMemo(() => {
    if (!filtered.some((map) => map.chapter)) return [{ title: null as string | null, maps: filtered }];

    const buckets = new Map<string, MapEntry[]>();
    for (const map of filtered) {
      const key = map.chapter ?? "（未分组）";
      const bucket = buckets.get(key);
      if (bucket) bucket.push(map);
      else buckets.set(key, [map]);
    }
    return [...buckets.entries()].map(([title, entries]) => ({ title, maps: entries }));
  }, [filtered]);

  const mountedCount = mods.filter((item) => item.mounted).length;
  const hasMods = mods.length > 0;
  const mainMap = choice?.path ?? null;

  function toggleGroup(title: string): void {
    const next = new Set(collapsed);
    if (next.has(title)) next.delete(title);
    else next.add(title);
    setCollapsed(next);
  }

  /** 启用 / 停用这个版本（停用就是把装进去的东西撤回来）。 */
  async function toggleActive(): Promise<void> {
    setBusy(true);
    try {
      await api.activateVariant(slot, active ? null : variant.id);
      refresh();
      notify("success", active ? "已停用，装进去的文件都撤回了" : "已启用");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(false);
    }
  }

  /** 勾 / 取消一个模组。 */
  async function toggleMod(entry: ModEntry): Promise<void> {
    const next = mods
      .map((item) => (item.path === entry.path ? !item.mounted : item.mounted))
      .map((mounted, index) => (mounted ? mods[index].path : null))
      .filter((path): path is string => path !== null);

    setBusy(true);
    try {
      await api.setMountedMods(slot, variant.id, next);
      setMods(mods.map((item) => ({ ...item, mounted: next.includes(item.path) })));
      notify("success", next.length ? "已挂载 " + next.length + " 个模组" : "已全部取消挂载");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(false);
    }
  }

  /** 把某张地图设为主地图。 */
  async function markAsMain(map: MapEntry): Promise<void> {
    setBusy(true);
    try {
      await api.setMainMap(slot, variant.id, map.path);
      await load();
      notify("success", "已把「" + map.name + "」设为主地图");
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(false);
    }
  }

  /** 复制地图在库里的绝对路径（走共用的那一份剪贴板实现）。 */
  async function copyMapPath(map: MapEntry): Promise<void> {
    const text = map.path;
    const ok = await copyText(text);
    notify(ok ? "success" : "info", ok ? "已复制：" + text : text);
  }

  /** 启动：没挂模组先警告，然后直接启动星际争霸进这张图（不经过编辑器）。 */
  async function launch(map?: string): Promise<void> {
    const target = map ?? mainMap;
    if (!target) {
      notify("info", "还没定主地图 —— 在地图列表里挑一张，点「设为主地图」");
      return;
    }

    // 带了模组却一个都没挂：地图的依赖找不到，进游戏就是一堆丢失的资源
    if (hasMods && mountedCount === 0) {
      setShowMountWarning(true);
      return;
    }

    setBusy(true);
    try {
      const result = await api.launchGameWithMap(slot, variant.id, target);
      refresh();
      notify("success", result.guidance);
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(false);
    }
  }

  /** 收到警告后仍然要试。 */
  async function launchAnyway(): Promise<void> {
    setShowMountWarning(false);
    const target = mainMap;
    if (!target) return;

    setBusy(true);
    try {
      const result = await api.launchGameWithMap(slot, variant.id, target);
      notify("success", result.guidance);
    } catch (error) {
      notify("error", errorText(error));
    } finally {
      setBusy(false);
    }
  }

  /** 地图行上的右键菜单：不用先点「设为主地图」再点「启动」那么绕。 */
  function showMapMenu(event: ReactMouseEvent, map: MapEntry): void {
    showMenu(
      event,
      [
        { id: "open", label: "用编辑器打开" },
        { id: "main", label: "设为主地图", disabled: map.is_main },
        { id: "copy", label: "复制地图路径", separatorBefore: true },
      ],
      (id) => {
        if (id === "open") void launch(map.path);
        if (id === "main") void markAsMain(map);
        if (id === "copy") void copyMapPath(map);
      },
    );
  }

  return (
    <section className="campaigns-custom">
      <div className="custom">
        {/*
          背景用**包自带的封面**。没有就退回渐变 —— 不拿官方美术顶替，
          否则一个自制战役配着虚空之遗的图，纯属张冠李戴。
        */}
        <header
          className="banner"
          style={cover ? { backgroundImage: "url(" + cover + ")" } : undefined}
        >
          <div className="banner__scrim" />
          <div className="banner__body">
            <h3 className="banner__title">{variant.name}</h3>
            <p className="banner__sub">
              {active
                ? "已装进游戏目录，点右边那个绿色的可以停用卸下。"
                : "还没装进游戏目录 —— 点右边那个把它装进去，就能直接启动游戏。"}
              {maps.length > 0 && <>共 {maps.length} 张地图。</>}
            </p>
          </div>
          <div className="banner__actions">
            {doc && (
              <button className="btn btn-text banner__btn" type="button" onClick={() => onOpenDoc(doc)}>
                查看说明
              </button>
            )}
            {/*
              **这个按钮显示的是"现在的状态"，不是"点了会发生什么"** ——
              绿=开着、灰=关着；动作写在 title 里。
            */}
            <button
              className={"btn btn-text banner__btn banner__state " + (active ? "is-on" : "is-off")}
              type="button"
              disabled={busy || loading}
              title={active ? "点击停用：把装进游戏目录的文件撤回" : "点击启用：装进游戏目录"}
              onClick={() => void toggleActive()}
            >
              {active ? "已启用" : "已停用"}
            </button>
            <button
              className="btn btn-primary banner__btn"
              type="button"
              disabled={busy || loading}
              onClick={() => void launch()}
            >
              启动
            </button>
          </div>
        </header>

        {mainMap ? (
          <p className="custom__main">
            主地图：<code>{mainMap}</code>
            {choice?.automatic && <span className="custom__auto">（这个版本只有一张，自动选的）</span>}
          </p>
        ) : choice?.warning ? (
          <p className="custom__warn">{choice.warning}</p>
        ) : (
          <p className="custom__warn">
            还没定主地图。挑一张点「设为主地图」，之后「启动」按钮就打开它。
          </p>
        )}

        {/* 挂载模组 */}
        {hasMods && (
          <div className="mount">
            <div className="mount__head">
              <span className="mount__title">挂载模组</span>
              <span className="mount__count">
                已挂 {mountedCount} / {mods.length}
              </span>
            </div>
            <p className="mount__hint">
              地图需要这些模组才能正常打开。不同战役的模组可能互相冲突，按需勾选即可；
              <strong>一个都不挂的话，地图打开会报错</strong>。
            </p>
            <ul className="mount__list">
              {mods.map((entry) => (
                <li key={entry.path} className="mod">
                  <label className="mod__label">
                    <input
                      type="checkbox"
                      checked={entry.mounted}
                      disabled={busy}
                      onChange={() => void toggleMod(entry)}
                    />
                    <span className="mod__name">{entry.name}</span>
                  </label>
                  <code className="mod__path">{entry.path}</code>
                </li>
              ))}
            </ul>
          </div>
        )}

        {/* 地图列表 */}
        <div className="maplist">
          <div className="maplist__head">
            <span className="maplist__title">地图</span>
            <input
              className="maplist__search"
              type="search"
              value={keyword}
              placeholder="搜索地图，例如 Terran、Zerg"
              onChange={(event) => setKeyword(event.target.value)}
            />
            <span className="maplist__count">
              {filtered.length} / {maps.length}
            </span>
          </div>

          {loading ? (
            <p className="maplist__empty">正在读取库里地图…</p>
          ) : maps.length === 0 ? (
            <p className="maplist__empty">这个版本里没有 .SC2Map 地图。</p>
          ) : filtered.length === 0 ? (
            <p className="maplist__empty">没有匹配的地图。</p>
          ) : null}

          {groups.map((group) => (
            <div key={group.title ?? "__flat__"} className="group">
              {group.title && (
                <button className="group__head" type="button" onClick={() => toggleGroup(group.title as string)}>
                  <span className="group__arrow">{collapsed.has(group.title) ? "▸" : "▾"}</span>
                  <span className="group__name">{group.title}</span>
                  <span className="group__count">{group.maps.length}</span>
                </button>
              )}

              {(!group.title || !collapsed.has(group.title)) && (
                <ul className="group__list">
                  {group.maps.map((map) => (
                    <li
                      key={map.path}
                      className={"map" + (map.is_main ? " map--main" : "")}
                      onContextMenu={(event) => showMapMenu(event, map)}
                    >
                      <div className="map__info">
                        <span className="map__name">{map.name}</span>
                        {map.is_main && <span className="map__badge">主地图</span>}
                        <span className="map__size">{formatBytes(map.size)}</span>
                      </div>
                      <div className="map__actions">
                        <button
                          className="btn btn-text btn--tiny"
                          type="button"
                          disabled={busy}
                          onClick={() => void markAsMain(map)}
                        >
                          设为主地图
                        </button>
                        <button
                          className="btn btn-tonal btn--tiny"
                          type="button"
                          disabled={busy}
                          onClick={() => void launch(map.path)}
                        >
                          启动这张
                        </button>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </div>
          ))}
        </div>
      </div>

      {/* 没挂模组就点启动 */}
      {showMountWarning && (
        <div className="sheet" onClick={(event) => event.target === event.currentTarget && setShowMountWarning(false)}>
          <div className="sheet__card">
            <div className="sheet__badge sheet__badge--warn">
              <svg className="sheet__icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 3.5 2.5 20h19L12 3.5Z" />
                <path d="M12 10v4.5" />
                <path d="M12 17.4v.1" />
              </svg>
            </div>
            <h3 className="sheet__title">一个模组都没挂</h3>
            <p className="sheet__text">
              这个战役带了 <strong>{mods.length}</strong> 个模组，而你一个都没勾。
              <br />
              <br />
              地图需要它们才能正常打开，现在直接启动，
              <strong>多半会报错或者出现紫色方块</strong>。
            </p>
            <div className="sheet__actions">
              <button className="btn btn-text" type="button" onClick={() => setShowMountWarning(false)}>
                我去挂上
              </button>
              <button className="btn btn-primary" type="button" onClick={() => void launchAnyway()}>
                还是打开
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}