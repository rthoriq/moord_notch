<#
  Pasang notch ke lokasi tetap (%LOCALAPPDATA%\Programs\NotchWindows), lengkap dengan identitas
  paket supaya bisa membaca notifikasi pesan (WhatsApp, dll.). Cukup sekali, dan diulang hanya
  kalau isi folder packaging berubah; untuk memasang build baru cukup packaging\update.ps1.
  Jalankan di PowerShell "Run as Administrator", setelah `cargo build --release`:

      powershell -ExecutionPolicy Bypass -File "<folder proyek>\packaging\install.ps1"

  Yang dilakukan:
  1. Menyalin target\release\notch.exe ke %LOCALAPPDATA%\Programs\NotchWindows. Build ulang di
     folder proyek tidak lagi mengganggu notch yang sedang dipakai.
  2. Membuat sertifikat tanda tangan pribadi "CN=NotchWindows Pribadi", hanya di laptop ini.
  3. Membungkus packaging\package jadi target\package\NotchWindows.msix lalu menandatanganinya.
  4. Memercayai sertifikat itu (LocalMachine\TrustedPeople), lalu MENGHAPUS kunci privatnya,
     jadi sertifikat ini tidak bisa dipakai menandatangani apa pun lagi.
  5. Mendaftarkan paket untuk notch di lokasi tetap, memindahkan "Mulai bersama Windows" ke
     sana (kalau sedang aktif), lalu menjalankan notch.

  Tidak ada yang diunduh. Untuk mencabut semuanya: packaging\uninstall.ps1
#>
#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$built = Join-Path $root 'target\release\notch.exe'
$installDir = Join-Path $env:LOCALAPPDATA 'Programs\NotchWindows'
$installed = Join-Path $installDir 'notch.exe'
$source = Join-Path $PSScriptRoot 'package'
$out = Join-Path $root 'target\package'
$msix = Join-Path $out 'NotchWindows.msix'
$cer = Join-Path $out 'NotchWindows.cer'
$subject = 'CN=NotchWindows Pribadi'
$run = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'

if (-not (Test-Path $built)) { throw 'notch.exe belum ada. Jalankan dulu: cargo build --release' }

# Alat dari Windows SDK, ikut terpasang bersama Visual Studio Build Tools.
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (-not $makeappx) { throw 'Windows SDK (makeappx.exe) tidak ditemukan.' }
$signtool = Join-Path $makeappx.DirectoryName 'signtool.exe'

Write-Host 'Menghentikan notch...'
Get-Process notch -ErrorAction SilentlyContinue | Where-Object { $_.Path -in @($built, $installed) } | Stop-Process -Force
Start-Sleep -Milliseconds 500

Write-Host 'Membersihkan pemasangan lama...'
Get-AppxPackage -Name NotchWindows | Remove-AppxPackage
Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Subject -eq $subject | Remove-Item

Write-Host "Menyalin notch ke $installDir..."
New-Item -ItemType Directory -Force $installDir | Out-Null
Copy-Item $built $installed -Force

Write-Host 'Membungkus paket...'
New-Item -ItemType Directory -Force $out | Out-Null
# /nv: notch.exe sengaja tidak ada di dalam paket (letaknya di folder pemasangan).
& $makeappx.FullName pack /o /nv /d $source /p $msix | Out-Null
if ($LASTEXITCODE) { throw "makeappx gagal (kode $LASTEXITCODE)" }

Write-Host 'Menandatangani paket...'
$cert = New-SelfSignedCertificate -Type Custom -Subject $subject -FriendlyName 'Notch Windows (pribadi)' `
    -KeyUsage DigitalSignature -KeyAlgorithm RSA -KeyLength 3072 -NotAfter (Get-Date).AddYears(5) `
    -CertStoreLocation Cert:\CurrentUser\My `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
try {
    & $signtool sign /q /fd SHA256 /sha1 $cert.Thumbprint /s My $msix
    if ($LASTEXITCODE) { throw "signtool gagal (kode $LASTEXITCODE)" }
    Export-Certificate -Cert $cert -FilePath $cer | Out-Null
    Import-Certificate -FilePath $cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople | Out-Null
} finally {
    # Kunci privat dibuang: sertifikat yang dipercaya ini tidak bisa menandatangani apa pun lagi.
    Remove-Item "Cert:\CurrentUser\My\$($cert.Thumbprint)" -DeleteKey
}

Write-Host 'Mendaftarkan paket...'
Add-AppxPackage -Path $msix -ExternalLocation $installDir

# "Mulai bersama Windows" yang masih menunjuk ke folder build dipindah ke lokasi tetap.
if (Get-ItemProperty $run -Name NotchWindows -ErrorAction SilentlyContinue) {
    Set-ItemProperty $run -Name NotchWindows -Value "`"$installed`""
    Write-Host 'Mulai bersama Windows: sekarang memakai notch di lokasi tetap.'
}

# Dijalankan lewat Explorer supaya notch tidak ikut berjalan sebagai Administrator.
Start-Process explorer.exe -ArgumentList "`"$installed`""
Write-Host ''
Write-Host "Selesai. Notch terpasang di $installDir dan sudah berjalan."
Write-Host 'Kalau Windows bertanya izin membaca notifikasi, pilih Yes/Izinkan.'
Write-Host 'Build berikutnya cukup dipasang dengan packaging\update.ps1 (tanpa Administrator).'
