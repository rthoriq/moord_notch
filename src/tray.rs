//! Ikon di system tray beserta menunya.

use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos, HICON, HMENU,
    MENU_ITEM_FLAGS, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED,
    PostMessageW, SetForegroundWindow, TPM_BOTTOMALIGN, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP, WM_NULL,
};
use windows::core::{HSTRING, PCWSTR, w};

use crate::settings::{Messages, Settings, Theme};

/// Pesan yang dikirim Windows saat ikon tray diklik.
pub const WM_TRAY: u32 = WM_APP + 1;

const ID_VISIBLE: usize = 1;
const ID_AUTOSTART: usize = 2;
const ID_SOUND: usize = 3;
const ID_LYRICS: usize = 4;
const ID_LOCK_KEYS: usize = 5;
const ID_BRIGHTNESS: usize = 6;
const ID_HIDE_CONTENT: usize = 7;
const ID_EXIT: usize = 9;
/// Pilihan berurutan mulai dari id ini.
const ID_MESSAGES: usize = 50;
const ID_THEME: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    ToggleVisible,
    ToggleAutostart,
    ToggleSound,
    ToggleLyrics,
    ToggleLockKeys,
    ToggleBrightness,
    ToggleHideContent,
    Messages(Messages),
    Theme(Theme),
    Exit,
}

pub struct MenuState {
    pub visible: bool,
    pub autostart: bool,
    pub settings: Settings,
    /// Notch terpasang sebagai paket, jadi notifikasi pesan bisa dibaca kalau diizinkan.
    pub messages_available: bool,
    /// Pengguna menolak izin membaca notifikasi.
    pub messages_denied: bool,
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
        item(messages, check(s.hide_content), ID_HIDE_CONTENT, "Sembunyikan isi pesan");

        item(options, check(s.sound), ID_SOUND, "Suara notifikasi");
        item(options, check(s.lyrics), ID_LYRICS, "Lirik lagu (memakai internet)");
        item(options, check(s.lock_keys), ID_LOCK_KEYS, "Kabar Caps Lock / Num Lock");
        item(options, check(s.brightness), ID_BRIGHTNESS, "Kabar kecerahan layar");
        // Submenu ikut dihancurkan bersama menu induknya.
        let _ = AppendMenuW(options, MF_POPUP, messages.0 as usize, w!("Pesan masuk"));

        for (i, theme) in Theme::ALL.iter().enumerate() {
            item(themes, check(*theme == s.theme), ID_THEME + i, theme.label());
        }

        item(menu, check(state.visible), ID_VISIBLE, "Tampilkan notch");
        item(menu, check(state.autostart), ID_AUTOSTART, "Mulai bersama Windows");
        let _ = AppendMenuW(menu, MF_POPUP, options.0 as usize, w!("Pengaturan"));
        let _ = AppendMenuW(menu, MF_POPUP, themes.0 as usize, w!("Tema"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        item(menu, MF_UNCHECKED, ID_EXIT, "Keluar");

        let mut cursor = POINT::default();
        let _ = GetCursorPos(&mut cursor);
        // Tanpa ini menu tidak tertutup saat pengguna mengklik di luar menu.
        let _ = SetForegroundWindow(hwnd);
        let flags = TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN;
        let chosen = TrackPopupMenu(menu, flags, cursor.x, cursor.y, None, hwnd, None).0 as usize;
        let _ = DestroyMenu(menu);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));

        match chosen {
            ID_VISIBLE => Some(Command::ToggleVisible),
            ID_AUTOSTART => Some(Command::ToggleAutostart),
            ID_SOUND => Some(Command::ToggleSound),
            ID_LYRICS => Some(Command::ToggleLyrics),
            ID_LOCK_KEYS => Some(Command::ToggleLockKeys),
            ID_BRIGHTNESS => Some(Command::ToggleBrightness),
            ID_HIDE_CONTENT => Some(Command::ToggleHideContent),
            ID_EXIT => Some(Command::Exit),
            id if id >= ID_THEME => Theme::ALL.get(id - ID_THEME).map(|t| Command::Theme(*t)),
            id if id >= ID_MESSAGES => Messages::ALL.get(id - ID_MESSAGES).map(|m| Command::Messages(*m)),
            _ => None,
        }
    }
}
