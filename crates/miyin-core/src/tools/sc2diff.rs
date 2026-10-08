//! 调用 SC2Diff 这个外部工具。
//!
//! SC2Diff 是**可选**的：没装就走不下去，但我们不该因此报错吓人 ——
//! 调用方拿到 `Err(未安装)` 时安静降级即可（比如「对比」按钮直接不显示）。
//!
//! 顺带说一句为什么要有这一层：SC2Diff 的用法是 `sc2diff -C <repo> <命令>`，
//! 参数形状比较讲究（仓库目录用 `-C` 传而不是 `cd`），
//! 散在业务代码里到处拼命令行迟早出错，统一收在这里。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, Result};

/// SC2Diff 的输出。
#[derive(Debug, Clone)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    /// 命令成功了没。
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    /// 把两路输出拼起来 —— SC2Diff 有的信息走 stderr。
    pub fn combined(&self) -> String {
        let mut text = self.stdout.clone();
        if !self.stderr.trim().is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&self.stderr);
        }
        text.trim().to_string()
    }
}

/// 找到 sc2diff 的可执行文件。
///
/// `data` 是启动器的数据目录，工具装在 `<data>/tools/sc2diff/` 下。
pub fn locate(data: &Path) -> Option<PathBuf> {
    crate::tools::executable(data, &crate::tools::SC2DIFF)
}

/// 没装时的统一说法。
pub fn missing() -> Error {
    Error::PackageRejected(
        "还没装 SC2Diff —— 到「设置 → 可选工具」里装一下就能用这个功能".to_string(),
    )
}

/// 跑一条 SC2Diff 命令。
///
/// `repo` 传 `Some` 时会加上 `-C` —— 不改变当前进程的工作目录，
/// 免得并发调用互相踩。
pub fn run(data: &Path, repo: Option<&Path>, args: &[&str]) -> Result<Output> {
    let binary = locate(data).ok_or_else(missing)?;

    let mut command = Command::new(&binary);
    if let Some(repo) = repo {
        command.arg("-C").arg(repo);
    }
    command.args(args);
    // 中文输出别被代码页搞乱
    command.env("LC_ALL", "C.UTF-8");

    let output = command
        .output()
        .map_err(|error| Error::PackageRejected(format!("启动 SC2Diff 失败：{error}")))?;

    Ok(Output {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// 在一个临时仓库里，把两份文档的**语义 diff** 跑出来。
///
/// 流程（SC2Diff 没有「直接 diff 两个文件」的命令，得借仓库走一圈）：
///
/// 1. `init` 建仓并导入 A
/// 2. `commit` 把 A 记为基线
/// 3. `unpack` 把 B 解成组件，覆盖工作树
/// 4. `add -A` + `commit` 记为改动
/// 5. `diff HEAD~1` 拿到语义 diff
///
/// 任何一步失败都把原始输出带回去 —— 与其吞掉，不如让用户看见 SC2Diff 怎么说。
pub fn semantic_diff(data: &Path, before: &Path, after: &Path, repo: &Path) -> Result<String> {
    std::fs::create_dir_all(repo)?;

    let step = |args: &[&str]| -> Result<Output> {
        let output = run(data, Some(repo), args)?;
        if !output.ok() {
            return Err(Error::PackageRejected(format!(
                "SC2Diff {} 失败：{}",
                args.join(" "),
                output.combined()
            )));
        }
        Ok(output)
    };

    step(&["init", &before.to_string_lossy()])?;
    step(&["commit", "-m", "base"])?;
    step(&["unpack", &after.to_string_lossy()])?;
    step(&["add", "-A"])?;
    step(&["commit", "-m", "after"])?;

    let diff = step(&["diff", "HEAD~1"])?;
    Ok(diff.combined())
}
