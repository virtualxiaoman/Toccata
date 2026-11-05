[CmdletBinding()]
param([switch]$DebugBuild)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

. (Join-Path $PSScriptRoot 'env.ps1')

$CargoExe = Join-Path $Root '.tools\cargo\bin\cargo.exe'
$MsvcEnv = Join-Path $Root '.tools\msvc-env.ps1'
if (-not (Test-Path $CargoExe) -or -not (Test-Path $MsvcEnv)) {
    Write-Host '首次运行：正在准备项目内工具链（Rust + MSVC + Windows SDK）...'
    & (Join-Path $PSScriptRoot 'setup-toolchain.ps1')
    . (Join-Path $PSScriptRoot 'env.ps1')
}

if (Get-Process -Name toccata-studio -ErrorAction SilentlyContinue) {
    throw 'Toccata 正在运行，请先退出应用再重新构建。'
}

$Manifest = Join-Path $Root 'apps\toccata-windows\src-tauri\Cargo.toml'
$Arguments = @('build', '--manifest-path', $Manifest)
if (-not $DebugBuild) {
    $Arguments += '--release'
}

Write-Host "构建 Toccata for Windows ..."
& $CargoExe @Arguments
if ($LASTEXITCODE -ne 0) {
    throw "cargo build 失败，退出码 $LASTEXITCODE"
}

$Profile = if ($DebugBuild) { 'debug' } else { 'release' }
Write-Host "构建完成：$(Join-Path $env:CARGO_TARGET_DIR "$Profile\toccata-studio.exe")"
