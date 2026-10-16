# 前端架构

> **一句话**：两侧（启动器用户侧 / 开发工具侧）**分开成两个 app**，只共享框架无关的包；
> 每侧先立"控件层 + 令牌"，再写页面。
>
> **先立地基再迁页面。** 反过来做，等于把现在这堆手写 HTML 换个框架搬家。

## 0. 现在的真实状态（两头都不现代）

| | 用户侧（Vue） | 开发工具侧（React） |
| --- | --- | --- |
| 框架能力 | `<slot>` **0** 次、`provide/inject` **0** 次、`KeepAlive` **0** 次 | `useContext` **0** 次、自建 hook **0** 个、错误边界 **0** 个 |
| 组件库 | **没有**（全手搓） | 装了 Ring UI，只用了 **6 个 Button + 1 个 Input**；其余 **21 原生 button / 10 原生 input** |
| 路由 / 状态 | 手搓 `currentView` + 696 行的 `useLauncher` | 全在一个 **1169 行**的组件里 |
| 样式 | 组件内 scoped CSS **3625 行** | `ide.css` **1100 行**手写 |
| 组件复用 | 9 个里 **5 个只被引用一次** | 文件内定义 2 个子组件 |

**结论：两边都是"框架语法 + 手写 HTML/CSS"。换框架不解决，立分层才解决。**

---

## 1. 工作区：两个 app + 两个共享包

```text
ui/                                  # 前端工作区（pnpm workspace）
├── pnpm-workspace.yaml              # packages: ["apps/*", "packages/*"]（+ 现有 allowBuilds）
├── package.json                     # 只放编排脚本与共享 devDeps
├── tsconfig.base.json               # 共享编译选项 + 路径别名
├── eslint.config.js                 # 分层边界 / 禁原生控件 / 禁硬编码色
├── stylelint.config.js              # 颜色只准来自令牌
├── vitest.workspace.ts              # 一次跑全部包与 app 的测试
│
├── apps/
│   ├── launcher/                    # 【启动器用户侧】玩家看的那个（现 Vue → 迁 React）
│   └── devtools/                    # 【开发工具侧】开发者页（现 React + Ring UI）
│
└── packages/
    ├── shared/                      # 框架无关：bridge / types / lib / markdown / art
    └── tokens/                      # 尺度令牌 + 主题变量契约（两侧共用）
```

**为什么两侧分开**（这是当前定下的思路）：

| 理由 | 说明 |
| --- | --- |
| 使用人群不同 | 玩家要稳、要好看；作者要信息密度、要快 |
| 依赖不同 | 工具侧要 Ring UI（重）；用户侧要轻（绿色版 exe 体积） |
| 节奏不同 | 工具侧可以实验、可以常改；用户侧改动要克制 |
| 出问题的影响面 | 工具侧崩了不影响玩家；分开后一个 app 的 bug 不会带倒另一个 |

**代价（说清楚）**：**控件层不共享。** 两边视觉语言差太远（Material 蓝大卡片 vs IDE 紧凑深色），
硬做成一个库会退化成"一个组件十个 variant"的配置地狱。
**只共享尺度**（间距/圆角/字号/动效），配色各自一套。

---

## 2. 单个 app 的目录形状（两个 app 同构）

```text
apps/launcher/
├── index.html                       # 入口（构建后仍是 /index.html，URL 不变）
├── vite.config.ts
├── tsconfig.json                    # extends ../../tsconfig.base.json
├── package.json
└── src/
    ├── main.tsx                     # 唯一入口：createRoot
    ├── app/                         # 壳：只做装配，不放业务
    │   ├── AppShell.tsx             # 顶栏 / 页签 / 窗口按钮
    │   ├── Providers.tsx            # app 级 context 的组装
    │   ├── Dialogs.tsx              # 全局弹窗宿主（Sheet 只挂一次）
    │   └── navigation.ts            # 页面表（四个页面用状态切换，不引 router）
    ├── features/                    # 按领域切，一个领域一个目录
    │   ├── installation/            # 找游戏、显示版本
    │   ├── library/                 # 战役列表 / 版本卡片 / 启用停用 / 更新
    │   ├── mods/                    # 模组列表与挂载
    │   ├── saves/                   # 存档备份 / 还原 / 隔离
    │   ├── update/                  # 启动器自身更新
    │   └── settings/                # 设置页
    │       ├── api.ts               # 调 shared/bridge（只有这一层知道 IPC）
    │       ├── hooks.ts             # useXxx：数据 + 动作，返回给页面
    │       ├── components/          # 该领域的叶子组件（不知道 IPC）
    │       └── View.tsx             # 页面：只做组合
    ├── ui/                          # 控件层（用户侧视觉语言，唯一允许写 CSS 的地方）
    │   ├── Button.tsx  Field.tsx  Select.tsx  Checkbox.tsx
    │   ├── Sheet.tsx   Menu.tsx   Card.tsx    Tag.tsx   Toolbar.tsx
    │   └── *.css
    ├── styles/
    │   ├── base.css                 # reset + 全局共用类（滚动条、选中态…）
    │   └── theme.css                # 用户侧配色（把 packages/tokens 的值绑成变量）
    └── public/                      # 美术资产（现 ui/public 原样搬）
```

**开发工具侧同构**，只是 `features/` 换成它自己的领域（`tree` / `editor` / `meta` / `export` / `history`），
`ui/` 里装的是**对 Ring UI 的包装**（图标、IDE 专用的树行/标签页），`ide.html` 作入口。

**依赖方向（不可违反）**：

```text
app → features → ui → styles
features → shared（bridge/types/lib 视情况）
ui      ✗ 不许 import features        ← 这条靠 ESLint 拦
shared  ✗ 不许 import react/vue       ← 这条也靠 ESLint 拦
```

---

## 3. 两个共享包

```text
packages/shared/src/
├── bridge.ts        # 现在 ui/src/api/bridge.ts：invoke 封装 + 演示模式桩
├── types.ts         # 与 Rust 对齐的类型（现在 ui/src/api/types.ts）
├── markdown.ts      # marked + DOMPurify（已有测试）
├── art.ts           # 战役美术映射
├── format.ts        # formatBytes 之类
└── tree.ts          # 目录树（开发者页用）

packages/tokens/src/
├── scale.css        # 间距 / 圆角 / 字号 / 行高 / 动效时长 —— **两侧共用**
└── theme.css        # 语义变量名（--surface / --on-surface / --accent …）
                     # 两侧各在自己的 theme.css 里给这些名字**赋不同的值**
```

**为什么令牌要拆成"尺度"和"主题"**：两侧共用的是**尺度**（8 的倍数、圆角几档、字号几档），
而**配色必须各管各的** —— 用户侧是品牌蓝 + 浅色卡片，工具侧是 IDE 深色。
把这个边界画清楚，两边都不会去动对方的审美。

---

## 4. 构建库（这套怎么搭起来、用什么跑）

| 用途 | 选型 | 说明 |
| --- | --- | --- |
| 包管理 | **pnpm workspace** | 已在用；把 `ui/` 扩成真的 workspace（`apps/*` + `packages/*`） |
| 打包 | **Vite 8（rolldown）** | **每个 app 一份 `vite.config.ts`**，不再用 `--config` 切两套 |
| 语言 | **TypeScript** | `tsconfig.base.json` + 每包一份；`paths` 指到 `packages/*/src`（先不上 project references，等编译真慢了再说） |
| 测试 | **Vitest**（`vitest.workspace.ts`） | 已在用；扩成"一次跑全部 app 与包"。组件测试用 jsdom（已装） |
| Lint | **ESLint flat config + typescript-eslint** | 分层边界、禁原生控件、禁硬编码色 —— 见 §6 |
| 样式检查 | **Stylelint** | `color-no-hex` / `function-disallowed-list: rgb,rgba` |
| 行数门禁 | `scripts/check-file-size.py`（已有） | 扩成"组件 200 / 其它 500" |

**构建产物怎么合**（关键，踩过坑）：

- Tauri 只认**一个** `frontendDist`（现在是 `../ui/dist`），所以两个 app 的产物要落进**同一个 `ui/dist`**：
  - launcher 先构建（**负责清空 `dist`**）→ 产出 `index.html` + `assets/*`
  - devtools 后构建（`emptyOutDir: false`）→ 产出 `ide.html` + `assets/*`
- **顺序不能乱、不能合并成一次构建**：放一起 rolldown 会把两边的公共块塞进 `index.html` 预加载，
  用户侧执行到 `createRoot(#ide)` 直接白屏（踩过，见 AGENTS §18）。
- URL 保持不变：`/index.html`、`/ide.html`（`openDevPage` 就是跳 `/ide.html` ✓）。
- 根 `package.json` 的 `build` 负责编排；**CI 里那一条 `pnpm -C ui build` 不用改**。

---

## 5. 状态与组件（两侧同一套规矩）

**状态跟着"谁在用"走**：

| 谁在用 | 放哪 |
| --- | --- |
| 一个组件自己 | `useState` 就地 |
| 一个领域内多个组件 | 该领域的 **context + provider**（`features/<domain>/hooks.ts` 里建） |
| 跨页面（安装信息 / 主题 / 更新） | **app 级 provider**，一个领域一个 |

- **禁止上帝 store**：用户侧那个 696 行的 `useLauncher` 拆成六个领域 hook
  （`useInstallation` / `useLibrary` / `useMods` / `useSaves` / `useUpdate` / `useTheme`）。
- **异步统一收口**：写一个 `useCommand`（`{ data, loading, error, run }`），
  消灭每个组件里手写的 `try / catch / setStatus`。
- **不引 react-query / SWR**：IPC 是本地的，没有缓存失效那套问题，多一层库不值。

**组件规则**：

1. 一个文件一个组件，**>200 行就该拆**（500 是 CI 告警线）。
2. **组合优先**：容器组件不自己画内容，用 `children` / render props 交给调用方
   （Vue 侧 `<slot>` 用了 0 次，这就是"全是叶子组件"的根因）。
3. **叶子组件不知道 IPC**：数据由页面/hook 传进来 —— 好测、好渲染、好复用。
4. **弹窗 / 菜单 / 下拉只有一份实现**（`ui/Sheet`、`ui/Menu`），不许每页各写一份遮罩。
5. **组件不许知道自己在哪一页**（不读全局导航、不 import 别的 feature）。

**样式**：颜色只从令牌拿，**组件里不许出现十六进制或 `rgb(`**；控件的样式跟着控件走一次；
**页面不写 CSS**；需要布局就用少量工具类（`.stack` / `.row` / `.grid-2`）。
目标：现在 **3133 + 1100 ≈ 4200 行 CSS** → **1500 行以内**。

---

## 5.1 控件必须先上开发者房间

**规矩**：一个控件只有在**开发者房间**（设置页 →「开发者模式」→「进入开发者房间」）里
展示过，才能拿到生产界面用。房间只放控件与通用组件；没控件化的东西一律列在房间的
「待迁移」清单里，**不许再往里加新的**。规矩的正文见 AGENTS §18.1。

### 5.2 截图用的调试开关

本地起 dev server 截图时有几个 query 开关（都只影响演示模式，生产路径不变）：

| 开关 | 作用 |
| --- | --- |
| `?theme=dark` | 固定深色主题 |
| `?view=settings` 等 | 直接进某个视图 |
| `#slot=wol` / `#room` | 直接进某个战役 / 开发者房间 |
| `?noupdate=1` | 跳过启动时自动检查更新 —— 不跳过的话演示模式 6 秒后会自动下载，公告与「重启安装」弹窗会把内容区挡死，截不到东西 |

### 5.3 两个踩过的构建坑

1. **UI 的 tsconfig 里，include 必须含 src 下的 .tsx，并且 jsx 必须是 react-jsx。**
   .tsx 不在 include 里时，**没有任何人 import 的 .tsx 不在 program 里** ——
   测试文件会静默逃过 pnpm build 的类型门禁。但只把它加进 include 会更糟：
   vue-tsc 那份配置写着 jsx: preserve，vite/esbuild 会照着它**跳过 JSX 转换**，
   于是 .tsx 直接报 invalid JS syntax。两者要一起改。

2. **Vue 的 style scoped 搬到 React 会变成全局 CSS，同名选择器会打架。**
   多个来源合并时按「首个定义优先」，并且**用括号配对解析**，不要用扁平正则 ——
   正则会切开 @keyframes，构建报 Invalid empty selector。

## 6. 门禁：全部可脚本化，全部**告警不拦**

| 线 | 查什么 | 工具 | 白名单 |
| --- | --- | --- | --- |
| 500 / 200 | 单文件 >500 行（硬信号）/ 组件 >200 行（该拆了） | `scripts/check-file-size.py` | 核心测试等长文件除外 |
| 分层 | `ui/` import `features/`、`shared/` import 框架 | **ESLint**（`no-restricted-imports`） | —— |
| 原生控件 | 组件里出现 `<button` / `<input` / `<select` | **ESLint**（`no-restricted-syntax`） | `ui/`、`packages/` |
| 硬编码色 | 出现 `#rrggbb` / `rgb(` | **Stylelint** | `packages/tokens/` |

**为什么全部只告警**：行数与控件选择是**判断**，不是判据；拦下来只会逼人写 `// eslint-disable`。
但**每次 CI 都会说一句** —— 决定权在人手上，提醒由机器负责。

---

## 7. 测试

- **纯逻辑必测**：tree / format / markdown / 类型映射（已有先例：`markdown.test.ts`）。
- **每个领域一个冒烟测试**：渲染一次 + 走一遍主流程。
- **不追覆盖率**：界面测试的价值在"改了立刻知道炸了"，不在数字。

---

## 8. 迁移顺序（重构与迁移是**同一个动作**）

```
0. 搭 workspace 骨架：apps/launcher、apps/devtools、packages/shared、packages/tokens
   —— **这一步只搬文件、不改一行逻辑**：Vue 原样进 launcher，React 原样进 devtools，
      跑通"两个 app 各自构建 → 合成一个 dist"。跑不通就别往下走。
1. 立 packages/tokens + 两侧各自的 ui/ 控件层（先只做 Button / Field / Sheet / Menu）
2. 重写 devtools（React，已有基础）              ← **验证点**
3. 迁 launcher：settings → mods → saves → campaigns
4. 删 Vue、删旧配置、把 lint 从"告警"提到"拦"
```

**第 2 步是硬验证**：devtools 重写后 `IdeApp.tsx` 1169 行 + `ide.css` 1100 行
如果没明显下来（比如没到 400 行 + 0 行 CSS），**说明这套架构没设计对 —— 先改架构，别继续迁**。

---

## 9. 现在**不**定的（等真动手时按情况定）

- **组件 API 的具体形状**（props 签名、受控/非受控约定）——先做两个真实的用起来再定。
- **表单方案**（受控 / 非受控 / 校验库）——现在手写够用，等表单真的复杂了再说。
- **引不引 router**——四个页面用状态切换没问题；页面多了或要深链再引（`?view=` 已经够用）。
- **CSS Modules / 工具类 / 原子化的取舍**——先按"控件自带样式 + 少量工具类"做，痛了再换。

---

## 10. 迁移期间的红线（AGENTS §18 仍然有效）

- 两个入口同时活着；谁也不 import 谁（**注意：这是"入口"层面的隔离；两个 app 共享 `packages/*`**）。
- 共享的只有**框架无关的纯 TS**；共享文件里不许出现 `vue` / `react` 的 import。
- 构建是两个 app 的两次构建、顺序不能乱（公共块预加载会把用户侧打白屏，踩过）。
- **新界面一律 React**；Vue 只修 bug，不加功能。
