# 前端

> **现状（2026-10）：单包、两个入口、都是 React 18。Vue 已迁完并从依赖里移除。**
> 本文是前端唯一的说明文档 —— 原先的 `frontend-architecture` / `frontend-modernization` /
> `frontend-migration` 三份已并到这里（那三份里有大半是**已经不存在的代码**的体检数字，
> 和一份**从没采用**的包结构方案）。

> 开发者页的编辑器的**设计稿**（工作区、落点预览、metadata 生成）见 [dev-editor.md](dev-editor.md)。

## 结构

```text
ui/index.html → src/main.tsx      用户侧（战役 / 模组 / 设置 / 开发者房间）
ui/ide.html   → src/ide/main.tsx  开发者页（IDE 工作台 + #gallery 控件展台）

ui/src/
  api/        框架无关：bridge / types / markdown / art / format —— 不许 import react
  react/
    app/      外壳：AppShell / TopBar / Dialogs / ContextMenu
    state/    领域状态：useInstallation / useLibrary / useSaves / useUpdate / useTheme / …
    ui/       控件层（Button / Field / Sheet）—— 用户侧唯一的控件来源
    views/    页面：campaigns / custom / mods / settings / cheats / room
    lib/      useCommand / clipboard / devMode / contextMenu
  ide/        workbench / gallery / lib（Ring UI）
  styles/     tokens.css（设计令牌）+ base.css
```

## 两条边界（越了必出事）

1. **两个入口不共享组件。** 用户侧只 import `react/**` 与 `api/**`；开发者页只 import `ide/**` 与 `api/**`。
   Ring UI 不进用户侧（绿色版体积）。跨页 = **换入口整页加载**，不是路由。
2. **共享的只有 `api/**` 那层纯 TS**，那里不许 import react。

## 构建：两次，顺序不能乱

`pnpm -C ui build` = 清 dist → 用户侧（`vite.config.ts`）→ 开发者页（`vite.ide.config.ts`）。
**放一次构建里，rolldown 会把两边公共块塞进 `index.html` 预加载，用户侧白屏。**

两个坑：

- tsconfig 的 include 要含 `.tsx`，**且** `jsx` 必须是 `react-jsx` —— 只改一半，`.tsx` 直接报 invalid JS syntax。
- 原来 Vue 的 `<style scoped>` 搬过来会变成全局 CSS，同名选择器打架。合并多份样式时按「首个定义优先」，
  并**用括号配对解析**（扁平正则会切开 `@keyframes`，报 Invalid empty selector）。

## 预览草稿（`ui/src/preview/`）

设计方案的渲染草稿放这里（gitignore 的），`?preview=<名字>` 打开，机制在 `AppShell.tsx` 的 `previewModules`。
**`src/preview` 也会被类型检查**（tsconfig 的 include 是 `src/**/*.tsx` + `noUnusedLocals`）——
草稿里有类型错误或未用变量会让 `pnpm -C ui build` **直接失败**。写草稿时就当它在生产代码里写。

## 控件规矩

见 AGENTS §18.1：**控件先在开发者房间（`#room`）展示过，再上生产**。
房间分两页：用户侧（`react/ui`）与开发者工具侧（`iframe` 嵌 `ide.html#gallery`）。

## 截图用的开关（都只影响演示模式）

| 开关 | 作用 |
| --- | --- |
| `?theme=dark` | 固定深色主题 |
| `?view=settings` | 直接进某个视图 |
| `#slot=wol` / `#room` / `#room=dev` | 直接进某个战役 / 开发者房间的两页 |
| `?noupdate=1` | 跳过启动时的自动更新检查 —— 不跳过的话 6 秒后会自动下载，公告与「重启安装」弹窗把内容区挡死 |

## 状态与组件规矩

- 状态跟着「谁在用」走：一个组件用 `useState`；一个领域多个组件用该领域 context；跨页面才进 app 级 provider。
- **禁止上帝 store** —— 原来那个 697 行的 `useLauncher` 就是这么烂掉的。
- 异步统一走 `react/lib/useCommand`，别每个组件手写 try/catch/notify。
- 一个文件一个组件；>200 行该拆，>500 行 CI 告警。
- 叶子组件不知道 IPC：数据由页面 / hook 传进来。
- 弹窗、菜单只有一份实现（`react/ui/Sheet`、`react/app/ContextMenu`）。
- 颜色只从令牌拿；页面里不写 CSS。

## 为什么现在是这个样子（历史，2026-10 体检）

改之前的数字：用户侧是 Vue；`useLauncher` **697 行 / 26 个导出**、`App.vue` **1109 行**、`ModsView` **1312 行**；
手写遮罩 **16 处**、颜色硬编码 **130 处**、**40 个源文件只有 2 个测试**。

结论是**缝补不如重写**：先立 `react/ui` 控件层与 `react/lib` 四件设施，再一个视图一个视图**整块**换掉，
每迁一块立刻删掉旧文件。现在 Vue 已完全移除。

**没采用的方案**：曾设想过把 `ui/` 拆成 `apps/launcher` + `apps/devtools` + `packages/{shared,tokens}`
的 pnpm 工作区。实际迁移是整块换视图，没动包结构 —— 单包两个入口已经够用，**先不引工作区**。
