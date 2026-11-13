# 战役数据库数据区

一个 JSON 对应一个战役资料。目录名即战役类型：

- `wol/`：自由之翼（含资料片改版）
- `hots/`：虫群之心（含进化）
- `lotv/`：虚空之遗（含序章）
- `nova/`：诺娃隐秘行动
- `custom/`：自制战役

字段约定见仓库根目录的 `docs/campaign-profile.md`。除 `title` 外均可为空：

```json
{
  "title": "战役标题",
  "type": "wol",
  "authors": ["作者"],
  "description": "战役描述。",
  "download_sources": ["https://pan.example.com/s/xxxx"],
  "author_links": ["https://example.com/author"]
}
```

- 文件名任意，以 `.json` 结尾。
- `type` 字段可省略；实际类型以所在文件夹为准。
- 下载源与作者关联只接受 http/https 链接；点开走系统浏览器。
- 新增文件后重新构建前端即可被收录。
