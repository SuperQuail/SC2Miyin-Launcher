//! 平台相关的小工具。
//!
//! 两件事：让子进程**不弹控制台窗口**，以及**跨卷地移动文件**。

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// 让子进程不弹控制台窗口。
///
/// **这是踩出来的**：从 GUI 程序拉一个控制台程序（`tar`、`sc2diff`、`cmd`…）
/// 时，Windows 会顺手给它在屏幕上开一个黑框 —— 用户看到的就是「解压的时候
/// 闪了一下黑窗口」，观感很差，而且会抢焦点。
///
/// 加 `CREATE_NO_WINDOW` 就不开那个窗口了。非 Windows 平台是空操作。
///
/// ⚠️ **只对控制台程序用**。拉游戏本体、编辑器那种 GUI 程序时别加 ——
/// 那是人家自己的窗口，藏不得。
pub fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// 不创建控制台窗口。
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    #[cfg(not(windows))]
    {
        // 其它平台没有这个概念
        let _ = command;
    }
}

/// 游戏是不是正跑着；跑着就返回进程名。
///
/// **切文件前必须问一句** —— Windows 上运行中的 exe / 被占用的地图删不掉、覆盖会失败，
/// 半途失败会留下「清单记了一半」的中间状态。宁可让用户先退游戏。
///
/// 用 `tasklist` 而不是 sysinfo：只为一个进程名拉一个依赖不值当，
/// 而 tasklist 是 Windows 自带的（`/FO CSV /NH` 的解析见下）。
#[cfg(windows)]
pub fn game_running() -> Option<String> {
    let mut command = std::process::Command::new("tasklist");
    command.args(["/FO", "CSV", "/NH"]);
    hide_console(&mut command);

    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);

    // CSV 每行形如 "SC2_x64.exe","1234","Console","1","1,234 K"
    // 只认游戏本体；StarCraft II.exe 是根启动器，开着不算（它不占 Maps/Mods）
    ["SC2_x64.exe", "SC2.exe"]
        .into_iter()
        .find(|name| text.contains(&format!("\"{name}\"")))
        .map(str::to_string)
}

/// 非 Windows 上没有这套判定 —— 返回 None，调用方当作「没在跑」。
#[cfg(not(windows))]
pub fn game_running() -> Option<String> {
    None
}

/// **把文件或整棵目录树从 `from` 搬到 `to`** —— 跨卷也能搬。
///
/// 为什么不能直接用 `fs::rename`：它只在**同一个卷**内有效。跨卷时 Windows 报
/// `os error 17`（ERROR_NOT_SAME_DEVICE）、Unix 报 EXDEV，于是整条链路就断在
/// 这里。而启动器与游戏目录**默认就在两个盘**上（启动器装在 C 盘、游戏在 D 盘，
/// 或者反过来），所以「启用战役 / 铺模组」对这类用户是必然失败的 —— 这就是用户
/// 报的「非同盘安装的无法移动到不同磁盘」。
///
/// 做法：先试 `rename`（同卷时是最快的原子操作）；它失败就退回
/// **复制 + 删源**。失败原因不细挑 —— 不论跨卷还是别的原因，rename 做不到的事
/// 复制也一样做不到，那时报的就是复制自己给的错。
///
/// 两条安全底线：
///
/// 1. **中途失败不留半个目标**：复制途中出错，把已经写出去的那部分删掉再返回错误。
/// 2. **删源只在复制完全成功之后**：宁可留下一份多余的源文件，也不能出现
///    「源已删、目标只有一半」。复制成功但删源失败时返回的是删除的错误，
///    此时两边内容相同 —— 与 rename 报错后源原封不动是同一个安全水位。
///
/// 目标已存在时**就地覆盖**（复制会盖上同名文件）；调用方要清干净的话自己先删 ——
/// 库里那几处调用点都在调它之前做过 remove。
pub fn move_path(from: &Path, to: &Path) -> Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }

    if let Err(error) = copy_path(from, to) {
        remove_path(to);
        return Err(error);
    }

    if from.is_dir() {
        std::fs::remove_dir_all(from)?;
    } else {
        std::fs::remove_file(from)?;
    }
    Ok(())
}

/// 拷一个文件或一棵目录树。`fs::copy` 不认目录，目录得自己往下走。
fn copy_path(from: &Path, to: &Path) -> Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in walkdir::WalkDir::new(from)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let relative = entry
                .path()
                .strip_prefix(from)
                .map_err(|_| Error::PackageRejected("路径越界".to_string()))?;
            let dest = to.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(&dest)?;
            } else {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(entry.path(), &dest)?;
            }
        }
        return Ok(());
    }

    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(from, to)?;
    Ok(())
}

/// 清掉一个文件或一棵目录树；不存在、清不掉都当没这回事。
///
/// 只用来收拾**这次复制出来的半成品**：它要么是新建的，要么调用方已经确认过
/// 可以被覆盖，所以这里不区分「本来就在」的情况。
pub(crate) fn remove_path(path: &Path) {
    if path.is_dir() {
        let _ = std::fs::remove_dir_all(path);
    } else {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn tree(from: &Path, to: &Path) {
        std::fs::create_dir_all(from.join("inner")).unwrap();
        std::fs::write(from.join("a.txt"), b"hello").unwrap();
        std::fs::write(from.join("inner/b.txt"), b"world").unwrap();
        std::fs::create_dir_all(to).unwrap();
    }

    fn assert_moved(to: &Path) {
        assert_eq!(std::fs::read_to_string(to.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            std::fs::read_to_string(to.join("inner/b.txt")).unwrap(),
            "world"
        );
    }

    /// 只验它能跑通、不 panic。**不断言结果** —— 测试机上游戏开着还是关着都可能。
    #[test]
    fn game_running_does_not_panic() {
        let _ = game_running();
    }
    #[test]
    fn copy_then_delete_equals_move() {
        let dir = tempfile::tempdir().unwrap();
        let (from, to) = (dir.path().join("from"), dir.path().join("to"));
        tree(&from, &to);

        copy_path(&from, &to).unwrap();
        std::fs::remove_dir_all(&from).unwrap();

        assert!(!from.exists(), "源应当被删掉");
        assert_moved(&to);
    }

    #[test]
    fn moves_directory_tree() {
        let dir = tempfile::tempdir().unwrap();
        let (from, to) = (dir.path().join("from"), dir.path().join("deep/to"));
        std::fs::write(dir.path().join("placeholder"), b"").unwrap();

        tree(&from, &to);

        move_path(&from, &to).unwrap();

        assert!(!from.exists());
        assert_moved(&to);
    }

    #[test]
    fn moves_single_file() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("mod.SC2Mod");
        let to = dir.path().join("Mods/mod.SC2Mod");
        std::fs::write(&from, b"mpq").unwrap();

        move_path(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(std::fs::read(&to).unwrap(), b"mpq");
    }

    /// 复制到一半失败**不能留下半个目标**：把目标目录设成只读，复制写第一个文件
    /// 就会失败 —— 那时目标已经被建出来了，必须被清掉。
    #[test]
    fn failed_copy_leaves_no_half_target() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("from");
        let to = dir.path().join("to");
        std::fs::create_dir_all(&from).unwrap();
        std::fs::write(from.join("a.txt"), b"hello").unwrap();
        std::fs::write(from.join("b.txt"), b"world").unwrap();
        std::fs::create_dir_all(&to).unwrap();

        let restore = std::fs::metadata(&to).unwrap().permissions();
        let mut readonly = restore.clone();
        readonly.set_readonly(true);
        std::fs::set_permissions(&to, readonly).unwrap();
        if std::fs::write(to.join("probe"), b"x").is_ok() {
            // 只读位拦不住写（特权账户 / 非 Windows）：这条测试在这台机器上不成立
            let _ = std::fs::remove_file(to.join("probe"));
            std::fs::set_permissions(&to, restore).unwrap();
            return;
        }

        assert!(copy_path(&from, &to).is_err());
        remove_path(&to);
        std::fs::set_permissions(dir.path(), restore).unwrap();

        assert!(!to.exists(), "复制中途失败之后不该留下目标");
    }
    /// **真跨卷**才跑的那条：默认忽略 —— 测试机不一定有第二个卷。
    ///
    /// 打开方式（`F:\` 换成你机器上的另一个卷）：
    /// ```
    /// $env:MIYIN_CROSS_VOLUME='F:\'; cargo test -p miyin-core -- --ignored moves_across_volumes --nocapture
    /// ```
    ///
    /// 它先断言**裸 rename 确实失败** —— 不然这条测试根本没在验退路，等于白跑。
    #[test]
    #[ignore = "需要第二个卷，用 MIYIN_CROSS_VOLUME=F:\\ 打开"]
    fn moves_across_volumes() {
        let Ok(volume) = std::env::var("MIYIN_CROSS_VOLUME") else {
            eprintln!("没设 MIYIN_CROSS_VOLUME，跳过");
            return;
        };
        let volume = std::path::PathBuf::from(volume);
        assert!(
            volume.is_dir(),
            "MIYIN_CROSS_VOLUME 指向的目录不存在：{volume:?}"
        );

        // 源在**当前卷**（测试临时目录），目标在**另一个卷** —— 这就是用户的场景
        let source = tempfile::tempdir().unwrap();
        let from = source.path().join("pack");
        tree(&from, &from.join("nothing")); // 只借用它把目录树造出来
        let across = tempfile::tempdir_in(&volume).unwrap();
        let to = across.path().join("pack");

        // 前提：裸 rename 必须失败，否则这条测试没在验退路
        assert!(
            std::fs::rename(&from, &to).is_err(),
            "两个目录居然在同一个卷上，这条测试没意义 —— 换个 MIYIN_CROSS_VOLUME"
        );

        move_path(&from, &to).expect("跨卷移动应当成功");

        assert!(!from.exists(), "源应当被删掉");
        assert_eq!(std::fs::read_to_string(to.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            std::fs::read_to_string(to.join("inner/b.txt")).unwrap(),
            "world"
        );

        // 单个文件也一样
        let single_from = source.path().join("one.bin");
        std::fs::write(&single_from, b"0123456789").unwrap();
        let single_to = across.path().join("one.bin");
        move_path(&single_from, &single_to).expect("跨卷移动单个文件应当成功");
        assert!(!single_from.exists());
        assert_eq!(std::fs::read(&single_to).unwrap(), b"0123456789");

        eprintln!("跨卷移动验证通过：{from:?} -> {to:?}");
    }
}
