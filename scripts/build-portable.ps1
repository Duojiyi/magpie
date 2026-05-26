# 打包便携版（Windows）。
# 前置条件：完成一次 `npm run tauri:build`，src-tauri\target\release\tiez-app.exe 存在。
# 产物：artifacts\portable\TieZ_<version>_x64_portable.zip
#
# Tauri 中"便携模式"由运行时检测决定：当 exe 同目录存在 data 文件夹时，
# 应用会把所有数据存到 data 内而非 AppData。本脚本据此打包。

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$pkg = Get-Content (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json
$version = $pkg.version

$exePath = Join-Path $repoRoot 'src-tauri\target\release\tiez-app.exe'
if (-not (Test-Path $exePath)) {
    throw "tiez-app.exe not found at $exePath. Run 'npm run tauri:build' first."
}

$outRoot = Join-Path $repoRoot 'artifacts\portable'
$stage   = Join-Path $outRoot "TieZ_${version}_x64_portable"
$zipPath = "$stage.zip"

Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
Remove-Item -Force $zipPath -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage -Force | Out-Null

# 1. 复制 exe，并改名为 TieZ.exe 让用户更直观
Copy-Item $exePath (Join-Path $stage 'TieZ.exe')

# 2. 创建 data 目录（触发 Tauri 便携模式检测）
New-Item -ItemType Directory -Path (Join-Path $stage 'data') -Force | Out-Null
Set-Content -Path (Join-Path $stage 'data\.keep') -Value '' -Encoding ascii

# 3. 携带 LICENSE / README 副本，符合 GPL-3.0 第 4 条
Copy-Item (Join-Path $repoRoot 'LICENSE')            (Join-Path $stage 'LICENSE.txt')
Copy-Item (Join-Path $repoRoot 'README.zh-CN.md')    (Join-Path $stage 'README.zh-CN.md') -ErrorAction SilentlyContinue
Copy-Item (Join-Path $repoRoot 'README.md')          (Join-Path $stage 'README.md') -ErrorAction SilentlyContinue
Copy-Item (Join-Path $repoRoot 'CHANGELOG.md')       (Join-Path $stage 'CHANGELOG.md') -ErrorAction SilentlyContinue

# 4. 写一份便携版使用说明
$portableReadme = @"
# TieZ Portable 便携版

直接双击 ``TieZ.exe`` 即可启动。

数据存储在与 exe 同目录的 ``data\`` 文件夹中，包括：
- 剪贴板历史数据库
- 设置 / 标签 / 主题
- 日志

把整个目录复制到 U 盘 / 其他电脑 即可携带使用，不会在 AppData 留下数据。

## 注意事项

- 请勿将便携版放在系统受保护目录（如 ``C:\Program Files``），否则数据写入会被拒绝。
- 如需升级版本，覆盖 ``TieZ.exe`` 即可，``data\`` 目录会被保留。
- 卸载时直接删除整个目录即可，不会在系统注册表或 AppData 留下残留。

来源：https://github.com/Duojiyi/tiez-clipboard
"@

Set-Content -Path (Join-Path $stage 'README_PORTABLE.md') -Value $portableReadme -Encoding utf8

# 5. 压缩为 zip
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zipPath -CompressionLevel Optimal -Force

Write-Host ""
Write-Host "[OK] Portable bundle:" -ForegroundColor Green
Write-Host "   $zipPath"
Write-Host "   size: $([math]::Round((Get-Item $zipPath).Length / 1MB, 2)) MB"
