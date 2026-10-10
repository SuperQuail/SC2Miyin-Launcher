/**
 * 打开外部站点 —— **接口化**，连同那句提示一起。
 *
 * 用法：任何页面里 `requestSite(SOME_SITE)`，弹窗由 App.vue 统一渲染
 * （状态在模块级，不在某个组件里）。之后再加站点，只往下面那张表里排一行。
 *
 * 两件事在这里定死：
 *
 * 1. **先问一句再跳** —— 会离开启动器，不该悄悄发生
 * 2. **HTTPS 优先，不通用 HTTP** —— 站点暂时没上 HTTPS 时照样能用，
 *    而提示里显示的是**真正会打开的那个地址**
 */
import { ref } from "vue";

import { api } from "../api/bridge";

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

/** 正在问的那个站点。null = 没在问。 */
const sitePending = ref<SiteLink | null>(null);
/** 探出来的最终地址（HTTPS 还是 HTTP）。 */
const siteResolved = ref("");
/** 还在探。 */
const siteProbing = ref(false);

/** 探过的结果记一下 —— 一个会话里没必要每次点都探一遍。 */
const probed = new Map<string, string>();

export function useSiteLinks() {
  /** 请求打开某个站点：先探协议，再弹确认框。 */
  async function requestSite(site: SiteLink): Promise<void> {
    sitePending.value = site;
    siteResolved.value = probed.get(site.https) ?? site.https;
    if (probed.has(site.https)) return;

    siteProbing.value = true;
    try {
      const url = await api.resolveSiteUrl(site.https, site.http);
      probed.set(site.https, url);
      // 探的过程中用户可能已经取消了
      if (sitePending.value?.https === site.https) siteResolved.value = url;
    } catch {
      // 探失败就用 HTTS 那份试 —— 打不开是浏览器那边的事，这里不拦
      siteResolved.value = site.https;
    } finally {
      siteProbing.value = false;
    }
  }

  /** 用户点头了：交给系统浏览器。 */
  function confirmSite(): void {
    const url = siteResolved.value;
    sitePending.value = null;
    if (url) void api.openUrl(url);
  }

  function cancelSite(): void {
    sitePending.value = null;
  }

  return { sitePending, siteResolved, siteProbing, requestSite, confirmSite, cancelSite };
}
