# Project-local environment. Nothing is read from or written to user-global locations.
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path

$env:RUSTUP_HOME = Join-Path $Root '.tools\rustup'
$env:CARGO_HOME = Join-Path $Root '.tools\cargo'
$env:CARGO_TARGET_DIR = Join-Path $Root '.tools\target'
$env:TOCCATA_DATA_DIR = Join-Path $Root 'data'
$env:PATH = (Join-Path $Root '.tools\cargo\bin') + ';' + $env:PATH

$MsvcEnv = Join-Path $Root '.tools\msvc-env.ps1'
if (Test-Path $MsvcEnv) {
    . $MsvcEnv
}

New-Item -ItemType Directory -Force -Path $env:TOCCATA_DATA_DIR | Out-Null
