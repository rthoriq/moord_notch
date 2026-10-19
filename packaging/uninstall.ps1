<#
  Cabut paket notch dan sertifikatnya. Notch tetap bisa dipakai seperti biasa, hanya tanpa
  kabar pesan masuk. Jalankan di PowerShell "Run as Administrator":

      powershell -ExecutionPolicy Bypass -File packaging\uninstall.ps1
#>
#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root 'target\release\notch.exe'

Get-Process notch -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe } | Stop-Process -Force
Get-AppxPackage -Name NotchWindows | Remove-AppxPackage
Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Subject -eq 'CN=NotchWindows Pribadi' | Remove-Item

if (Test-Path $exe) {
    Start-Process explorer.exe -ArgumentList "`"$exe`""
}
Write-Host 'Paket dan sertifikat notch sudah dicabut.'
