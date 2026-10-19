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
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Pesan diagnostik, hanya muncul di build debug (yang punya jendela konsol).
macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*);
        }
    };
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
mod lyrics;
mod media;
mod messages;
mod network;
mod power;
mod privacy;
mod render;
mod services;
mod settings;
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
    // Pemeriksaan kabar pesan: `notch.exe --cek-notifikasi [file] [detik]` (lihat README).
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--cek-notifikasi") {
        let out = args.get(2).map_or_else(|| std::env::temp_dir().join("notch-cek-notifikasi.txt"), Into::into);
        let listen = std::time::Duration::from_secs(args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0));
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if let Err(e) = messages::diagnose(&out, listen) {
            debug_log!("hasil pemeriksaan tidak bisa ditulis: {e}");
        }
        return Ok(());
    }
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
