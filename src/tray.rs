//! Ikon di system tray beserta menunya.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, HICON, HMENU,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_DEFAULT, MF_GRAYED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TRACK_POPUP_MENU_FLAGS, TrackPopupMenu, WM_APP, WM_NULL,
};
use windows::core::{HSTRING, PCWSTR};

use crate::settings::{Messages, Settings, Size, Theme};
use crate::widgets::Page;

/// Pesan yang dikirim Windows saat ikon tray diklik.
pub const WM_TRAY: u32 = WM_APP + 1;

const ID_SETTINGS: usize = 1;
const ID_VISIBLE: usize = 2;
const ID_EXIT: usize = 3;
/// Menu klik kanan panel.
const ID_MOVE_LEFT: usize = 1;
const ID_MOVE_RIGHT: usize = 2;
const ID_HIDE_PAGE: usize = 3;
const ID_SHOW_PAGE: usize = 10;

/// Perintah dari jendela pengaturan, menu tray, atau panel; semuanya dijalankan `App::on_command`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    OpenSettings,
    ToggleVisible,
    ToggleAutostart,
    ToggleSound,
    ToggleLyrics,
    ToggleLockKeys,
    ToggleBrightness,
    ToggleHideContent,
    ToggleHotkey,
    ToggleWifi,
    ToggleDownloads,
    ToggleScreenshots,
    ToggleBrowserMessages,
    ToggleMeeting,
    ToggleCalls,
    /// Hitung atau abaikan aplikasi ke-n di daftar aplikasi panggilan.
    ToggleCallApp(usize),
    ToggleWeather,
    ToggleCompactHud,
    ToggleCalendar,
    /// Pil tetap utuh saat jendela di-maximize.
    ToggleKeepPill,
    TogglePrayerReminder,
    TogglePrayerCountdown,
    /// Geser koreksi menit waktu sholat ke-n (Subuh, Dzuhur, Ashar, Maghrib, Isya) sebanyak
    /// -1 atau +1.
    PrayerOffset(usize, i8),
    /// Siapkan identitas paket supaya pesan masuk bisa dibaca (lihat `identity`).
    ActivateMessages,
    OpenLogs,
    /// Tampilkan/sembunyikan halaman panel.
    TogglePage(Page),
    /// Geser halaman ke kiri (`-1`) atau kanan (`1`) di urutan panel.
    MovePage(Page, i32),
    /// Buka panel di halaman kontrol dan minta nama kota cuaca.
    EditCity,
    Messages(Messages),
    Size(Size),
    Theme(Theme),
    Exit,
}

/// Pilihan di menu klik kanan panel, untuk halaman yang sedang dibuka.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PageCommand {
    /// Geser ke kiri (`-1`) atau kanan (`1`).
    Move(i32),
    Hide,
    Show(Page),
}

pub struct PageMenu {
    pub page: Page,
    pub can_left: bool,
    pub can_right: bool,
    /// Halaman yang sedang disembunyikan, untuk ditampilkan lagi.
    pub hidden: Vec<Page>,
}

/// Keadaan yang ditampilkan jendela pengaturan.
#[derive(Clone)]
pub struct MenuState {
    pub visible: bool,
    pub autostart: bool,
    pub settings: Settings,
    /// Notch terpasang sebagai paket, jadi notifikasi pesan bisa dibaca kalau diizinkan.
    pub messages_available: bool,
    /// Pengguna menolak izin membaca notifikasi.
    pub messages_denied: bool,
    /// Pintasan keyboard gagal didaftarkan (sudah dipakai aplikasi lain).
    pub hotkey_taken: bool,
    /// Notch berjalan dari folder pemasangan, jadi pesan masuk bisa diaktifkan.
    pub can_activate: bool,
    /// Kota cuaca yang tersimpan.
    pub city: Option<String>,
    /// Aplikasi yang pernah memakai mikrofon, dan apakah dihitung sebagai panggilan.
    pub call_apps: Vec<(String, bool)>,
}

pub struct Tray {
    data: NOTIFYICONDATAW,
}

impl Tray {
    pub fn new(hwnd: HWND, icon: HICON) -> Self {
        let mut data = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAY,
            hIcon: icon,
            ..Default::default()
        };
        for (dst, src) in data.szTip.iter_mut().zip("Notch".encode_utf16()) {
            *dst = src;
        }
        let tray = Self { data };
        tray.add();
        tray
    }

    /// Dipanggil lagi saat Explorer restart, karena ikon tray ikut hilang.
    pub fn add(&self) {
        let _ = unsafe { Shell_NotifyIconW(NIM_ADD, &self.data) };
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &self.data);
            let _ = DestroyIcon(self.data.hIcon);
        }
    }
}

fn check(on: bool) -> MENU_ITEM_FLAGS {
    if on { MF_CHECKED } else { MF_UNCHECKED }
}

fn item(menu: HMENU, flags: MENU_ITEM_FLAGS, id: usize, label: &str) {
    let _ = unsafe { AppendMenuW(menu, MF_STRING | flags, id, &HSTRING::from(label)) };
}

/// Menu klik kanan ikon tray: buka pengaturan, tampilkan/sembunyikan notch, atau keluar. Semua
/// pengaturan lain ada di jendela pengaturan. Menu menjalankan loop pesannya sendiri, jadi jangan
/// dipanggil sambil memegang state aplikasi.
pub fn show_menu(hwnd: HWND, visible: bool) -> Option<Command> {
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        item(menu, MF_DEFAULT, ID_SETTINGS, "Pengaturan…");
        item(menu, check(visible), ID_VISIBLE, "Tampilkan notch");
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        item(menu, MF_UNCHECKED, ID_EXIT, "Keluar");
        match track(hwnd, menu, TPM_BOTTOMALIGN) {
            ID_SETTINGS => Some(Command::OpenSettings),
            ID_VISIBLE => Some(Command::ToggleVisible),
            ID_EXIT => Some(Command::Exit),
            _ => None,
        }
    }
}

/// Menu klik kanan panel: geser atau sembunyikan halaman yang sedang dibuka, atau tampilkan
/// lagi halaman yang disembunyikan. Seperti [`show_menu`], jangan dipanggil sambil memegang
/// state aplikasi.
pub fn show_page_menu(hwnd: HWND, state: &PageMenu) -> Option<PageCommand> {
    let enabled = |on: bool| if on { MF_UNCHECKED } else { MF_GRAYED };
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        item(menu, enabled(state.can_left), ID_MOVE_LEFT, "Geser halaman ke kiri");
        item(menu, enabled(state.can_right), ID_MOVE_RIGHT, "Geser halaman ke kanan");
        let hide = format!("Sembunyikan halaman {}", state.page.label());
        item(menu, enabled(state.page != Page::Home), ID_HIDE_PAGE, &hide);
        if !state.hidden.is_empty() {
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            for (i, page) in state.hidden.iter().enumerate() {
                item(menu, MF_UNCHECKED, ID_SHOW_PAGE + i, &format!("Tampilkan lagi: {}", page.label()));
            }
        }
        match track(hwnd, menu, TRACK_POPUP_MENU_FLAGS(0)) {
            ID_MOVE_LEFT => Some(PageCommand::Move(-1)),
            ID_MOVE_RIGHT => Some(PageCommand::Move(1)),
            ID_HIDE_PAGE => Some(PageCommand::Hide),
            id if id >= ID_SHOW_PAGE => state.hidden.get(id - ID_SHOW_PAGE).map(|p| PageCommand::Show(*p)),
            _ => None,
        }
    }
}

/// Tampilkan menu di posisi kursor, tunggu pilihan, lalu hancurkan menunya.
unsafe fn track(hwnd: HWND, menu: HMENU, align: TRACK_POPUP_MENU_FLAGS) -> usize {
    unsafe {
        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        // Tanpa ini menu tidak tertutup saat pengguna mengklik di luar menu.
        let _ = SetForegroundWindow(hwnd);
        let flags = TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | align;
        let chosen = TrackPopupMenu(menu, flags, cursor.x, cursor.y, None, hwnd, None).0 as usize;
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        chosen
    }
}
