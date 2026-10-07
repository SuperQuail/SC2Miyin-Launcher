# 第三方素材说明

本目录下的美术素材**不属于本项目代码的授权范围**，请勿单独再许可或用于其它项目。

## 1. 战役美术（`campaigns/` 与 `backdrop.jpg`）

| 文件 | 内容 |
| --- | --- |
| `campaigns/wol.jpg` | 《星际争霸 II：自由之翼》标题图 |
| `campaigns/hots.jpg` | 《星际争霸 II：虫群之心》标题图 |
| `campaigns/lotv.jpg` | 《星际争霸 II：虚空之遗》主视觉 |
| `campaigns/lotv-prologue.jpg` | 《虚空之遗》序章美术 |
| `campaigns/nco.jpg` | 《诺娃隐秘行动》美术 |
| `campaigns/sc-remastered.jpg` | 《星际争霸：重制版》美术 |
| `campaigns/sc2-logo.png` | 《星际争霸 II》标识 |
| `backdrop.jpg` | 《星际争霸 II》主视觉（用作应用背景） |

**版权归属**：以上均为 Blizzard Entertainment, Inc. 的版权作品与商标。

**使用方式**：仅用于**标识性使用**（nominative use）—— 让用户在本启动器中一眼认出
某个自制战役归属于哪一部资料片。启动器不销售、不再分发这些素材，也不声称对其拥有权利。

**注意**：原始下载时未记录逐条的来源 URL 与授权条款（`assets-staging/` 未入库）。
若要公开发布到 GitHub 或其它渠道，请先自行评估 Blizzard 的素材使用条款；
如无法接受该风险，建议替换为自制插画或纯色/渐变占位图
（只需替换 `ui/public/campaigns/*` 并重跑 `scripts/prepare-assets.py`）。

## 2. 弥音立绘（`miyin/`）

| 文件 | 用途 |
| --- | --- |
| `miyin/wink.png` | 封面看板娘（首页看板） |
| `miyin/chibi.png` | 品牌头像、安装对话框 |
| `miyin/portrait.png` | 「关于」立绘，同时用于生成应用图标 |
| `miyin/cry.png` | 空状态插画 |

由项目作者提供，用于「弥音启动器」自身的形象。项目正式选择许可证时，
应同时明确这组立绘的授权方式（建议在仓库根目录的 `LICENSE` 之外单独声明）。

## 3. 素材的再生成

```bash
python scripts/prepare-assets.py   # 压缩为 Web 尺寸，输出到 ui/public/
python scripts/prepare-icons.py    # 由立绘生成 src-tauri/icons/
```

原始素材放在 `assets-staging/`（已加入 `.gitignore`，不入库）。
