# 大转盘（自绘 SVG 版）

> 状态：技术验证页（`?view=wheel` / 顶栏「转盘」）已改为**自绘轮盘 + 旋转圆心指针**，使用示例数据。
> 背景：先试了 `react-custom-roulette` 和 `lucky-canvas`，两者转的都是扇区；而我们要的是
> **指针转、轮盘不动**，加上选型报告里没有库满足可靠性门槛，所以改为自己做。
> 选型过程见 [wheel-library-report.md](wheel-library-report.md)。

## 1. 结构

- `WheelDisc.tsx`：静止轮盘，SVG 画扇区 + 短标签；颜色/文字从 `WheelPalette` 传入。
- `wheelGeometry.ts`：扇区 path 与标签位置，纯函数；`WheelDisc` 与单测共用。
- `WheelView.tsx`：抽奖与指针旋转；`targetRotation` 至少 4 圈、最后指向中奖扇区中心。
- `wheelData.ts`：`pickIndex` / `sectorCenter` / `targetRotation` / `shortLabel`。
- 主题：`readPalette` 读设计令牌；主题变化时重绘轮盘、指针同步换色。
- 没有第三方转盘库；`react-custom-roulette` 与 `lucky-canvas` 都已从依赖里移除。

## 2. 角度约定

- 0° = 12 点，顺时针；扇区 i 从 i×360/n 铺到 (i+1)×360/n，中心在 (i+0.5)×360/n。
- 指针初始 0°；`targetRotation` 保证只往顺时针、至少 4 圈，最后落在中奖扇区中心。
- 轮盘不转，所以扇区几何与指针角度共用同一套坐标系，不需要再对齐任何库常量。

## 3. 指针图片

- 默认是内置 SVG 针头（扇形中心为转轴、朝上）；
- 页面「换指针图片…」可选本地图片（读成 data URL）；代码里也可以传 `pointerSrc`；
- 图片约定：正方形、转轴在图片中心、朝上、透明背景；整张图按轮盘方框 100% 放。
- 上限 2 MB；只活在当前页面内存（demo 不落盘）。

## 4. 动效

- 正常：CSS transition 4 秒，`cubic-bezier(0.15, 0.85, 0.15, 1)`；
- `prefers-reduced-motion`：时长 0，结果几乎立刻出来。

## 5. 已知限制

- 扇区标签是径向文字，左半圈翻 180°；长名字靠 4 字短标签 + 候选列表。
- 固定 440×440（应用最小窗口 960，够用）；要做响应式再量。
- 扇区很多（>12）时 4 字标签会挤，接真实池子前再定策略。
- 指针图片 modal 只在内存，刷新回默认。

## 6. 本次验收

- `pnpm -C ui test`：36 个测试文件 / 231 个用例通过（含 wheelData / wheelGeometry / WheelView）。
- `pnpm -C ui build`：通过；主 JS 903.67 kB（gzip 277.82 kB），比 lucky-canvas 版小。
- `pnpm -C ui dev` + headless Chrome：轮盘静止、指针转 4 秒后停在中奖扇区中心；
  结果卡与高亮候选一致。
- 截图：`sandbox/wheel-*.png`（未入库）。

## 7. 下一期

- 接真实池子与启用/启动；
- 指针图片持久化进配置；
- 视觉定稿前按 AGENTS §17 出三版预览。