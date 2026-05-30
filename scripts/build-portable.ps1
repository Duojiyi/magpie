# 打包便携版（Windows）。
# 前置条件：完成一次 `npm run tauri:build`，src-tauri\target\release\magpie.exe 存在。
# 产物：artifacts\portable\Magpie_<version>_x64_portable.zip
#
# Tauri 中"便携模式"由运行时检测决定：当 exe 同目录存在 data 文件夹时，
# 应用会把所有数据存到 data 内而非 AppData。本脚本据此打包。

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

$pkg = Get-Content (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json
$version = $pkg.version

$exePath = Join-Path $repoRoot 'src-tauri\target\release\magpie.exe'
if (-not (Test-Path $exePath)) {
    throw "magpie.exe not found at $exePath. Run 'npm run tauri:build' first."
}

$outRoot = Join-Path $repoRoot 'artifacts\portable'
$stage   = Join-Path $outRoot "Magpie_${version}_x64_portable"
$zipPath = "$stage.zip"

Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
Remove-Item -Force $zipPath -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path $stage -Force | Out-Null

# 1. 复制 exe，并改名为 Magpie.exe 让用户更直观
Copy-Item $exePath (Join-Path $stage 'Magpie.exe')

# 2. 创建 data 目录（触发 Tauri 便携模式检测）
New-Item -ItemType Directory -Path (Join-Path $stage 'data') -Force | Out-Null
Set-Content -Path (Join-Path $stage 'data\.keep') -Value '' -Encoding ascii

# 3. 携带 LICENSE / README 副本，符合 GPL-3.0 第 4 条
Copy-Item (Join-Path $repoRoot 'LICENSE')            (Join-Path $stage 'LICENSE.txt')
Copy-Item (Join-Path $repoRoot 'README.zh-CN.md')    (Join-Path $stage 'README.zh-CN.md') -ErrorAction SilentlyContinue
Copy-Item (Join-Path $repoRoot 'README.md')          (Join-Path $stage 'README.md') -ErrorAction SilentlyContinue
Copy-Item (Join-Path $repoRoot 'CHANGELOG.md')       (Join-Path $stage 'CHANGELOG.md') -ErrorAction SilentlyContinue

# 3.1 携带云同步教程（MQTT / WebDAV），放在便携包根目录与 README/LICENSE 同级，便于离线查阅
Copy-Item (Join-Path $repoRoot 'docs\cloud-sync-tutorial-mqtt.md')   (Join-Path $stage 'cloud-sync-tutorial-mqtt.md')   -ErrorAction SilentlyContinue
Copy-Item (Join-Path $repoRoot 'docs\cloud-sync-tutorial-webdav.md') (Join-Path $stage 'cloud-sync-tutorial-webdav.md') -ErrorAction SilentlyContinue

# 4. 写一份便携版使用说明
# 注意：使用外置模板文件 README_PORTABLE.template.md 而非 here-string——避免 Windows
# PowerShell 5.1 在中文系统下把无 BOM 的 .ps1 中的中文字面量按 GBK 误读后写出双重编码乱码。
$portableReadmeSrc = Join-Path $PSScriptRoot 'README_PORTABLE.template.md'
$portableReadmeDst = Join-Path $stage 'README_PORTABLE.md'
if (-not (Test-Path $portableReadmeSrc)) {
    throw "README_PORTABLE.template.md not found at $portableReadmeSrc"
}
Copy-Item $portableReadmeSrc $portableReadmeDst -Force

# 4.1 编码自检：核实模板文件以 UTF-8 BOM 起头，且复制后字节序完整一致。
# 自检故意不使用中文字面量，以兼容任何 PS 解析编码（避免脚本自身被 GBK 误读）。
$srcBytes = [System.IO.File]::ReadAllBytes($portableReadmeSrc)
$dstBytes = [System.IO.File]::ReadAllBytes($portableReadmeDst)
if ($srcBytes.Length -ne $dstBytes.Length) {
    throw "README_PORTABLE.md size mismatch: src=$($srcBytes.Length) dst=$($dstBytes.Length)"
}
# 模板必须以 UTF-8 BOM 起头（EF BB BF），保证 Windows 资源管理器 / 记事本正确识别中文。
if ($dstBytes.Length -lt 3 -or $dstBytes[0] -ne 0xEF -or $dstBytes[1] -ne 0xBB -or $dstBytes[2] -ne 0xBF) {
    throw "README_PORTABLE.md missing UTF-8 BOM at start (got: $('{0:X2}' -f $dstBytes[0]) $('{0:X2}' -f $dstBytes[1]) $('{0:X2}' -f $dstBytes[2]))"
}

# 5. 压缩为 zip
Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zipPath -CompressionLevel Optimal -Force

Write-Host ""
Write-Host "[OK] Portable bundle:" -ForegroundColor Green
Write-Host "   $zipPath"
Write-Host "   size: $([math]::Round((Get-Item $zipPath).Length / 1MB, 2)) MB"
