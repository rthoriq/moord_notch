//! Ikon di system tray beserta menunya.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, HICON, HMENU,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TRACK_POPUP_MENU_FLAGS, TrackPopupMenu, WM_APP, WM_NULL,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::settings::{Messages, Settings, Size, Theme};
use crate::widgets::Page;

/// Pesan yang dikirim Windows saat ikon tray diklik.
pub const WM_TRAY: u32 = WM_APP + 1;

const ID_VISIBLE: usize = 1;
const ID_AUTOSTART: usize = 2;
const ID_SOUND: usize = 3;
const ID_LYRICS: usize = 4;
const ID_LOCK_KEYS: usize = 5;
const ID_BRIGHTNESS: usize = 6;
const ID_HIDE_CONTENT: usize = 7;
const ID_HOTKEY: usize = 8;
const ID_EXIT: usize = 9;
const ID_WIFI: usize = 10;
const ID_DOWNLOADS: usize = 11;
const ID_SCREENSHOTS: usize = 12;
const ID_BROWSER_MESSAGES: usize = 13;
const ID_MEETING: usize = 14;
const ID_WEATHER: usize = 15;
/// Pilihan berurutan mulai dari id ini.
const ID_MESSAGES: usize = 50;
const ID_PAGES: usize = 60;
const ID_SIZE: usize = 70;
const ID_THEME: usize = 100;
/// Menu klik kanan panel.
const ID_MOVE_LEFT: usize = 1;
const ID_MOVE_RIGHT: usize = 2;
const ID_HIDE_PAGE: usize = 3;
const ID_SHOW_PAGE: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
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
    ToggleWeather,
    /// Tampilkan/sembunyikan halaman panel.
    TogglePage(Page),
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

/// Tampilkan menu dan tunggu pilihan pengguna. Menu menjalankan loop pesannya sendiri,
/// jadi jangan dipanggil sambil memegang state aplikasi.
pub fn show_menu(hwnd: HWND, state: MenuState) -> Option<Command> {
    let s = state.settings;
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        let options = CreatePopupMenu().ok()?;
        let messages = CreatePopupMenu().ok()?;
        let themes = CreatePopupMenu().ok()?;
        let pages = CreatePopupMenu().ok()?;
        let sizes = CreatePopupMenu().ok()?;

        for (i, mode) in Messages::ALL.iter().enumerate() {
            let usable = *mode == Messages::Off || state.messages_available;
            let flags = check(*mode == s.messages) | if usable { MF_UNCHECKED } else { MF_GRAYED };
            item(messages, flags, ID_MESSAGES + i, mode.label());
        }
        if !state.messages_available {
            item(messages, MF_GRAYED, 0, "Perlu dipasang sebagai paket (lihat README)");
        } else if state.messages_denied && s.messages != Messages::Off {
            item(messages, MF_GRAYED, 0, "Izin ditolak: Pengaturan Windows > Privasi > Notifikasi");
        }
        let _ = AppendMenuW(messages, MF_SEPARATOR, 0, PCWSTR::null());
        let browser_usable = state.messages_available && s.messages != Messages::Off;
        let browser_flags = check(s.browser_messages) | if browser_usable { MF_UNCHECKED } else { MF_GRAYED };
        item(messages, browser_flags, ID_BROWSER_MESSAGES, "Notifikasi browser (WhatsApp Web, dll.)");
        item(messages, check(s.hide_content), ID_HIDE_CONTENT, "Sembunyikan isi pesan");

        item(options, check(s.sound), ID_SOUND, "Suara notifikasi");
        item(options, check(s.lyrics), ID_LYRICS, "Lirik lagu (memakai internet)");
        item(options, check(s.weather), ID_WEATHER, "Cuaca (memakai internet)");
        item(options, check(s.meeting), ID_MEETING, "Mode rapat (saat kamera/mikrofon dipakai)");
        let hotkey = if s.hotkey && state.hotkey_taken {
            "Pintasan Win+Alt+N (dipakai aplikasi lain)"
        } else {
            "Pintasan Win+Alt+N"
        };
        item(options, check(s.hotkey), ID_HOTKEY, hotkey);
        let _ = AppendMenuW(options, MF_SEPARATOR, 0, PCWSTR::null());
        item(options, check(s.lock_keys), ID_LOCK_KEYS, "Kabar Caps Lock / Num Lock");
        item(options, check(s.brightness), ID_BRIGHTNESS, "Kabar kecerahan layar");
        item(options, check(s.wifi), ID_WIFI, "Kabar Wi-Fi");
        item(options, check(s.downloads), ID_DOWNLOADS, "Kabar unduhan selesai");
        item(options, check(s.screenshots), ID_SCREENSHOTS, "Screenshot masuk shelf");
        // Submenu ikut dihancurkan bersama menu induknya.
        let _ = AppendMenuW(options, MF_POPUP, messages.0 as usize, w!("Pesan masuk"));

        for (i, theme) in Theme::ALL.iter().enumerate() {
            item(themes, check(*theme == s.theme), ID_THEME + i, theme.label());
        }
        for (i, size) in Size::ALL.iter().enumerate() {
            item(sizes, check(*size == s.size), ID_SIZE + i, size.label());
        }
        for page in s.pages.order() {
            let index = Page::ALL.iter().position(|p| *p == page).unwrap_or(0);
            let locked = if page == Page::Home { MF_GRAYED } else { MF_UNCHECKED };
            item(pages, check(s.pages.shown(page)) | locked, ID_PAGES + index, page.label());
        }
        let _ = AppendMenuW(pages, MF_SEPARATOR, 0, PCWSTR::null());
        item(pages, MF_GRAYED, 0, "Urutan: klik kanan panel");

        item(menu, check(state.visible), ID_VISIBLE, "Tampilkan notch");
        item(menu, check(state.autostart), ID_AUTOSTART, "Mulai bersama Windows");
        let _ = AppendMenuW(menu, MF_POPUP, options.0 as usize, w!("Pengaturan"));
        let _ = AppendMenuW(menu, MF_POPUP, pages.0 as usize, w!("Halaman"));
        let _ = AppendMenuW(menu, MF_POPUP, sizes.0 as usize, w!("Ukuran"));
        let _ = AppendMenuW(menu, MF_POPUP, themes.0 as usize, w!("Tema"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        item(menu, MF_UNCHECKED, ID_EXIT, "Keluar");

        let chosen = track(hwnd, menu, TPM_BOTTOMALIGN);
        match chosen {
            ID_VISIBLE => Some(Command::ToggleVisible),
            ID_AUTOSTART => Some(Command::ToggleAutostart),
            ID_SOUND => Some(Command::ToggleSound),
            ID_LYRICS => Some(Command::ToggleLyrics),
            ID_LOCK_KEYS => Some(Command::ToggleLockKeys),
            ID_BRIGHTNESS => Some(Command::ToggleBrightness),
            ID_HIDE_CONTENT => Some(Command::ToggleHideContent),
            ID_HOTKEY => Some(Command::ToggleHotkey),
            ID_WIFI => Some(Command::ToggleWifi),
            ID_DOWNLOADS => Some(Command::ToggleDownloads),
            ID_SCREENSHOTS => Some(Command::ToggleScreenshots),
            ID_BROWSER_MESSAGES => Some(Command::ToggleBrowserMessages),
            ID_MEETING => Some(Command::ToggleMeeting),
            ID_WEATHER => Some(Command::ToggleWeather),
            ID_EXIT => Some(Command::Exit),
            id if id >= ID_THEME => Theme::ALL.get(id - ID_THEME).map(|t| Command::Theme(*t)),
            id if id >= ID_SIZE => Size::ALL.get(id - ID_SIZE).map(|s| Command::Size(*s)),
            id if id >= ID_PAGES => Page::ALL.get(id - ID_PAGES).map(|p| Command::TogglePage(*p)),
            id if id >= ID_MESSAGES => Messages::ALL.get(id - ID_MESSAGES).map(|m| Command::Messages(*m)),
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
