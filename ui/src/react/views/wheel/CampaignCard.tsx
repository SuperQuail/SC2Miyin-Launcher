import { api } from "../../../api/bridge";
import { Button, Sheet } from "../../ui";
import type { DatabaseCampaign } from "./campaignDatabase";

function ExternalLink({ url }: { url: string }) {
  if (!/^https?:\/\//i.test(url)) return <span className="campaign-card__url">{url}</span>;
  return (
    <a
      className="campaign-card__link"
      href={url}
      target="_blank"
      rel="noreferrer"
      title={url}
      onClick={(event) => {
        event.preventDefault();
        void api.openUrl(url).catch(() => {});
      }}
    >
      {url}
    </a>
  );
}

/** 数据库抽取结果卡片：只显示资料本身，下载源点开走系统浏览器。 */
export function CampaignCard({ campaign, onClose }: { campaign: DatabaseCampaign; onClose: () => void }) {
  return (
    <Sheet
      title={campaign.title}
      onClose={onClose}
      actions={<Button onClick={onClose}>关闭</Button>}
    >
      <dl className="campaign-card">
        <dt>类型</dt>
        <dd>{campaign.typeLabel}</dd>
        {campaign.authors.length > 0 && (
          <>
            <dt>作者</dt>
            <dd>{campaign.authors.join("、")}</dd>
          </>
        )}
        {campaign.description && (
          <>
            <dt>描述</dt>
            <dd className="campaign-card__description">{campaign.description}</dd>
          </>
        )}
        {campaign.download_sources.length > 0 && (
          <>
            <dt>下载源</dt>
            <dd>
              {campaign.download_sources.map((url) => (
                <ExternalLink key={url} url={url} />
              ))}
            </dd>
          </>
        )}
        {campaign.author_links.length > 0 && (
          <>
            <dt>作者关联</dt>
            <dd>
              {campaign.author_links.map((url) => (
                <ExternalLink key={url} url={url} />
              ))}
            </dd>
          </>
        )}
      </dl>
    </Sheet>
  );
}
