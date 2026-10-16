import { useRef, useState } from "react";

import { api } from "../../api/bridge";

/** 一个外部站点：给人看的名字与域名 + 两种协议的地址。 */
export interface SiteLink {
  name: string;
  /** 显示给人看的域名（中文域名就写中文）。 */
  host: string;
  https: string;
  http: string;
}

/**
 * 资源网站。
 *
 * 真正打开的是 **punycode** 那份：中文域名交给系统"打开链接"时各家编码处理不一致，
 * punycode 是纯 ASCII，谁都不会弄错；浏览器地址栏照样显示中文域名。
 */
export const RESOURCE_SITE: SiteLink = {
  name: "资源网站",
  host: "www.叽叽咕咕.fun",
  https: "https://www.xn--xpra07ba.fun",
  http: "http://www.xn--xpra07ba.fun",
};

export interface SiteLinksApi {
  /** 正在问的那个站点。null = 没在问。 */
  sitePending: SiteLink | null;
  /** 探出来的最终地址（HTTPS 还是 HTTP）。 */
  siteResolved: string;
  /** 还在探。 */
  siteProbing: boolean;
  /** 请求打开某个站点：先探协议，再弹确认框。 */
  requestSite: (site: SiteLink) => Promise<void>;
  /** 用户点头了：交给系统浏览器。 */
  confirmSite: () => void;
  cancelSite: () => void;
}

/**
 * 打开外部站点 —— **接口化**，连同那句提示一起。
 *
 * 两件事在这里定死：
 *
 * 1. **先问一句再跳** —— 会离开启动器，不该悄悄发生
 * 2. **HTTPS 优先，不通用 HTTP** —— 站点暂时没上 HTTPS 时照样能用，
 *    而提示里显示的是**真正会打开的那个地址**
 */
export function useSiteLinks(): SiteLinksApi {
  const [sitePending, setSitePending] = useState<SiteLink | null>(null);
  const [siteResolved, setSiteResolved] = useState("");
  const [siteProbing, setSiteProbing] = useState(false);
  /** 探过的结果记一下 —— 一个会话里没必要每次点都探一遍。 */
  const probed = useRef(new Map<string, string>());
  /** 探的过程中用户可能已经取消了；用 ref 认人，不靠 state 闭包。 */
  const asking = useRef<SiteLink | null>(null);

  async function requestSite(site: SiteLink): Promise<void> {
    asking.current = site;
    setSitePending(site);
    setSiteResolved(probed.current.get(site.https) ?? site.https);
    if (probed.current.has(site.https)) return;

    setSiteProbing(true);
    try {
      const url = await api.resolveSiteUrl(site.https, site.http);
      probed.current.set(site.https, url);
      if (asking.current?.https === site.https) setSiteResolved(url);
    } catch {
      // 探失败就用 HTTPS 那份试 —— 打不开是浏览器那边的事，这里不拦
      setSiteResolved(site.https);
    } finally {
      setSiteProbing(false);
    }
  }

  function confirmSite(): void {
    asking.current = null;
    setSitePending(null);
    if (siteResolved) void api.openUrl(siteResolved);
  }

  function cancelSite(): void {
    asking.current = null;
    setSitePending(null);
  }

  return { sitePending, siteResolved, siteProbing, requestSite, confirmSite, cancelSite };
}
