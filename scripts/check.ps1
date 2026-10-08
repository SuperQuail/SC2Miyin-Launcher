# 一键质量门禁：把该跑的检查全串起来，任何一项失败就非零退出。
#
#     pwsh scripts/check.ps1            # 日常：快
#     pwsh scripts/check.ps1 -Full      # 连上 G 的真实包一起跑
#
# 为什么要有个脚本：门禁散在好几个命令里，靠人记得跑全是不现实的。
# 这个脚本本身就是「我们到底检查什么」的唯一说明。

param(
    [switch]$Full
)

# **不要**设 $ErrorActionPreference = 'Stop'。
# 原生命令（cargo / pnpm）往 stderr 写一行进度，PowerShell 就会当成终止错误直接
# 中断整个脚本 —— 而 cargo 正常干活时 stderr 上一直有字。
# 判断成败统一看 $LASTEXITCODE。
#
# （Windows PowerShell 5.1 会把原生命令 stderr 上的每一行渲染成一条红色的
# NativeCommandError，那是显示层的老毛病，不影响结果 —— 看最后那句
# 「全部通过（N 项）」和退出码就行。）
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$env:HTTPS_PROXY = 'http://127.0.0.1:7897'

$failed = @()
$passed = @()

function Step {
    param([string]$Name, [scriptblock]$Body)
    Write-Host ""
    Write-Host "=== $Name ===" -ForegroundColor Cyan
    # *>&1 把所有流并成一条再逐行输出。
    # 不这么做的话，原生命令往 stderr 写的每一行进度都会被 PowerShell 渲染成
    # 一条红色的 NativeCommandError —— 满屏红字，用的人会习惯性忽略它。
    & $Body *>&1 | ForEach-Object { Out-Host -InputObject $_ }
    if ($LASTEXITCODE -ne 0) {
        $script:failed += $Name
        Write-Host "  FAILED (exit $LASTEXITCODE)" -ForegroundColor Red
    } else {
        $script:passed += $Name
        Write-Host "  ok" -ForegroundColor Green
    }
}

Step '格式 rustfmt' {
    cargo fmt --all
    cargo fmt --all -- --check
}

Step '静态检查 clippy（-D warnings）' {
    cargo clippy --workspace --all-targets --all-features -- -D warnings
}

Step 'Rust 测试' {
    cargo test --workspace
}

Step '前端类型检查 + 构建' {
    pnpm -C ui build
}

Step '前端单元测试（组件接线）' {
    pnpm -C ui test
}

# 真实包的端到端测试。没有 reference/ 目录就跳过 —— 那些包是版权内容，不进仓库。
$reference = Join-Path $root 'reference'
if (Test-Path $reference) {
    Step '真实包端到端（快）' {
        cargo test -p miyin-core --test real_packages
    }
    if ($Full) {
        Step '真实包端到端（含 1.4G 大包）' {
            cargo test -p miyin-core --test real_packages -- --ignored
        }
    }
} else {
    Write-Host ""
    Write-Host "（没有 reference/ 目录，跳过真实包测试）" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
if ($failed.Count -eq 0) {
    Write-Host "全部通过（$($passed.Count) 项）" -ForegroundColor Green
    exit 0
} else {
    Write-Host "没通过 $($failed.Count) 项：" -ForegroundColor Red
    $failed | ForEach-Object { Write-Host "  - $_" -ForegroundColor Red }
    exit 1
}
