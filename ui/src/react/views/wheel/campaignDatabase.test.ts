import { describe, expect, it } from "vitest";

import { parseDatabaseCampaign } from "./campaignDatabase";

describe("parseDatabaseCampaign", () => {
  it("从文件夹决定类型，并保留可点击的下载源", () => {
    const item = parseDatabaseCampaign("data/campaign-db/wol/foo.json", {
      title: "自由之翼：重生",
      type: "hots",
      authors: ["作者甲", "作者乙"],
      description: "描述",
      download_sources: ["https://pan.example.com/s/abc"],
      author_links: ["https://example.com/author"],
    });

    expect(item?.type).toBe("wol");
    expect(item?.typeLabel).toBe("自由之翼");
    expect(item?.authors).toEqual(["作者甲", "作者乙"]);
    expect(item?.download_sources).toEqual(["https://pan.example.com/s/abc"]);
  });

  it("缺标题或未知类型返回 null", () => {
    expect(parseDatabaseCampaign("data/campaign-db/wol/a.json", { type: "wol" })).toBeNull();
    expect(parseDatabaseCampaign("data/campaign-db/other/a.json", { title: "甲" })).toBeNull();
    expect(parseDatabaseCampaign("data/campaign-db/wol/a.json", null)).toBeNull();
  });
});
