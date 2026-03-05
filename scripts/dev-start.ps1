param(
  [int]$Port = 5188
)

$ErrorActionPreference = "Stop"

$stopScript = Join-Path $PSScriptRoot "dev-stop.ps1"
& powershell -ExecutionPolicy Bypass -File $stopScript -Port $Port

$cargoBin = Join-Path $env:USERPROFILE ".cargo\\bin"
if (Test-Path $cargoBin) {
  if (-not ($env:Path -split ';' | Where-Object { $_ -eq $cargoBin })) {
    $env:Path = "$cargoBin;$env:Path"
  }
}

$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-msvc"

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  throw "cargo not found in PATH. Install Rustup or open a new terminal after install."
}

if (-not (Get-Command npm.cmd -ErrorAction SilentlyContinue)) {
  throw "npm.cmd not found in PATH. Install Node.js."
}

Write-Host "Starting Tauri dev on port $Port..."
& npm.cmd run tauri:dev
