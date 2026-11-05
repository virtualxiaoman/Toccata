[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

. (Join-Path $PSScriptRoot 'env.ps1')

$Exe = Join-Path $env:CARGO_TARGET_DIR 'release\toccata-studio.exe'
$Watched = @(
    (Join-Path $Root 'apps\toccata-windows\src-tauri\src'),
    (Join-Path $Root 'apps\toccata-windows\src-tauri\Cargo.toml'),
    (Join-Path $Root 'apps\toccata-windows\src-tauri\build.rs'),
    (Join-Path $Root 'apps\toccata-windows\src-tauri\tauri.conf.json'),
    (Join-Path $Root 'apps\toccata-windows\src-tauri\icons'),
    (Join-Path $Root 'apps\toccata-windows\ui')
)
$NeedsBuild = -not (Test-Path $Exe)
if (-not $NeedsBuild) {
    $Latest = Get-ChildItem -Path $Watched -Recurse -File |
        Measure-Object -Property LastWriteTime -Maximum
    if ($Latest.Maximum -gt (Get-Item $Exe).LastWriteTime) {
        $NeedsBuild = $true
    }
}
if ($NeedsBuild) {
    Write-Host '检测到源码更新或尚未构建，开始构建（首次需要下载依赖）。'
    & (Join-Path $PSScriptRoot 'build.ps1')
}

if (-not (Test-Path $Exe)) {
    throw "未找到可执行文件：$Exe"
}

Write-Host "启动 $Exe"
Write-Host "数据目录：$env:TOCCATA_DATA_DIR"
$Process = Start-Process -FilePath $Exe -WorkingDirectory $Root -PassThru
Start-Sleep -Seconds 2
if ($Process.HasExited) {
    throw "应用启动后立即退出，退出码 $($Process.ExitCode)。"
}
Write-Host "Toccata 已启动（PID $($Process.Id)）。"
