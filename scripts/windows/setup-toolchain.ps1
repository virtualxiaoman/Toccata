# Install the Rust toolchain and the MSVC/SDK subset into .tools inside this repository.
# Nothing is written outside the project directory.
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Tools = Join-Path $Root '.tools'
$RustupHome = Join-Path $Tools 'rustup'
$CargoHome = Join-Path $Tools 'cargo'
$MsvcRoot = Join-Path $Tools 'msvc'

New-Item -ItemType Directory -Force -Path $Tools, $RustupHome, $CargoHome, $MsvcRoot | Out-Null

$env:RUSTUP_HOME = $RustupHome
$env:CARGO_HOME = $CargoHome

function Copy-Tree([string]$Source, [string]$Target) {
    if (-not (Test-Path $Source)) {
        throw "缺少目录：$Source"
    }
    New-Item -ItemType Directory -Force -Path $Target | Out-Null
    & robocopy $Source $Target /E /NFL /NDL /NJH /NJS /NP /R:1 /W:1 | Out-Null
    if ($LASTEXITCODE -ge 8) {
        throw "复制失败：$Source -> $Target (robocopy $LASTEXITCODE)"
    }
    $global:LASTEXITCODE = 0
}

function Get-LatestDirectory([string]$Path, [scriptblock]$Filter) {
    Get-ChildItem -LiteralPath $Path -Directory |
        Where-Object { -not $Filter -or (& $Filter $_) } |
        Sort-Object { [version]($_.Name -replace '[^0-9.]', '') } -Descending |
        Select-Object -First 1
}

# 1. Rust toolchain (rustup + cargo + rustc) inside .tools
$CargoExe = Join-Path $CargoHome 'bin\cargo.exe'
if (-not (Test-Path $CargoExe)) {
    Write-Host '[1/3] 下载并安装 Rust 工具链到 .tools ...'
    $RustupInit = Join-Path $Tools 'rustup-init.exe'
    if (-not (Test-Path $RustupInit)) {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -UseBasicParsing `
            -Uri 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe' `
            -OutFile $RustupInit
    }
    & $RustupInit -y --profile minimal --default-toolchain stable-x86_64-pc-windows-msvc --no-modify-path
    if ($LASTEXITCODE -ne 0) {
        throw "rustup 安装失败，退出码 $LASTEXITCODE"
    }
} else {
    Write-Host '[1/3] 已存在项目内 Rust 工具链，跳过安装。'
}

$RustupExe = Join-Path $CargoHome 'bin\rustup.exe'
& cmd.exe /c "`"$RustupExe`" run stable-x86_64-pc-windows-msvc rustc --version >nul 2>nul"
if ($LASTEXITCODE -ne 0) {
    Write-Host '检测到 Rust 工具链不完整，正在重新安装 ...'
    & cmd.exe /c "`"$RustupExe`" toolchain uninstall stable-x86_64-pc-windows-msvc >nul 2>nul"
    & cmd.exe /c "`"$RustupExe`" toolchain install stable-x86_64-pc-windows-msvc"
    if ($LASTEXITCODE -ne 0) {
        throw "Rust 工具链安装失败，退出码 $LASTEXITCODE"
    }
}
& cmd.exe /c "`"$RustupExe`" default stable-x86_64-pc-windows-msvc >nul"

# 2. Locate the locally installed MSVC Build Tools and Windows SDK
$VsWhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $VsWhere)) {
    throw '未找到 vswhere.exe。请先安装 Visual Studio Build Tools（勾选“使用 C++ 的桌面开发”），再运行本脚本。'
}
$InstallPath = (& $VsWhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath | Select-Object -First 1)
if (-not $InstallPath) {
    throw '未找到包含 C++ 生成工具的 Visual Studio / Build Tools。'
}
$VcVersion = Get-LatestDirectory (Join-Path $InstallPath.Trim() 'VC\Tools\MSVC') {
    param($d) $d.Name -match '^\d+(\.\d+)+$'
}
if (-not $VcVersion) {
    throw "未找到 MSVC 工具集：$InstallPath\VC\Tools\MSVC"
}

$KitsRoot = $null
foreach ($Key in @(
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows Kits\Installed Roots',
        'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots')) {
    $Item = Get-ItemProperty -Path $Key -ErrorAction SilentlyContinue
    if ($Item -and $Item.KitsRoot10) { $KitsRoot = $Item.KitsRoot10; break }
}
if (-not $KitsRoot) {
    throw '未找到 Windows 10/11 SDK 安装目录。'
}
$SdkVersion = Get-LatestDirectory (Join-Path $KitsRoot 'Include') { param($d) Test-Path (Join-Path $d.FullName 'um') }
if (-not $SdkVersion) {
    throw "未找到 Windows SDK Include：$KitsRoot\Include"
}

# 3. Copy only the x64 host/target subset into .tools/msvc
$VcTarget = Join-Path $MsvcRoot "VC\Tools\MSVC\$($VcVersion.Name)"
$SdkIncludeTarget = Join-Path $MsvcRoot "Kits\10\Include\$($SdkVersion.Name)"
$SdkLibTarget = Join-Path $MsvcRoot "Kits\10\Lib\$($SdkVersion.Name)"
$SdkBinTarget = Join-Path $MsvcRoot "Kits\10\bin\$($SdkVersion.Name)\x64"

Write-Host "[2/3] 复制 MSVC $($VcVersion.Name) 与 Windows SDK $($SdkVersion.Name) 到 .tools（约 1.1 GB）..."

$VcComplete = (Test-Path (Join-Path $VcTarget 'bin\Hostx64\x64\cl.exe')) -and
    (Test-Path (Join-Path $VcTarget 'bin\Hostx64\x64\link.exe')) -and
    (Test-Path (Join-Path $VcTarget 'include\stdio.h')) -and
    (Test-Path (Join-Path $VcTarget 'lib\x64\msvcrt.lib'))
if (-not $VcComplete) {
    Copy-Tree (Join-Path $VcVersion.FullName 'bin\Hostx64\x64') (Join-Path $VcTarget 'bin\Hostx64\x64')
    Copy-Tree (Join-Path $VcVersion.FullName 'include') (Join-Path $VcTarget 'include')
    Copy-Tree (Join-Path $VcVersion.FullName 'lib\x64') (Join-Path $VcTarget 'lib\x64')
} else {
    Write-Host '      已存在 MSVC 副本，跳过。'
}

if (-not (Test-Path (Join-Path $SdkIncludeTarget 'um\windows.h'))) {
    foreach ($Part in @('ucrt', 'um', 'shared')) {
        Copy-Tree (Join-Path $KitsRoot "Include\$($SdkVersion.Name)\$Part") (Join-Path $SdkIncludeTarget $Part)
    }
} else {
    Write-Host '      已存在 Windows SDK 头文件副本，跳过。'
}

$SdkLibComplete = (Test-Path (Join-Path $SdkLibTarget 'um\x64\kernel32.lib')) -and
    (Test-Path (Join-Path $SdkLibTarget 'ucrt\x64\ucrt.lib'))
if (-not $SdkLibComplete) {
    foreach ($Part in @('um', 'ucrt')) {
        Copy-Tree (Join-Path $KitsRoot "Lib\$($SdkVersion.Name)\$Part\x64") (Join-Path $SdkLibTarget "$Part\x64")
    }
} else {
    Write-Host '      已存在 Windows SDK 库文件副本，跳过。'
}

if (-not (Test-Path (Join-Path $SdkBinTarget 'rc.exe'))) {
    Copy-Tree (Join-Path $KitsRoot "bin\$($SdkVersion.Name)\x64") $SdkBinTarget
}

# 4. Generate environment files that point only at .tools
$MsvcRel = 'msvc'
$VcRel = "$MsvcRel\VC\Tools\MSVC\$($VcVersion.Name)"
$SdkIncludeRel = "$MsvcRel\Kits\10\Include\$($SdkVersion.Name)"
$SdkLibRel = "$MsvcRel\Kits\10\Lib\$($SdkVersion.Name)"
$SdkBinRel = "$MsvcRel\Kits\10\bin\$($SdkVersion.Name)\x64"

$PsEnv = @"
`$MsvcBase = Join-Path `$PSScriptRoot '$MsvcRel'
`$env:INCLUDE = "`$MsvcBase\VC\Tools\MSVC\$($VcVersion.Name)\include;`$MsvcBase\Kits\10\Include\$($SdkVersion.Name)\ucrt;`$MsvcBase\Kits\10\Include\$($SdkVersion.Name)\um;`$MsvcBase\Kits\10\Include\$($SdkVersion.Name)\shared;`$env:INCLUDE"
`$env:LIB = "`$MsvcBase\VC\Tools\MSVC\$($VcVersion.Name)\lib\x64;`$MsvcBase\Kits\10\Lib\$($SdkVersion.Name)\ucrt\x64;`$MsvcBase\Kits\10\Lib\$($SdkVersion.Name)\um\x64;`$env:LIB"
`$env:PATH = "`$MsvcBase\VC\Tools\MSVC\$($VcVersion.Name)\bin\Hostx64\x64;`$MsvcBase\Kits\10\bin\$($SdkVersion.Name)\x64;`$env:PATH"
"@
Set-Content -Path (Join-Path $Tools 'msvc-env.ps1') -Value $PsEnv -Encoding UTF8

$BatEnv = @"
@echo off
set "INCLUDE=%~dp0$($VcRel.Replace('\', '\'))\include;%~dp0$($SdkIncludeRel)\ucrt;%~dp0$($SdkIncludeRel)\um;%~dp0$($SdkIncludeRel)\shared;%INCLUDE%"
set "LIB=%~dp0$($VcRel)\lib\x64;%~dp0$($SdkLibRel)\ucrt\x64;%~dp0$($SdkLibRel)\um\x64;%LIB%"
set "PATH=%~dp0$($VcRel)\bin\Hostx64\x64;%~dp0$($SdkBinRel);%PATH%"
"@
Set-Content -Path (Join-Path $Tools 'msvc-env.bat') -Value $BatEnv -Encoding ASCII

Write-Host '[3/3] 完成。'
Write-Host "  Rust:  $CargoExe"
Write-Host "  MSVC:  $VcTarget"
Write-Host "  SDK:   $SdkIncludeTarget"
