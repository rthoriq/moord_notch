<#
  Pasang notch sebagai paket "sparse" supaya bisa membaca notifikasi pesan (WhatsApp, dll.).
  Cukup sekali, dan diulang hanya kalau isi folder packaging berubah. Jalankan di PowerShell
  "Run as Administrator", setelah `cargo build --release`:

      powershell -ExecutionPolicy Bypass -File packaging\install.ps1

  Yang dilakukan:
  1. Membuat sertifikat tanda tangan pribadi "CN=NotchWindows Pribadi", hanya di laptop ini.
  2. Membungkus packaging\package jadi target\package\NotchWindows.msix lalu menandatanganinya.
  3. Memercayai sertifikat itu (LocalMachine\TrustedPeople), lalu MENGHAPUS kunci privatnya,
     jadi sertifikat ini tidak bisa dipakai menandatangani apa pun lagi.
  4. Mendaftarkan paket dengan notch.exe tetap di target\release, lalu menjalankan notch lagi.

  Tidak ada yang diunduh. Untuk mencabut semuanya: packaging\uninstall.ps1
#>
#Requires -RunAsAdministrator
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$release = Join-Path $root 'target\release'
$exe = Join-Path $release 'notch.exe'
$source = Join-Path $PSScriptRoot 'package'
$out = Join-Path $root 'target\package'
$msix = Join-Path $out 'NotchWindows.msix'
$cer = Join-Path $out 'NotchWindows.cer'
$subject = 'CN=NotchWindows Pribadi'

if (-not (Test-Path $exe)) { throw 'notch.exe belum ada. Jalankan dulu: cargo build --release' }

# Alat dari Windows SDK, ikut terpasang bersama Visual Studio Build Tools.
$makeappx = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\10.*\x64\makeappx.exe" -ErrorAction SilentlyContinue |
    Sort-Object { [version]$_.Directory.Parent.Name } -Descending | Select-Object -First 1
if (-not $makeappx) { throw 'Windows SDK (makeappx.exe) tidak ditemukan.' }
$signtool = Join-Path $makeappx.DirectoryName 'signtool.exe'

Write-Host 'Menghentikan notch...'
Get-Process notch -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe } | Stop-Process -Force

Write-Host 'Membersihkan pemasangan lama...'
Get-AppxPackage -Name NotchWindows | Remove-AppxPackage
Get-ChildItem Cert:\LocalMachine\TrustedPeople | Where-Object Subject -eq $subject | Remove-Item

Write-Host 'Membungkus paket...'
New-Item -ItemType Directory -Force $out | Out-Null
# /nv: notch.exe sengaja tidak ada di dalam paket (letaknya di target\release).
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
Add-AppxPackage -Path $msix -ExternalLocation $release

# Dijalankan lewat Explorer supaya notch tidak ikut berjalan sebagai Administrator.
Start-Process explorer.exe -ArgumentList "`"$exe`""
Write-Host ''
Write-Host 'Selesai. Notch berjalan lagi dengan identitas paket.'
Write-Host 'Kalau Windows bertanya izin membaca notifikasi, pilih Yes/Izinkan.'
Write-Host 'Pilih jenis pesan di ikon tray: Pengaturan > Pesan masuk.'
