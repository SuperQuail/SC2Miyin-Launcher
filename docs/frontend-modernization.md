# 前端体检：哪里不现代化，怎么改

> 2026-10 实测。**结论：类型纪律是好的，除此之外的"工程骨架"基本是手搓的。**
> 这份文档是改造的依据 —— 每一条都带数字，不是感觉。

## 一、九条病（按危害排）

### 1. 一个上帝 composable

`useLauncher.ts`：**697 行、26 个导出、25 个 async 函数、12 个状态**，
**13 个文件 import 它**。安装 / 战役 / 模组 / 存档 / 更新 / 主题 / 通知全在一个对象里。

**危害**：改一处要读全篇；纯逻辑挑不出来 → **没法测**；谁都能往里面加东西 → 只会更长。

### 2. 错误处理重复 56 遍

`try { … } catch (error) { notify("error", errorText(error)) }` 这个模板写了 **56 次**，
`errorText(...)` 被调用 **62 次**。

**危害**：没有"动作"这个概念 —— 每次调用都要手写三件套（loading、try/catch、notify），
漏一个就是静默失败。

### 3. 加载态各写各的

`loading = ref()` 5 处、`busy = ref()` 一堆（还有 `busy = "export-many"` 这种拿字符串当状态用的）。
**危害**：同一个动作在不同页面的"在跑"表示不一样，界面上也没法统一禁用。

### 4. 硬编码颜色 130 处

`#rrggbb` / `rgba()` 散在 6 个组件里：
App 35 · SlotMenuView 20 · CustomCampaignPanel 16 · ModsView 15 · CampaignsView 12 · VariantCard 7。

**危害**：换主题、换视觉语言时得满仓库找；深浅色模式必然漏掉几处。

### 5. 控件全是手搓的

**16 处**手写遮罩、**49 处**手写表单行、**100+ 处**手写按钮；
**剪贴板写了两套**（`navigator.clipboard` 6 处 + 自造 textarea 回退 1 处）。

**危害**：同一种东西在 N 个文件里各有一份样式与行为，改一处等于改 N 处。

### 6. 框架能力几乎没用

`<slot>` **0** · `provide/inject` **0** · router **0** · store **0** · 组件库 **0**。

对照：`v-if` **131** 个挤在 6 个巨型文件里，而 `defineProps` 只有 **10** 处、`defineEmits` **11** 处。
**这组数字说明：绝大多数界面是"页面里内联写出来的"，不是组件组合出来的。**

### 7. 测试几乎没有

**40 个源码文件 vs 2 个测试**（且都是纯逻辑）。**界面改动没有任何护栏** ——
上一轮我把顶栏挤成两行、发出去两个提交才自己发现，就是这个原因。

### 8. 巨型文件

`ModsView` 1312 · `SlotMenuView` 1108 · `App.vue` 928 · `CustomCampaignPanel` 789 · `ImportDialog` 741。
CI 已经加了 500 行告警（`scripts/check-file-size.py`），当前 **24 份**超标。

### 9. 类型纪律反而是好的 ✓

`: any` **0** · `as any` **0** · `@ts-ignore` **0**（只有 1 处 `as unknown as`、11 处非空断言）。
**这条要保住** —— 改造时不许用 `any` 抄近路。

---

## 二、改造策略

**缝补不如重写。** 已迁的两块（作弊码页、更新公告）证明 React 侧能干净落地；
**剩下的每迁一个页面，就顺手按下面的规矩拆**，不要"先搬过去再修"。

### 先立四件止血设施（迁页面之前）

| 设施 | 替掉什么 |
| --- | --- |
| `react/lib/useCommand` | 那 56 遍 try/catch 模板 —— 统一 `{ run, loading, error }` |
| 控件层补全（Sheet/Field/Select/Toggle/Chip/Empty/Loading） | 16 处遮罩、49 处表单行、100+ 按钮 |
| `react/lib/clipboard` | 两套剪贴板实现 |
| `react/lib/format` | `formatBytes` 之类纯函数（现在混在 `api/art.ts` 的美术映射里） |

### 然后按领域迁，**新的 React 侧一律不许 import `useLauncher`**

状态按领域拆（`useLibrary` / `useMods` / `useSaves` / `useUpdate` / `useTheme`），
外壳通过 `ReactIsland` 的 `viewProps` 把数据递进去（桥已支持）。

### 样式

- 控件自带样式；页面只写布局
- **硬编码颜色 130 处 → 0**：迁到哪清到哪，颜色只从令牌拿

### 迁移顺序

```
0. 四件基础设施              ← 不碰业务
1. ModsView（1312，自洽）     ← 拆成 模组行 / 独立卡片 / 三个弹窗 + useMods
2. SettingsView 子树（+ ToolsPanel + UpdatePanel）
3. 战役三页 + 卡片/对话框组件（SlotCard / VariantCard / ImportDialog / CustomCampaignPanel / DocViewer）
4. App.vue 外壳                ← Vue 全退场
```

**每迁一块：类型检查 + 构建 + 测试 + 真渲染截图**，旧 .vue 立刻删。
