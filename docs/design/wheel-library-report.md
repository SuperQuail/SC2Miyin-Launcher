# 大转盘 · 库选型报告（2026-11-13）

> 后续（2026-11-13）：试过 lucky-canvas 后仍决定不用库，改为自绘，见 [wheel.md](wheel.md)。
>
> 结论先行：**按「≥500★ 且最近一个月内有提交」这条线，没有任何专用转盘库合格。**
> 唯一同时满足两条的 "wheel" 仓库是 React 滚轮选择器，不是抽奖转盘。
> 建议自绘 SVG/CSS 轮盘；如果必须用库，只能在下面 §4 的候选中放宽条件。

## 1. 口径

- 查询时间：2026-11-13；「一个月内」= 仓库 `pushed_at ≥ 2026-10-13`。
- ★ 与 `pushed_at` 来自 GitHub REST API 快照；`pushed_at` 是最近一次 push，不是 release 时间。
- 许可必须与项目 MIT 兼容：MIT / Apache-2.0 / ISC / BSD；GPL / AGPL 直接排除。
- 使用场景：React 18 + WebView2，需要能自定义指针图片（最好圆心指针）、颜色跟设计令牌、可测。

## 2. 专用转盘库

| 库 | ★ | 最近提交 | 许可 | 自定义指针 | npm 月下载 | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| react-custom-roulette 1.4.1 | 364 | 2026-04-14 | MIT | ✅ `pointerProps.src` | 34,485 | 当前在用；两项都不达标 |
| spin-wheel 5.0.2 | 348 | 2025-07-17 | MIT | ✅ 指针完全自己画（`pointerAngle` + `overlayImage`/DOM） | 17,991 | 指针自由度最高，但两项都不达标 |
| lucky-canvas 1.7.27 | 8,785 | 2026-07-30 | Apache-2.0 | 中心按钮可配图片；指针图片需按文档再验 | 27,225 | 只差「一个月」；React 绑定 2022 年后未发版 |
| react-wheel-of-prizes 2.1.0 | 101 | 2026-09-03 | **GPL-3.0** | 未验 | 12,402 | 许可不兼容 + 星数/时效不达标 |
| wheelnav | 705 | 2021-03-18 | MIT | 无（是饼状菜单，不是抽奖盘） | 948 | 时效不达标，非 React |
| landluck/lucky_wheel | 656 | 2024-03-24 | 无 | — | — | Vue；时效不达标 |
| spin-wheel-game 1.5.1 | 16 | 2026-11-12 | MIT | 未明确 | 1,401 | 近一个月活跃，但社区太小 |
| react-prize-wheel 2.0.0 | 0–8 | 2026-04-10 | MIT | 有限 | 49 | 社区太小 |
| ncdai/react-wheel-picker | 781 | 2026-11-11 | MIT | 不适用 | — | **两条都达标，但它是滚轮选择器，不是抽奖转盘** |

补充搜索证据：GitHub 搜索 `roulette stars:>=500`、`wheel of fortune stars:>=500`、
`prize wheel stars:>=500` 均无结果；`wheel stars:>=500 pushed:>2026-10-13` 的前排
全是 Python wheel、鼠标滚轮、汽车轮、Android WheelView 等无关项目。

当前依赖 `react-custom-roulette` 的实际情况：364★、最后提交 2026-04-14、npm 最后发布 2023-04-20。
它支持 `pointerProps.src`（自定义指针图片），但按这份标准不能算可靠依赖。

## 3. 满足门槛的「底层库」（不是成品转盘）

如果愿意用「底层动画/渲染库 + 自己画轮盘」，这些库两条都达标：

| 库 | ★ | 最近提交 | 许可 | 能给什么 |
| --- | --- | --- | --- | --- |
| motion（原 framer-motion） | 35,101 | 2026-11-12 | MIT | 弹簧/缓动动画 |
| react-spring | 29,275 | 2026-11-01 | MIT | 弹簧动画 |
| pixijs | 48,691 | 2026-11-11 | MIT | Canvas/WebGL 渲染 |
| konva | 14,946 | 2026-11-10 | MIT（GitHub 识别为 NOASSERTION） | Canvas 场景图 |
| three.js | 118,265 | 2026-11-12 | MIT | 3D（对本需求过重） |
| d3 | 113,914 | 2026-11-03 | ISC | 弧线生成 + 过渡动画 |

它们都不直接提供轮盘、抽奖落点、指针；用了仍然要自己写核心逻辑。
对「8–20 个扇区、一次旋转」的转盘来说，引入这些库的收益很小。

## 4. 建议

### 方案 A（推荐）：自绘 SVG/CSS 轮盘，零新增依赖

- 扇区：SVG `<path>`（或 CSS `conic-gradient`），标签是 DOM/SVG 文字；
- 旋转：一个 `transform: rotate()` + CSS transition / Web Animations；
- 落点：自己算「中奖扇区中心 → 目标角度」，不再受库的 43° 常量限制；
- 指针：独立 `<img>`，既可以放圆心，也可以放顶部；自定义图片就是换 `src`；
- 测试：几何与落点纯函数 + jsdom DOM 断言，不需要 canvas mock；
- 代价：约 150–250 行代码 + 样式；比继续适配一个不达标库更可控。

现 demo 里的 `-43°` 对齐、canvas 文字不翻转、canvas 高分屏发糊，都会因为自绘而消失。

### 方案 B：必须用成品库，接受放宽时效

- 最接近的是 `lucky-canvas`（8,785★，Apache-2.0，最后提交 2026-07-30）：
  功能最全（大转盘/九宫格/老虎机），但差约 3.5 个月；React 绑定 `@lucky-canvas/react`
  最后发布 2022-09-29，React 18 兼容性要实测。
- `spin-wheel`（348★）：指针完全交给你画（`overlayImage`/DOM），理念最贴需求，但社区小、
  canvas 方案、最后提交 2025-07；适合「指针图片自定义优先」的场景。
- `react-custom-roulette`：只建议继续当原型，不建议作为最终依赖。

### 方案 C：必须「维护中的库」且不自绘

用 `motion` 或 `pixijs` 当底座 + 自己实现轮盘；对本需求属于过度设计。

## 5. 采用任何库之前的复核清单

1. 指针：图片入口、位置（圆心/边缘）、旋转对齐、是否支持透明背景；
2. 主题：颜色能否接 CSS 变量，还是只能传 JS 常量；
3. 测试：canvas 需要 mock，SVG/DOM 可以直接断言；
4. 许可：GPL/AGPL 排除；bundle 内第三方代码的许可也要一起算；
5. 体积：npm unpacked 与真实 bundle gzip 是两回事；
6. 升级：锁版本，升级后复验落点与指针角度。

## 6. 数据来源

- GitHub REST：`/search/repositories`（star 排序、`pushed:>2026-10-13`、`stars:>=500`）与 `/repos/{owner}/{repo}`；
- npm registry `/latest`、`api.npmjs.org/downloads/point/last-month`；
- 查询式包括：`spinning wheel`、`wheel of fortune`、`roulette`、`lucky wheel`、`prize wheel`、
  `wheel in:name`、`roulette in:name`、`spin wheel in:name`、`fortune wheel in:name`、`抽奖`、`转盘` 等。

## 7. 局限

- GitHub 搜索只覆盖名称/描述/topic 等，可能漏掉没打 wheel 关键词的轮盘实现；
- `pushed_at` 可能来自文档/CI 提交，不等于功能更新；
- ★ 是快照，会变；这份报告只代表 2026-11-13 的查询结果；
- 本报告不改代码，也不对候选库做安全审计。