# 战役资料 JSON

> 仅约定数据结构，暂未接入运行时；转盘页的「已有战役 / 战役数据库」当前共用同一内容。

## 字段

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `title` | string | 是 | 战役标题，非空 |
| `type` | enum | 否 | `wol` / `hots` / `lotv` / `nova` / `custom`；进化归 `hots`，序章归 `lotv` |
| `authors` | string[] | 否 | 战役作者，允许空数组 |
| `description` | string | 否 | 战役描述，允许空字符串 |
| `download_sources` | string[] | 否 | 战役下载源，允许空数组 |
| `author_links` | string[] | 否 | 战役作者关联（作者主页、发布页等），允许空数组 |

除 `title` 外均可省略；显式给出空字符串 / 空数组同样合法。未列出的字段不要写。

## 示例

```json
{
  "title": "Starcraft Mass Recall",
  "type": "wol",
  "authors": ["SCMR Team"],
  "description": "自由之翼战役复刻。",
  "download_sources": ["https://example.com/scmr.zip"],
  "author_links": ["https://example.com/scmr"]
}
```

最小合法对象：

```json
{ "title": "边境之地" }
```