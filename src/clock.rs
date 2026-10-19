//! Waktu lokal dalam format Indonesia.

use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::System::SystemInformation::{GetLocalTime, GetTickCount64};
use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToFileTime};

const HARI: [&str; 7] = ["Minggu", "Senin", "Selasa", "Rabu", "Kamis", "Jumat", "Sabtu"];
const BULAN: [&str; 12] = [
    "Januari", "Februari", "Maret", "April", "Mei", "Juni", "Juli", "Agustus", "September",
    "Oktober", "November", "Desember",
];
/// Satu hari dalam satuan FILETIME (100 nanodetik).
const DAY_TICKS: u64 = 24 * 60 * 60 * 10_000_000;

pub fn now() -> SYSTEMTIME {
    unsafe { GetLocalTime() }
}

/// "19:21"
pub fn time(t: &SYSTEMTIME) -> String {
    format!("{:02}:{:02}", t.wHour, t.wMinute)
}

/// "Senin, 19 Oktober 2026"
pub fn date(t: &SYSTEMTIME) -> String {
    format!(
        "{}, {} {} {}",
        HARI[t.wDayOfWeek as usize % 7],
        t.wDay,
        BULAN[(t.wMonth as usize + 11) % 12],
        t.wYear
    )
}

/// Kunci harian untuk penyimpanan, misal "2026-10-19".
pub fn day_key(t: &SYSTEMTIME) -> String {
    format!("{:04}-{:02}-{:02}", t.wYear, t.wMonth, t.wDay)
}

pub fn greeting(t: &SYSTEMTIME) -> &'static str {
    match t.wHour {
        4..=10 => "Selamat pagi",
        11..=14 => "Selamat siang",
        15..=17 => "Selamat sore",
        _ => "Selamat malam",
    }
}

/// `n` hari terakhir, terlama dulu dan hari ini terakhir: kunci harian dan huruf awal nama harinya.
pub fn recent_days(n: usize) -> Vec<(String, char)> {
    let today = now();
    let mut ft = FILETIME::default();
    if unsafe { SystemTimeToFileTime(&today, &mut ft) }.is_err() {
        return Vec::new();
    }
    let ticks = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
    (0..n as u64)
        .rev()
        .filter_map(|back| {
            let t = ticks - back * DAY_TICKS;
            let ft = FILETIME { dwLowDateTime: t as u32, dwHighDateTime: (t >> 32) as u32 };
            let mut day = SYSTEMTIME::default();
            unsafe { FileTimeToSystemTime(&ft, &mut day) }.ok()?;
            let initial = HARI[day.wDayOfWeek as usize % 7].chars().next().unwrap_or(' ');
            Some((day_key(&day), initial))
        })
        .collect()
}

/// Jam dinding `minutes` menit dari sekarang, misal "19:37".
pub fn time_in(minutes: u32) -> String {
    let t = now();
    let total = (t.wHour as u32 * 60 + t.wMinute as u32 + minutes) % (24 * 60);
    format!("{:02}:{:02}", total / 60, total % 60)
}

/// Lama komputer menyala sejak terakhir dinyalakan: "3 jam 12 mnt" atau "2 hari 5 jam".
pub fn uptime() -> String {
    let minutes = unsafe { GetTickCount64() } / 60_000;
    let (days, hours, mins) = (minutes / 1440, minutes / 60 % 24, minutes % 60);
    if days > 0 { format!("{days} hari {hours} jam") } else { format!("{hours} jam {mins} mnt") }
}

/// Durasi lagu: "1:06", atau "1:02:03" kalau lebih dari sejam.
pub fn duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let (h, m, s) = (total / 3600, total / 60 % 60, total % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// Milidetik sampai menit berikutnya, plus sedikit jeda supaya pasti sudah lewat.
pub fn ms_to_next_minute(t: &SYSTEMTIME) -> u32 {
    let elapsed = t.wSecond as u32 * 1000 + t.wMilliseconds as u32;
    60_000u32.saturating_sub(elapsed) + 15
}
