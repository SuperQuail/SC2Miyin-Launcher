# 前端架构

> 目标：**先把地基立起来，再迁页面**。反过来做，等于把现在这堆手写 HTML 换个框架搬家。
>
> 现状（2026-10 实测）：前端 31 个手写文件 13011 行，其中 **3625 行是组件内联 CSS**；
> `ui/src/ide/ide.css` 单独 1100 行；最大的"组件"1312 行。
> 见过的问题都在下面逐条对着解。

## 0. 现在的真实状态（两头都不现代）

| | 用户侧（Vue） | 开发者页（React） |
| --- | --- | --- |
| 框架能力 | `<slot>` **0** 次、`provide/inject` **0** 次、KeepAlive **0** 次 | `useContext` **0** 次、自建 hook **0** 个、错误边界 **0** 个 |
| 组件库 | **没有**（全手搓） | 装了 Ring UI，但只用了 **6 个 Button + 1 个 Input**，其余 **21 个原生 button / 10 个原生 input** |
| 路由 / 状态 | 手搓 `currentView` + 一个 696 行的 `useLauncher` | 全在一个 1169 行的组件里 |
| 样式 | 组件内 scoped CSS 3625 行 | `ide.css` 1100 行手写 |
| 复用的组件 | 9 个里 **5 个只被引用一次** | 本文件内定义 2 个子组件 |

**结论：两边都是"框架语法 + 手写 HTML/CSS"。换框架不解决这个问题，立分层才解决。**

---

## 1. 分层（唯一允许的目录形状）

```text
ui/src/
├── styles/
│   ├── tokens.css        # 颜色 / 间距 / 圆角 / 阴影：**唯一来源**
│   └── base.css          # reset + 全局共用（.sheet / toast / 滚动条）
├── ui/                   # **基础控件层**（自家控件，不含业务）
│   ├── Button.tsx  Field.tsx  Select.tsx  Checkbox.tsx
│   ├── Sheet.tsx   Menu.tsx   Card.tsx    Tag.tsx  Toolbar.tsx
│   └── *.css             # 每个控件的样式跟着控件走一次
├── features/             # 按领域切，一个领域一个目录
│   └── <domain>/
│       ├── api.ts        # 这个领域的命令封装（调 bridge，不碰 UI）
│       ├── hooks.ts      # 数据与动作（useXxx）
│       ├── components/   # 该领域的叶子组件
│       └── View.tsx      # 页面：只做组合
├── app/
│   ├── AppShell.tsx      # 顶栏 / 页签 / 窗口按钮
│   ├── providers.tsx     # app 级 context
│   └── dialogs.tsx       # 全局弹窗宿主（Sheet 只挂一次）
├── api/                  # 框架无关（bridge / types / art / markdown）——**已有，保持**
└── lib/                  # 纯函数（formatBytes / buildTree / …）
```

**依赖方向（不可违反）**：`app → features → ui → styles`，`features → api/lib`。
**反向一律禁止**：`ui` 不许 import `features`；`api` 不许 import 任何 React/Vue。

---

## 2. 状态：跟着"谁在用"走

| 谁在用 | 放哪 |
| --- | --- |
| 一个组件自己 | `useState` 就地 |
| 一个领域内多个组件 | 该领域的 **context + provider**（`features/<domain>/hooks.ts` 里建） |
| 跨页面（安装信息 / 主题 / 更新状态） | **app 级 provider**，一个领域一个 |

**明令禁止**：
- **上帝 store**。现在的 `useLauncher`（696 行，管安装 + 战役 + 模组 + 存档 + 更新 + 主题）必须拆成
  `useInstallation` / `useLibrary` / `useMods` / `useSaves` / `useUpdate` / `useTheme`。
- **页面之间靠 ref 传状态**（现在 React 侧全在一个组件里就是因为这个）。

**异步调用统一收口**：写一个 `useCommand`（`{ data, loading, error, run }`），
消灭每个组件里手写的 `try / catch / setStatus`。**不引 react-query** ——
IPC 是本地的，没有缓存失效那套问题，多一层库不值。

---

## 3. 组件规则

1. **一个文件一个组件**，**>200 行就该拆**（500 行是 CI 告警线，200 是设计目标）。
2. **组合优先**：容器组件不自己画内容，用 `children` / render props 把内容交给调用方
   （Vue 侧 `<slot>` 用了 0 次，这就是"全是叶子组件"的根因）。
3. **叶子组件不知道 IPC**：数据由页面/hook 传进来 —— 好测、好复用、能单独渲染。
4. **弹窗 / 菜单 / 下拉只有一份实现**：`ui/Sheet`、`ui/Menu`，不允许每个页面各写一份遮罩 + 定位。
5. **组件不许知道自己在哪一页**（不读全局路由、不 import 别的 feature）。

---

## 4. 样式

- **颜色/间距/圆角/阴影只从 `tokens.css` 拿**，组件里**不许出现十六进制色值或 `rgb(`**。
- 基础控件的样式**跟着控件走一次**；**页面不写 CSS**。
- 页面级布局允许：少量工具类（`.stack` / `.row` / `.grid-2`）或 CSS Modules。
- **不允许**每个页面一份 300~500 行的 scoped CSS（现在就是这个）。

**目标**：3133 + 1100 = 4200 多行 CSS 压到 **1500 行以内**（大部分进 `ui/` 那一层）。

---

## 5. 门禁（能自动查的，才算门禁）

已有的：**任何一份原始代码 >500 行 → CI 告警**（`scripts/check-file-size.py`）。

再补三条（脚本可查，全部**告警不拦**）：

| 线 | 查什么 | 为什么 |
| --- | --- | --- |
| 200 | 单个组件文件 >200 行 | 500 是"必须看"，200 是"该拆了" |
| 原生控件 | 组件文件里出现 `<button` / `<input` / `<select` | 该用 `ui/` 里的控件；**白名单：`ui/` 目录本身** |
| 硬编码颜色 | 组件/CSS 里出现 `#rrggbb` / `rgb(` | 该用令牌；**白名单：`styles/tokens.css`** |

不搞"架构评审"那种靠人盯的规矩 —— **查不了的规矩等于没有**。

---

## 6. 测试

- **纯逻辑必测**：tree / format / api 映射（已有先例：`markdown.test.ts`、`CustomView.test.ts`）。
- **每个领域一个冒烟测试**：渲染一次 + 走一遍主流程。
- **不追覆盖率**：界面测试的价值在"改动时能立刻发现炸了"，不在数字。

---

## 7. 迁移顺序（重构与迁移是**同一个动作**）

```
1. 立 ui/ + styles/（基础控件层）        ← 不碰任何业务
2. 用它重写开发者页（React，我自己写的、最熟）← 验证这套架构真能把行数压下去
3. 按领域迁用户侧：settings → mods → saves → campaigns
4. 合并两个入口 → 一次构建 → 删掉 Vue
```

**第 2 步是验证点**：如果开发者页重写后 `IdeApp.tsx` 1169 行 + `ide.css` 1100 行
没有明显下来（比如没到 400 行 + 0 行），说明这个架构设计得不对，**先改架构，别继续迁**。

---

## 8. 迁移期间的红线（AGENTS §18 仍然有效）

- 两个入口同时活着；谁也不 import 谁。
- 共享的只有**框架无关的纯 TS**；共享文件里不许出现 `vue` / `react` 的 import。
- 构建是两次、顺序不能乱（公共块预加载会把用户侧打白屏，踩过）。
- **新界面一律 React**；Vue 只修 bug，不加功能。
