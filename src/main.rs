//! Notch pribadi untuk Windows.
//!
//! Tahap 1: jendela notch di tengah atas layar yang membuka saat di-hover,
//! jam, sembunyi saat ada aplikasi full-screen, ikon tray, dan autostart.
//! Tahap 2: kontrol musik/video, serta kabar singkat volume, daya, dan Bluetooth.
//! Tahap 3: halaman shelf file, pomodoro, pengingat minum, dan daftar tugas.
//! Tahap 4: indikator kamera/mikrofon, lirik opsional, monitor sistem, dan tema.
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
mod clock;
mod desktop;
mod dragdrop;
mod lyrics;
mod media;
mod messages;
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
mod widgets;

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::core::{Result, w};

fn main() -> Result<()> {
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
