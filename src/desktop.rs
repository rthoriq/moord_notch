//! Memantau jendela aktif supaya notch tahu kapan harus minggir: sembunyi saat ada
//! aplikasi full-screen, dan mengecil jadi garis tipis saat jendela di-maximize.
//!
//! Sumber kabarnya dipilih yang jarang berbunyi: ganti jendela aktif, minimize, selesai
//! geser/ubah ukuran, dan notifikasi full-screen dari Explorer (AppBar). Hook posisi jendela
//! sengaja tidak dipakai karena aplikasi yang sibuk (browser, Electron) bisa mengirim
//! ratusan event per detik. Maximize lewat tombol tidak punya event sendiri, jadi dicek
//! ringan sekali per detik oleh jendela notch.

use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{HMONITOR, MONITOR_DEFAULTTONULL, MonitorFromWindow};
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Shell::{
    ABM_NEW, ABM_REMOVE, APPBARDATA, QUNS_BUSY, QUNS_PRESENTATION_MODE,
    QUNS_RUNNING_D3D_FULL_SCREEN, SHAppBarMessage, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
    EVENT_SYSTEM_MOVESIZEEND, GetClassNameW, GetForegroundWindow, GetWindowRect, IsIconic,
    IsWindowVisible, IsZoomed, PostMessageW, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    WM_APP,
};

/// Dikirim ke jendela notch saat jendela aktif berganti, di-minimize, atau selesai digeser.
pub const WM_DESKTOP_CHANGED: u32 = WM_APP + 2;
/// Dikirim Explorer saat ada aplikasi masuk/keluar full-screen.
pub const WM_APPBAR: u32 = WM_APP + 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    /// Jendela aktif di-maximize di monitor notch: notch mengecil supaya tidak menutupi tab.
    Maximized,
    /// Ada aplikasi full-screen (game, video, presentasi): notch disembunyikan.
    Fullscreen,
}

/// Jendela yang menerima pemberitahuan dari callback hook.
static TARGET: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
/// Berapa kali callback hook dipanggil; untuk memantau seberapa sering proses dibangunkan.
static CALLBACKS: AtomicU32 = AtomicU32::new(0);

/// Jumlah callback sejak pemanggilan sebelumnya.
pub fn take_callback_count() -> u32 {
    CALLBACKS.swap(0, Ordering::Relaxed)
}

pub struct Watcher {
    hooks: Vec<HWINEVENTHOOK>,
    appbar: APPBARDATA,
}

impl Watcher {
    pub fn new(notify: HWND) -> Self {
        TARGET.store(notify.0, Ordering::Relaxed);
        let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
        let hooks = [
            (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
            (EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZEEND),
            (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
        ]
        .into_iter()
        .map(|(min, max)| unsafe { SetWinEventHook(min, max, None, Some(on_event), 0, 0, flags) })
        .filter(|h| !h.is_invalid())
        .collect();

        // Mendaftar sebagai AppBar tanpa memesan ruang layar, hanya untuk menerima kabar full-screen.
        let mut appbar = APPBARDATA {
            cbSize: size_of::<APPBARDATA>() as u32,
            hWnd: notify,
            uCallbackMessage: WM_APPBAR,
            ..Default::default()
        };
        unsafe { SHAppBarMessage(ABM_NEW, &mut appbar) };
        Self { hooks, appbar }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        unsafe {
            for h in self.hooks.drain(..) {
                let _ = UnhookWinEvent(h);
            }
            SHAppBarMessage(ABM_REMOVE, &mut self.appbar);
        }
    }
}

unsafe extern "system" fn on_event(
    _hook: HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    CALLBACKS.fetch_add(1, Ordering::Relaxed);
    let target = HWND(TARGET.load(Ordering::Relaxed));
    let _ = unsafe { PostMessageW(Some(target), WM_DESKTOP_CHANGED, WPARAM(0), LPARAM(0)) };
}

/// Tentukan mode notch berdasarkan jendela yang sedang aktif di monitor notch.
/// Pemeriksaan murah didahulukan karena fungsi ini juga dipanggil berkala.
pub fn current_mode(own: HWND, monitor: HMONITOR, area: &RECT) -> Mode {
    unsafe {
        let fg = GetForegroundWindow();
        if fg.is_invalid()
            || fg == own
            || !IsWindowVisible(fg).as_bool()
            || IsIconic(fg).as_bool()
            || MonitorFromWindow(fg, MONITOR_DEFAULTTONULL) != monitor
            || is_shell(fg)
        {
            return Mode::Normal;
        }
        let zoomed = IsZoomed(fg).as_bool();
        let mut r = RECT::default();
        let covers = GetWindowRect(fg, &mut r).is_ok()
            && r.left <= area.left
            && r.top <= area.top
            && r.right >= area.right
            && r.bottom >= area.bottom;
        if !covers {
            return if zoomed { Mode::Maximized } else { Mode::Normal };
        }
        if !zoomed {
            return Mode::Fullscreen;
        }
        // Jendela maximize dengan taskbar auto-hide juga menutupi layar penuh; bedanya,
        // Windows sendiri tidak menganggapnya sedang full-screen.
        match SHQueryUserNotificationState() {
            Ok(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE) => Mode::Fullscreen,
            _ => Mode::Maximized,
        }
    }
}

/// Desktop dan taskbar juga bisa jadi "jendela aktif", tapi bukan aplikasi.
fn is_shell(hwnd: HWND) -> bool {
    let mut buf = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) }.clamp(0, 64) as usize;
    let class = String::from_utf16_lossy(&buf[..len]);
    matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
}
