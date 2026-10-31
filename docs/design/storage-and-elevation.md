# 数据落盘与按需提权（2026-10-30 修订版，以此为准）

> 修订原因：白屏真因已确认是“发布包 exe 名带空格导致 WebView2 崩溃”，与权限无关。
> 因此撤销“数据目录整体迁移 + 启动时提权 + 只读模式”的方案，保留真正需要的两件事：
> **单实例锁** 和 **按需 UAC（写游戏目录/库根被拒时才提权，UI 不重启、启动时不提权）**。

## 1. 落盘位置（保持现状，不做迁移）

- WebView2 profile：Tauri 默认 `%LOCALAPPDATA%\<identifier>\EBWebView`，**不迁移**；
  这样 UI 永远能以普通权限启动，C 盘用户也不会因为“看界面”被要求提权。
- `config.json` / `network.json`：`%APPDATA%\net.miyin.launcher\`（`app_config_dir()`），**不迁移**。
- 临时文件：`%TEMP%`，**不重定向**。
- 战役库 / 数据根：**保持** `<游戏安装目录的父目录>/miyin_launcher_data/`（既有设计，见 AGENTS §14.1）。
  **不再**在 exe 同级创建 `data/`。
- 首次没有游戏目录时：原生 rfd 选择 `StarCraft II.exe` 所在目录（仅首次配置，不是权限提权）；
  用户取消则提示后退出。

## 2. 启动流程（普通权限）

1. 装 crash hook。
2. `worker::run_if_requested()`：若是 `--privileged-worker` 模式，跑助手后返回（不加载 Tauri）。
3. **单实例锁**：命名内核对象（`CreateMutexW`/`CreateEventW`，`Local\\net.miyin.launcher.single`）。
   已被占用 → 尝试聚焦已有窗口；失败则原生提示“启动器已在运行”后退出。**与数据根是否可写无关。**
4. 解析星际争霸安装：`%APPDATA%` 旧 config → 注册表 → rfd 选目录。
5. 数据根 = `miyin_core::library::data_root(&installation.root)`；库数据迁移沿用 `resolve_data_root`。
   **不创建/不迁移 config、network、webview2、temp 到数据根**；不设置 `WEBVIEW2_USER_DATA_FOLDER`/`TEMP`。
6. 数据根不可写**不拦启动、不提权**：bootstrap 照常跑，写操作失败时按 §4 现场提权。
7. 窗口按 `tauri.conf.json` 的普通配置创建（**不要** `create:false`、不要手动 `.data_directory`）。
8. setup 里起 bootstrap 线程（`installation`、`data_root`、`config_path` 从 `app_config_dir()` 取），
   然后维持现有 `tame_webview`。

## 3. 按需 UAC

- UI 始终以普通权限运行，**启动时不弹 UAC**。
- 写操作返回 `[PERMISSION]` 时：
  1. UI 调 `request_privileged_worker()`：若助手未启动，用 `ShellExecuteEx(runas)` 启动，
     UAC 只弹这一次；助手驻留到 UI 退出/空闲超时；
  2. 调 `privileged_op(op, args)` 把该操作交给助手执行；UI 状态不丢、不重启；
  3. 用户拒绝 UAC：操作失败，UI 弹说明（“需要管理员权限”，给“再次提权/保持现状”），其余功能继续可用。
- 助手只代理**写游戏目录 / 库根**的操作：activate / deactivate_all / import_package /
  update_variant / delete_variant / import_patch / mods（import/edit/remove/set_enabled/sync）/ merge_apply。
  **存档、导出、config/network 写**继续在 UI 进程内完成。
- 游戏/编辑器始终由 UI（普通权限）启动，**不经过助手**。

## 4. 权限错误约定

- `miyin_core::error::Error` 权限错误 Display 以 `[PERMISSION] ` 开头（W1 已实现），
  `is_permission_denied()` 供后端判断；Tauri 命令错误字符串保留该前缀。
- 前端只按前缀识别，不猜 `os error 5` 文本。

## 5. 单实例

- 保留命名内核对象锁；不写文件、不依赖数据根权限。
- 第二个实例：聚焦已有“弥音启动器”窗口（`FindWindowW`）；找不到就提示后退出。

## 6. 明确不再需要（不要实现）

- WebView2 profile / config / network / temp 迁到数据根；
- 启动时写探针 + `relaunch_elevated` 启动提权；
- `storage_status` / 全局只读横幅 / 写按钮全局禁用；
- 管理员模式下用 explorer 中转启动游戏；
- `prepare_app_paths` 的产品路径（core 保留 API/测试可以，启动流程不得使用）；
- 首次启动复杂的数据根迁移/半份目录处理（库迁移沿用既有 `resolve_data_root`）。

## 7. 文件归属（本轮并行）

| 工作组 | 文件范围 | 交付 |
| --- | --- | --- |
| P1 startup | `src-tauri/src/lib.rs`、`main.rs`、`platform.rs`、`elevation.rs`、`tauri.conf.json`、`commands/campaign.rs`、`src-tauri/Cargo.toml` | 按 §2 回退启动提权/迁移；单实例；普通窗口；保留按需 UAC 接口 |
| P2 worker | `src-tauri/src/worker/**` | 助手跑通 + op 白名单收窄到 §3；不做启动提权 |
| P3 UI | `ui/src/**` | 去掉只读横幅/存储状态；保留 `[PERMISSION]` 对话框与“提权后重放”；测试 |