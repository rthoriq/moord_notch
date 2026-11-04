//! Jendela pengaturan: semua pengaturan notch di satu jendela bergaya Windows 11 (latar Mica,
//! terang/gelap mengikuti Windows), digambar sendiri dengan Direct2D seperti notch-nya. Setiap
//! perubahan langsung dijalankan lewat perintah yang sama dengan menu tray ([`Command`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Once;

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED,
    D2D1_FIGURE_END_CLOSED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_ELLIPSE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES, D2D1_ROUNDED_RECT,
    D2D1CreateFactory, ID2D1Brush, ID2D1Factory, ID2D1HwndRenderTarget, ID2D1RenderTarget, ID2D1SolidColorBrush,
    ID2D1StrokeStyle,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_MEASURING_MODE_NATURAL,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PARAGRAPH_ALIGNMENT_NEAR, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TEXT_METRICS, DWRITE_WORD_WRAPPING_NO_WRAP,
    DWRITE_WORD_WRAPPING_WRAP, DWriteCreateFactory,
    IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dwm::{
    DWMSBT_MAINWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_USE_IMMERSIVE_DARK_MODE, DwmExtendFrameIntoClientArea,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint, PAINTSTRUCT,
};
use windows::Win32::Storage::EnhancedStorage::{
    PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand, PKEY_AppUserModel_RelaunchDisplayNameResource,
    PKEY_AppUserModel_RelaunchIconResource,
};
use windows::Win32::System::Com::CoTaskMemAlloc;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Variant::VT_LPWSTR;
use windows::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW};
use windows::Win32::UI::Controls::{MARGINS, WM_MOUSELEAVE};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_NEXT, VK_PRIOR,
    VK_RETURN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetCursorPos,
    GetSystemMetrics, HICON, IDC_ARROW, IMAGE_ICON, IsIconic, LR_DEFAULTCOLOR, LoadCursorW, LoadImageW, MINMAXINFO,
    RegisterClassExW, SM_CXICON, SM_CXSMICON, SYSTEM_METRICS_INDEX,
    SW_RESTORE, SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_NOZORDER, SetForegroundWindow, SetWindowPos, ShowWindow,
    WINDOW_EX_STYLE, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_PAINT, WM_SETTINGCHANGE, WM_SIZE,
    WNDCLASSEXW, WS_OVERLAPPEDWINDOW,
};
use windows::core::{BOOL, HSTRING, PCWSTR, PWSTR, Result, w};
use windows_numerics::Vector2;

use crate::settings::{Messages, Size, Theme};
use crate::tray::{Command, MenuState};
use crate::widgets::Page;
use crate::widgets::prayer::Time;

const CLASS: PCWSTR = w!("NotchWindows.Pengaturan");
/// ID aplikasi jendela ini di taskbar (lihat `set_taskbar_id`).
const TASKBAR_ID: &str = "NotchWindows.Pengaturan";
const GITHUB: &str = "https://github.com/rthoriq/moord_notch";
/// Ukuran awal dan terkecil jendela (DIP).
const START_SIZE: (f32, f32) = (920.0, 660.0);
const MIN_SIZE: (f32, f32) = (720.0, 500.0);
/// Lebar daftar kategori: lebih sempit kalau jendelanya sempit, seperti Pengaturan Windows.
fn nav_width(window_width: f32) -> f32 {
    if window_width < 860.0 { 220.0 } else { 280.0 }
}
const CONTENT_MAX: f32 = 860.0;

const TEXT: &str = "Segoe UI Variable Text";
const DISPLAY: &str = "Segoe UI Variable Display";
const ICONS: &str = "Segoe Fluent Icons";

// ---------------------------------------------------------------- isi

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    General,
    Look,
    Pages,
    Notices,
    Prayer,
    Messages,
    Privacy,
    Shortcuts,
    About,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::General,
        Category::Look,
        Category::Pages,
        Category::Notices,
        Category::Prayer,
        Category::Messages,
        Category::Privacy,
        Category::Shortcuts,
        Category::About,
    ];

    fn label(self) -> &'static str {
        match self {
            Category::General => "Umum",
            Category::Look => "Tampilan",
            Category::Pages => "Halaman",
            Category::Notices => "Kabar",
            Category::Prayer => "Sholat",
            Category::Messages => "Pesan masuk",
            Category::Privacy => "Privasi & internet",
            Category::Shortcuts => "Pintasan",
            Category::About => "Tentang",
        }
    }

    fn icon(self) -> char {
        match self {
            Category::General => '\u{E713}',
            Category::Look => '\u{E790}',
            Category::Pages => '\u{E8FD}',
            Category::Notices => '\u{EA8F}',
            Category::Prayer => '\u{E708}',
            Category::Messages => '\u{E8BD}',
            Category::Privacy => '\u{E72E}',
            Category::Shortcuts => '\u{E765}',
            Category::About => '\u{E946}',
        }
    }
}

/// Yang terjadi saat sebuah kontrol diklik.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Action {
    Nav(Category),
    Run(Command),
    GitHub,
}

/// Satu baris di halaman pengaturan.
enum Row {
    /// Sakelar nyala/mati.
    Toggle { icon: char, title: String, detail: String, on: bool, enabled: bool, command: Command },
    /// Tombol di sebelah kanan.
    Button { icon: char, title: &'static str, detail: String, label: &'static str, accent: bool, action: Action },
    /// Angka dengan tombol kurangi dan tambah.
    Stepper { icon: char, title: &'static str, value: String, minus: Command, plus: Command },
    /// Pilihan berderet (salah satu).
    Choice { icon: char, title: &'static str, detail: &'static str, options: Vec<(&'static str, bool, bool, Command)> },
    /// Kartu-kartu tema dengan contoh notch.
    Themes(Theme),
    /// Satu halaman panel: tampil/sembunyi dan urutan.
    Page { page: Page, shown: bool, up: bool, down: bool },
    /// Keterangan kecil tanpa kartu; `true` = peringatan.
    Note(String, bool),
    /// Kepala halaman Tentang.
    Hero,
}

fn page_icon(page: Page) -> Option<char> {
    match page {
        Page::Home => Some('\u{E80F}'),
        Page::Controls => Some('\u{E9E9}'),
        Page::Shelf => Some('\u{E8B7}'),
        Page::Focus => Some('\u{E916}'),
        Page::Timer => Some('\u{E121}'),
        // Tetes air digambar sendiri, sama seperti di panel.
        Page::Water => None,
        Page::Tasks => Some('\u{E9D5}'),
        Page::System => Some('\u{E9D9}'),
        Page::Prayer => Some('\u{E708}'),
    }
}

fn rows(category: Category, s: &MenuState) -> Vec<Row> {
    let settings = &s.settings;
    let toggle = |icon, title: &str, detail: &str, on, command| Row::Toggle {
        icon,
        title: title.to_string(),
        detail: detail.to_string(),
        on,
        enabled: true,
        command,
    };
    match category {
        Category::General => vec![
            toggle('\u{E7B3}', "Tampilkan notch", "Sembunyikan sementara tanpa menutup aplikasinya", s.visible, Command::ToggleVisible),
            toggle('\u{E7E8}', "Mulai bersama Windows", "Notch langsung ada setiap kali laptop menyala", s.autostart, Command::ToggleAutostart),
            toggle('\u{E767}', "Suara notifikasi", "Bunyi bawaan Windows untuk pengingat, timer, charger, dan lainnya", settings.sound, Command::ToggleSound),
            Row::Button {
                icon: '\u{E8BB}',
                title: "Keluar dari NotchWindows",
                detail: "Notch berhenti sampai dibuka lagi dari menu Start".into(),
                label: "Keluar",
                accent: false,
                action: Action::Run(Command::Exit),
            },
        ],
        Category::Look => vec![
            Row::Themes(settings.theme),
            Row::Choice {
                icon: '\u{E740}',
                title: "Ukuran notch",
                detail: "Seberapa besar notch di layar",
                options: Size::ALL.iter().map(|&size| (size.label(), size == settings.size, true, Command::Size(size))).collect(),
            },
            toggle('\u{E922}', "Tetap tampil saat jendela penuh", "Notch tidak mengecil jadi garis tipis saat jendela di-maximize", settings.keep_pill, Command::ToggleKeepPill),
            toggle('\u{E787}', "Kalender di Beranda", "Beranda melebar dengan kalender dan agenda di kanan", settings.calendar, Command::ToggleCalendar),
            toggle('\u{E8A0}', "Kabar ringkas", "Volume, kecerahan, Caps Lock, dan charger setinggi pil (gaya Dynamic Island)", settings.compact_hud, Command::ToggleCompactHud),
        ],
        Category::Pages => {
            let pages = settings.pages;
            let mut rows = vec![Row::Note(
                "Urutan halaman di panel notch. Halaman yang disembunyikan ikut mematikan fiturnya, misal pengingat minum.".into(),
                false,
            )];
            rows.extend(pages.order().into_iter().map(|page| Row::Page {
                page,
                shown: pages.shown(page),
                up: pages.can_move(page, -1),
                down: pages.can_move(page, 1),
            }));
            rows
        }
        Category::Notices => {
            let mut rows = vec![
                toggle('\u{E765}', "Caps Lock dan Num Lock", "Kabar \"Nyala\" atau \"Mati\" setiap tombolnya ditekan", settings.lock_keys, Command::ToggleLockKeys),
                toggle('\u{E706}', "Kecerahan layar", "Bilah dan persen setiap kecerahan berubah", settings.brightness, Command::ToggleBrightness),
                toggle('\u{E701}', "Wi-Fi", "Tersambung, terputus, atau tanpa internet", settings.wifi, Command::ToggleWifi),
                toggle('\u{E896}', "Unduhan", "Ukuran unduhan di pil dan kabar saat selesai", settings.downloads, Command::ToggleDownloads),
                toggle('\u{E722}', "Screenshot masuk shelf", "Hasil Win+Shift+S atau PrtScn langsung siap diseret", settings.screenshots, Command::ToggleScreenshots),
                toggle(
                    '\u{E717}',
                    "Panggilan aktif",
                    "Lama panggilan dan tombol bisu di pil selama Zoom, Teams, Discord, atau aplikasi lain memakai mikrofon",
                    settings.calls,
                    Command::ToggleCalls,
                ),
            ];
            if !s.call_apps.is_empty() {
                rows.push(Row::Note(
                    "Aplikasi yang pernah memakai mikrofon. Matikan yang bukan panggilan, misal Discord yang dibiarkan di voice channel.".into(),
                    false,
                ));
                rows.extend(s.call_apps.iter().enumerate().map(|(i, (app, counted))| Row::Toggle {
                    icon: '\u{E720}',
                    title: app.clone(),
                    detail: "Dihitung sebagai panggilan saat memakai mikrofon".into(),
                    on: *counted,
                    enabled: settings.calls,
                    command: Command::ToggleCallApp(i),
                }));
            }
            rows
        }
        Category::Prayer => {
            let city = s.city.clone().unwrap_or_else(|| "Belum dipilih".into());
            let mut rows = vec![
                Row::Note(
                    "Jadwal dihitung di laptop ini dari koordinat kota cuaca dengan kriteria Kemenag RI: Subuh 20°, Isya 18°, ihtiyat 2 menit. Internet hanya dipakai sekali, saat mencari kota lewat Open-Meteo.".into(),
                    false,
                ),
                Row::Button {
                    icon: '\u{E707}',
                    title: "Kota",
                    detail: city,
                    label: "Ganti kota",
                    accent: false,
                    action: Action::Run(Command::EditCity),
                },
                toggle('\u{EA8F}', "Pengingat waktu sholat", "Kabar dan bunyi saat masuk Subuh, Dzuhur, Ashar, Maghrib, dan Isya", settings.prayer_reminder, Command::TogglePrayerReminder),
                toggle('\u{E916}', "Hitung mundur di pil", "Sisa menit tampil di pil mulai 10 menit sebelum waktu sholat", settings.prayer_countdown, Command::TogglePrayerCountdown),
                Row::Note("Koreksi menit, misal supaya sama dengan jadwal masjid terdekat.".into(), false),
            ];
            rows.extend(Time::PRAYERS.iter().zip(settings.prayer_offsets).enumerate().map(|(i, (time, offset))| Row::Stepper {
                icon: '\u{E823}',
                title: time.label(),
                value: match offset {
                    0 => "Tepat".to_string(),
                    m if m > 0 => format!("+{m} menit"),
                    m => format!("−{} menit", -m),
                },
                minus: Command::PrayerOffset(i, -1),
                plus: Command::PrayerOffset(i, 1),
            }));
            rows
        }
        Category::Messages => {
            let mut rows = Vec::new();
            if !s.messages_available && s.can_activate {
                rows.push(Row::Button {
                    icon: '\u{E8BD}',
                    title: "Aktifkan pesan masuk",
                    detail: "Windows hanya mengizinkan aplikasi berpaket membaca notifikasi; butuh izin Administrator sekali".into(),
                    label: "Aktifkan",
                    accent: true,
                    action: Action::Run(Command::ActivateMessages),
                });
            } else if !s.messages_available {
                rows.push(Row::Note("Pasang notch dulu (jalankan notch.exe lalu pilih Pasang) untuk memakai pesan masuk.".into(), true));
            } else if s.messages_denied && settings.messages != Messages::Off {
                rows.push(Row::Note("Izin ditolak. Nyalakan di Pengaturan Windows › Privasi & keamanan › Notifikasi.".into(), true));
            }
            rows.push(Row::Choice {
                icon: '\u{E8BD}',
                title: "Tampilkan pesan dari",
                detail: "Pengirim dan potongan pesannya muncul di notch",
                options: Messages::ALL
                    .iter()
                    .map(|&mode| (mode.label(), mode == settings.messages, mode == Messages::Off || s.messages_available, Command::Messages(mode)))
                    .collect(),
            });
            rows.push(Row::Toggle {
                icon: '\u{E774}',
                title: "Notifikasi browser".into(),
                detail: "WhatsApp Web dan situs chat lain, berlabel nama browsernya".into(),
                on: settings.browser_messages,
                enabled: s.messages_available && settings.messages != Messages::Off,
                command: Command::ToggleBrowserMessages,
            });
            rows.push(toggle('\u{ED1A}', "Sembunyikan isi pesan", "Hanya nama pengirim yang tampil", settings.hide_content, Command::ToggleHideContent));
            rows
        }
        Category::Privacy => {
            let city = s.city.clone().unwrap_or_else(|| "Belum dipilih".into());
            vec![
                toggle('\u{E714}', "Mode rapat", "Saat kamera atau mikrofon dipakai: pesan disamarkan dan notch tidak berbunyi", settings.meeting, Command::ToggleMeeting),
                toggle('\u{E8D6}', "Lirik lagu", "Memakai internet (LRCLIB): hanya judul, artis, album, dan durasi lagu yang dikirim", settings.lyrics, Command::ToggleLyrics),
                toggle('\u{E753}', "Cuaca", "Memakai internet (Open-Meteo): hanya nama kota yang dikirim, lokasi laptop tidak dibaca", settings.weather, Command::ToggleWeather),
                Row::Button {
                    icon: '\u{E707}',
                    title: "Kota cuaca",
                    detail: city,
                    label: "Ganti kota",
                    accent: false,
                    action: Action::Run(Command::EditCity),
                },
                Row::Note("Semua data disimpan sebagai file teks di laptop ini, tanpa akun.".into(), false),
            ]
        }
        Category::Shortcuts => {
            let detail = if s.hotkey_taken {
                "Sebagian sedang dipakai aplikasi lain"
            } else {
                "N membuka panel (tekan lagi untuk menutup), S membisukan atau menyalakan mikrofon"
            };
            vec![
                toggle('\u{E765}', "Win + Alt + N dan Win + Alt + S", detail, settings.hotkey, Command::ToggleHotkey),
                Row::Note("Geser dua jari ke kiri atau kanan di pil untuk ganti lagu selama musik diputar.".into(), false),
                Row::Note("Klik kanan panel untuk menggeser atau menyembunyikan halaman yang sedang dibuka.".into(), false),
            ]
        }
        Category::About => vec![
            Row::Hero,
            Row::Button {
                icon: '\u{E8A7}',
                title: "Kode sumber di GitHub",
                detail: "github.com/rthoriq/moord_notch · lisensi MIT".into(),
                label: "Buka",
                accent: false,
                action: Action::GitHub,
            },
            Row::Button {
                icon: '\u{E8B7}',
                title: "Folder data dan log",
                detail: "Shelf, tugas, agenda, pengaturan, dan log error (tanpa isi pesan)".into(),
                label: "Buka folder",
                accent: false,
                action: Action::Run(Command::OpenLogs),
            },
        ],
    }
}

// ---------------------------------------------------------------- warna

#[derive(Clone, Copy)]
struct Colors {
    dark: bool,
    background: u32,
    text: u32,
    secondary: u32,
    card: (u32, f32),
    card_hover: (u32, f32),
    stroke: (u32, f32),
    control: (u32, f32),
    control_hover: (u32, f32),
    subtle_hover: (u32, f32),
    subtle_selected: (u32, f32),
    accent: u32,
    on_accent: u32,
    warning: u32,
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let channel = |shift: u32| {
        let (x, y) = (((a >> shift) & 0xFF) as f32, ((b >> shift) & 0xFF) as f32);
        ((x + (y - x) * t).round() as u32) << shift
    };
    channel(16) | channel(8) | channel(0)
}

fn colors(dark: bool, accent: u32) -> Colors {
    if dark {
        Colors {
            dark,
            background: 0x202020,
            text: 0xFFFFFF,
            secondary: 0xC8C8C8,
            card: (0xFFFFFF, 0.051),
            card_hover: (0xFFFFFF, 0.08),
            stroke: (0x000000, 0.1),
            control: (0xFFFFFF, 0.061),
            control_hover: (0xFFFFFF, 0.084),
            subtle_hover: (0xFFFFFF, 0.061),
            subtle_selected: (0xFFFFFF, 0.084),
            accent: mix(accent, 0xFFFFFF, 0.45),
            on_accent: 0x000000,
            warning: 0xFCE100,
        }
    } else {
        Colors {
            dark,
            background: 0xF3F3F3,
            text: 0x1B1B1B,
            secondary: 0x5D5D5D,
            card: (0xFFFFFF, 0.7),
            card_hover: (0xF9F9F9, 0.5),
            stroke: (0x000000, 0.06),
            control: (0xFFFFFF, 0.7),
            control_hover: (0xF9F9F9, 0.5),
            subtle_hover: (0x000000, 0.037),
            subtle_selected: (0x000000, 0.058),
            accent: mix(accent, 0x000000, 0.12),
            on_accent: 0xFFFFFF,
            warning: 0x9D5D00,
        }
    }
}

fn dword(path: PCWSTR, name: PCWSTR) -> Option<u32> {
    let mut value = 0u32;
    let mut size = size_of::<u32>() as u32;
    let ok = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, path, name, RRF_RT_REG_DWORD, None, Some((&mut value as *mut u32).cast()), Some(&mut size))
    };
    ok.is_ok().then_some(value)
}

/// Tema aplikasi Windows (gelap atau terang) dan warna aksennya (0xRRGGBB).
fn system_look() -> (bool, u32) {
    let light = dword(w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"), w!("AppsUseLightTheme")).unwrap_or(1);
    // AccentColor disimpan sebagai 0xAABBGGRR.
    let accent = dword(w!("Software\\Microsoft\\Windows\\DWM"), w!("AccentColor"))
        .map(|abgr| ((abgr & 0xFF) << 16) | (abgr & 0xFF00) | ((abgr >> 16) & 0xFF))
        .unwrap_or(0x0078D4);
    (light == 0, accent)
}

const fn rgb(hex: u32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a,
    }
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F { left, top, right, bottom }
}

fn inside(r: &D2D_RECT_F, x: f32, y: f32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

// ---------------------------------------------------------------- teks

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Font {
    Text,
    Display,
    Icons,
}

struct Fonts {
    factory: IDWriteFactory,
    formats: RefCell<HashMap<(Font, u32, i32, i32, bool), IDWriteTextFormat>>,
}

impl Fonts {
    fn new() -> Result<Self> {
        Ok(Self { factory: unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? }, formats: RefCell::new(HashMap::new()) })
    }

    /// Format teks; `wrap` = paragraf yang turun ke baris berikutnya, rata atas.
    fn format(&self, font: Font, size: f32, weight: DWRITE_FONT_WEIGHT, align: i32, wrap: bool) -> Option<IDWriteTextFormat> {
        let key = (font, (size * 10.0) as u32, weight.0, align, wrap);
        if let Some(format) = self.formats.borrow().get(&key) {
            return Some(format.clone());
        }
        let family = match font {
            Font::Text => TEXT,
            Font::Display => DISPLAY,
            Font::Icons => ICONS,
        };
        let format = unsafe {
            let format = self
                .factory
                .CreateTextFormat(&HSTRING::from(family), None, weight, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!("id-ID"))
                .ok()?;
            let alignment = match align {
                -1 => DWRITE_TEXT_ALIGNMENT_LEADING,
                1 => DWRITE_TEXT_ALIGNMENT_TRAILING,
                _ => DWRITE_TEXT_ALIGNMENT_CENTER,
            };
            let _ = format.SetTextAlignment(alignment);
            let _ = format.SetParagraphAlignment(if wrap { DWRITE_PARAGRAPH_ALIGNMENT_NEAR } else { DWRITE_PARAGRAPH_ALIGNMENT_CENTER });
            let _ = format.SetWordWrapping(if wrap { DWRITE_WORD_WRAPPING_WRAP } else { DWRITE_WORD_WRAPPING_NO_WRAP });
            format
        };
        self.formats.borrow_mut().insert(key, format.clone());
        Some(format)
    }

    fn width(&self, s: &str, font: Font, size: f32, weight: DWRITE_FONT_WEIGHT) -> f32 {
        let Some(format) = self.format(font, size, weight, -1, false) else { return 0.0 };
        let wide: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            let Ok(layout) = self.factory.CreateTextLayout(&wide, &format, 10_000.0, size * 2.0) else { return 0.0 };
            let mut metrics = DWRITE_TEXT_METRICS::default();
            let _ = layout.GetMetrics(&mut metrics);
            metrics.widthIncludingTrailingWhitespace
        }
    }

    /// Tinggi paragraf teks biasa selebar `width`.
    fn height(&self, s: &str, size: f32, width: f32) -> f32 {
        let Some(format) = self.format(Font::Text, size, DWRITE_FONT_WEIGHT_NORMAL, -1, true) else { return size * 1.4 };
        let wide: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            let Ok(layout) = self.factory.CreateTextLayout(&wide, &format, width.max(10.0), 10_000.0) else { return size * 1.4 };
            let mut metrics = DWRITE_TEXT_METRICS::default();
            let _ = layout.GetMetrics(&mut metrics);
            metrics.height
        }
    }
}

// ---------------------------------------------------------------- menggambar

#[derive(Clone, Copy)]
struct Hit {
    area: D2D_RECT_F,
    action: Action,
    /// Di area isi yang bisa digulir (dipotong di tepi atas/bawah jendela).
    scrolled: bool,
}

/// Keadaan tampilan yang bukan bagian dari pengaturan.
#[derive(Clone, Copy)]
struct View {
    category: Category,
    hover: Option<Action>,
    pressed: Option<Action>,
    focus: Option<Action>,
    /// Garis fokus hanya tampil setelah keyboard dipakai, seperti di Windows.
    keyboard: bool,
    scroll: f32,
}

/// Satu kali menggambar seluruh isi jendela pada sebuah target.
struct Sheet<'a> {
    target: &'a ID2D1RenderTarget,
    factory: &'a ID2D1Factory,
    brush: &'a ID2D1SolidColorBrush,
    kind: &'a Fonts,
    colors: Colors,
    view: View,
    size: (f32, f32),
    hits: Vec<Hit>,
}

impl Sheet<'_> {
    fn color(&self, hex: u32, alpha: f32) -> &ID2D1SolidColorBrush {
        unsafe { self.brush.SetColor(&rgb(hex, alpha)) };
        self.brush
    }

    fn fill(&self, area: D2D_RECT_F, radius: f32, (hex, alpha): (u32, f32)) {
        let shape = D2D1_ROUNDED_RECT { rect: area, radiusX: radius, radiusY: radius };
        unsafe { self.target.FillRoundedRectangle(&shape, self.color(hex, alpha)) };
    }

    fn stroke(&self, area: D2D_RECT_F, radius: f32, width: f32, (hex, alpha): (u32, f32)) {
        let half = width / 2.0;
        let shape = D2D1_ROUNDED_RECT {
            rect: rect(area.left + half, area.top + half, area.right - half, area.bottom - half),
            radiusX: radius,
            radiusY: radius,
        };
        unsafe { self.target.DrawRoundedRectangle(&shape, self.color(hex, alpha), width, None::<&ID2D1StrokeStyle>) };
    }

    fn circle(&self, x: f32, y: f32, r: f32, (hex, alpha): (u32, f32)) {
        let shape = D2D1_ELLIPSE { point: Vector2 { X: x, Y: y }, radiusX: r, radiusY: r };
        unsafe { self.target.FillEllipse(&shape, self.color(hex, alpha)) };
    }

    #[allow(clippy::too_many_arguments)]
    fn text(&self, s: &str, font: Font, size: f32, weight: DWRITE_FONT_WEIGHT, area: D2D_RECT_F, align: i32, color: (u32, f32)) {
        self.draw(s, font, size, weight, area, align, false, color);
    }

    /// Paragraf yang turun ke baris berikutnya di dalam `area`.
    fn paragraph(&self, s: &str, size: f32, area: D2D_RECT_F, color: (u32, f32)) {
        self.draw(s, Font::Text, size, DWRITE_FONT_WEIGHT_NORMAL, area, -1, true, color);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw(&self, s: &str, font: Font, size: f32, weight: DWRITE_FONT_WEIGHT, area: D2D_RECT_F, align: i32, wrap: bool, (hex, alpha): (u32, f32)) {
        let Some(format) = self.kind.format(font, size, weight, align, wrap) else { return };
        let wide: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            self.target.DrawText(&wide, &format, &area, self.color(hex, alpha), D2D1_DRAW_TEXT_OPTIONS_CLIP, DWRITE_MEASURING_MODE_NATURAL)
        };
    }

    fn icon(&self, glyph: char, size: f32, x: f32, y: f32, color: (u32, f32)) {
        let area = rect(x - size, y - size, x + size, y + size);
        self.text(&glyph.to_string(), Font::Icons, size, DWRITE_FONT_WEIGHT_NORMAL, area, 0, color);
    }

    /// Tetes air, untuk halaman Minum air yang tidak punya ikon di font.
    fn drop_icon(&self, x: f32, y: f32, size: f32, (hex, alpha): (u32, f32)) {
        unsafe {
            let Ok(geometry) = self.factory.CreatePathGeometry() else { return };
            let Ok(sink) = geometry.Open() else { return };
            let p = |dx: f32, dy: f32| Vector2 { X: x + dx * size, Y: y + dy * size };
            sink.BeginFigure(p(0.0, -0.55), D2D1_FIGURE_BEGIN_FILLED);
            sink.AddLine(p(0.32, 0.0));
            sink.AddLine(p(0.25, 0.3));
            sink.AddLine(p(0.0, 0.45));
            sink.AddLine(p(-0.25, 0.3));
            sink.AddLine(p(-0.32, 0.0));
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            let _ = sink.Close();
            self.target.FillGeometry(&geometry, self.color(hex, alpha), None::<&ID2D1Brush>);
        }
    }

    fn state(&self, action: Action) -> (bool, bool) {
        (self.view.hover == Some(action), self.view.pressed == Some(action))
    }

    fn hit(&mut self, area: D2D_RECT_F, action: Action, scrolled: bool) {
        self.hits.push(Hit { area, action, scrolled });
        if self.view.keyboard && self.view.focus == Some(action) {
            let ring = rect(area.left - 3.0, area.top - 3.0, area.right + 3.0, area.bottom + 3.0);
            self.stroke(ring, 7.0, 2.0, (self.colors.text, 1.0));
        }
    }

    fn paint(&mut self, state: &MenuState, mica: bool) {
        let c = self.colors;
        unsafe { self.target.Clear(Some(&if mica { rgb(0, 0.0) } else { rgb(c.background, 1.0) })) };
        let (width, height) = self.size;
        let nav = nav_width(width);

        // Navigasi kategori di kiri.
        app_glyph(self, 40.0, 38.0, 0.62);
        self.text("Pengaturan", Font::Text, 15.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, rect(64.0, 24.0, nav, 52.0), -1, (c.text, 1.0));
        for (i, category) in Category::ALL.into_iter().enumerate() {
            let area = rect(12.0, 72.0 + i as f32 * 44.0, nav - 12.0, 112.0 + i as f32 * 44.0);
            let action = Action::Nav(category);
            let (hover, pressed) = self.state(action);
            let selected = self.view.category == category;
            if selected || hover {
                self.fill(area, 6.0, if selected { c.subtle_selected } else { c.subtle_hover });
            }
            if selected {
                let bar = rect(area.left, area.top + 12.0, area.left + 3.0, area.bottom - 12.0);
                self.fill(bar, 1.5, (c.accent, 1.0));
            }
            let alpha = if pressed { 0.75 } else { 1.0 };
            self.icon(category.icon(), 16.0, area.left + 28.0, (area.top + area.bottom) / 2.0, (c.text, alpha));
            let label = rect(area.left + 52.0, area.top, area.right - 8.0, area.bottom);
            self.text(category.label(), Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, label, -1, (c.text, alpha));
            self.hit(area, action, false);
        }

        // Isi kategori, bisa digulir.
        let left = nav + 16.0;
        let right = (width - 36.0).min(left + CONTENT_MAX);
        unsafe { self.target.PushAxisAlignedClip(&rect(nav, 0.0, width, height), D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
        let mut y = 24.0 - self.view.scroll;
        let title = rect(left, y, right, y + 48.0);
        self.text(self.view.category.label(), Font::Display, 28.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, title, -1, (c.text, 1.0));
        y += 68.0;
        for row in rows(self.view.category, state) {
            y = self.row(&row, left, right, y) + 4.0;
        }
        unsafe { self.target.PopAxisAlignedClip() };
        // Panjang isi seluruhnya, untuk batas gulir.
        self.size.1 = y + self.view.scroll + 24.0;
    }

    /// Kartu dengan ikon, judul, dan keterangan (turun ke baris berikutnya kalau panjang),
    /// menyisakan `control` DIP di kanan untuk kontrolnya dan `extra` DIP di bawah untuk isi
    /// tambahan. Mengembalikan tepi bawah bagian judul dan tepi bawah kartu.
    #[allow(clippy::too_many_arguments)]
    fn card(&mut self, icon: Option<char>, title: &str, detail: &str, left: f32, right: f32, top: f32, control: f32, extra: f32, enabled: bool, hover: bool) -> (f32, f32) {
        let c = self.colors;
        let text_left = left + if icon.is_some() { 56.0 } else { 20.0 };
        let text_right = (right - control).max(text_left + 120.0);
        let lines = if detail.is_empty() { 0.0 } else { self.kind.height(detail, 12.0, text_right - text_left) };
        let head = if detail.is_empty() { 68.0 } else { (36.0 + lines + 14.0).max(68.0) };
        let bottom = top + head + extra;
        let area = rect(left, top, right, bottom);
        self.fill(area, 6.0, if hover { c.card_hover } else { c.card });
        self.stroke(area, 6.0, 1.0, c.stroke);
        let alpha = if enabled { 1.0 } else { 0.45 };
        if let Some(glyph) = icon {
            self.icon(glyph, 18.0, left + 28.0, top + head.min(68.0) / 2.0, (c.text, alpha));
        }
        let (title_top, title_bottom) = if detail.is_empty() { (top, top + 68.0) } else { (top + 14.0, top + 36.0) };
        self.text(title, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, rect(text_left, title_top, text_right, title_bottom), -1, (c.text, alpha));
        if !detail.is_empty() {
            self.paragraph(detail, 12.0, rect(text_left, top + 36.0, text_right, top + 36.0 + lines + 2.0), (c.secondary, alpha));
        }
        (top + head, bottom)
    }

    fn row(&mut self, row: &Row, left: f32, right: f32, top: f32) -> f32 {
        let c = self.colors;
        match row {
            Row::Toggle { icon, title, detail, on, enabled, command } => {
                let action = Action::Run(*command);
                let (hover, _) = self.state(action);
                let (_, bottom) = self.card(Some(*icon), title, detail, left, right, top, 170.0, 0.0, *enabled, hover && *enabled);
                let (x, y) = (right - 20.0 - 40.0, (top + bottom) / 2.0);
                self.switch(x, y, *on, *enabled, hover);
                let label = if *on { "Nyala" } else { "Mati" };
                let alpha = if *enabled { 1.0 } else { 0.45 };
                self.text(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, rect(x - 80.0, top, x - 12.0, bottom), 1, (c.text, alpha));
                if *enabled {
                    self.hit(rect(left, top, right, bottom), action, true);
                }
                bottom
            }
            Row::Button { icon, title, detail, label, accent, action } => {
                let room = self.kind.width(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL) + 72.0;
                let (_, bottom) = self.card(Some(*icon), title, detail, left, right, top, room, 0.0, true, false);
                self.button(label, *accent, *action, right - 20.0, (top + bottom) / 2.0);
                bottom
            }
            Row::Stepper { icon, title, value, minus, plus } => {
                let (_, bottom) = self.card(Some(*icon), title, "", left, right, top, 220.0, 0.0, true, false);
                let y = (top + bottom) / 2.0;
                let plus_w = self.kind.width("+", Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL) + 40.0;
                let value_right = right - 20.0 - plus_w - 6.0;
                self.button("+", false, Action::Run(*plus), right - 20.0, y);
                self.text(value, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, rect(value_right - 76.0, top, value_right, bottom), 0, (c.text, 1.0));
                self.button("−", false, Action::Run(*minus), value_right - 82.0, y);
                bottom
            }
            Row::Choice { icon, title, detail, options } => {
                // Pilihan berderet di bawah judul seperti tombol segmen, turun ke baris berikutnya
                // kalau tidak muat.
                let widths: Vec<f32> = options.iter().map(|o| self.kind.width(o.0, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL) + 36.0).collect();
                let (start, end) = (left + 56.0, right - 20.0);
                let mut spots = Vec::new();
                let (mut x, mut line) = (start, 0.0f32);
                for &w in &widths {
                    if x + w > end && x > start {
                        x = start;
                        line += 1.0;
                    }
                    spots.push((x, line));
                    x += w + 8.0;
                }
                let lines = line + 1.0;
                let (head, bottom) = self.card(Some(*icon), title, detail, left, right, top, 20.0, lines * 40.0 + 4.0, true, false);
                for ((label, chosen, enabled, command), (&w, &(x, line))) in options.iter().zip(widths.iter().zip(&spots)) {
                    let y = head - 4.0 + line * 40.0;
                    let area = rect(x, y, x + w, y + 32.0);
                    let action = Action::Run(*command);
                    let (hover, pressed) = self.state(action);
                    let alpha = if *enabled { 1.0 } else { 0.4 };
                    if *chosen {
                        self.fill(area, 16.0, (c.accent, alpha));
                        self.text(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, area, 0, (c.on_accent, alpha));
                    } else {
                        self.fill(area, 16.0, if hover && *enabled { c.control_hover } else { c.control });
                        self.stroke(area, 16.0, 1.0, c.stroke);
                        self.text(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, area, 0, (c.text, alpha * if pressed { 0.7 } else { 1.0 }));
                    }
                    if *enabled && !*chosen {
                        self.hit(area, action, true);
                    }
                }
                bottom
            }
            Row::Themes(current) => {
                let columns = 3usize;
                let tile_h = 118.0;
                let rows_count = Theme::ALL.len().div_ceil(columns);
                let extra = rows_count as f32 * (tile_h + 12.0) + 8.0;
                let (head, bottom) = self.card(Some('\u{E790}'), "Tema notch", "Warna badan notch dan panelnya", left, right, top, 20.0, extra, true, false);
                let inner = (left + 56.0, right - 20.0);
                let tile_w = (inner.1 - inner.0 - 12.0 * (columns as f32 - 1.0)) / columns as f32;
                for (i, theme) in Theme::ALL.into_iter().enumerate() {
                    let (col, line) = ((i % columns) as f32, (i / columns) as f32);
                    let x = inner.0 + col * (tile_w + 12.0);
                    let y = head + line * (tile_h + 12.0);
                    let area = rect(x, y, x + tile_w, y + tile_h);
                    let action = Action::Run(Command::Theme(theme));
                    let (hover, _) = self.state(action);
                    self.theme_tile(theme, area, theme == *current, hover);
                    if theme != *current {
                        self.hit(area, action, true);
                    }
                }
                bottom
            }
            Row::Page { page, shown, up, down } => {
                let bottom = top + 60.0;
                let locked = *page == Page::Home;
                let action = Action::Run(Command::TogglePage(*page));
                let (hover, _) = self.state(action);
                let area = rect(left, top, right, bottom);
                self.fill(area, 6.0, if hover && !locked { c.card_hover } else { c.card });
                self.stroke(area, 6.0, 1.0, c.stroke);
                let alpha = if *shown { 1.0 } else { 0.5 };
                let cy = (top + bottom) / 2.0;
                match page_icon(*page) {
                    Some(glyph) => self.icon(glyph, 18.0, left + 28.0, cy, (c.text, alpha)),
                    None => self.drop_icon(left + 28.0, cy, 22.0, (c.text, alpha)),
                }
                let detail = if locked { " · selalu tampil" } else if *shown { "" } else { " · disembunyikan" };
                let label = format!("{}{detail}", page.label());
                self.text(&label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, rect(left + 56.0, top, right - 216.0, bottom), -1, (c.text, alpha));
                let x = right - 20.0 - 40.0;
                self.switch(x, cy, *shown, !locked, hover);
                for (dx, glyph, enabled, delta) in [(-150.0, '\u{E70E}', *up, -1), (-108.0, '\u{E70D}', *down, 1)] {
                    let b = rect(x + dx, cy - 16.0, x + dx + 36.0, cy + 16.0);
                    let action = Action::Run(Command::MovePage(*page, delta));
                    let (hover, _) = self.state(action);
                    if enabled {
                        self.fill(b, 4.0, if hover { c.control_hover } else { c.control });
                        self.stroke(b, 4.0, 1.0, c.stroke);
                    }
                    self.icon(glyph, 12.0, (b.left + b.right) / 2.0, cy, (c.text, if enabled { 1.0 } else { 0.25 }));
                    if enabled {
                        self.hit(b, action, true);
                    }
                }
                if !locked {
                    self.hit(rect(x - 6.0, top, right, bottom), action, true);
                }
                bottom
            }
            Row::Note(text, warning) => {
                let lines = self.kind.height(text, 13.0, right - left - 30.0);
                let bottom = top + (lines + 20.0).max(40.0);
                let color = if *warning { c.warning } else { c.secondary };
                self.icon(if *warning { '\u{E7BA}' } else { '\u{E946}' }, 14.0, left + 10.0, top + 20.0, (color, 1.0));
                self.paragraph(text, 13.0, rect(left + 30.0, top + 11.0, right, bottom), (color, 1.0));
                bottom
            }
            Row::Hero => {
                let bottom = top + 104.0;
                let area = rect(left, top, right, bottom);
                self.fill(area, 6.0, c.card);
                self.stroke(area, 6.0, 1.0, c.stroke);
                app_glyph(self, left + 60.0, top + 52.0, 1.3);
                let name = rect(left + 110.0, top + 18.0, right, top + 56.0);
                self.text("NotchWindows", Font::Display, 22.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, name, -1, (c.text, 1.0));
                let version = format!("Versi {} · gratis dan open source", env!("CARGO_PKG_VERSION"));
                self.text(&version, Font::Text, 13.0, DWRITE_FONT_WEIGHT_NORMAL, rect(left + 110.0, top + 56.0, right, top + 80.0), -1, (c.secondary, 1.0));
                bottom
            }
        }
    }

    /// Sakelar ala Windows 11, tepi kirinya di `x`, tengahnya di `y`.
    fn switch(&self, x: f32, y: f32, on: bool, enabled: bool, hover: bool) {
        let c = self.colors;
        let area = rect(x, y - 10.0, x + 40.0, y + 10.0);
        let alpha = if enabled { 1.0 } else { 0.4 };
        if on {
            self.fill(area, 10.0, (c.accent, alpha));
            self.circle(x + 30.0, y, if hover { 7.0 } else { 6.0 }, (c.on_accent, alpha));
        } else {
            self.fill(area, 10.0, if hover { c.control_hover } else { (c.background, 0.0) });
            self.stroke(area, 10.0, 1.0, (c.secondary, alpha));
            self.circle(x + 10.0, y, if hover { 6.0 } else { 5.0 }, (c.secondary, alpha));
        }
    }

    /// Tombol dengan tepi kanan di `right` dan tengah di `y`.
    fn button(&mut self, label: &str, accent: bool, action: Action, right: f32, y: f32) {
        let c = self.colors;
        let w = self.kind.width(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL) + 40.0;
        let area = rect(right - w, y - 16.0, right, y + 16.0);
        let (hover, pressed) = self.state(action);
        if accent {
            self.fill(area, 4.0, (c.accent, if hover { 0.9 } else { 1.0 }));
            self.text(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_SEMI_BOLD, area, 0, (c.on_accent, if pressed { 0.8 } else { 1.0 }));
        } else {
            self.fill(area, 4.0, if hover { c.control_hover } else { c.control });
            self.stroke(area, 4.0, 1.0, c.stroke);
            self.text(label, Font::Text, 14.0, DWRITE_FONT_WEIGHT_NORMAL, area, 0, (c.text, if pressed { 0.7 } else { 1.0 }));
        }
        self.hit(area, action, true);
    }

    /// Kartu tema: contoh notch dengan warna tema itu di atas latar netral.
    fn theme_tile(&self, theme: Theme, area: D2D_RECT_F, chosen: bool, hover: bool) {
        let c = self.colors;
        let preview = rect(area.left, area.top, area.right, area.bottom - 30.0);
        self.fill(preview, 6.0, if c.dark { (0x3A3A3A, 1.0) } else { (0xDADDE3, 1.0) });
        if theme == Theme::Glass {
            // Kaca dipamerkan di atas warna supaya tembus pandangnya terlihat.
            let (w, h) = (preview.right - preview.left, preview.bottom - preview.top);
            self.circle(preview.left + w * 0.38, preview.top + h * 0.45, h * 0.3, (0x7A5AF8, 0.85));
            self.circle(preview.left + w * 0.62, preview.top + h * 0.5, h * 0.28, (0x22B8F0, 0.8));
        }
        let (body, eye, glass) = crate::render::theme_colors(theme);
        let (cx, w, h) = ((preview.left + preview.right) / 2.0, 120.0f32.min(preview.right - preview.left - 24.0), 30.0);
        let pill = rect(cx - w / 2.0, preview.top + 24.0, cx + w / 2.0, preview.top + 24.0 + h);
        let shape = D2D1_ROUNDED_RECT { rect: pill, radiusX: h / 2.0, radiusY: h / 2.0 };
        unsafe {
            self.brush.SetColor(&body);
            self.target.FillRoundedRectangle(&shape, self.brush);
            if glass {
                self.brush.SetColor(&rgb(0xFFFFFF, 0.35));
                self.target.DrawRoundedRectangle(&shape, self.brush, 1.0, None::<&ID2D1StrokeStyle>);
            }
            self.brush.SetColor(&eye);
            for dx in [-7.0, 7.0] {
                let e = D2D1_ROUNDED_RECT { rect: rect(cx + dx - 3.0, pill.top + 8.0, cx + dx + 3.0, pill.bottom - 8.0), radiusX: 3.0, radiusY: 3.0 };
                self.target.FillRoundedRectangle(&e, self.brush);
            }
        }
        let label = rect(area.left, area.bottom - 28.0, area.right, area.bottom);
        self.text(theme.label(), Font::Text, 13.0, if chosen { DWRITE_FONT_WEIGHT_SEMI_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL }, label, 0, (c.text, 1.0));
        if chosen {
            self.stroke(preview, 6.0, 2.0, (c.accent, 1.0));
            self.circle(preview.right - 14.0, preview.top + 14.0, 10.0, (c.accent, 1.0));
            self.icon('\u{E73E}', 11.0, preview.right - 14.0, preview.top + 14.0, (c.on_accent, 1.0));
        } else if hover {
            self.stroke(preview, 6.0, 1.0, (c.text, 0.4));
        }
    }
}

/// Ikon aplikasi: pil hitam dengan dua mata, sebesar `k` kali ukuran dasarnya.
fn app_glyph(sheet: &Sheet, cx: f32, cy: f32, k: f32) {
    let (w, h) = (44.0 * k, 22.0 * k);
    sheet.fill(rect(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0), h / 2.0, (0x000000, 1.0));
    if !sheet.colors.dark {
        sheet.stroke(rect(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0), h / 2.0, 1.0, (0x000000, 0.2));
    }
    for dx in [-6.0 * k, 6.0 * k] {
        sheet.fill(rect(cx + dx - 2.5 * k, cy - 5.0 * k, cx + dx + 2.5 * k, cy + 5.0 * k), 2.5 * k, (0xF5F5F7, 1.0));
    }
}

// ---------------------------------------------------------------- jendela

struct Window {
    hwnd: HWND,
    factory: ID2D1Factory,
    kind: Fonts,
    target: Option<(ID2D1HwndRenderTarget, ID2D1SolidColorBrush)>,
    dpi: f32,
    dark: bool,
    accent: u32,
    mica: bool,
    view: View,
    hits: Vec<Hit>,
    content_height: f32,
    /// Keadaan terakhir dari notch, dipakai kalau notch sedang sibuk saat menggambar.
    last: Option<MenuState>,
}

thread_local! {
    static WINDOW: RefCell<Option<Window>> = const { RefCell::new(None) };
}

fn with_window<R>(f: impl FnOnce(&mut Window) -> R) -> Option<R> {
    WINDOW.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

/// Buka jendela pengaturan, atau bawa ke depan kalau sudah terbuka.
pub fn open() {
    let existing = with_window(|window| window.hwnd);
    unsafe {
        if let Some(hwnd) = existing {
            if IsIconic(hwnd).as_bool() {
                let _ = ShowWindow(hwnd, SW_RESTORE);
            }
            let _ = SetForegroundWindow(hwnd);
            return;
        }
    }
    if let Err(e) = create() {
        error_log!("jendela pengaturan gagal dibuka: {e}");
    }
}

/// Gambar ulang setelah pengaturan berubah dari tempat lain (menu tray, panel, layanan).
pub fn refresh() {
    WINDOW.with(|cell| {
        if let Ok(window) = cell.try_borrow()
            && let Some(window) = window.as_ref()
        {
            let _ = unsafe { InvalidateRect(Some(window.hwnd), None, false) };
        }
    });
}

/// Tutup jendela (saat notch keluar).
pub fn close() {
    if let Some(hwnd) = with_window(|window| window.hwnd) {
        let _ = unsafe { DestroyWindow(hwnd) };
    }
}

fn register() -> Result<()> {
    static ONCE: Once = Once::new();
    let mut result = Ok(());
    ONCE.call_once(|| unsafe {
        let instance = match GetModuleHandleW(None) {
            Ok(instance) => instance,
            Err(e) => {
                result = Err(e);
                return;
            }
        };
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            // Ikon notch.exe (resource 1) untuk taskbar dan Alt+Tab, dalam ukuran yang pas.
            hIcon: app_icon(instance.into(), SM_CXICON),
            hIconSm: app_icon(instance.into(), SM_CXSMICON),
            lpszClassName: CLASS,
            ..Default::default()
        };
        if RegisterClassExW(&class) == 0 {
            result = Err(windows::core::Error::from_thread());
        }
    });
    result
}

/// Ikon notch.exe berukuran `metric` (`SM_CXICON` atau `SM_CXSMICON`), diambil dari ukuran yang
/// paling dekat di notch.ico.
fn app_icon(instance: HINSTANCE, metric: SYSTEM_METRICS_INDEX) -> HICON {
    unsafe {
        let size = GetSystemMetrics(metric);
        LoadImageW(Some(instance), PCWSTR(1 as _), IMAGE_ICON, size, size, LR_DEFAULTCOLOR).map_or(HICON::default(), |icon| HICON(icon.0))
    }
}

/// Jendela ini tampil di taskbar dengan ID aplikasinya sendiri. Saat pesan masuk aktif, notch
/// berjalan dengan identitas paket, dan tanpa ID sendiri taskbar memakai logo paket di atas pelat
/// warna aksen. Dengan ID ini taskbar memakai ikon jendela (ikon notch.exe), dan tombol yang
/// disematkan menjalankan notch.exe, yang membuka jendela ini kalau notch sudah berjalan.
/// Dilepas lagi (`on` = false) sebelum jendelanya hilang, seperti anjuran Windows.
fn set_taskbar_id(hwnd: HWND, on: bool) -> Result<()> {
    unsafe {
        let store: IPropertyStore = SHGetPropertyStoreForWindow(hwnd)?;
        let exe = std::env::current_exe().map_err(|e| windows::core::Error::new(windows::Win32::Foundation::E_FAIL, e.to_string()))?;
        let exe = exe.display();
        // ID-nya diisi terakhir, setelah keterangan untuk menjalankan ulang.
        let values = [
            (PKEY_AppUserModel_RelaunchCommand, format!("\"{exe}\"")),
            (PKEY_AppUserModel_RelaunchDisplayNameResource, "NotchWindows".to_string()),
            (PKEY_AppUserModel_RelaunchIconResource, format!("{exe},-1")),
            (PKEY_AppUserModel_ID, TASKBAR_ID.to_string()),
        ];
        if on {
            for (key, text) in &values {
                store.SetValue(key, &text_value(text)?)?;
            }
        } else {
            for (key, _) in values.iter().rev() {
                store.SetValue(key, &PROPVARIANT::default())?;
            }
        }
        store.Commit()
    }
}

/// Teks sebagai PROPVARIANT VT_LPWSTR (jenis yang diminta properti taskbar). Memorinya dilepas
/// oleh `PROPVARIANT` sendiri saat dibuang.
fn text_value(text: &str) -> Result<PROPVARIANT> {
    let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    unsafe {
        let memory = CoTaskMemAlloc(wide.len() * 2).cast::<u16>();
        if memory.is_null() {
            return Err(windows::Win32::Foundation::E_OUTOFMEMORY.into());
        }
        memory.copy_from_nonoverlapping(wide.as_ptr(), wide.len());
        let mut value = PROPVARIANT::default();
        let parts = &mut *value.Anonymous.Anonymous;
        parts.vt = VT_LPWSTR;
        parts.Anonymous.pwszVal = PWSTR(memory);
        Ok(value)
    }
}

fn create() -> Result<()> {
    register()?;
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS,
            w!("Pengaturan NotchWindows"),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        let (dark, accent) = system_look();
        let window = Window {
            hwnd,
            factory: D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?,
            kind: Fonts::new()?,
            target: None,
            dpi: GetDpiForWindow(hwnd) as f32,
            dark,
            accent,
            mica: false,
            view: View { category: Category::General, hover: None, pressed: None, focus: None, keyboard: false, scroll: 0.0 },
            hits: Vec::new(),
            content_height: 0.0,
            last: None,
        };
        WINDOW.with(|cell| *cell.borrow_mut() = Some(window));
        with_window(|window| {
            window.apply_look();
            window.place_center();
        });
        // Sebelum jendelanya tampil, supaya tombol taskbar langsung memakai ID ini.
        if let Err(e) = set_taskbar_id(hwnd, true) {
            error_log!("ID taskbar jendela Pengaturan gagal dipasang: {e}");
        }
        let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

impl Window {
    /// Gelap/terang dan latar Mica (Windows 11 22H2 ke atas; kalau tidak, latar polos).
    fn apply_look(&mut self) {
        unsafe {
            let dark = BOOL::from(self.dark);
            let _ = DwmSetWindowAttribute(self.hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, std::ptr::from_ref(&dark).cast(), size_of_val(&dark) as u32);
            let backdrop = DWMSBT_MAINWINDOW;
            self.mica = DwmSetWindowAttribute(self.hwnd, DWMWA_SYSTEMBACKDROP_TYPE, std::ptr::from_ref(&backdrop).cast(), size_of_val(&backdrop) as u32).is_ok();
            if self.mica {
                let margins = MARGINS { cxLeftWidth: -1, cxRightWidth: -1, cyTopHeight: -1, cyBottomHeight: -1 };
                self.mica = DwmExtendFrameIntoClientArea(self.hwnd, &margins).is_ok();
            }
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        }
    }

    /// Ukuran awal di tengah monitor tempat kursor berada.
    fn place_center(&self) {
        unsafe {
            let mut cursor = POINT::default();
            let _ = GetCursorPos(&mut cursor);
            let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTOPRIMARY);
            let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
            let _ = GetMonitorInfoW(monitor, &mut info);
            let (mut dx, mut dy) = (96u32, 96u32);
            let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
            let k = dx as f32 / 96.0;
            let work = info.rcWork;
            let w = ((START_SIZE.0 * k) as i32).min(work.right - work.left);
            let h = ((START_SIZE.1 * k) as i32).min(work.bottom - work.top);
            let x = work.left + (work.right - work.left - w) / 2;
            let y = work.top + (work.bottom - work.top - h) / 2;
            let _ = SetWindowPos(self.hwnd, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
        }
    }

    fn client_size(&self) -> (u32, u32) {
        let mut r = RECT::default();
        let _ = unsafe { GetClientRect(self.hwnd, &mut r) };
        ((r.right - r.left).max(1) as u32, (r.bottom - r.top).max(1) as u32)
    }

    fn ensure_target(&mut self) -> Option<(ID2D1HwndRenderTarget, ID2D1SolidColorBrush)> {
        if self.target.is_none() {
            let (w, h) = self.client_size();
            let props = D2D1_RENDER_TARGET_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
                dpiX: self.dpi,
                dpiY: self.dpi,
                ..Default::default()
            };
            let hwnd_props = D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd: self.hwnd,
                pixelSize: D2D_SIZE_U { width: w, height: h },
                presentOptions: D2D1_PRESENT_OPTIONS_NONE,
            };
            unsafe {
                let target = self.factory.CreateHwndRenderTarget(&props, &hwnd_props).ok()?;
                let brush = target.CreateSolidColorBrush(&rgb(0, 1.0), None).ok()?;
                self.target = Some((target, brush));
            }
        }
        self.target.clone()
    }

    fn snapshot(&mut self) -> Option<MenuState> {
        if let Some(state) = crate::app::snapshot() {
            self.last = Some(state);
        }
        self.last.clone()
    }

    fn paint(&mut self) {
        let Some(state) = self.snapshot() else { return };
        let Some((target, brush)) = self.ensure_target() else { return };
        let (w, h) = self.client_size();
        let k = self.dpi / 96.0;
        let size = (w as f32 / k, h as f32 / k);
        self.clamp_scroll(size.1);
        let mut sheet = Sheet {
            target: &target,
            factory: &self.factory,
            brush: &brush,
            kind: &self.kind,
            colors: colors(self.dark, self.accent),
            view: self.view,
            size,
            hits: Vec::new(),
        };
        unsafe { target.BeginDraw() };
        sheet.paint(&state, self.mica);
        let result = unsafe { target.EndDraw(None, None) };
        self.hits = sheet.hits;
        self.content_height = sheet.size.1;
        if result.is_err() {
            // Misal kartu grafis berganti: target dibuat ulang di gambar berikutnya.
            self.target = None;
        }
    }

    fn visible_height(&self) -> f32 {
        self.client_size().1 as f32 / (self.dpi / 96.0)
    }

    fn clamp_scroll(&mut self, visible: f32) {
        let max = (self.content_height - visible).max(0.0);
        self.view.scroll = self.view.scroll.clamp(0.0, max);
    }

    fn hit_at(&self, lparam: LPARAM) -> Option<Action> {
        let k = self.dpi / 96.0;
        let x = (lparam.0 & 0xFFFF) as i16 as f32 / k;
        let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32 / k;
        let visible = self.visible_height();
        let nav = nav_width(self.client_size().0 as f32 / k);
        self.hits
            .iter()
            .rev()
            .find(|hit| inside(&hit.area, x, y) && (!hit.scrolled || (x >= nav && y >= 0.0 && y < visible)))
            .map(|hit| hit.action)
    }

    /// Kontrol yang bisa difokus keyboard, sesuai urutan gambarnya.
    fn focusables(&self) -> Vec<Action> {
        let mut out: Vec<Action> = Vec::new();
        for hit in &self.hits {
            if !out.contains(&hit.action) {
                out.push(hit.action);
            }
        }
        out
    }

    /// Geser isi supaya kontrol yang difokus terlihat.
    fn reveal_focus(&mut self) {
        let Some(focus) = self.view.focus else { return };
        let visible = self.visible_height();
        if let Some(hit) = self.hits.iter().find(|hit| hit.action == focus && hit.scrolled) {
            if hit.area.top < 8.0 {
                self.view.scroll += hit.area.top - 8.0;
            } else if hit.area.bottom > visible - 8.0 {
                self.view.scroll += hit.area.bottom - visible + 8.0;
            }
            self.clamp_scroll(visible);
        }
    }

    fn select(&mut self, category: Category) {
        if self.view.category != category {
            self.view.category = category;
            self.view.scroll = 0.0;
        }
    }

    fn invalidate(&self) {
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    /// Tombol keyboard; mengembalikan aksi yang harus dijalankan.
    fn key(&mut self, key: u16) -> Option<Action> {
        let shift = unsafe { GetKeyState(VK_SHIFT.0 as i32) } < 0;
        let list = self.focusables();
        let index = self.view.focus.and_then(|focus| list.iter().position(|a| *a == focus));
        self.view.keyboard = true;
        let mut action = None;
        match key {
            k if k == VK_TAB.0 && !list.is_empty() => {
                let next = match (index, shift) {
                    (Some(i), false) => (i + 1) % list.len(),
                    (Some(i), true) => (i + list.len() - 1) % list.len(),
                    (None, false) => 0,
                    (None, true) => list.len() - 1,
                };
                self.view.focus = Some(list[next]);
            }
            k if k == VK_UP.0 || k == VK_DOWN.0 => {
                // Di daftar kategori, panah berpindah kategori; di isi, panah menggulir.
                if let Some(Action::Nav(current)) = self.view.focus {
                    let i = Category::ALL.iter().position(|c| *c == current).unwrap_or(0) as i32;
                    let delta = if k == VK_UP.0 { -1 } else { 1 };
                    let next = Category::ALL[(i + delta).clamp(0, Category::ALL.len() as i32 - 1) as usize];
                    self.view.focus = Some(Action::Nav(next));
                    self.select(next);
                } else {
                    self.view.scroll += if k == VK_UP.0 { -48.0 } else { 48.0 };
                }
            }
            k if k == VK_PRIOR.0 || k == VK_NEXT.0 => {
                let page = self.visible_height() * 0.8;
                self.view.scroll += if k == VK_PRIOR.0 { -page } else { page };
            }
            k if k == VK_HOME.0 => self.view.scroll = 0.0,
            k if k == VK_END.0 => self.view.scroll = f32::MAX,
            k if k == VK_RETURN.0 || k == VK_SPACE.0 => action = self.view.focus,
            k if k == VK_ESCAPE.0 => {
                let _ = unsafe { DestroyWindow(self.hwnd) };
                return None;
            }
            _ => return None,
        }
        self.clamp_scroll(self.visible_height());
        self.paint();
        self.reveal_focus();
        self.invalidate();
        action
    }
}

/// Jalankan aksi di luar pinjaman state jendela (perintah notch bisa menggambar ulang jendela ini).
fn perform(action: Action) {
    match action {
        Action::Nav(category) => {
            with_window(|window| {
                window.select(category);
                window.view.focus = Some(Action::Nav(category));
                window.invalidate();
            });
        }
        Action::Run(command) => {
            crate::app::command(command);
            refresh();
        }
        Action::GitHub => unsafe {
            ShellExecuteW(None, w!("open"), &HSTRING::from(GITHUB), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
        },
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let handled = match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            unsafe { BeginPaint(hwnd, &mut ps) };
            with_window(|window| window.paint());
            let _ = unsafe { EndPaint(hwnd, &ps) };
            Some(LRESULT(0))
        }
        WM_ERASEBKGND => Some(LRESULT(1)),
        WM_SIZE => {
            with_window(|window| {
                let (w, h) = window.client_size();
                if let Some((target, _)) = &window.target {
                    let _ = unsafe { target.Resize(&D2D_SIZE_U { width: w, height: h }) };
                }
                window.invalidate();
            });
            Some(LRESULT(0))
        }
        WM_DPICHANGED => {
            with_window(|window| {
                window.dpi = (wparam.0 & 0xFFFF) as f32;
                window.target = None;
                let suggested = unsafe { &*(lparam.0 as *const RECT) };
                let _ = unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        suggested.left,
                        suggested.top,
                        suggested.right - suggested.left,
                        suggested.bottom - suggested.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    )
                };
            });
            Some(LRESULT(0))
        }
        WM_GETMINMAXINFO => {
            let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96) as f32 / 96.0;
            let info = unsafe { &mut *(lparam.0 as *mut MINMAXINFO) };
            info.ptMinTrackSize = POINT { x: (MIN_SIZE.0 * dpi) as i32, y: (MIN_SIZE.1 * dpi) as i32 };
            Some(LRESULT(0))
        }
        WM_SETTINGCHANGE => {
            // Tema terang/gelap atau warna aksen Windows berubah.
            with_window(|window| {
                let (dark, accent) = system_look();
                if (dark, accent) != (window.dark, window.accent) {
                    window.dark = dark;
                    window.accent = accent;
                    window.apply_look();
                }
            });
            None
        }
        WM_MOUSEMOVE => {
            with_window(|window| {
                let hover = window.hit_at(lparam);
                if hover != window.view.hover {
                    window.view.hover = hover;
                    window.invalidate();
                }
                let mut track = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
                let _ = unsafe { TrackMouseEvent(&mut track) };
            });
            Some(LRESULT(0))
        }
        WM_MOUSELEAVE => {
            with_window(|window| {
                window.view.hover = None;
                window.view.pressed = None;
                window.invalidate();
            });
            Some(LRESULT(0))
        }
        WM_LBUTTONDOWN => {
            with_window(|window| {
                window.view.pressed = window.hit_at(lparam);
                window.view.keyboard = false;
                window.invalidate();
            });
            Some(LRESULT(0))
        }
        WM_LBUTTONUP => {
            let action = with_window(|window| {
                let action = window.hit_at(lparam).filter(|a| window.view.pressed == Some(*a));
                window.view.pressed = None;
                window.view.focus = action;
                window.invalidate();
                action
            })
            .flatten();
            if let Some(action) = action {
                perform(action);
            }
            Some(LRESULT(0))
        }
        WM_MOUSEWHEEL => {
            with_window(|window| {
                let delta = (wparam.0 >> 16) as u16 as i16 as f32 / 120.0;
                window.view.scroll -= delta * 64.0;
                window.clamp_scroll(window.visible_height());
                window.view.hover = None;
                window.invalidate();
            });
            Some(LRESULT(0))
        }
        WM_KEYDOWN => {
            if let Some(action) = with_window(|window| window.key(wparam.0 as u16)).flatten() {
                perform(action);
            }
            Some(LRESULT(0))
        }
        WM_CLOSE => {
            let _ = unsafe { DestroyWindow(hwnd) };
            Some(LRESULT(0))
        }
        WM_DESTROY => {
            let _ = set_taskbar_id(hwnd, false);
            WINDOW.with(|cell| {
                if let Ok(mut window) = cell.try_borrow_mut() {
                    window.take();
                }
            });
            Some(LRESULT(0))
        }
        _ => None,
    };
    handled.unwrap_or_else(|| unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) })
}

// ---------------------------------------------------------------- pratinjau untuk tes

/// Gambar satu kategori ke piksel (BGRA premultiplied) tanpa jendela, untuk dicek di tes.
#[cfg(test)]
pub fn preview(category: Category, dark: bool, state: &MenuState, narrow: bool) -> (Vec<u32>, i32, i32) {
    let size = if narrow { MIN_SIZE } else { START_SIZE };
    let (w, h) = (size.0 as i32, size.1 as i32);
    let mut surface = crate::render::Surface::new(w, h).unwrap();
    let factory: ID2D1Factory = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap() };
    let target = crate::render::dc_target(&factory, 96.0).unwrap();
    let kind = Fonts::new().unwrap();
    unsafe {
        target.BindDC(surface.dc, &RECT { left: 0, top: 0, right: w, bottom: h }).unwrap();
        let brush = target.CreateSolidColorBrush(&rgb(0, 1.0), None).unwrap();
        let view = View { category, hover: None, pressed: None, focus: None, keyboard: false, scroll: 0.0 };
        let mut sheet = Sheet { target: &target, factory: &factory, brush: &brush, kind: &kind, colors: colors(dark, 0x0078D4), view, size, hits: Vec::new() };
        target.BeginDraw();
        sheet.paint(state, false);
        target.EndDraw(None, None).unwrap();
        let _ = windows::Win32::Graphics::Gdi::GdiFlush();
    }
    (surface.pixels().to_vec(), w, h)
}

#[cfg(test)]
mod tests {
    /// Teks untuk properti taskbar terbaca utuh sebagai VT_LPWSTR.
    #[test]
    fn taskbar_text() {
        let text = r#""C:\Program Files\Notch Windows\notch.exe",-1"#;
        let value = super::text_value(text).unwrap();
        assert_eq!(value.vt(), super::VT_LPWSTR);
        assert_eq!(value.to_string(), text);
    }
}
