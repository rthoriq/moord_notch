<#
  Cabut notch yang terpasang: paket dan sertifikatnya, "Mulai bersama Windows", dan file
  notch.exe di %LOCALAPPDATA%\Programs\NotchWindows. Data notch (shelf, tugas, catatan minum,
  pengaturan) di %APPDATA%\NotchWindows tidak dihapus. Jalankan di PowerShell "Run as
  Administrator":

      powershell -ExecutionPolicy Bypass -File "<folder proyek>\packaging\uninstall.ps1"
#>
#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$built = Join-Path $root 'target\release\notch.exe'
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\NotchWindows'
$installed = Join-Path $installDir 'notch.exe'
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'

Get-Process notch -ErrorAction SilentlyContinue | Where-Object { $_.Path -in @($built, $installed) } | Stop-Process -Force
Start-Sleep -Milliseconds 500
Get-AppxPackage -Name NotchWindows | Remove-AppxPackage
Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Subject -eq 'CN=NotchWindows Pribadi' | Remove-Item
Remove-ItemProperty $run -Name NotchWindows -ErrorAction SilentlyContinue
if (Test-Path $installed) { Remove-Item $installed -Force }
if ((Test-Path $installDir) -and -not (Get-ChildItem $installDir)) { Remove-Item $installDir }

Write-Host 'Notch sudah dicabut. Datanya tetap ada di %APPDATA%\NotchWindows.'
