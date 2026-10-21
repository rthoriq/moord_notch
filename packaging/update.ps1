<#
  Untuk pengembang: pasang build terbaru (target\release\notch.exe) ke notch yang sudah
  terpasang, lalu jalankan ulang. Tidak perlu Administrator. Jalankan setelah
  `cargo build --release` (atau klik dua kali update.cmd):

      powershell -ExecutionPolicy Bypass -File "<folder proyek>\packaging\update.ps1"

  Pemasangan pertama: jalankan `target\release\notch.exe --pasang`.
#>
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$built = Join-Path $root 'target\release\notch.exe'
$installed = Join-Path $env:LOCALAPPDATA 'Programs\NotchWindows\notch.exe'

if (-not (Test-Path $built)) { throw 'Build belum ada. Jalankan dulu: cargo build --release' }
if (-not (Test-Path $installed)) { throw 'Notch belum terpasang. Jalankan dulu: target\release\notch.exe --pasang' }

Write-Host 'Menghentikan notch...'
Get-Process notch -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $installed } | Stop-Process -Force
# Tunggu sampai file exe dilepas Windows.
for ($i = 0; $i -lt 20; $i++) {
    try { Copy-Item $built $installed -Force; break }
    catch { if ($i -eq 19) { throw }; Start-Sleep -Milliseconds 250 }
}

Start-Process explorer.exe -ArgumentList "`"$installed`""
Write-Host 'Notch sudah diperbarui dan berjalan lagi.'
