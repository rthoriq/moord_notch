//! Notch pribadi untuk Windows.
//!
//! Tahap 1: jendela notch di tengah atas layar yang membuka saat di-hover,
//! jam, sembunyi saat ada aplikasi full-screen, ikon tray, dan autostart.
//! Tahap 2: kontrol musik/video, serta kabar singkat volume, daya, dan Bluetooth.
//! Tahap 3: halaman shelf file, pomodoro, pengingat minum, dan daftar tugas.
//! Tahap 4: indikator kamera/mikrofon, lirik opsional, monitor sistem, dan tema.
//! Tahap 5: pintasan Win+Alt+N, atur halaman dan ukuran, maskot berekspresi, screenshot ke
//! shelf, kabar unduhan dan Wi-Fi, serta suhu GPU.
//! Tahap 6: kontrol cepat, timer, cuaca opsional, mode rapat, dan pemasangan di lokasi tetap.
//! Tahap 7: tampilan ala notch Mac (seret file, buka/tutup, bayangan, intip, kabar ringkas).
//! Tahap 8: kalender dan agenda di Beranda, gestur touchpad, unduhan berjalan, zona Bagikan,
//! dan maskot bangun.
//! Tahap 9: tampilan musik baru, kolom lirik yang bergeser masuk, seret bar untuk lompat, dan
//! pendaran warna sampul.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Pesan diagnostik, hanya muncul di build debug (yang punya jendela konsol).
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*);
        }
    };
}

/// Kegagalan yang juga dicatat di versi rilis, ke log lokal (lihat `log`). Jangan sertakan isi
/// pesan, judul lagu, atau nama file/folder pengguna.
macro_rules! error_log {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        debug_log!("{message}");
        crate::log::write(message);
    }};
}

mod anim;
mod app;
mod audio;
mod autostart;
mod bluetooth;
mod brightness;
mod clock;
mod desktop;
mod dragdrop;
mod http;
mod folders;
mod identity;
mod install;
mod log;
mod lyrics;
mod media;
mod messages;
mod network;
mod power;
mod privacy;
mod render;
mod services;
mod settings;
mod share;
mod sound;
mod store;
mod sysinfo;
mod thumbnail;
mod tray;
mod weather;
mod widgets;

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::core::{Result, w};

fn main() -> Result<()> {
    log::catch_panics();
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        // Pemeriksaan kabar pesan: `notch.exe --cek-notifikasi [file] [detik]` (lihat README).
        Some("--cek-notifikasi") => {
            let out = args.get(2).map_or_else(|| std::env::temp_dir().join("notch-cek-notifikasi.txt"), Into::into);
            let listen = std::time::Duration::from_secs(args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0));
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            if let Err(e) = messages::diagnose(&out, listen) {
                debug_log!("hasil pemeriksaan tidak bisa ditulis: {e}");
            }
            return Ok(());
        }
        // Langkah Administrator untuk pesan masuk (lihat `identity`), tanpa masukan dari luar.
        Some("--siapkan-identitas") => {
            let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            let result = identity::prepare();
            if let Err(e) = &result {
                error_log!("paket identitas gagal disiapkan: {e}");
            }
            std::process::exit(i32::from(result.is_err()));
        }
        Some("--hapus-sertifikat") => std::process::exit(i32::from(identity::remove_certificates().is_err())),
        Some("--daftarkan-identitas") => {
            register_identity();
            return Ok(());
        }
        // Pasang tanpa bertanya.
        Some("--pasang") => std::process::exit(i32::from(!install::install_quietly())),
        // Dari Pengaturan > Aplikasi.
        Some("--cabut") => {
            install::uninstall(args.get(2).map(String::as_str) == Some("--diam"));
            return Ok(());
        }
        _ => {}
    }
    // Ditawari memasang kalau dijalankan dari luar folder pemasangan (misal folder Unduhan).
    if install::offer() {
        return Ok(());
    }
    // Sisa salinan sementara dari langkah pendaftaran pesan masuk.
    let _ = std::fs::remove_file(std::env::temp_dir().join(identity::REGISTER_COPY));
    unsafe {
        // Harus sebelum jendela apa pun dibuat, supaya notch tajam di semua skala layar.
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // Hanya satu salinan yang boleh berjalan. Handle sengaja tidak ditutup
        // supaya mutex tetap hidup sampai proses selesai.
        let _mutex = CreateMutexW(None, true, w!("Local\\NotchWindows.SingleInstance"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return Ok(());
        }
    }
    app::run()
}

/// Dijalankan dari salinan sementara notch (tanpa identitas paket): tunggu notch lama keluar,
/// daftarkan paket identitas, lalu jalankan lagi notch dari folder pemasangan.
fn register_identity() {
    let Some(dir) = install::dir() else { return };
    install::stop_running();
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if let Err(e) = identity::register(&dir) {
        error_log!("paket identitas gagal didaftarkan: {e}");
        let text = format!("Pesan masuk gagal diaktifkan:
{e}

Notch tetap bisa dipakai seperti biasa.");
        install::ask(&text, windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR);
    }
    install::launch(&dir.join("notch.exe"), None);
}
