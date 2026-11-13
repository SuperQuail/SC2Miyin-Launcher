# 大转盘（技术验证）

> 状态：技术验证页已接（顶栏「转盘」/ `?view=wheel`）。**抽奖池、启用、启动还没接**，
> 页面用 8 条示例数据。视觉定稿在接真实数据前按 AGENTS §17 出三版预览再定。

## 1. 选型结论

用 **react-custom-roulette 1.4.1**（MIT），不自绘轮盘。

| 候选 | 许可 | 体积 | 维护 | 结论 |
| --- | --- | --- | --- | --- |
| react-custom-roulette 1.4.1 | MIT | dist/bundle.js 63 KB | 364★，仓库 2026-04 仍有提交 | **选**：指定中奖项 + 停止回调 + 转速 + 配色 + 指针，API 正好覆盖需求 |
| spin-wheel 5.0.2 | MIT | ~807 KB | 2024-10 后未发版 | 不用：原生 canvas、命令式 API、无 React 绑定 |
| react-wheel-of-prizes 2.1.0 | GPL-3.0 | ~52 KB | 2026-09 发布 | 不用：许可与 MIT 分发不兼容（AGENTS §3.2） |
| @lucky-canvas/react 0.1.13 | Apache-2.0 | ~335 KB + 核心包 | React 绑定 2022 后未发版 | 不用：自带抽奖视觉体系，不跟设计令牌 |
| 自绘 SVG + CSS | 本项目 MIT | 0 B | — | 保留为库出问题时的退路 |

npm 数据 2026-11 实查；unpacked 体积与 bundle 体积是两回事（前者含类型与未压缩文件）。

## 2. 接法

- 页面：`ui/src/react/views/wheel/WheelView.tsx`；页签与 `?view=wheel` 在 `state/flags.ts` / `app/TopBar.tsx` / `app/AppShell.tsx`。
- 纯逻辑：`wheelData.ts`（抽样、截断、映射 WheelData）、`wheelPalette.ts`（读设计令牌，可注入测试）。
- 转速：`spinDuration = 0.5` → 11350ms × 0.5 ≈ 5.7 秒；系统「减少动态效果」时 0.01 ≈ 0.1 秒。
- 字体：固定 `fontFamily="sans-serif"`。库用 WEB_FONTS 白名单判断；传别的会走 `WebFont.load` 去 Google Fonts 拉字体（CSP 是 default-src self，会被拦，离线也没网）。
- 尺寸：库内部 canvas 是 900×900，根容器宽度 80vw / 最大 445。我们把根 div 撑满 445 的方框，最高 2 倍屏仍是 1:1，不需要按 DPR 缩。
- 主题：canvas 拿不到 CSS 变量，渲染时从 `--accent-soft` / `--surface-3` / `--on-surface` / `--outline` 读，`theme` 变化时重读。
- 指针：库自带的边缘指针 `display:none`；在转子外面放一个固定的圆心指针。
  默认是一张内置 SVG；页面上「换指针图片…」可以选本地图片（读成 data URL），
  代码里也可以传 `pointerSrc`（URL / data URL）。图片按轮盘方框 100% 放，
  约定「转轴在图片中心、箭头朝上、透明背景」。
  库把中奖扇区落在 43° 方向，所以 `.wheel-rotor` 整体转 -43° 把中奖扇区搬回 12 点 ——
  指针不转、轮盘转，中奖扇区最后正好停在圆心指针指向的方向。
- 标签：轮上只放 4 个字（canvas 一行），完整名字在候选列表与结果卡。

## 3. 已知限制

- 库不对下半圈文字翻转，部分标签是倒的；可读性靠结果卡与候选列表补。要彻底解决只能改库或换自绘。
- 轮盘整体带 -43° 偏转（扇区不从 12 点开始排列），这是把指针摆到正上方的代价；
  要「正装扇区 + 圆心指针」得改库的 `initialRotation` 或换自绘。
- canvas 内容对读屏不可见：结果用 `role="status" aria-live="polite"`，候选列表是 DOM。
- jsdom 没有 canvas 2D 上下文：组件测试 mock 掉库，纯逻辑单独测。
- 体积：接入后主 JS 961.96 kB（gzip 302.37 kB）；库自身 bundle 63 KB。没有单独隔离增量。
- 库最后一次发版较早（1.4.1），仓库仍有提交；升级时看 CHANGELOG / diff。

## 4. 许可证

- react-custom-roulette 1.4.1：MIT（Copyright (c) 2020 Effectus Software）。
- 它打包的 `dist/bundle.js` 里还有 webfontloader 1.6.28（Apache-2.0）与 styled-components（MIT）。
- 记录：`docs/third-party.md`；全文：`docs/licenses/`。

## 5. 下一期（接真实数据）

1. 池子：`list_slots` 的 `SlotView[]`。
   - 「已安装」= `enabled_variants` 里的版本 + `vanilla_enabled` 的原版；
   - 「战役库」= 所有 `variants`（自制战役可多条）；
   - 补丁不在 `slot.variants` 里，不用额外过滤。
2. 抽中之后：已装 → 启动；未装 → 复用现有 `requestActivation`（预演/冲突确认）再启动；
   自制战役沿用 `launch_game_with_map`（主地图）。
3. 视觉：按 AGENTS §17 出三版预览再定，撤掉「技术验证 · 示例数据」。
4. 池子超过约 12 项时轮上文字会挤，届时再定「缩字号 / 换成序号 + 列表」策略。

## 6. 本次验收（实跑）

- `pnpm -C ui test`：36 个测试文件 / 223 个用例通过。
- `pnpm -C ui build`：通过（tsc + 两个入口）。
- `pnpm -C ui dev` + Chrome headless：`?view=wheel` 正常渲染；点「开始抽」约 5.7 秒后
  结果卡显示「抽到 自制战役 · 示例」，对应扇区高亮、按钮变回「再抽一次」。
- 圆心指针固定指向 12 点；实测抽中「自制战役 · 示例」后，该扇区停在指针正对的方向。
- 自定义指针：页面上选一张本地图片后，圆心指针换成该图（读成 data URL，不落盘）。
- 明暗主题各截图一张（`sandbox/` 下，未入库）。