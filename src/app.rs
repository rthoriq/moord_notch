//! Jendela notch: status, input mouse/keyboard, timer, dan loop pesan.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{HGLOBAL, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmFlush;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, HMONITOR, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows::Win32::System::Ole::CF_UNICODETEXT;
use windows::Win32::System::RemoteDesktop::{
    NOTIFY_FOR_THIS_SESSION, WTSRegisterSessionNotification, WTSUnRegisterSessionNotification,
};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId, Sleep};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, MOD_ALT, MOD_NOREPEAT, MOD_WIN, RegisterHotKey, ReleaseCapture, SetCapture, SetFocus,
    TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent, UnregisterHotKey, VK_CAPITAL, VK_NUMLOCK,
};
use windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK,
    RIDEV_REMOVE, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetWindowThreadProcessId,
    WM_HOTKEY,
    GWL_EXSTYLE, GetForegroundWindow, GetMessageW, GetSystemMetrics, GetWindowLongPtrW,
    HWND_TOPMOST, IDC_ARROW, IDC_HAND, IsWindow, KillTimer, LoadCursorW, MA_NOACTIVATE, MSG,
    PBT_APMRESUMEAUTOMATIC, PBT_POWERSETTINGCHANGE, PM_REMOVE, PeekMessageW, PostMessageW,
    PostQuitMessage, RegisterClassExW, RegisterWindowMessageW, SM_CXDRAG, SM_CXSMICON, SM_CYDRAG,
    SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetCursor,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage,
    RI_KEY_BREAK, WA_INACTIVE, WM_ACTIVATE, WM_APP, WM_CHAR, WM_DISPLAYCHANGE, WM_DPICHANGED, WM_INPUT,
    WM_LBUTTONDBLCLK, WM_WTSSESSION_CHANGE, WTS_SESSION_LOCK, WTS_SESSION_UNLOCK,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
    WM_POWERBROADCAST, WM_QUIT, WM_RBUTTONUP, WM_SETCURSOR, WM_TIMECHANGE, WM_TIMER, WNDCLASSEXW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{Error, PCWSTR, Result, w};

use crate::anim::Spring;
use crate::desktop::{self, Mode, Watcher};
use crate::dragdrop::{self, DRAG_DROP, DRAG_ENTER, DRAG_LEAVE, DRAG_OVER, DropRegistration};
use crate::lyrics::{self, Lyrics};
use crate::media::{self, Snapshot};
use crate::power::{self, Power};
use crate::privacy::Usage;
use crate::render::{
    self, AgendaItem, Button, CONTENTS, CalendarView, ControlsView, DayView, Eyes, FocusView, Hit, Hud, LyricsView,
    MediaView, Mood, Renderer, Roll, Scene, ShelfFx, TimerView, View, WaterView, hud_value,
};
use crate::services::{self, Event, Services, Task};
use crate::settings::{Messages, Settings, Size};
use crate::sound::{self, Sound};
use crate::folders::{self, Watch};
use crate::{messages, network, share};
use crate::sysinfo::Stats;
use crate::tray::{self, Command, MenuState, PageCommand, PageMenu, Tray};
use crate::widgets::Page;
use crate::widgets::focus::{self, Focus};
use crate::widgets::shelf::{self, Shelf};
use crate::widgets::tasks::{self, Tasks};
use crate::widgets::agenda::{self, Agenda, Date};
use crate::widgets::timer::{self, Timer};
use crate::widgets::water::{GOAL_ML, Water};
use crate::weather::{self, City, Weather};
use crate::{autostart, clock};

const CLASS: PCWSTR = w!("NotchWindow");
/// Didefinisikan di fitur `Win32_UI_Controls` yang besar; cukup satu konstanta ini.
const WM_MOUSELEAVE: u32 = 0x02A3;
/// Mulai menyeret file dari shelf, di luar state aplikasi (lihat `wndproc`).
const WM_START_DRAG: u32 = WM_APP + 6;

const TIMER_CLOCK: usize = 1;
const TIMER_OPEN: usize = 2;
const TIMER_CLOSE: usize = 3;
const TIMER_BLINK: usize = 4;
const TIMER_DESKTOP: usize = 5;
const TIMER_HUD: usize = 6;
/// Gelombang musik, progres lagu, dan hitung mundur pomodoro.
const TIMER_AMBIENT: usize = 7;
/// Cek ringan jendela maximize, yang tidak punya event sendiri.
const TIMER_POLL: usize = 8;
/// Saat fase pomodoro habis.
const TIMER_FOCUS: usize = 9;
/// Sesaat setelah Caps/Num Lock dilepas, saat status lampunya sudah berganti.
const TIMER_LOCK_KEYS: usize = 10;
/// Mata maskot melirik ke arah kursor.
const TIMER_GAZE: usize = 11;
/// Ekspresi sementara maskot (senang, kaget) selesai.
const TIMER_MOOD: usize = 12;
/// Timer cepat habis.
const TIMER_ALARM: usize = 13;
/// Perbarui cuaca.
const TIMER_WEATHER: usize = 14;
/// Mode senyap berakhir.
const TIMER_QUIET: usize = 15;
/// Id pintasan Win+Alt+N.
const HOTKEY_ID: i32 = 1;

const POLL_MS: u32 = 1_000;
/// Jeda sebelum membuka, supaya tidak terbuka saat kursor hanya lewat menuju tab browser.
const OPEN_DELAY_MS: u32 = 200;
const CLOSE_DELAY_MS: u32 = 380;
/// Setelah file dilepas, panel tetap terbuka sebentar supaya hasilnya terlihat.
const DROP_CLOSE_MS: u32 = 2_500;
/// Tunggu jendela berhenti bergerak sebelum mengecek full-screen/maximize.
const DESKTOP_SETTLE_MS: u32 = 60;
const BLINK_SECS: f64 = 0.16;
const PAGE_ANIM_SECS: f64 = 0.24;
/// Selisih ukuran (DIP) yang sudah tidak terlihat; di bawahnya animasi bentuk dianggap selesai.
const SHAPE_PRECISION: f32 = 0.05;
/// Seperti notch di Mac: membuka sedikit memantul, menutup mulus tanpa pantulan.
const OPEN_BOUNCE: f32 = 0.72;
const CLOSE_BOUNCE: f32 = 1.0;
/// Animasi shelf: ubin baru muncul, ubin dihapus menghilang, pendaran sesudah file dilepas.
const TILE_IN_SECS: f64 = 0.4;
const TILE_OUT_SECS: f64 = 0.22;
const GLOW_SECS: f64 = 0.7;
/// Lama angka kabar bergulir ke nilai baru.
const ROLL_SECS: f64 = 0.22;
/// Garis putus-putus area lepas berjalan selama file diseret (~25 gambar per detik).
const DASH_MS: u32 = 40;
/// Pil yang disentuh kursor langsung sedikit membesar sebelum panel terbuka.
const PEEK_GROW: (f32, f32) = (12.0, 4.0);

const VOLUME_HUD_MS: u32 = 1_600;
const POWER_HUD_MS: u32 = 3_000;
const BLUETOOTH_HUD_MS: u32 = 3_500;
const FOCUS_HUD_MS: u32 = 6_000;
const WATER_HUD_MS: u32 = 6_000;
const PRIVACY_HUD_MS: u32 = 3_500;
const BRIGHTNESS_HUD_MS: u32 = 1_600;
const LOCK_KEY_HUD_MS: u32 = 1_400;
const MESSAGE_HUD_MS: u32 = 5_000;
const SCREENSHOT_HUD_MS: u32 = 4_000;
const DOWNLOAD_HUD_MS: u32 = 4_500;
const WIFI_HUD_MS: u32 = 3_000;
/// Panel yang dibuka dengan pintasan menutup sendiri kalau kursor tidak datang.
const HOTKEY_CLOSE_MS: u32 = 5_000;
/// Seberapa sering posisi kursor dilihat selama mata maskot terlihat; lebih jarang kalau
/// kursor sudah lama diam.
const GAZE_MS: u32 = 200;
const GAZE_IDLE_MS: u32 = 1_000;
/// Kursor diam selama ini: mata kembali menatap ke depan.
const GAZE_REST: Duration = Duration::from_secs(30);
/// Lama ekspresi sementara setelah kabarnya hilang.
const MOOD_MS: u32 = 5_000;
const TIMER_HUD_MS: u32 = 8_000;
const AGENDA_HUD_MS: u32 = 8_000;
/// Geser dua jari di pil sejauh ini (satuan roda mouse) untuk pindah lagu, dan jeda minimal
/// antar-pindah supaya satu gerakan tidak melompati beberapa lagu.
const SWIPE_DISTANCE: i32 = 240;
const SWIPE_COOLDOWN: f64 = 0.7;
/// Pil melebar ke kanan selama ada unduhan, untuk ukuran file yang sedang diunduh.
const DOWNLOAD_GROW: f32 = 60.0;
/// Maskot bangun: mata terpejam sebentar, lalu terbuka perlahan.
const WAKE_CLOSED_SECS: f64 = 0.3;
const WAKE_OPEN_SECS: f64 = 0.45;
const WEATHER_EVERY_MS: u32 = 30 * 60 * 1_000;
/// Cuaca yang lebih tua dari ini tidak ditampilkan lagi.
const WEATHER_STALE: Duration = Duration::from_secs(3 * 60 * 60);
const QUIET: Duration = Duration::from_secs(60 * 60);
/// Batas panjang nama kota yang diketik.
const CITY_MAX: usize = 40;
/// Gelombang di pil digambar ulang ~12 kali per detik: cukup hidup, nyaris tanpa beban CPU.
const WAVE_MS: u32 = 80;
const PROGRESS_MS: u32 = 1_000;
/// Lama kolom lirik bergulir ke baris berikutnya.
const LYRIC_SCROLL_SECS: f64 = 0.38;
/// Hitung mundur di halaman pomodoro; lebih rapat dari sedetik supaya angkanya tidak meloncat.
const FOCUS_TICK_MS: u32 = 500;
/// Cincin pomodoro di pil cukup diperbarui sesekali.
const PILL_FOCUS_MS: u32 = 5_000;
/// Peringatan saat baterai turun melewati batas ini tanpa charger.
const LOW_BATTERY: [u32; 2] = [20, 10];
/// Pengingat minum: tiap jam setelah minum terakhir, diulang tiap 30 menit kalau belum minum.
const WATER_EVERY: Duration = Duration::from_secs(60 * 60);
const WATER_AGAIN: Duration = Duration::from_secs(30 * 60);
/// Jam aktif pengingat minum (07.00–21.59).
const WATER_HOURS: Range<u16> = 7..22;

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Jumlah frame animasi; dicatat di log debug tiap menit untuk memantau beban.
static FRAMES: AtomicU32 = AtomicU32::new(0);

pub fn run() -> Result<()> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_DBLCLKS,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: CLASS,
            ..Default::default()
        };
        if RegisterClassExW(&class) == 0 {
            return Err(Error::from_thread());
        }
        // Layered: bentuk bebas + transparan. Tool window: tidak muncul di taskbar/Alt-Tab.
        // No-activate: mengklik notch tidak merebut fokus dari aplikasi yang sedang dipakai.
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            CLASS,
            w!("Notch"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance.into()),
            None,
        )?;

        let mut app = App::new(hwnd)?;
        app.mode = desktop::current_mode(hwnd, app.monitor, &app.monitor_rect);
        app.apply_visibility();
        app.wake_up();
        app.refresh();
        app.schedule_clock();
        app.set_timer(TIMER_POLL, POLL_MS);
        APP.with(|cell| *cell.borrow_mut() = Some(app));
        // Kabar layanan yang tiba sebelum state siap belum sempat diproses.
        let _ = PostMessageW(Some(hwnd), services::WM_SERVICE, WPARAM(0), LPARAM(0));

        let mut msg = MSG::default();
        'pump: loop {
            if with_app(|app| app.animating()).unwrap_or(false) {
                // Saat animasi: proses semua pesan, gambar satu frame, tunggu refresh layar berikutnya.
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        break 'pump;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                with_app(|app| app.tick());
                if DwmFlush().is_err() {
                    Sleep(7);
                }
            } else {
                // Saat diam: tidur sampai ada pesan. Di sinilah CPU tetap 0%.
                if GetMessageW(&mut msg, None, 0, 0).0 <= 0 {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Melepas App menghapus ikon tray, hook, dan menghentikan layanan sebelum jendela dihancurkan.
        APP.with(|cell| cell.borrow_mut().take());
        let _ = DestroyWindow(hwnd);
        Ok(())
    }
}

/// Akses state aplikasi. Mengembalikan `None` kalau state sedang dipakai, misalnya saat
/// Windows mengirim pesan secara bersarang di tengah pemanggilan API.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| {
        let mut guard = cell.try_borrow_mut().ok()?;
        guard.as_mut().map(f)
    })
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // Menu tray dan seret-keluar punya loop pesan sendiri, jadi dijalankan di luar state aplikasi.
    if msg == tray::WM_TRAY {
        let event = (lparam.0 & 0xFFFF) as u32;
        if (event == WM_LBUTTONUP || event == WM_RBUTTONUP)
            && let Some(state) = with_app(|app| app.menu_state())
            && let Some(command) = tray::show_menu(hwnd, state)
        {
            with_app(|app| app.on_command(command));
        }
        return LRESULT(0);
    }
    if msg == WM_START_DRAG {
        if let Some(path) = with_app(|app| app.drag_path()).flatten() {
            let _ = dragdrop::drag_out(hwnd, &path);
            with_app(|app| app.after_drag());
        }
        return LRESULT(0);
    }
    if msg == WM_RBUTTONUP
        && let Some(menu) = with_app(|app| app.page_menu(lparam)).flatten()
    {
        let command = tray::show_page_menu(hwnd, &menu);
        with_app(|app| app.after_page_menu(menu.page, command));
        return LRESULT(0);
    }
    match with_app(|app| app.handle(msg, wparam, lparam)).flatten() {
        Some(result) => result,
        None => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

struct App {
    hwnd: HWND,
    renderer: Renderer,
    tray: Tray,
    // Disimpan hanya supaya hook dan pendaftarannya tetap hidup sampai aplikasi ditutup.
    _watcher: Watcher,
    _power: Power,
    _drop: Option<DropRegistration>,
    services: Services,
    monitor: HMONITOR,
    monitor_rect: RECT,
    /// Bentuk yang sedang dituju.
    view: View,
    width: Spring,
    height: Spring,
    /// Ukuran saat perpindahan bentuk dimulai, untuk menghitung seberapa jauh sudah berjalan.
    from: (f32, f32),
    /// Transparansi tiap isi; lihat [`View::content`].
    alpha: [f32; CONTENTS],
    page: Page,
    /// Perpindahan halaman yang sedang berjalan: halaman asal, waktu mulai, arah.
    page_anim: Option<(Page, f64, f32)>,
    blink_since: Option<f64>,
    expanded: bool,
    hovering: bool,
    tracking_leave: bool,
    hovered: Option<Hit>,
    mode: Mode,
    user_hidden: bool,
    shown: bool,
    media: Option<Snapshot>,
    /// Isi HUD terakhir; tetap disimpan selama HUD memudar keluar.
    hud: Option<Hud>,
    hud_visible: bool,
    plugged: Option<bool>,
    battery: Option<u32>,
    shelf: Shelf,
    focus: Focus,
    water: Water,
    tasks: Tasks,
    settings: Settings,
    /// Aplikasi yang sedang memakai kamera/mikrofon; `privacy_known` false sampai kabar pertama.
    privacy: Usage,
    privacy_known: bool,
    /// Lirik lagu terakhir yang diambil, dan kunci lagu yang sedang diambilkan liriknya.
    lyrics: Option<Lyrics>,
    lyric_pending: Option<String>,
    /// Baris lirik yang terakhir digambar, untuk tahu kapan perlu gambar ulang.
    lyric_shown: Option<usize>,
    /// Kolom kanan Beranda: 0 = kalender, 1 = lirik (bergeser halus di antaranya).
    lyric_mix: f32,
    /// Kolom lirik mulai bergulir ke baris berikutnya pada waktu ini.
    lyric_scroll: Option<f64>,
    /// Baris kolom lirik (-1 = sebelum baris pertama) dan detik lagu yang terakhir digambar.
    lyric_drawn: Option<isize>,
    second_drawn: Option<u64>,
    /// Posisi bar progres lagu (0..1) yang sedang diseret.
    seek_preview: Option<f32>,
    /// Bentuk berpindah antara dua ukuran panel (misal kolom lirik masuk): isinya tetap tampil.
    panel_resize: bool,
    system: Option<Stats>,
    /// Thread layanan sedang diminta mengukur sistem.
    system_watch: bool,
    /// Ukuran bentuk yang sedang dituju (pil lirik ukurannya mengikuti baris lirik).
    target: (f32, f32),
    /// Laptop sedang terkunci: notch tetap tampil di lock screen dengan isi terbatas.
    locked: bool,
    /// Kecerahan terakhir yang diketahui (kabar pertama hanya keadaan awal).
    brightness: Option<u32>,
    /// Status Caps Lock dan Num Lock terakhir.
    lock_keys: (bool, bool),
    /// Notch mendengarkan input keyboard mentah (hanya untuk Caps/Num Lock).
    raw_keyboard: bool,
    /// Notch terpasang sebagai paket, jadi notifikasi pesan bisa dibaca kalau diizinkan.
    messages_available: bool,
    /// Pengguna menolak izin membaca notifikasi.
    messages_denied: bool,
    /// Pintasan Win+Alt+N terdaftar.
    hotkey: bool,
    /// Kursor menyentuh pil: pil sedikit membesar sebelum panel terbuka.
    peek: bool,
    /// Zona lepas yang sedang berada tepat di bawah file yang diseret.
    drop_over: Option<DropZone>,
    /// Jendela Bagikan Windows; dibuat saat pertama dipakai.
    share: Option<share::Share>,
    /// Kapan file terakhir dilepas, untuk pendaran.
    drop_glow: Option<f64>,
    /// Isi bilah kabar volume/kecerahan saat ini dan yang dituju (bergeser halus).
    hud_fill: f32,
    hud_fill_target: f32,
    /// Angka kabar yang sedang bergulir: tulisan lama, mulai, naik/turun.
    hud_roll: Option<(String, f64, bool)>,
    /// Ubin shelf yang sedang muncul (`false`) atau menghilang (`true`), dan kapan mulainya.
    tile_fx: HashMap<u64, (f64, bool)>,
    /// Kekuatan bayangan saat ini (lihat [`Scene::shadow`]).
    shadow: f32,
    /// Arah lirikan mata saat ini dan yang dituju (lihat [`Eyes::look`]).
    gaze: (f32, f32),
    gaze_target: (f32, f32),
    /// Posisi kursor terakhir dan kapan terakhir bergerak.
    cursor: POINT,
    cursor_moved: Instant,
    /// Interval timer lirikan yang sedang aktif.
    gaze_poll: Option<u32>,
    /// Ekspresi sementara maskot dan kapan berakhir.
    mood: Option<(Mood, Instant)>,
    /// Menu klik kanan panel sedang terbuka; panel jangan ditutup dulu.
    menu_open: bool,
    /// Keadaan Wi-Fi terakhir; `None` sampai kabar pertama (keadaan awal tidak diumumkan).
    network: Option<network::State>,
    /// Pemantau folder Screenshots dan Downloads.
    screenshots: Option<Watch>,
    downloads: Option<Watch>,
    timer: Timer,
    agenda: Agenda,
    /// Hari yang dipilih di kalender, sebagai selisih dari hari ini.
    cal_offset: i32,
    /// Geseran dua jari di pil yang terkumpul, kapan terakhir bergeser, dan kapan terakhir pindah lagu.
    swipe: (i32, f64, f64),
    /// Kapan maskot mulai bangun (notch baru muncul, setelah buka kunci atau bangun dari tidur).
    wake: Option<f64>,
    /// Unduhan yang sedang berjalan: jumlah file dan total byte.
    downloading: Option<(usize, u64)>,
    /// Volume speaker (0..1, bisu) dan bisu mikrofon terakhir, untuk halaman kontrol.
    volume: Option<(f32, bool)>,
    mic: Option<bool>,
    /// Slider yang sedang diseret.
    slider_drag: Option<Hit>,
    /// Mode senyap notch sampai waktu ini.
    quiet_until: Option<Instant>,
    city: Option<City>,
    /// Nama kota yang sedang diketik.
    city_draft: Option<String>,
    /// Keterangan pencarian kota, misal "Mencari kota…".
    city_status: Option<String>,
    weather: Option<(Weather, Instant)>,
    /// Kunci hari ini ("2026-10-19"), untuk catatan minum dan reset harian.
    day: String,
    water_due: Instant,
    /// Ubin shelf yang sedang ditekan dan titik awalnya, untuk mendeteksi seretan.
    press: Option<(u64, POINT)>,
    /// Ubin yang sedang diseret keluar.
    drag_item: Option<u64>,
    /// Jendela yang aktif sebelum mulai mengetik tugas; fokus dikembalikan ke sini.
    edit_return: Option<HWND>,
    /// Separuh pertama karakter UTF-16 (misal emoji) yang menunggu pasangannya.
    pending_surrogate: Option<u16>,
    /// Sisa putaran roda mouse yang belum genap satu langkah (touchpad mengirim langkah kecil).
    wheel: i32,
    /// Interval timer ambient yang sedang aktif.
    ambient: Option<u32>,
    epoch: Instant,
    last_frame: f64,
    /// Kapan terakhir digambar penuh (gambar gelombang saja tidak memperbarui jam/cincin).
    last_full: f64,
    rng: u32,
    taskbar_created: u32,
}

impl App {
    fn new(hwnd: HWND) -> Result<Self> {
        let (monitor, monitor_rect) = primary_monitor();
        let settings = Settings::load();
        let renderer = Renderer::new(scaled_dpi(monitor_dpi(monitor), settings.size), settings.theme)?;
        let icon = render::make_icon(unsafe { GetSystemMetrics(SM_CXSMICON) })?;
        let seed = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1);
        let drop = DropRegistration::register(hwnd)
            .inspect_err(|e| debug_log!("seret-lepas file tidak aktif: {e}"))
            .ok();
        let (w, h) = View::Pill.size();
        let mut app = Self {
            hwnd,
            renderer,
            tray: Tray::new(hwnd, icon),
            _watcher: Watcher::new(hwnd),
            _power: Power::register(hwnd),
            _drop: drop,
            services: Services::start(hwnd),
            monitor,
            monitor_rect,
            view: View::Pill,
            width: Spring::new(w, 300.0, 0.74, SHAPE_PRECISION),
            height: Spring::new(h, 300.0, 0.74, SHAPE_PRECISION),
            from: (w, h),
            alpha: [1.0, 0.0, 0.0, 0.0],
            page: Page::Home,
            page_anim: None,
            blink_since: None,
            expanded: false,
            hovering: false,
            tracking_leave: false,
            hovered: None,
            mode: Mode::Normal,
            user_hidden: false,
            shown: false,
            media: None,
            hud: None,
            hud_visible: false,
            plugged: None,
            battery: None,
            shelf: Shelf::load(),
            focus: Focus::default(),
            water: Water::load(),
            tasks: Tasks::load(),
            settings,
            privacy: Usage::default(),
            privacy_known: false,
            lyrics: None,
            lyric_pending: None,
            lyric_shown: None,
            lyric_mix: 0.0,
            lyric_scroll: None,
            lyric_drawn: None,
            second_drawn: None,
            seek_preview: None,
            panel_resize: false,
            system: None,
            system_watch: false,
            target: (w, h),
            locked: false,
            brightness: None,
            lock_keys: lock_key_states(),
            raw_keyboard: false,
            messages_available: false,
            messages_denied: false,
            hotkey: false,
            peek: false,
            drop_over: None,
            share: None,
            drop_glow: None,
            hud_fill: 0.0,
            hud_fill_target: 0.0,
            hud_roll: None,
            tile_fx: HashMap::new(),
            shadow: 0.0,
            gaze: (0.0, 0.0),
            gaze_target: (0.0, 0.0),
            cursor: POINT::default(),
            cursor_moved: Instant::now(),
            gaze_poll: None,
            mood: None,
            menu_open: false,
            network: None,
            screenshots: None,
            downloads: None,
            timer: Timer::default(),
            agenda: Agenda::load(today()),
            cal_offset: 0,
            swipe: (0, 0.0, f64::MIN),
            wake: Some(0.0),
            downloading: None,
            volume: None,
            mic: None,
            slider_drag: None,
            quiet_until: None,
            city: City::load(),
            city_draft: None,
            city_status: None,
            weather: None,
            day: clock::day_key(&clock::now()),
            water_due: Instant::now() + WATER_EVERY,
            press: None,
            drag_item: None,
            edit_return: None,
            pending_surrogate: None,
            wheel: 0,
            ambient: None,
            epoch: Instant::now(),
            last_frame: 0.0,
            last_full: 0.0,
            rng: seed | 1,
            taskbar_created: unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) },
        };
        app.services.thumbnails(app.shelf.items.iter().map(|item| (item.id, item.path.clone())).collect());
        // Kabar kunci/buka kunci laptop, supaya notch tetap tampil di lock screen.
        if let Err(e) = unsafe { WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) } {
            debug_log!("kabar lock screen tidak aktif: {e}");
        }
        app.set_raw_keyboard(app.settings.lock_keys);
        app.messages_available = messages::packaged();
        debug_log!("identitas paket: {}", app.messages_available);
        if app.messages_available && app.settings.messages != Messages::Off {
            messages::request_access(app.services.notifier());
        }
        app.set_hotkey(app.settings.hotkey);
        app.update_watchers();
        app.services.send(Task::WatchNetwork(app.settings.wifi));
        app.fetch_weather();
        Ok(app)
    }

    fn handle(&mut self, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        match msg {
            WM_MOUSEMOVE => self.on_mouse_move(lparam),
            WM_MOUSELEAVE => {
                self.on_leave();
                self.set_hovered(None);
            }
            WM_LBUTTONDOWN => self.on_press(lparam),
            WM_LBUTTONUP => self.on_click(lparam),
            WM_LBUTTONDBLCLK => self.on_double_click(lparam),
            WM_RBUTTONUP => self.on_right_click(lparam),
            WM_MOUSEWHEEL | WM_MOUSEHWHEEL => self.on_wheel(wparam, lparam, msg == WM_MOUSEHWHEEL),
            WM_CHAR => self.on_char(wparam.0 as u16),
            WM_ACTIVATE => {
                // Klik di aplikasi lain saat sedang mengetik: simpan yang sudah diketik.
                if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE {
                    self.end_edit(true, false);
                    self.refresh();
                }
                return None;
            }
            WM_SETCURSOR => {
                if !self.hovered.is_some_and(Hit::clickable) {
                    return None;
                }
                unsafe { SetCursor(LoadCursorW(None, IDC_HAND).ok()) };
                return Some(LRESULT(1));
            }
            WM_MOUSEACTIVATE => return Some(LRESULT(MA_NOACTIVATE as isize)),
            WM_TIMER => self.on_timer(wparam.0),
            WM_HOTKEY => self.on_hotkey(),
            WM_INPUT => {
                self.on_raw_input(lparam);
                // Windows perlu membersihkan data input mentahnya sendiri.
                return None;
            }
            WM_WTSSESSION_CHANGE => self.on_session_change(wparam.0 as u32),
            dragdrop::WM_DRAG => self.on_drag(wparam.0, lparam),
            desktop::WM_DESKTOP_CHANGED => self.set_timer(TIMER_DESKTOP, DESKTOP_SETTLE_MS),
            desktop::WM_APPBAR => self.refresh_mode(false),
            services::WM_SERVICE => self.on_service(),
            WM_DISPLAYCHANGE | WM_DPICHANGED => {
                self.place();
                self.refresh_mode(true);
            }
            WM_TIMECHANGE => self.on_clock(),
            WM_POWERBROADCAST => return self.on_power_broadcast(wparam, lparam),
            m if m == self.taskbar_created => self.tray.add(),
            _ => return None,
        }
        Some(LRESULT(0))
    }

    fn on_mouse_move(&mut self, lparam: LPARAM) {
        self.on_hover();
        if let Some(slider) = self.slider_drag {
            self.apply_slider(slider, lparam);
            return;
        }
        // Ubin shelf yang ditekan lalu digeser cukup jauh mulai diseret keluar.
        if let Some((id, start)) = self.press {
            let (x, y) = point(lparam);
            let (dx, dy) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
            if (x - start.x).abs() > dx || (y - start.y).abs() > dy {
                self.press = None;
                let _ = unsafe { ReleaseCapture() };
                self.drag_item = Some(id);
                let _ = unsafe { PostMessageW(Some(self.hwnd), WM_START_DRAG, WPARAM(0), LPARAM(0)) };
                return;
            }
        }
        let hit = if self.expanded { self.hit_at(lparam) } else { None };
        self.set_hovered(hit);
    }

    fn on_hover(&mut self) {
        if !self.tracking_leave {
            let mut track = TRACKMOUSEEVENT {
                cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: self.hwnd,
                dwHoverTime: 0,
            };
            self.tracking_leave = unsafe { TrackMouseEvent(&mut track) }.is_ok();
        }
        if !self.hovering {
            self.hovering = true;
            self.kill_timer(TIMER_CLOSE);
            if !self.expanded {
                self.set_timer(TIMER_OPEN, OPEN_DELAY_MS);
                self.peek = true;
                self.refresh();
            }
        }
    }

    fn on_leave(&mut self) {
        self.tracking_leave = false;
        self.hovering = false;
        if std::mem::take(&mut self.peek) {
            self.refresh();
        }
        self.kill_timer(TIMER_OPEN);
        // Selama mengetik, panel tetap terbuka walau kursor pergi.
        if self.expanded && !self.editing() {
            self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
        }
    }

    fn on_press(&mut self, lparam: LPARAM) {
        if !self.expanded {
            return;
        }
        match self.hit_at(lparam) {
            Some(Hit::ShelfItem(id)) => {
                let (x, y) = point(lparam);
                self.press = Some((id, POINT { x, y }));
                unsafe { SetCapture(self.hwnd) };
            }
            // Slider langsung mengikuti kursor, lalu bisa diseret.
            Some(slider @ (Hit::VolumeSlider | Hit::BrightnessSlider | Hit::MediaSeek)) => {
                self.slider_drag = Some(slider);
                unsafe { SetCapture(self.hwnd) };
                self.apply_slider(slider, lparam);
            }
            _ => {}
        }
    }

    /// Ubah volume/kecerahan sesuai posisi kursor di slider.
    fn apply_slider(&mut self, slider: Hit, lparam: LPARAM) {
        let x = point(lparam).0 as f32 / self.renderer.scale();
        let Some(value) = self.renderer.slider_value(slider, x) else { return };
        self.set_level(slider, value);
    }

    /// Volume (0..1) atau kecerahan; tampilan langsung berubah, perangkatnya menyusul.
    fn set_level(&mut self, slider: Hit, value: f32) {
        let value = value.clamp(0.0, 1.0);
        match slider {
            Hit::VolumeSlider => {
                self.volume = Some((value, false));
                self.services.send(Task::SetVolume(value));
            }
            Hit::BrightnessSlider if self.brightness.is_some() => {
                let percent = (value * 100.0).round() as u8;
                self.brightness = Some(percent as u32);
                self.services.send(Task::SetBrightness(percent));
            }
            // Bar lagu hanya berpindah di layar selama diseret; lompatnya saat dilepas.
            Hit::MediaSeek => self.seek_preview = Some(value),
            _ => return,
        }
        if !self.animating() {
            self.redraw();
        }
    }

    fn on_click(&mut self, lparam: LPARAM) {
        if let Some(slider) = self.slider_drag.take() {
            let _ = unsafe { ReleaseCapture() };
            if slider == Hit::MediaSeek {
                self.finish_seek();
            }
            return;
        }
        // Klik biasa pada ubin shelf tidak melakukan apa-apa; buka dengan klik dua kali.
        if self.press.take().is_some() {
            let _ = unsafe { ReleaseCapture() };
            return;
        }
        self.kill_timer(TIMER_OPEN);
        if !self.expanded {
            // Kabar pesan membuka aplikasinya, kabar unduhan menunjukkan file-nya di folder
            // (tidak dari lock screen).
            if self.hud_visible && !self.locked {
                let opened = match &self.hud {
                    Some(Hud::Message { aumid, app, browser, .. }) => {
                        messages::open(self.hwnd, aumid, app, *browser);
                        true
                    }
                    Some(Hud::Download { path, .. }) => {
                        dragdrop::reveal(self.hwnd, path);
                        true
                    }
                    _ => false,
                };
                if opened {
                    self.hud_visible = false;
                    self.refresh();
                    return;
                }
            }
            // Mengklik kabar singkat lain membuka halaman yang bersangkutan.
            if self.hud_visible
                && let Some(hud) = &self.hud
            {
                let page = match hud {
                    Hud::Focus { .. } => Page::Focus,
                    Hud::Water { .. } => Page::Water,
                    Hud::Privacy { .. } => Page::System,
                    Hud::Screenshot { .. } => Page::Shelf,
                    Hud::Timer { .. } => Page::Timer,
                    _ => Page::Home,
                };
                self.page = if self.settings.pages.shown(page) { page } else { Page::Home };
                self.page_anim = None;
            }
            if self.locked {
                self.page = Page::Home;
            }
            self.set_expanded(true);
            return;
        }
        let hit = self.hit_at(lparam);
        let typing_here = matches!(hit, Some(Hit::TaskInput | Hit::CityInput | Hit::AgendaInput));
        if self.editing() && !typing_here {
            self.end_edit(true, true);
        }
        if let Some(hit) = hit {
            self.on_hit(hit);
        }
        self.refresh();
    }

    fn on_hit(&mut self, hit: Hit) {
        let now = Instant::now();
        match hit {
            Hit::PagePrev => self.go_page(self.settings.pages.step(self.page, -1), -1.0),
            Hit::PageNext => self.go_page(self.settings.pages.step(self.page, 1), 1.0),
            Hit::PageDot(page) => self.go_page(page, self.page_dir(page)),
            Hit::Media(button) => self.media_command(button),
            Hit::LyricsToggle => {
                self.start_motion();
                self.settings.lyric_panel = !self.settings.lyric_panel;
                self.settings.save();
                self.request_lyrics();
            }
            // Ubin mengecil dan memudar dulu; baru dibuang dari shelf setelah animasinya selesai.
            Hit::ShelfClear => {
                let ids: Vec<u64> = self.shelf.items.iter().map(|item| item.id).collect();
                ids.into_iter().for_each(|id| self.fade_tile(id));
            }
            Hit::ShelfRemove(id) => self.fade_tile(id),
            Hit::FocusToggle => {
                self.focus.toggle(now);
                self.schedule_focus();
            }
            Hit::FocusReset => {
                self.focus.reset();
                self.schedule_focus();
            }
            Hit::FocusPhase(phase) => {
                if self.focus.phase != phase {
                    self.focus.select(phase);
                    self.schedule_focus();
                }
            }
            Hit::WaterDrink => {
                let before = self.water.on(&self.day);
                self.water.drink(&self.day);
                self.water.save();
                self.water_due = now + WATER_EVERY;
                // Target hari ini tercapai: maskot ikut senang.
                if before < GOAL_ML && self.water.on(&self.day) >= GOAL_ML {
                    self.set_mood(Mood::Happy, 8_000);
                }
            }
            Hit::WaterUndo => {
                self.water.undo(&self.day);
                self.water.save();
            }
            Hit::TaskToggle(index) => {
                self.tasks.toggle(index);
                self.tasks.save();
            }
            Hit::TaskRemove(index) => {
                self.tasks.remove(index);
                self.tasks.save();
            }
            Hit::TaskInput => self.begin_edit(Field::Task),
            Hit::TasksClearDone => {
                self.tasks.clear_done();
                self.tasks.save();
            }
            Hit::VolumeMute => self.services.send(Task::ToggleMute),
            Hit::MicToggle => self.services.send(Task::ToggleMic),
            Hit::QuietToggle => {
                if self.quiet() {
                    self.quiet_until = None;
                    self.kill_timer(TIMER_QUIET);
                } else {
                    self.quiet_until = Some(now + QUIET);
                    self.set_timer(TIMER_QUIET, QUIET.as_millis() as u32);
                }
            }
            Hit::CityInput => self.begin_edit(Field::City),
            Hit::CalendarDay(offset) => {
                self.cal_offset = offset;
                self.agenda.scroll = 0;
            }
            Hit::AgendaToggle(id) => {
                self.agenda.toggle(id);
                self.agenda.save();
            }
            Hit::AgendaRemove(id) => {
                self.agenda.remove(id);
                self.agenda.save();
                let count = self.agenda.on(self.selected_day()).len();
                self.agenda.scroll_by(0, count);
            }
            Hit::AgendaInput => self.begin_edit(Field::Agenda),
            Hit::TimerPreset(minutes) => {
                self.timer.start(minutes, now);
                self.schedule_alarm();
            }
            Hit::TimerToggle => {
                self.timer.toggle_pause(now);
                self.schedule_alarm();
            }
            Hit::TimerAdd => {
                self.timer.add_minute(now);
                self.schedule_alarm();
            }
            Hit::TimerCancel => {
                self.timer.cancel();
                self.schedule_alarm();
            }
            Hit::ShelfItem(_)
            | Hit::ShelfStrip
            | Hit::TaskList
            | Hit::VolumeSlider
            | Hit::BrightnessSlider
            | Hit::MediaSeek
            | Hit::CalendarStrip
            | Hit::AgendaList
            | Hit::AgendaRow(_)
            | Hit::DropShare => {}
        }
    }

    fn on_double_click(&mut self, lparam: LPARAM) {
        if self.expanded
            && let Some(Hit::ShelfItem(id)) = self.hit_at(lparam)
            && let Some(item) = self.shelf.get(id)
        {
            dragdrop::open(self.hwnd, &item.path);
        }
    }

    fn on_right_click(&mut self, lparam: LPARAM) {
        if self.expanded
            && let Some(Hit::ShelfItem(id)) = self.hit_at(lparam)
            && let Some(item) = self.shelf.get(id)
        {
            dragdrop::reveal(self.hwnd, &item.path);
        }
    }

    /// Roda mouse: gulir shelf/daftar tugas kalau penuh, selain itu pindah halaman.
    fn on_wheel(&mut self, wparam: WPARAM, lparam: LPARAM, horizontal: bool) {
        if !self.expanded && horizontal && !self.locked {
            self.on_swipe(((wparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32);
            return;
        }
        if !self.expanded || self.locked {
            return;
        }
        self.wheel += ((wparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32;
        if self.wheel.abs() < 120 {
            return;
        }
        let up = self.wheel > 0;
        self.wheel = 0;
        // Roda ke bawah atau geser ke kanan = berikutnya.
        let step = if horizontal == up { 1 } else { -1 };
        // Posisi roda dalam koordinat layar, bukan koordinat jendela.
        let (pos, scale) = (self.window_pos(), self.renderer.scale());
        let (x, y) = point(lparam);
        let hit = self.renderer.hit_at((x - pos.x) as f32 / scale, (y - pos.y) as f32 / scale);
        match hit {
            Some(Hit::ShelfStrip | Hit::ShelfItem(_) | Hit::ShelfRemove(_))
                if !horizontal && self.shelf.items.len() > shelf::VISIBLE =>
            {
                self.shelf.scroll_by(step)
            }
            Some(Hit::TaskList | Hit::TaskToggle(_) | Hit::TaskRemove(_))
                if !horizontal && self.tasks.items.len() > tasks::VISIBLE =>
            {
                self.tasks.scroll_by(step)
            }
            // Roda di atas deretan tanggal: geser hari (ke bawah/kanan = hari berikutnya).
            Some(Hit::CalendarStrip | Hit::CalendarDay(_)) => {
                self.cal_offset += step;
                self.agenda.scroll = 0;
            }
            Some(Hit::AgendaList | Hit::AgendaRow(_) | Hit::AgendaToggle(_) | Hit::AgendaRemove(_))
                if !horizontal && self.agenda.on(self.selected_day()).len() > agenda::VISIBLE =>
            {
                let count = self.agenda.on(self.selected_day()).len();
                self.agenda.scroll_by(step, count);
            }
            // Roda di atas slider: naik/turun 2%.
            Some(slider @ (Hit::VolumeSlider | Hit::BrightnessSlider)) if !horizontal => {
                let current = match slider {
                    Hit::VolumeSlider => self.volume.map(|(level, _)| level),
                    _ => self.brightness.map(|b| b as f32 / 100.0),
                };
                if let Some(current) = current {
                    self.set_level(slider, current - step as f32 * 0.02);
                }
            }
            _ => self.go_page(self.settings.pages.step(self.page, step), step as f32),
        }
        self.hovered = None;
        self.refresh();
    }

    /// Geser dua jari ke kanan/kiri di pil selama musik diputar: lagu berikutnya/sebelumnya.
    fn on_swipe(&mut self, delta: i32) {
        if !self.playing() {
            return;
        }
        let now = self.now();
        let (mut total, last, skipped) = self.swipe;
        // Geseran yang terputus lama dianggap gerakan baru.
        if now - last > 0.4 {
            total = 0;
        }
        total += delta;
        self.swipe = (total, now, skipped);
        if total.abs() >= SWIPE_DISTANCE && now - skipped >= SWIPE_COOLDOWN {
            self.swipe = (0, now, now);
            self.media_command(if total > 0 { Button::Next } else { Button::Previous });
        }
    }

    /// Mulai animasi maskot bangun.
    fn wake_up(&mut self) {
        self.start_motion();
        self.wake = Some(self.now());
    }

    fn go_page(&mut self, page: Page, dir: f32) {
        if page == self.page || self.locked {
            return;
        }
        self.end_edit(true, true);
        self.start_motion();
        self.page_anim = Some((self.page, self.now(), dir));
        self.page = page;
        self.hovered = None;
        if page == Page::Shelf && self.shelf.prune_missing() {
            self.shelf_changed();
        }
    }

    fn media_command(&mut self, button: Button) {
        let Some(m) = &mut self.media else { return };
        let command = match button {
            Button::Previous if m.can_previous => media::Command::Previous,
            Button::Next if m.can_next => media::Command::Next,
            Button::PlayPause => {
                // Tampilkan perubahannya seketika; kabar resmi dari Windows menyusul.
                m.position = m.position_now();
                m.at = Instant::now();
                m.playing = !m.playing;
                media::Command::TogglePlayPause
            }
            _ => return,
        };
        self.services.send(Task::MediaCommand(command));
    }

    /// Lompat ke posisi bar lagu yang dilepas; tampilan langsung pindah, aplikasinya menyusul.
    fn finish_seek(&mut self) {
        let Some(fraction) = self.seek_preview.take() else { return };
        let Some(m) = self.media.as_mut().filter(|m| m.duration > 0.0) else { return };
        let seconds = fraction as f64 * m.duration;
        m.position = seconds;
        m.at = Instant::now();
        let ticks = ((m.start + seconds) * 1e7) as i64;
        self.services.send(Task::MediaCommand(media::Command::Seek(ticks)));
        // Baris lirik melompat ke posisi baru tanpa bergulir.
        self.lyric_scroll = None;
        self.refresh();
    }

    fn shelf_changed(&mut self) {
        self.shelf.save();
        let shelf = &self.shelf;
        self.renderer.retain_thumbnails(|id| shelf.get(id).is_some());
    }

    fn on_drag(&mut self, kind: usize, lparam: LPARAM) {
        match kind {
            DRAG_OVER => {
                // Area lepas baru menyala saat file tepat di atasnya.
                let (x, y) = point(lparam);
                let (pos, scale) = (self.window_pos(), self.renderer.scale());
                let hit = self.renderer.hit_at((x - pos.x) as f32 / scale, (y - pos.y) as f32 / scale);
                let over = match hit {
                    Some(Hit::DropShare) => Some(DropZone::Share),
                    Some(Hit::ShelfStrip | Hit::ShelfItem(_) | Hit::ShelfRemove(_)) => Some(DropZone::Shelf),
                    _ => None,
                };
                if over == self.drop_over {
                    return;
                }
                self.drop_over = over;
            }
            DRAG_ENTER => {
                // Shelf yang disembunyikan tidak menerima file.
                if !self.settings.pages.shown(Page::Shelf) {
                    return;
                }
                self.shelf.drop_hover = true;
                self.kill_timer(TIMER_CLOSE);
                if self.expanded {
                    self.go_page(Page::Shelf, self.page_dir(Page::Shelf));
                } else {
                    self.page = Page::Shelf;
                    self.page_anim = None;
                    self.set_expanded(true);
                }
            }
            DRAG_LEAVE => {
                self.shelf.drop_hover = false;
                self.drop_over = None;
                if self.expanded && !self.hovering {
                    self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
                }
            }
            DRAG_DROP => {
                self.shelf.drop_hover = false;
                let dropped = dragdrop::take_dropped();
                // Dilepas di zona Bagikan: buka jendela Bagikan Windows, tidak masuk shelf.
                if self.drop_over.take() == Some(DropZone::Share) {
                    self.share_files(&dropped);
                    self.refresh();
                    return;
                }
                let added = if self.settings.pages.shown(Page::Shelf) { self.shelf.add(dropped) } else { Vec::new() };
                if !added.is_empty() {
                    self.shelf.save();
                    self.start_motion();
                    self.drop_glow = Some(self.now());
                    self.pop_tiles(&added);
                    self.services.thumbnails(added);
                }
                if !self.hovering {
                    self.set_timer(TIMER_CLOSE, DROP_CLOSE_MS);
                }
            }
            _ => {}
        }
        self.refresh();
    }

    /// Buka jendela Bagikan Windows untuk file yang dilepas di zona Bagikan.
    fn share_files(&mut self, paths: &[std::path::PathBuf]) {
        if self.share.is_none() {
            self.share = share::Share::new(self.hwnd).inspect_err(|e| debug_log!("jendela Bagikan tidak tersedia: {e}")).ok();
        }
        let Some(share) = &self.share else { return };
        // Windows hanya menampilkan jendela Bagikan untuk jendela yang sedang di depan, sedangkan
        // notch biasanya tidak pernah aktif. Sesudah jendelanya muncul, notch kembali seperti biasa.
        self.set_activatable(true);
        bring_to_front(self.hwnd);
        if let Err(e) = share.show(paths) {
            debug_log!("jendela Bagikan gagal dibuka: {e}");
        }
        self.set_activatable(false);
    }

    /// Ubin baru muncul membesar.
    fn pop_tiles(&mut self, added: &[(u64, std::path::PathBuf)]) {
        self.start_motion();
        let now = self.now();
        for (id, _) in added {
            self.tile_fx.insert(*id, (now, false));
        }
    }

    /// Ubin mulai menghilang; dibuang dari shelf di `tick` setelah animasinya selesai.
    fn fade_tile(&mut self, id: u64) {
        if self.tile_fx.get(&id).is_some_and(|(_, removing)| *removing) {
            return;
        }
        self.start_motion();
        let now = self.now();
        self.tile_fx.insert(id, (now, true));
    }

    fn drag_path(&self) -> Option<std::path::PathBuf> {
        self.drag_item.and_then(|id| self.shelf.get(id)).map(|item| item.path.clone())
    }

    fn after_drag(&mut self) {
        // File yang dipindahkan ke tempat lain sudah tidak ada di lokasi lamanya.
        if let Some(id) = self.drag_item.take()
            && self.shelf.get(id).is_some_and(|item| !item.path.exists())
        {
            self.shelf.remove(id);
            self.shelf_changed();
        }
        // Kursor sudah pergi selama menyeret.
        self.hovering = false;
        self.tracking_leave = false;
        if self.expanded {
            self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
        }
        self.refresh();
    }

    /// Sedang mengetik tugas atau nama kota.
    fn editing(&self) -> bool {
        self.tasks.draft.is_some() || self.city_draft.is_some() || self.agenda.draft.is_some()
    }

    /// Mulai mengetik tugas baru, atau nama kota cuaca (`city`). Notch sementara boleh
    /// menerima fokus keyboard.
    fn begin_edit(&mut self, field: Field) {
        if self.editing() {
            return;
        }
        match field {
            Field::Task => self.tasks.draft = Some(String::new()),
            Field::City => {
                self.city_draft = Some(String::new());
                self.city_status = None;
            }
            Field::Agenda => self.agenda.draft = Some(String::new()),
        }
        self.pending_surrogate = None;
        self.edit_return = Some(unsafe { GetForegroundWindow() });
        self.set_activatable(true);
        unsafe {
            let _ = SetForegroundWindow(self.hwnd);
            let _ = SetFocus(Some(self.hwnd));
        }
    }

    /// Selesai mengetik. `commit` menyimpan teksnya; `restore_focus` mengembalikan fokus ke
    /// aplikasi sebelumnya (tidak perlu kalau pengguna sendiri sudah pindah aplikasi).
    fn end_edit(&mut self, commit: bool, restore_focus: bool) {
        let (task, city, event) = (self.tasks.draft.take(), self.city_draft.take(), self.agenda.draft.take());
        if task.is_none() && city.is_none() && event.is_none() {
            return;
        }
        if commit
            && let Some(event) = event
            && self.agenda.add(self.selected_day(), &event)
        {
            self.agenda.save();
        }
        if commit
            && let Some(task) = task
            && self.tasks.add(&task)
        {
            self.tasks.reveal(self.tasks.items.len() - 1);
            self.tasks.save();
        }
        if commit
            && let Some(name) = city.map(|c| c.trim().to_string())
            && !name.is_empty()
        {
            self.city_status = Some("Mencari kota…".to_string());
            weather::find_city(name, self.services.notifier());
        }
        self.set_activatable(false);
        if let Some(previous) = self.edit_return.take()
            && restore_focus
            && previous != self.hwnd
            && unsafe { IsWindow(Some(previous)) }.as_bool()
        {
            let _ = unsafe { SetForegroundWindow(previous) };
        }
        if self.expanded && !self.hovering {
            self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
        }
    }

    fn set_activatable(&self, on: bool) {
        unsafe {
            let style = GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE);
            let flag = WS_EX_NOACTIVATE.0 as isize;
            SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, if on { style & !flag } else { style | flag });
        }
    }

    fn on_char(&mut self, unit: u16) {
        if !self.editing() {
            return;
        }
        match unit {
            0x08 => {
                if let Some(draft) = self.tasks.draft.as_mut().or(self.city_draft.as_mut()).or(self.agenda.draft.as_mut()) {
                    draft.pop();
                }
            }
            // Enter di kotak kota: cari kotanya.
            0x0D if self.city_draft.is_some() => self.end_edit(true, true),
            // Enter di kotak agenda: simpan lalu siap mengetik agenda berikutnya; kotak kosong menutup.
            0x0D if self.agenda.draft.is_some() => {
                let text = self.agenda.draft.replace(String::new()).unwrap_or_default();
                if self.agenda.add(self.selected_day(), &text) {
                    self.agenda.save();
                } else {
                    self.end_edit(false, true);
                }
            }
            0x0D => {
                // Enter: simpan lalu siap mengetik tugas berikutnya; Enter di kotak kosong menutup.
                let text = self.tasks.draft.replace(String::new()).unwrap_or_default();
                if self.tasks.add(&text) {
                    self.tasks.reveal(self.tasks.items.len() - 1);
                    self.tasks.save();
                } else {
                    self.end_edit(false, true);
                }
            }
            0x1B => self.end_edit(false, true),
            0x16 => {
                if let Some(text) = clipboard_text() {
                    self.type_text(text.lines().next().unwrap_or(""));
                }
            }
            0xD800..=0xDBFF => self.pending_surrogate = Some(unit),
            _ if unit < 0x20 || unit == 0x7F => {}
            _ => {
                let units = match self.pending_surrogate.take() {
                    Some(high) => vec![high, unit],
                    None => vec![unit],
                };
                self.type_text(&String::from_utf16_lossy(&units));
            }
        }
        self.refresh();
    }

    fn type_text(&mut self, text: &str) {
        let (draft, max) = match (&mut self.tasks.draft, &mut self.city_draft, &mut self.agenda.draft) {
            (Some(draft), _, _) => (draft, tasks::MAX_LEN),
            (None, Some(draft), _) => (draft, CITY_MAX),
            (None, None, Some(draft)) => (draft, agenda::MAX_LEN),
            (None, None, None) => return,
        };
        let room = max.saturating_sub(draft.chars().count());
        draft.extend(text.chars().filter(|c| !c.is_control()).take(room));
    }

    fn hit_at(&self, lparam: LPARAM) -> Option<Hit> {
        let scale = self.renderer.scale();
        let (x, y) = point(lparam);
        self.renderer.hit_at(x as f32 / scale, y as f32 / scale)
    }

    fn set_hovered(&mut self, hit: Option<Hit>) {
        if self.hovered != hit {
            self.hovered = hit;
            if !self.animating() {
                self.redraw();
            }
        }
    }

    fn on_timer(&mut self, id: usize) {
        match id {
            TIMER_CLOCK => self.on_clock(),
            TIMER_OPEN => {
                self.kill_timer(TIMER_OPEN);
                if self.hovering {
                    self.set_expanded(true);
                }
            }
            TIMER_CLOSE => {
                self.kill_timer(TIMER_CLOSE);
                if !self.hovering && !self.editing() && !self.shelf.drop_hover && !self.menu_open && self.slider_drag.is_none() {
                    self.set_expanded(false);
                }
            }
            TIMER_BLINK => {
                self.start_blink();
                self.schedule_blink();
            }
            TIMER_DESKTOP => {
                self.kill_timer(TIMER_DESKTOP);
                self.refresh_mode(true);
            }
            // Di lock screen, posisi teratas diambil lagi tiap detik.
            TIMER_POLL => self.refresh_mode(self.locked),
            TIMER_HUD => {
                self.kill_timer(TIMER_HUD);
                self.hud_visible = false;
                self.refresh();
            }
            TIMER_AMBIENT => self.on_ambient(),
            TIMER_FOCUS => {
                if let Some(finished) = self.focus.finish_if_due(Instant::now()) {
                    self.alert(Hud::Focus { finished }, FOCUS_HUD_MS, Sound::FocusDone);
                    if finished == focus::Phase::Focus {
                        self.set_mood(Mood::Happy, FOCUS_HUD_MS + MOOD_MS);
                    }
                }
                self.schedule_focus();
                self.refresh();
            }
            TIMER_LOCK_KEYS => self.on_lock_keys(),
            TIMER_GAZE => self.on_gaze(),
            TIMER_ALARM => {
                self.kill_timer(TIMER_ALARM);
                if self.timer.finish_if_due(Instant::now()) {
                    let minutes = self.timer.total_minutes();
                    self.alert(Hud::Timer { minutes }, TIMER_HUD_MS, Sound::Alarm);
                }
                self.schedule_alarm();
                self.refresh();
            }
            TIMER_WEATHER => self.fetch_weather(),
            TIMER_QUIET => {
                self.kill_timer(TIMER_QUIET);
                self.quiet_until = None;
                self.refresh();
            }
            TIMER_MOOD => {
                self.kill_timer(TIMER_MOOD);
                self.mood = None;
                if !self.animating() {
                    self.redraw();
                }
            }
            _ => {}
        }
    }

    fn on_ambient(&mut self) {
        if self.animating() {
            return;
        }
        // Lirik baru ditemukan atau lagu berganti: bentuknya ikut berubah.
        if self.wanted_view() != self.view {
            self.refresh();
            return;
        }
        let settled = self.view.content().is_some_and(|i| self.alpha[i] == 1.0);
        // Pemutar di panel: gelombang di kepala cukup digambar ulang sendiri. Gambar penuh saat
        // detik lagu berganti; baris lirik berikutnya bergulir naik, lompatan (misal sesudah
        // bar diseret) langsung digambar.
        if self.view.is_panel() && self.page == Page::Home && self.playing() {
            let index = self.lyric_index();
            if self.lyric_column() && index != self.lyric_drawn {
                if let (Some(new), Some(old)) = (index, self.lyric_drawn)
                    && new == old + 1
                {
                    self.start_motion();
                    self.lyric_scroll = Some(self.now());
                } else {
                    self.redraw();
                }
                return;
            }
            let second = self.media.as_ref().map(|m| m.position_now() as u64);
            if settled && second == self.second_drawn && self.page_anim.is_none() {
                let accent = self.media.as_ref().and_then(|m| m.art.as_ref()).and_then(|art| art.accent);
                let result = self
                    .renderer
                    .draw_header_wave(self.width.value, self.now(), accent)
                    .and_then(|dirty| self.renderer.present(self.hwnd, self.window_pos(), Some(&dirty)));
                if let Err(e) = result {
                    debug_log!("gagal menggambar gelombang: {e}");
                }
            } else {
                self.redraw();
            }
            return;
        }
        // Pil yang diam cukup diperbarui gelombangnya saja; gambar penuh kalau baris lirik
        // berganti, dan sesekali supaya jam dan cincin pomodoro ikut terbarui.
        let pill = matches!(self.view, View::Pill | View::LyricPill);
        let lyric_moved = self.view == View::LyricPill && self.lyric_line().map(|(i, _)| i) != Some(self.lyric_shown);
        if pill && self.playing() && settled && !lyric_moved && self.now() - self.last_full < 5.0 {
            let accent = self.media.as_ref().and_then(|m| m.art.as_ref()).and_then(|art| art.accent);
            let result = self
                .renderer
                .draw_wave(self.width.value, self.height.value, self.now(), accent)
                .and_then(|dirty| self.renderer.present(self.hwnd, self.window_pos(), Some(&dirty)));
            if let Err(e) = result {
                debug_log!("gagal menggambar gelombang: {e}");
            }
        } else if lyric_moved {
            // Baris baru bisa lebih panjang atau pendek: pilnya ikut menyesuaikan.
            self.refresh();
        } else {
            self.redraw();
        }
    }

    fn on_clock(&mut self) {
        debug_log!(
            "menit terakhir: callback hook {}, frame animasi {}",
            desktop::take_callback_count(),
            FRAMES.swap(0, Ordering::Relaxed)
        );
        let today = clock::day_key(&clock::now());
        if today != self.day {
            self.day = today;
            self.focus.new_day();
        }
        self.check_water();
        self.check_agenda();
        if !self.animating() {
            self.redraw();
        }
        self.schedule_clock();
    }

    /// Agenda yang jatuh tepat pada menit ini muncul sebagai kabar dengan bunyi pengingat.
    fn check_agenda(&mut self) {
        let t = clock::now();
        let due = self.agenda.due(today(), t.wHour as u32, t.wMinute as u32).first().map(|e| (e.text.clone(), e.time_text()));
        if let Some((text, time)) = due {
            self.alert(Hud::Agenda { text, time }, AGENDA_HUD_MS, Sound::Reminder);
            self.refresh();
        }
    }

    /// Hari yang sedang dipilih di kalender.
    fn selected_day(&self) -> Date {
        today().add_days(self.cal_offset as i64)
    }

    /// Kolom kalender untuk Beranda lebar.
    fn calendar_view(&self) -> Option<CalendarView> {
        if !self.settings.calendar || self.locked {
            return None;
        }
        let (today, selected) = (today(), self.selected_day());
        let days = (-2..=2)
            .map(|k| {
                let day = selected.add_days(k as i64);
                DayView {
                    offset: self.cal_offset + k,
                    name: day.weekday(),
                    number: day.day,
                    today: day == today,
                    selected: k == 0,
                    has_events: self.agenda.has(day),
                }
            })
            .collect();
        let events = self
            .agenda
            .on(selected)
            .into_iter()
            .map(|e| AgendaItem { id: e.id, time: e.time_text(), text: e.text.clone(), done: e.done })
            .collect();
        Some(CalendarView {
            month: selected.month_name().to_string(),
            year: selected.year.to_string(),
            days,
            events,
            scroll: self.agenda.scroll,
        })
    }

    /// Ingatkan minum kalau sudah waktunya, hanya di jam aktif dan selama target belum tercapai.
    fn check_water(&mut self) {
        let now = Instant::now();
        // Halaman minum yang disembunyikan berarti pengingatnya tidak dipakai.
        if now < self.water_due || !self.settings.pages.shown(Page::Water) {
            return;
        }
        let ml = self.water.on(&self.day);
        let awake = WATER_HOURS.contains(&clock::now().wHour);
        if awake && ml < GOAL_ML && self.shown && !self.expanded {
            self.alert(Hud::Water { ml, goal: GOAL_ML }, WATER_HUD_MS, Sound::Reminder);
            self.refresh();
        }
        self.water_due = now + WATER_AGAIN;
    }

    fn schedule_focus(&self) {
        if self.focus.running() {
            let left = self.focus.remaining(Instant::now()).as_millis().min(u32::MAX as u128) as u32;
            self.set_timer(TIMER_FOCUS, left.saturating_add(20));
        } else {
            self.kill_timer(TIMER_FOCUS);
        }
    }

    fn on_service(&mut self) {
        let mut battery_update = false;
        while let Some(event) = self.services.next_event() {
            match event {
                Event::Media(snapshot) => {
                    self.renderer.set_art(snapshot.as_ref().and_then(|s| s.art.as_deref()));
                    self.media = snapshot;
                    self.request_lyrics();
                }
                Event::Lyrics(found) => {
                    if self.lyric_pending.as_ref() == Some(&found.key) {
                        self.lyric_pending = None;
                    }
                    if self.settings.lyrics && self.media_key().as_ref() == Some(&found.key) {
                        self.lyrics = Some(found);
                    }
                }
                Event::Privacy(usage) => self.on_privacy(usage),
                Event::System(stats) => self.system = Some(stats),
                Event::Volume { level, muted, initial } => {
                    self.volume = Some((level, muted));
                    if !initial {
                        self.show_hud(Hud::Volume { level, muted }, VOLUME_HUD_MS);
                    }
                }
                Event::Mic(muted) => self.mic = muted,
                Event::City(found) => match found {
                    Some(city) => {
                        city.save();
                        self.city = Some(city);
                        self.city_status = None;
                        self.fetch_weather();
                    }
                    None => self.city_status = Some("Kota tidak ditemukan".to_string()),
                },
                Event::Weather(found) => {
                    debug_log!("cuaca: {found:?}");
                    if let Some(found) = found {
                        self.weather = Some((found, Instant::now()));
                    }
                }
                Event::Bluetooth(notice) => {
                    let sound = if notice.connected { Sound::Connect } else { Sound::Disconnect };
                    let hud = Hud::Bluetooth {
                        id: notice.id,
                        name: notice.name,
                        connected: notice.connected,
                        battery: notice.battery,
                        kind: notice.kind,
                    };
                    self.alert(hud, BLUETOOTH_HUD_MS, sound);
                }
                Event::MessagesAccess(allowed) => {
                    debug_log!("izin membaca notifikasi: {allowed}");
                    self.messages_denied = !allowed;
                    self.services.send(Task::WatchMessages(allowed && self.settings.messages != Messages::Off));
                }
                Event::Message(message) => self.on_message(message),
                Event::Network(state) => self.on_network(state),
                Event::Files { kind, paths } => self.on_files(kind, paths),
                Event::Downloading(state) => self.downloading = state.filter(|_| self.settings.downloads),
                Event::BluetoothBattery { id, battery } => {
                    // Baterai yang telat terbaca hanya dipakai kalau kabarnya masih tampil.
                    if self.hud_visible
                        && let Some(Hud::Bluetooth { id: shown, battery: slot, .. }) = &mut self.hud
                        && *shown == id
                    {
                        *slot = Some(battery);
                        battery_update = true;
                    }
                }
                Event::Thumbnail(thumb) => {
                    if self.shelf.get(thumb.id).is_some() {
                        self.renderer.set_thumbnail(&thumb);
                    }
                }
            }
        }
        if battery_update {
            self.set_timer(TIMER_HUD, 2_000);
        }
        self.refresh();
    }

    /// Kabar aplikasi yang baru mulai memakai kamera atau mikrofon. Kabar pertama hanya
    /// keadaan awal saat notch dijalankan, jadi tidak diumumkan.
    fn on_privacy(&mut self, usage: Usage) {
        let started = |now: &[String], before: &[String]| now.iter().find(|app| !before.contains(app)).cloned();
        let hud = if !self.privacy_known {
            None
        } else if let Some(app) = started(&usage.camera, &self.privacy.camera) {
            Some(Hud::Privacy { camera: true, app })
        } else {
            started(&usage.microphone, &self.privacy.microphone).map(|app| Hud::Privacy { camera: false, app })
        };
        self.privacy = usage;
        self.privacy_known = true;
        if let Some(hud) = hud {
            self.alert(hud, PRIVACY_HUD_MS, Sound::Privacy);
            self.set_mood(Mood::Surprised, PRIVACY_HUD_MS + MOOD_MS);
        }
    }

    /// Pesan masuk dari aplikasi chat, disaring sesuai pengaturan. Di lock screen atau kalau
    /// diminta, isi pesannya tidak ditampilkan.
    fn on_message(&mut self, message: messages::Message) {
        let wanted = match self.settings.messages {
            Messages::Off => false,
            // Dari browser (misal WhatsApp Web) hanya kalau "Notifikasi browser" menyala.
            _ if message.from_browser => self.settings.browser_messages,
            Messages::WhatsApp => message.is_whatsapp(),
            Messages::AllChat => true,
        };
        debug_log!("pesan dari {} (browser: {}), ditampilkan: {wanted}", message.app, message.from_browser);
        if !wanted {
            return;
        }
        if let Some(logo) = &message.logo {
            self.renderer.set_logo(&message.aumid, messages::LOGO_PX, logo);
        }
        let messages::Message { app, aumid, sender, text, from_browser, .. } = message;
        // Di lock screen, atau saat rapat (kamera/mikrofon dipakai, mungkin sambil berbagi
        // layar), orang lain bisa ikut melihat: pengirim dan isinya tidak ditampilkan.
        let private = self.locked || self.in_meeting();
        let (app, sender) = if private { (String::new(), app) } else { (app, sender) };
        let text = if self.settings.hide_content || private || text.is_empty() { "Pesan baru".to_string() } else { text };
        let hud = Hud::Message { app, aumid, sender, text, browser: from_browser };
        // Tanpa bunyi dari notch: WhatsApp/browser sudah berbunyi sendiri.
        self.show_hud(hud, MESSAGE_HUD_MS);
    }

    /// Kabar singkat yang disertai bunyi (kalau suara notifikasi aktif). Bunyi tetap diputar
    /// walau panel sedang terbuka, karena kabarnya penting.
    fn alert(&mut self, hud: Hud, ms: u32, sound: Sound) {
        // Saat rapat notch tidak berbunyi (bisa terdengar peserta lain), kecuali kabar bahwa
        // kamera/mikrofon baru saja dinyalakan. Saat senyap hanya kabar penting yang berbunyi.
        let meeting = self.in_meeting() && sound != Sound::Privacy;
        let quiet = self.quiet() && !urgent(&hud);
        if self.settings.sound && !meeting && !quiet {
            sound::play(sound);
        }
        self.show_hud(hud, ms);
    }

    /// Mode rapat aktif: kamera atau mikrofon sedang dipakai.
    fn in_meeting(&self) -> bool {
        self.settings.meeting && self.privacy.active()
    }

    fn quiet(&self) -> bool {
        self.quiet_until.is_some_and(|until| Instant::now() < until)
    }

    /// Jadwalkan kabar timer habis.
    fn schedule_alarm(&self) {
        if self.timer.running() {
            let left = self.timer.remaining(Instant::now()).as_millis().min(u32::MAX as u128) as u32;
            self.set_timer(TIMER_ALARM, left.saturating_add(20));
        } else {
            self.kill_timer(TIMER_ALARM);
        }
    }

    /// Ambil cuaca kota yang diatur, lalu jadwalkan pembaruan berikutnya.
    fn fetch_weather(&mut self) {
        match (&self.city, self.settings.weather) {
            (Some(city), true) => {
                weather::fetch(city, self.services.notifier());
                self.set_timer(TIMER_WEATHER, WEATHER_EVERY_MS);
            }
            _ => self.kill_timer(TIMER_WEATHER),
        }
    }

    /// Wi-Fi tersambung, terputus, atau kehilangan internet. Kabar pertama hanyalah keadaan awal.
    fn on_network(&mut self, state: network::State) {
        debug_log!("jaringan: {state:?}");
        let Some(before) = self.network.replace(state.clone()) else { return };
        if !self.settings.wifi {
            return;
        }
        let change = match (&before.wifi, &state.wifi) {
            (_, Some(name)) if before.wifi.as_ref() != Some(name) => {
                Some((Hud::Wifi { name: name.clone(), connected: true, internet: true }, Some(Sound::Connect)))
            }
            (Some(name), None) => Some((Hud::Wifi { name: name.clone(), connected: false, internet: false }, Some(Sound::Disconnect))),
            (Some(_), Some(name)) if before.internet && !state.internet => {
                Some((Hud::Wifi { name: name.clone(), connected: true, internet: false }, None))
            }
            _ => None,
        };
        match change {
            Some((hud, Some(sound))) => self.alert(hud, WIFI_HUD_MS, sound),
            Some((hud, None)) => self.show_hud(hud, WIFI_HUD_MS),
            None => {}
        }
    }

    /// File baru yang selesai ditulis: screenshot masuk shelf, unduhan dikabarkan.
    fn on_files(&mut self, kind: folders::Kind, paths: Vec<std::path::PathBuf>) {
        let name = |path: &std::path::Path| path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match kind {
            folders::Kind::Screenshot => {
                if !self.settings.screenshots || !self.settings.pages.shown(Page::Shelf) {
                    return;
                }
                let added = self.shelf.add(paths);
                let Some((id, path)) = added.last().cloned() else { return };
                self.shelf.save();
                self.pop_tiles(&added);
                let more = added.len() - 1;
                self.services.thumbnails(added);
                self.show_hud(Hud::Screenshot { id, name: name(&path), more }, SCREENSHOT_HUD_MS);
            }
            folders::Kind::Downloads => {
                let Some(path) = paths.first().cloned() else { return };
                if self.settings.downloads {
                    let hud = Hud::Download { name: name(&path), path, more: paths.len() - 1 };
                    self.alert(hud, DOWNLOAD_HUD_MS, Sound::Download);
                }
            }
        }
    }

    /// Nyalakan/matikan pemantau folder sesuai pengaturan.
    fn update_watchers(&mut self) {
        let shots = self.settings.screenshots && self.settings.pages.shown(Page::Shelf);
        if shots != self.screenshots.is_some() {
            self.screenshots = shots.then(|| Watch::start(folders::Kind::Screenshot, self.services.notifier())).flatten();
        }
        if self.settings.downloads != self.downloads.is_some() {
            self.downloads =
                self.settings.downloads.then(|| Watch::start(folders::Kind::Downloads, self.services.notifier())).flatten();
        }
    }

    /// Daftarkan atau lepas pintasan Win+Alt+N.
    fn set_hotkey(&mut self, on: bool) {
        if on == self.hotkey {
            return;
        }
        if on {
            let modifiers = MOD_WIN | MOD_ALT | MOD_NOREPEAT;
            match unsafe { RegisterHotKey(Some(self.hwnd), HOTKEY_ID, modifiers, u32::from(b'N')) } {
                Ok(()) => self.hotkey = true,
                Err(e) => debug_log!("pintasan Win+Alt+N tidak bisa dipakai: {e}"),
            }
        } else {
            let _ = unsafe { UnregisterHotKey(Some(self.hwnd), HOTKEY_ID) };
            self.hotkey = false;
        }
    }

    /// Win+Alt+N: buka panel, atau tutup kalau sedang terbuka.
    fn on_hotkey(&mut self) {
        if !self.shown {
            return;
        }
        if self.expanded {
            self.set_expanded(false);
            return;
        }
        self.kill_timer(TIMER_OPEN);
        self.set_expanded(true);
        if !self.hovering {
            self.set_timer(TIMER_CLOSE, HOTKEY_CLOSE_MS);
        }
    }

    /// Arah geser menuju `page`: maju kalau letaknya di kanan halaman sekarang.
    fn page_dir(&self, page: Page) -> f32 {
        let pages = &self.settings.pages;
        if pages.index(page) > pages.index(self.page) { 1.0 } else { -1.0 }
    }

    /// Klik kanan di panel (selain di ubin shelf): siapkan menu pengaturan halaman.
    fn page_menu(&mut self, lparam: LPARAM) -> Option<PageMenu> {
        let on_tile = matches!(self.hit_at(lparam), Some(Hit::ShelfItem(_) | Hit::ShelfRemove(_)));
        if !self.expanded || self.locked || on_tile {
            return None;
        }
        self.end_edit(true, false);
        self.menu_open = true;
        self.kill_timer(TIMER_CLOSE);
        let pages = self.settings.pages;
        Some(PageMenu {
            page: self.page,
            can_left: pages.can_move(self.page, -1),
            can_right: pages.can_move(self.page, 1),
            hidden: pages.order().into_iter().filter(|p| !pages.shown(*p)).collect(),
        })
    }

    fn after_page_menu(&mut self, page: Page, command: Option<PageCommand>) {
        self.menu_open = false;
        match command {
            Some(PageCommand::Move(delta)) => self.settings.pages.move_by(page, delta),
            Some(PageCommand::Hide) => {
                let next = self.settings.pages.step(page, 1);
                self.settings.pages.set_shown(page, false);
                self.page = next;
                self.page_anim = None;
            }
            Some(PageCommand::Show(shown)) => {
                self.settings.pages.set_shown(shown, true);
                self.go_page(shown, self.page_dir(shown));
            }
            None => {}
        }
        if command.is_some() {
            self.settings.save();
            self.update_watchers();
        }
        self.hovered = None;
        // Selama menu terbuka kursor dianggap keluar dari notch.
        if !self.cursor_over_window() {
            self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
        }
        self.refresh();
    }

    fn cursor_over_window(&self) -> bool {
        let mut cursor = POINT::default();
        if unsafe { GetCursorPos(&mut cursor) }.is_err() {
            return false;
        }
        let (pos, size) = (self.window_pos(), self.renderer.size());
        (pos.x..pos.x + size.cx).contains(&cursor.x) && (pos.y..pos.y + size.cy).contains(&cursor.y)
    }

    /// Kunci / buka kunci laptop. Notch tetap tampil di lock screen, tapi hanya jam dan musik.
    fn on_session_change(&mut self, change: u32) {
        match change {
            WTS_SESSION_LOCK => {
                self.end_edit(true, false);
                self.locked = true;
                self.page = Page::Home;
                self.page_anim = None;
                self.expanded = false;
                self.hovered = None;
                self.apply_visibility();
                self.raise();
                self.refresh();
            }
            WTS_SESSION_UNLOCK => {
                self.locked = false;
                self.refresh_mode(true);
                self.apply_visibility();
                self.wake_up();
                self.refresh();
            }
            _ => {}
        }
    }

    /// Dengarkan input keyboard mentah, hanya untuk mengetahui Caps Lock / Num Lock. Tombol lain
    /// langsung diabaikan dan tidak disimpan.
    fn set_raw_keyboard(&mut self, on: bool) {
        if on == self.raw_keyboard {
            return;
        }
        let device = RAWINPUTDEVICE {
            usUsagePage: 0x01, // perangkat desktop umum
            usUsage: 0x06,     // keyboard
            dwFlags: if on { RIDEV_INPUTSINK } else { RIDEV_REMOVE },
            hwndTarget: if on { self.hwnd } else { HWND::default() },
        };
        match unsafe { RegisterRawInputDevices(&[device], size_of::<RAWINPUTDEVICE>() as u32) } {
            Ok(()) => {
                self.raw_keyboard = on;
                self.lock_keys = lock_key_states();
            }
            Err(e) => debug_log!("input keyboard tidak bisa {}: {e}", if on { "didengarkan" } else { "dilepas" }),
        }
    }

    fn on_raw_input(&mut self, lparam: LPARAM) {
        let mut input = RAWINPUT::default();
        let mut size = size_of::<RAWINPUT>() as u32;
        let header = size_of::<RAWINPUTHEADER>() as u32;
        let read = unsafe {
            GetRawInputData(HRAWINPUT(lparam.0 as _), RID_INPUT, Some((&mut input as *mut RAWINPUT).cast()), &mut size, header)
        };
        if read == u32::MAX || input.header.dwType != RIM_TYPEKEYBOARD.0 {
            return;
        }
        let key = unsafe { input.data.keyboard };
        let released = key.Flags as u32 & RI_KEY_BREAK != 0;
        if released && (key.VKey == VK_CAPITAL.0 || key.VKey == VK_NUMLOCK.0) {
            // Status lampunya baru berganti sesaat setelah tombol dilepas.
            self.set_timer(TIMER_LOCK_KEYS, 40);
        }
    }

    fn on_lock_keys(&mut self) {
        self.kill_timer(TIMER_LOCK_KEYS);
        let (caps, num) = lock_key_states();
        let (old_caps, old_num) = std::mem::replace(&mut self.lock_keys, (caps, num));
        if caps != old_caps {
            self.show_hud(Hud::LockKey { caps: true, on: caps }, LOCK_KEY_HUD_MS);
        } else if num != old_num {
            self.show_hud(Hud::LockKey { caps: false, on: num }, LOCK_KEY_HUD_MS);
        }
        self.refresh();
    }

    /// Ambil lagi posisi teratas (misal di atas lock screen).
    fn raise(&self) {
        let _ = unsafe {
            SetWindowPos(self.hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)
        };
    }

    fn media_key(&self) -> Option<String> {
        self.media.as_ref().map(|m| lyrics::key(&m.title, &m.artist))
    }

    /// Minta lirik lagu yang sedang diputar, kalau fitur lirik aktif dan belum pernah diambil.
    fn request_lyrics(&mut self) {
        if !self.settings.lyrics {
            return;
        }
        let Some(m) = &self.media else { return };
        let key = lyrics::key(&m.title, &m.artist);
        let known = self.lyrics.as_ref().is_some_and(|l| l.key == key) || self.lyric_pending.as_ref() == Some(&key);
        if m.title.is_empty() || known {
            return;
        }
        self.lyric_pending = Some(key.clone());
        self.services.lyrics(lyrics::Request {
            key,
            title: m.title.clone(),
            artist: m.artist.clone(),
            album: m.album.clone(),
            duration: m.duration,
        });
    }

    /// Baris lirik saat ini (indeks dan teksnya), kalau lirik aktif dan cocok dengan lagu.
    /// Sebelum baris pertama, teksnya not balok.
    fn lyric_line(&self) -> Option<(Option<usize>, String)> {
        if !self.settings.lyrics {
            return None;
        }
        let m = self.media.as_ref()?;
        let key = lyrics::key(&m.title, &m.artist);
        let lyrics = self.lyrics.as_ref().filter(|l| l.key == key && !l.lines.is_empty())?;
        let index = lyrics.current(m.position_now());
        Some((index, index.map_or("♪", |i| lyrics.text(i)).to_string()))
    }

    /// Kolom lirik di samping pemutar diminta tampil (tombol lirik menyala).
    fn lyric_column(&self) -> bool {
        self.settings.lyrics && self.settings.lyric_panel && self.media.is_some()
    }

    fn lyric_goal(&self) -> f32 {
        if self.lyric_column() { 1.0 } else { 0.0 }
    }

    /// Baris lirik lagu saat ini untuk kolom lirik; -1 sebelum baris pertama.
    fn lyric_index(&self) -> Option<isize> {
        let m = self.media.as_ref()?;
        let key = lyrics::key(&m.title, &m.artist);
        let lyrics = self.lyrics.as_ref().filter(|l| l.key == key && !l.lines.is_empty())?;
        Some(lyrics.current(m.position_now()).map_or(-1, |i| i as isize))
    }

    /// Isi kolom lirik: lima baris di sekitar baris sekarang, atau keterangan kalau liriknya
    /// sedang dicari atau tidak ada. Tetap ada selama kolomnya bergeser keluar.
    fn lyrics_view(&self) -> Option<LyricsView> {
        if !self.settings.lyrics || (!self.lyric_column() && self.lyric_mix <= 0.0) {
            return None;
        }
        let m = self.media.as_ref()?;
        let key = lyrics::key(&m.title, &m.artist);
        let status = |text| Some(LyricsView { lines: Default::default(), scroll: 1.0, status: Some(text) });
        let Some(lyrics) = self.lyrics.as_ref().filter(|l| l.key == key && !l.lines.is_empty()) else {
            return if self.lyric_pending.as_ref() == Some(&key) { status("Mencari lirik…") } else { status("Lirik tidak tersedia") };
        };
        let current = lyrics.current(m.position_now()).map_or(-1, |i| i as isize);
        let lines = std::array::from_fn(|k| match current + k as isize - 2 {
            // Sebelum baris pertama: not balok di tempat baris yang sedang dinyanyikan.
            -1 => "♪".to_string(),
            i if i >= 0 && (i as usize) < lyrics.lines.len() => lyrics.text(i as usize).to_string(),
            _ => String::new(),
        });
        let scroll = self.lyric_scroll.map_or(1.0, |start| ((self.now() - start) / LYRIC_SCROLL_SECS).min(1.0) as f32);
        Some(LyricsView { lines, scroll, status: None })
    }

    fn playing(&self) -> bool {
        self.media.as_ref().is_some_and(|m| m.playing)
    }

    /// Ukur sistem hanya selama halaman Sistem terlihat.
    fn update_system_watch(&mut self) {
        let want = self.shown && self.expanded && self.page == Page::System;
        if want != self.system_watch {
            self.system_watch = want;
            self.services.send(Task::WatchSystem(want));
        }
    }

    fn on_power_broadcast(&mut self, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
        match wparam.0 as u32 {
            PBT_APMRESUMEAUTOMATIC => {
                self.on_clock();
                self.wake_up();
                None
            }
            PBT_POWERSETTINGCHANGE => {
                if let Some(change) = power::parse(lparam) {
                    self.on_power(change);
                }
                Some(LRESULT(1))
            }
            _ => None,
        }
    }

    fn on_power(&mut self, change: power::Change) {
        match change {
            power::Change::Plugged(plugged) => {
                // Kabar pertama hanyalah status awal saat notch dijalankan.
                if self.plugged.replace(plugged).is_some_and(|old| old != plugged) {
                    let percent = self.battery.or_else(power::battery_percent);
                    let sound = if plugged { Sound::Connect } else { Sound::Disconnect };
                    self.alert(Hud::Power { plugged, percent, low: false }, POWER_HUD_MS, sound);
                }
            }
            power::Change::Percent(percent) => {
                let before = self.battery.replace(percent);
                let crossed = before.is_some_and(|b| LOW_BATTERY.iter().any(|&limit| b > limit && percent <= limit));
                if crossed && self.plugged == Some(false) {
                    let hud = Hud::Power { plugged: false, percent: Some(percent), low: true };
                    self.alert(hud, POWER_HUD_MS + 1_000, Sound::LowBattery);
                    self.set_mood(Mood::Surprised, POWER_HUD_MS + 1_000 + MOOD_MS);
                }
            }
            power::Change::Brightness(level) => {
                // Kabar pertama hanyalah kecerahan saat notch dijalankan.
                let changed = self.brightness.replace(level).is_some_and(|old| old != level);
                if changed && self.settings.brightness {
                    self.show_hud(Hud::Brightness { level: level as f32 / 100.0 }, BRIGHTNESS_HUD_MS);
                }
            }
        }
        self.refresh();
    }

    /// Tampilkan kabar singkat. Diabaikan saat panel sedang dibuka atau notch tersembunyi.
    fn show_hud(&mut self, hud: Hud, ms: u32) {
        // Saat senyap hanya baterai lemah dan timer habis yang tetap muncul.
        if !self.shown || self.expanded || (self.quiet() && !urgent(&hud)) {
            return;
        }
        // Volume/kecerahan yang berubah lagi selagi kabarnya tampil: bilahnya bergeser halus
        // dan angkanya bergulir, bukan langsung melompat.
        if let Some(level) = hud_level(&hud) {
            let same = self.hud_visible && self.hud.as_ref().is_some_and(|old| std::mem::discriminant(old) == std::mem::discriminant(&hud));
            if same {
                self.start_motion();
                let before = self.hud.as_ref().and_then(hud_level).unwrap_or(level);
                if let (Some(old), Some(new)) = (self.hud.as_ref().and_then(hud_value), hud_value(&hud))
                    && old != new
                {
                    self.hud_roll = Some((old, self.now(), level >= before));
                }
                self.hud_fill_target = level;
            } else {
                (self.hud_fill, self.hud_fill_target, self.hud_roll) = (level, level, None);
            }
        }
        self.hud = Some(hud);
        self.hud_visible = true;
        self.set_timer(TIMER_HUD, ms);
    }

    /// Kabar terakhir memakai bentuk ringkas (juga selama sedang memudar keluar).
    fn compact_hud(&self) -> bool {
        self.settings.compact_hud
            && matches!(self.hud, Some(Hud::Volume { .. } | Hud::Brightness { .. } | Hud::LockKey { .. } | Hud::Power { .. }))
    }

    fn set_expanded(&mut self, expanded: bool) {
        if self.expanded == expanded || !self.shown {
            return;
        }
        if !expanded {
            self.end_edit(true, true);
        }
        self.expanded = expanded;
        if expanded {
            self.hud_visible = false;
            self.kill_timer(TIMER_HUD);
            if self.page == Page::Shelf && self.shelf.prune_missing() {
                self.shelf_changed();
            }
            // Sapaan kecil: maskot berkedip tak lama setelah panel terbuka.
            self.set_timer(TIMER_BLINK, 650);
        } else {
            self.hovered = None;
            self.page_anim = None;
            // Kalender kembali ke hari ini saat panel dibuka lagi.
            self.cal_offset = 0;
            self.agenda.scroll = 0;
        }
        self.refresh();
    }

    fn wanted_view(&self) -> View {
        // Titik privasi lebih penting daripada ruang tab: selama kamera/mikrofon dipakai,
        // notch tidak menciut jadi garis tipis.
        // Garis tipis yang disentuh kursor kembali jadi pil dulu.
        let slim = self.mode == Mode::Maximized && !self.privacy.active() && !self.locked && !self.peek;
        if self.expanded {
            // Beranda lebar dengan kalender (tidak di lock screen: agenda itu pribadi), atau
            // dengan kolom lirik di samping pemutar.
            let calendar = self.settings.calendar && !self.locked;
            if self.page == Page::Home && (calendar || self.lyric_column()) { View::PanelWide } else { View::Panel }
        } else if self.hud_visible {
            if self.compact_hud() { View::Strip } else { View::Hud }
        } else if slim {
            View::Slim
        } else if self.playing() && self.lyric_line().is_some() {
            View::LyricPill
        } else {
            View::Pill
        }
    }

    /// Arahkan bentuk ke keadaan terbaru lalu gambar ulang bila perlu.
    fn refresh(&mut self) {
        let view = self.wanted_view();
        let target = self.view_size(view);
        if view != self.view || target != self.target {
            self.start_motion();
            let opening = target.0 * target.1 > self.width.value * self.height.value;
            let bounce = if opening { OPEN_BOUNCE } else { CLOSE_BOUNCE };
            self.width.set_damping_ratio(bounce);
            self.height.set_damping_ratio(bounce);
            if view != self.view {
                debug_log!("bentuk: {:?} -> {view:?}", self.view);
                self.from = (self.width.value, self.height.value);
                self.panel_resize = self.view.is_panel() && view.is_panel();
            } else if self.transition() >= 1.0 {
                // Hanya ukurannya yang berubah (baris lirik baru): isinya tidak perlu memudar.
                self.from = target;
            }
            self.view = view;
            self.target = target;
            self.width.target = target.0;
            self.height.target = target.1;
        }
        // Kolom lirik baru bergeser selama panel terbuka; saat tertutup langsung di tempatnya.
        if !self.expanded {
            self.lyric_mix = self.lyric_goal();
        }
        self.update_ambient();
        self.update_system_watch();
        if !self.animating() {
            self.redraw();
        }
    }

    /// Langsung ke bentuk tujuan tanpa animasi (misal saat notch baru muncul lagi).
    fn snap_view(&mut self) {
        self.view = self.wanted_view();
        let (w, h) = self.view_size(self.view);
        self.target = (w, h);
        self.width.snap(w);
        self.height.snap(h);
        self.from = (w, h);
        self.panel_resize = false;
        self.lyric_mix = self.lyric_goal();
        self.alpha = self.alpha_goals();
    }

    /// Ukuran bentuk; pil lirik melebar, atau jadi dua baris, mengikuti baris liriknya.
    fn view_size(&self, view: View) -> (f32, f32) {
        let (w, h) = match (view, self.lyric_line()) {
            (View::LyricPill, Some((_, line))) => self.renderer.lyric_size(&line),
            _ => view.size(),
        };
        let w = if view == View::Pill && self.download_text().is_some() { w + DOWNLOAD_GROW } else { w };
        match view {
            View::Pill | View::LyricPill if self.peek => (w + PEEK_GROW.0, h + PEEK_GROW.1),
            _ => (w, h),
        }
    }

    /// Ukuran unduhan yang sedang berjalan untuk pil, selama tidak ada musik atau timer.
    fn download_text(&self) -> Option<String> {
        let (count, bytes) = self.downloading.filter(|_| !self.playing() && !self.timer.running())?;
        let size = crate::sysinfo::size(bytes);
        Some(if count > 1 { format!("{count} · {size}") } else { size })
    }

    /// Bayangan penuh saat panel terbuka, tipis saat pil disentuh kursor.
    fn shadow_goal(&self) -> f32 {
        if !self.shown {
            0.0
        } else if self.expanded {
            1.0
        } else if self.peek {
            0.6
        } else {
            0.0
        }
    }

    /// Timer untuk isi yang bergerak sendiri, hanya selama memang ada yang berjalan.
    fn update_ambient(&mut self) {
        let playing = self.playing();
        let focusing = self.focus.running();
        let timing = self.timer.running();
        let last_minute = self.timer.remaining(Instant::now()) < Duration::from_secs(60);
        let interval = match self.view {
            _ if !self.shown => None,
            v if v.is_panel() && self.page == Page::Shelf && self.shelf.drop_hover => Some(DASH_MS),
            View::Pill | View::LyricPill if playing => Some(WAVE_MS),
            // Di menit terakhir pil menghitung detik.
            View::Pill if timing => Some(if last_minute { PROGRESS_MS } else { PILL_FOCUS_MS }),
            View::Pill if focusing => Some(PILL_FOCUS_MS),
            v if v.is_panel() && self.page == Page::Timer && timing => Some(FOCUS_TICK_MS),
            // Gelombang di kepala pemutar; detik lagu dan baris lirik diperiksa sekalian.
            v if v.is_panel() && self.page == Page::Home && playing => Some(WAVE_MS),
            v if v.is_panel() && self.page == Page::Focus && focusing => Some(FOCUS_TICK_MS),
            _ => None,
        };
        if interval != self.ambient {
            self.ambient = interval;
            match interval {
                Some(ms) => self.set_timer(TIMER_AMBIENT, ms),
                None => self.kill_timer(TIMER_AMBIENT),
            }
        }
        self.update_gaze_poll();
    }

    /// Mata maskot sedang terlihat: di pil saat tidak ada musik/pomodoro, atau di halaman jam.
    fn eyes_visible(&self) -> bool {
        self.shown
            && match self.view {
                View::Pill => !self.playing() && !self.focus.running() && !self.timer.running(),
                View::Panel | View::PanelWide => self.page == Page::Home && self.media.is_none() && self.page_anim.is_none(),
                _ => false,
            }
    }

    /// Lihat posisi kursor hanya selama matanya terlihat.
    fn update_gaze_poll(&mut self) {
        let visible = self.eyes_visible();
        let interval = visible.then(|| if self.cursor_moved.elapsed() >= GAZE_REST { GAZE_IDLE_MS } else { GAZE_MS });
        if interval != self.gaze_poll {
            self.gaze_poll = interval;
            match interval {
                Some(ms) => self.set_timer(TIMER_GAZE, ms),
                None => self.kill_timer(TIMER_GAZE),
            }
        }
        // Mata yang tersembunyi kembali ke tengah, supaya tidak muncul sambil melirik.
        if !visible {
            self.gaze = (0.0, 0.0);
            self.gaze_target = (0.0, 0.0);
        }
    }

    /// Mata melirik ke arah kursor; kalau kursor lama diam, kembali menatap ke depan.
    fn on_gaze(&mut self) {
        let mut cursor = POINT::default();
        if !self.eyes_visible() || unsafe { GetCursorPos(&mut cursor) }.is_err() {
            self.update_gaze_poll();
            return;
        }
        let now = Instant::now();
        if cursor != self.cursor {
            self.cursor = cursor;
            self.cursor_moved = now;
        }
        let target = if now.duration_since(self.cursor_moved) >= GAZE_REST {
            (0.0, 0.0)
        } else {
            let (pos, size, scale) = (self.window_pos(), self.renderer.size(), self.renderer.scale());
            // Letak mata dalam DIP dari tengah atas kanvas: kiri pil, atau kanan halaman jam.
            // Mata di halaman jam: di kanan panel biasa, atau di kanan kolom kiri Beranda lebar.
            let (ex, ey) = if self.view == View::Panel {
                (148.0, 76.0)
            } else if self.view == View::PanelWide {
                (18.0, 76.0)
            } else {
                (18.0 - self.width.value / 2.0, self.height.value / 2.0)
            };
            let eye_x = pos.x as f32 + size.cx as f32 / 2.0 + ex * scale;
            let eye_y = pos.y as f32 + ey * scale;
            // Lirikan penuh saat kursor sekitar sepertiga layar jauhnya.
            let reach = ((self.monitor_rect.right - self.monitor_rect.left) as f32 / 3.0).max(1.0);
            (((cursor.x as f32 - eye_x) / reach).clamp(-1.0, 1.0), ((cursor.y as f32 - eye_y) / (reach * 0.6)).clamp(-1.0, 1.0))
        };
        let moved = (target.0 - self.gaze_target.0).abs().max((target.1 - self.gaze_target.1).abs());
        if moved > 0.05 {
            self.start_motion();
            self.gaze_target = target;
        }
        self.update_gaze_poll();
    }

    /// Ekspresi maskot: sementara (senang/kaget), mengantuk di malam hari, atau biasa.
    fn mood(&self) -> Mood {
        match self.mood {
            Some((mood, until)) if Instant::now() < until => mood,
            _ if night(clock::now().wHour) => Mood::Sleepy,
            _ => Mood::Normal,
        }
    }

    fn set_mood(&mut self, mood: Mood, ms: u32) {
        self.mood = Some((mood, Instant::now() + Duration::from_millis(ms as u64)));
        self.set_timer(TIMER_MOOD, ms + 20);
    }

    fn eyes(&self) -> Eyes {
        Eyes { open: self.eyes_open(), look: self.gaze, mood: self.mood() }
    }

    fn start_blink(&mut self) {
        // Mata yang sedang tersenyum ("^ ^") tidak berkedip.
        if self.eyes_visible() && self.mood() != Mood::Happy {
            self.start_motion();
            self.blink_since = Some(self.now());
        }
    }

    fn schedule_blink(&mut self) {
        // xorshift32: cukup untuk membuat jeda kedip terasa acak.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.set_timer(TIMER_BLINK, 3_500 + self.rng % 4_000);
    }

    fn schedule_clock(&self) {
        self.set_timer(TIMER_CLOCK, clock::ms_to_next_minute(&clock::now()));
    }

    /// Cek ulang jendela aktif: full-screen, maximize, atau biasa. `raise` mengambil lagi
    /// posisi teratas, cukup saat jendela aktif berganti (bukan pada cek berkala).
    fn refresh_mode(&mut self, raise: bool) {
        let mode = desktop::current_mode(self.hwnd, self.monitor, &self.monitor_rect);
        if mode != self.mode {
            debug_log!("mode: {mode:?}");
            self.mode = mode;
            self.apply_visibility();
            self.refresh();
        }
        if raise && self.shown {
            // Jendela "always on top" lain bisa saja naik di atas notch; ambil lagi posisi teratas.
            self.raise();
        }
    }

    fn apply_visibility(&mut self) {
        // Lock screen terdeteksi sebagai jendela full-screen, tapi di sana notch tetap tampil.
        let want = !self.user_hidden && (self.locked || self.mode != Mode::Fullscreen);
        if want == self.shown {
            return;
        }
        if !want {
            self.end_edit(true, false);
        }
        self.shown = want;
        if want {
            self.snap_view();
            self.redraw();
            let _ = unsafe { ShowWindow(self.hwnd, SW_SHOWNOACTIVATE) };
            self.schedule_blink();
        } else {
            let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
            // Muncul lagi nanti sebagai pil tertutup, bukan panel yang masih terbuka.
            self.expanded = false;
            self.hovering = false;
            self.tracking_leave = false;
            self.hovered = None;
            self.hud_visible = false;
            self.blink_since = None;
            self.page_anim = None;
            self.ambient = None;
            self.gaze_poll = None;
            for timer in [TIMER_OPEN, TIMER_CLOSE, TIMER_BLINK, TIMER_HUD, TIMER_AMBIENT, TIMER_GAZE] {
                self.kill_timer(timer);
            }
        }
    }

    /// Posisikan ulang di monitor utama, misalnya setelah resolusi atau skala layar berubah.
    fn place(&mut self) {
        let (monitor, rect) = primary_monitor();
        self.monitor = monitor;
        self.monitor_rect = rect;
        if let Err(e) = self.renderer.set_dpi(scaled_dpi(monitor_dpi(monitor), self.settings.size)) {
            debug_log!("gagal mengganti DPI: {e}");
        }
        self.redraw();
    }

    /// Seberapa jauh perpindahan bentuk sudah berjalan, 0..1.
    fn transition(&self) -> f32 {
        let (tw, th) = self.target;
        let total = (self.from.0 - tw).hypot(self.from.1 - th);
        if total < 0.5 {
            return 1.0;
        }
        (1.0 - (self.width.value - tw).hypot(self.height.value - th) / total).clamp(0.0, 1.0)
    }

    /// Isi bentuk tujuan baru muncul setelah bentuknya hampir sampai; isi lain memudar.
    fn alpha_goals(&self) -> [f32; CONTENTS] {
        let mut goals = [0.0; CONTENTS];
        if let Some(i) = self.view.content()
            && (self.transition() > 0.6 || self.panel_resize)
        {
            goals[i] = 1.0;
        }
        goals
    }

    fn animating(&self) -> bool {
        self.shown
            && (self.width.is_moving()
                || self.height.is_moving()
                || self.alpha != self.alpha_goals()
                || self.blink_since.is_some()
                || self.page_anim.is_some()
                || self.gaze != self.gaze_target
                || self.shadow != self.shadow_goal()
                || !self.tile_fx.is_empty()
                || self.drop_glow.is_some()
                || self.hud_fill != self.hud_fill_target
                || self.hud_roll.is_some()
                || self.wake.is_some()
                || self.lyric_mix != self.lyric_goal()
                || self.lyric_scroll.is_some())
    }

    /// Panggil sebelum mengubah target animasi, supaya frame pertama tidak melompat.
    fn start_motion(&mut self) {
        if !self.animating() {
            self.last_frame = self.now();
        }
    }

    fn tick(&mut self) {
        FRAMES.fetch_add(1, Ordering::Relaxed);
        // Pil yang diam dan hanya sedang berkedip/melirik cukup digambar ulang di sekitar matanya.
        let eyes_only = self.view == View::Pill
            && self.alpha[0] == 1.0
            && !self.width.is_moving()
            && !self.height.is_moving()
            && self.alpha == self.alpha_goals()
            && self.page_anim.is_none()
            && !self.playing()
            && !self.focus.running()
            && !self.timer.running();
        let now = self.now();
        let dt = (now - self.last_frame) as f32;
        self.last_frame = now;
        self.width.step(dt);
        self.height.step(dt);
        let goals = self.alpha_goals();
        for (alpha, goal) in self.alpha.iter_mut().zip(goals) {
            let tau = if goal > *alpha { 0.08 } else { 0.05 };
            *alpha += (goal - *alpha) * (1.0 - (-dt / tau).exp());
            if (goal - *alpha).abs() < 0.004 {
                *alpha = goal;
            }
        }
        if self.blink_since.is_some_and(|start| now - start >= BLINK_SECS) {
            self.blink_since = None;
        }
        if self.page_anim.is_some_and(|(_, start, _)| now - start >= PAGE_ANIM_SECS) {
            self.page_anim = None;
        }
        // Ubin yang selesai menghilang baru sekarang dibuang dari shelf.
        let finished: Vec<(u64, bool)> = self
            .tile_fx
            .iter()
            .filter(|(_, (start, removing))| now - start >= if *removing { TILE_OUT_SECS } else { TILE_IN_SECS })
            .map(|(id, (_, removing))| (*id, *removing))
            .collect();
        let mut removed = false;
        for (id, removing) in finished {
            self.tile_fx.remove(&id);
            if removing {
                self.shelf.remove(id);
                removed = true;
            }
        }
        if removed {
            self.shelf_changed();
        }
        if self.drop_glow.is_some_and(|start| now - start >= GLOW_SECS) {
            self.drop_glow = None;
        }
        if self.hud_roll.as_ref().is_some_and(|(_, start, _)| now - start >= ROLL_SECS) {
            self.hud_roll = None;
        }
        // Sesudah bangun, maskot langsung berkedip sekali.
        if self.wake.is_some_and(|start| now - start >= WAKE_CLOSED_SECS + WAKE_OPEN_SECS) {
            self.wake = None;
            self.set_timer(TIMER_BLINK, 350);
        }
        if self.lyric_scroll.is_some_and(|start| now - start >= LYRIC_SCROLL_SECS) {
            self.lyric_scroll = None;
        }
        let goal = self.lyric_goal();
        self.lyric_mix += (goal - self.lyric_mix) * (1.0 - (-dt / 0.09).exp());
        if (goal - self.lyric_mix).abs() < 0.004 {
            self.lyric_mix = goal;
        }
        self.hud_fill += (self.hud_fill_target - self.hud_fill) * (1.0 - (-dt / 0.07).exp());
        if (self.hud_fill_target - self.hud_fill).abs() < 0.002 {
            self.hud_fill = self.hud_fill_target;
        }
        let shadow = self.shadow_goal();
        self.shadow += (shadow - self.shadow) * (1.0 - (-dt / 0.1).exp());
        if (shadow - self.shadow).abs() < 0.01 {
            self.shadow = shadow;
        }
        if self.gaze != self.gaze_target {
            let k = 1.0 - (-dt / 0.09).exp();
            let (tx, ty) = self.gaze_target;
            self.gaze = (self.gaze.0 + (tx - self.gaze.0) * k, self.gaze.1 + (ty - self.gaze.1) * k);
            if (tx - self.gaze.0).abs() < 0.01 && (ty - self.gaze.1).abs() < 0.01 {
                self.gaze = self.gaze_target;
            }
        }
        if eyes_only && self.shown {
            let result = self
                .renderer
                .draw_eyes(self.width.value, self.height.value, self.eyes())
                .and_then(|dirty| self.renderer.present(self.hwnd, self.window_pos(), Some(&dirty)));
            if let Err(e) = result {
                debug_log!("gagal menggambar mata: {e}");
            }
        } else {
            self.redraw();
        }
    }

    fn redraw(&mut self) {
        if !self.shown {
            return;
        }
        let t = clock::now();
        let (time, date) = (clock::time(&t), clock::date(&t));
        let d = today();
        let day = format!("{} {} {} {}", d.weekday(), d.day, d.month_name(), d.year);
        let battery = self.battery.or_else(power::battery_percent).map(|b| (b, self.plugged == Some(true)));
        let media = self.media.as_ref().map(|m| media_view(m, self.seek_preview));
        let lyrics = self.lyrics_view();
        let focus = self.focus_view();
        let water = self.water_view();
        let timer = self.timer_view();
        let controls = self.controls_view();
        let weather = self.weather_text();
        let calendar = self.calendar_view();
        let download = self.download_text();
        let lyric = self.lyric_line();
        let now = self.now();
        let roll = self.hud_roll.as_ref().map(|(from, start, up)| Roll {
            from: from.clone(),
            t: ((now - start) / ROLL_SECS).min(1.0) as f32,
            up: *up,
        });
        let shelf_fx = ShelfFx {
            targeted: self.shelf.drop_hover && self.drop_over == Some(DropZone::Shelf),
            share_targeted: self.shelf.drop_hover && self.drop_over == Some(DropZone::Share),
            dash: (now * 5.0 % 5.0) as f32,
            glow: self.drop_glow.map(|start| ((now - start) / GLOW_SECS).min(1.0) as f32),
            tiles: self.tile_fx.iter().map(|(id, (start, removing))| (*id, tile_look(now - start, *removing))).collect(),
        };
        let uptime = clock::uptime();
        let scene = Scene {
            width: self.width.value,
            height: self.height.value,
            alpha: self.alpha,
            eyes: self.eyes(),
            shadow: self.shadow,
            time: &time,
            date: &date,
            day: &day,
            greeting: clock::greeting(&t),
            battery,
            media: media.as_ref(),
            lyric: lyric.as_ref().map(|(_, text)| text.as_str()),
            lyrics: lyrics.as_ref(),
            lyric_mix: self.lyric_mix,
            lyrics_button: self.settings.lyrics.then_some(self.settings.lyric_panel),
            hud: self.hud.as_ref(),
            // Mengikuti jenis kabarnya, bukan bentuk notch saat ini: selama kabar memudar, notch
            // sudah kembali ke bentuk pil, tapi isinya harus tetap bentuk ringkas.
            hud_compact: self.compact_hud(),
            hud_roll: roll.as_ref(),
            hud_fill: self.hud_fill,
            page: self.page,
            pages: self.settings.pages,
            page_from: self
                .page_anim
                .map(|(from, start, dir)| (from, ((now - start) / PAGE_ANIM_SECS).clamp(0.0, 1.0) as f32, dir)),
            shelf: &self.shelf,
            shelf_fx: &shelf_fx,
            tasks: &self.tasks,
            focus: &focus,
            water: &water,
            privacy: &self.privacy,
            system: self.system.as_ref(),
            uptime: &uptime,
            hovered: self.hovered,
            seconds: now,
            locked: self.locked,
            timer: &timer,
            controls: &controls,
            weather: weather.as_deref(),
            city_draft: self.city_draft.as_deref(),
            calendar: calendar.as_ref(),
            agenda_draft: self.agenda.draft.as_deref(),
            download: download.as_deref(),
        };
        let result = self.renderer.draw(&scene).and_then(|_| self.renderer.present(self.hwnd, self.window_pos(), None));
        if let Err(e) = result {
            debug_log!("gagal menggambar: {e}");
        }
        self.last_full = now;
        self.lyric_shown = lyric.and_then(|(index, _)| index);
        self.lyric_drawn = self.lyric_index();
        self.second_drawn = self.media.as_ref().map(|m| m.position_now() as u64);
    }

    fn timer_view(&self) -> TimerView {
        let now = Instant::now();
        let left = self.timer.remaining(now);
        TimerView {
            running: self.timer.running(),
            active: self.timer.active(),
            clock: focus::clock(left),
            short: timer::short(left),
            progress: self.timer.progress(now),
            total_minutes: self.timer.total_minutes(),
        }
    }

    fn controls_view(&self) -> ControlsView {
        let quiet = self.quiet_until.filter(|_| self.quiet());
        // Tombol kota sekaligus menunjukkan suhu, karena halaman jam tertutup pemutar musik.
        let fresh = self.weather.as_ref().filter(|(_, at)| at.elapsed() < WEATHER_STALE);
        let city = self.settings.weather.then(|| {
            self.city_status.clone().unwrap_or_else(|| match (&self.city, fresh) {
                (Some(city), Some((weather, _))) => format!("{} · {:.0}°", city.name, weather.temp),
                (Some(city), None) => format!("Kota: {}", city.name),
                (None, _) => "Atur kota cuaca".to_string(),
            })
        });
        ControlsView {
            volume: self.volume,
            brightness: self.brightness,
            mic: self.mic,
            quiet_minutes: quiet.map(|until| until.saturating_duration_since(Instant::now()).as_secs().div_ceil(60) as u32),
            city,
        }
    }

    /// "28° Mendung · Jakarta", selama cuaca aktif dan datanya belum basi.
    fn weather_text(&self) -> Option<String> {
        let (weather, _) = self.weather.as_ref().filter(|(_, at)| at.elapsed() < WEATHER_STALE)?;
        let city = self.city.as_ref().filter(|_| self.settings.weather)?;
        let desc = weather::describe(weather.code);
        let sky = if desc.is_empty() { String::new() } else { format!(" {desc}") };
        Some(format!("{:.0}°{sky} · {}", weather.temp, city.name))
    }

    fn focus_view(&self) -> FocusView {
        let now = Instant::now();
        let left = self.focus.remaining(now);
        FocusView {
            phase: self.focus.phase,
            running: self.focus.running(),
            active: self.focus.active(),
            clock: focus::clock(left),
            progress: self.focus.progress(now),
            sessions: self.focus.sessions_today,
            minutes_left: left.as_secs().div_ceil(60) as u32,
        }
    }

    fn water_view(&self) -> WaterView {
        let minutes = self.water_due.saturating_duration_since(Instant::now()).as_secs().div_ceil(60);
        WaterView {
            ml: self.water.on(&self.day),
            goal: GOAL_ML,
            week: clock::recent_days(7).iter().map(|(day, initial)| (self.water.on(day), *initial)).collect(),
            next: Some(clock::time_in(minutes as u32)),
        }
    }

    fn eyes_open(&self) -> f32 {
        // Bangun: terpejam sebentar, lalu terbuka perlahan (mulus di awal dan akhir).
        if let Some(start) = self.wake {
            let t = ((self.now() - start - WAKE_CLOSED_SECS) / WAKE_OPEN_SECS).clamp(0.0, 1.0) as f32;
            return t * t * (3.0 - 2.0 * t);
        }
        match self.blink_since {
            Some(start) => {
                let t = ((self.now() - start) / BLINK_SECS).clamp(0.0, 1.0) as f32;
                1.0 - (t * std::f32::consts::PI).sin()
            }
            None => 1.0,
        }
    }

    /// Tengah atas monitor utama.
    fn window_pos(&self) -> POINT {
        let size = self.renderer.size();
        let r = &self.monitor_rect;
        POINT { x: r.left + (r.right - r.left - size.cx) / 2, y: r.top }
    }

    fn menu_state(&self) -> MenuState {
        MenuState {
            visible: !self.user_hidden,
            autostart: autostart::is_enabled(),
            settings: self.settings,
            messages_available: self.messages_available,
            messages_denied: self.messages_denied,
            hotkey_taken: self.settings.hotkey && !self.hotkey,
        }
    }

    fn on_command(&mut self, command: Command) {
        match command {
            Command::ToggleVisible => {
                self.user_hidden = !self.user_hidden;
                self.apply_visibility();
                self.refresh();
            }
            Command::ToggleAutostart => {
                if let Err(e) = autostart::set_enabled(!autostart::is_enabled()) {
                    debug_log!("gagal mengubah autostart: {e}");
                }
            }
            Command::ToggleLyrics => {
                self.settings.lyrics = !self.settings.lyrics;
                self.settings.save();
                if self.settings.lyrics {
                    self.request_lyrics();
                } else {
                    self.lyrics = None;
                    self.lyric_pending = None;
                }
                self.refresh();
            }
            Command::ToggleSound => {
                self.settings.sound = !self.settings.sound;
                self.settings.save();
            }
            Command::ToggleLockKeys => {
                self.settings.lock_keys = !self.settings.lock_keys;
                self.settings.save();
                self.set_raw_keyboard(self.settings.lock_keys);
            }
            Command::ToggleBrightness => {
                self.settings.brightness = !self.settings.brightness;
                self.settings.save();
            }
            Command::ToggleHideContent => {
                self.settings.hide_content = !self.settings.hide_content;
                self.settings.save();
            }
            Command::Messages(mode) => {
                self.settings.messages = mode;
                self.settings.save();
                if mode == Messages::Off {
                    self.services.send(Task::WatchMessages(false));
                } else if self.messages_available {
                    // Izin dicek (dan diminta kalau belum pernah); hasilnya menyalakan pemantauan.
                    messages::request_access(self.services.notifier());
                }
            }
            Command::ToggleHotkey => {
                self.settings.hotkey = !self.settings.hotkey;
                self.settings.save();
                self.set_hotkey(self.settings.hotkey);
            }
            Command::ToggleWifi => {
                self.settings.wifi = !self.settings.wifi;
                self.settings.save();
                self.network = None;
                self.services.send(Task::WatchNetwork(self.settings.wifi));
            }
            Command::ToggleDownloads => {
                self.settings.downloads = !self.settings.downloads;
                self.downloading = None;
                self.settings.save();
                self.update_watchers();
            }
            Command::ToggleMeeting => {
                self.settings.meeting = !self.settings.meeting;
                self.settings.save();
            }
            Command::ToggleWeather => {
                self.settings.weather = !self.settings.weather;
                self.settings.save();
                if !self.settings.weather {
                    self.weather = None;
                } else if self.city.is_none() {
                    // Belum ada kota: buka halaman kontrol dan langsung minta nama kotanya.
                    self.page = Page::Controls;
                    self.page_anim = None;
                    self.set_expanded(true);
                    self.begin_edit(Field::City);
                }
                self.fetch_weather();
                self.refresh();
            }
            Command::ToggleCalendar => {
                self.settings.calendar = !self.settings.calendar;
                self.settings.save();
                self.refresh();
            }
            Command::ToggleCompactHud => {
                self.settings.compact_hud = !self.settings.compact_hud;
                self.settings.save();
            }
            Command::ToggleBrowserMessages => {
                self.settings.browser_messages = !self.settings.browser_messages;
                self.settings.save();
            }
            Command::ToggleScreenshots => {
                self.settings.screenshots = !self.settings.screenshots;
                self.settings.save();
                self.update_watchers();
            }
            Command::TogglePage(page) => {
                let shown = !self.settings.pages.shown(page);
                self.settings.pages.set_shown(page, shown);
                self.settings.save();
                if !shown && self.page == page {
                    self.page = Page::Home;
                    self.page_anim = None;
                }
                self.update_watchers();
                self.refresh();
            }
            Command::Size(size) => {
                self.settings.size = size;
                self.settings.save();
                self.place();
            }
            Command::Theme(theme) => {
                self.settings.theme = theme;
                self.settings.save();
                self.renderer.set_theme(theme);
                self.redraw();
            }
            Command::Exit => unsafe { PostQuitMessage(0) },
        }
    }

    fn now(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    fn set_timer(&self, id: usize, ms: u32) {
        unsafe { SetTimer(Some(self.hwnd), id, ms, None) };
    }

    fn kill_timer(&self, id: usize) {
        let _ = unsafe { KillTimer(Some(self.hwnd), id) };
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            let _ = WTSUnRegisterSessionNotification(self.hwnd);
            let _ = UnregisterHotKey(Some(self.hwnd), HOTKEY_ID);
        }
    }
}

/// Ubah data media mentah jadi teks yang siap digambar, dengan posisi yang terus berjalan.
/// Selama bar diseret, posisinya mengikuti `seek` (0..1).
fn media_view(s: &Snapshot, seek: Option<f32>) -> MediaView {
    let position = match seek {
        Some(fraction) if s.duration > 0.0 => fraction as f64 * s.duration,
        _ => s.position_now(),
    };
    MediaView {
        title: s.title.clone(),
        artist: s.artist.clone(),
        playing: s.playing,
        progress: (s.duration > 1.0).then(|| (position / s.duration) as f32),
        elapsed: clock::duration(position),
        remaining: format!("-{}", clock::duration(s.duration - position)),
        can_previous: s.can_previous,
        can_next: s.can_next,
        can_seek: s.can_seek && s.duration > 1.0,
        seeking: seek.is_some(),
        accent: s.art.as_ref().and_then(|art| art.accent),
    }
}

/// Titik dari `lparam` pesan mouse.
fn point(lparam: LPARAM) -> (i32, i32) {
    ((lparam.0 & 0xFFFF) as u16 as i16 as i32, ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32)
}

/// Status lampu Caps Lock dan Num Lock saat ini.
fn lock_key_states() -> (bool, bool) {
    unsafe { (GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0, GetKeyState(VK_NUMLOCK.0 as i32) & 1 != 0) }
}

/// Teks dari clipboard, untuk Ctrl+V saat mengetik tugas.
fn clipboard_text() -> Option<String> {
    unsafe {
        OpenClipboard(None).ok()?;
        let text = GetClipboardData(CF_UNICODETEXT.0 as u32).ok().and_then(|handle| {
            let memory = HGLOBAL(handle.0);
            let ptr = GlobalLock(memory) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            let _ = GlobalUnlock(memory);
            Some(text)
        });
        let _ = CloseClipboard();
        text
    }
}

fn primary_monitor() -> (HMONITOR, RECT) {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut info);
        (monitor, info.rcMonitor)
    }
}

/// Skala dan transparansi ubin shelf yang sedang muncul atau menghilang, `elapsed` detik sejak mulai.
fn tile_look(elapsed: f64, removing: bool) -> (f32, f32) {
    if removing {
        let t = (elapsed / TILE_OUT_SECS).min(1.0) as f32;
        (1.0 - 0.35 * t, 1.0 - t)
    } else {
        // Membesar melewati ukuran asli sedikit lalu kembali (ease-out-back).
        let t = (elapsed / TILE_IN_SECS).min(1.0) as f32;
        let (c1, u) = (1.70158, t - 1.0);
        let back = 1.0 + (c1 + 1.0) * u * u * u + c1 * u * u;
        (0.5 + 0.5 * back, (t * 2.0).min(1.0))
    }
}

/// Bawa jendela notch ke depan. Tepat sesudah file dilepas, aplikasi asal file (misal Explorer)
/// masih memegang input sehingga Windows menolak; antrean input aplikasi itu dipinjam sebentar,
/// cara yang memang disediakan Windows untuk berpindah jendela depan antar-aplikasi.
fn bring_to_front(hwnd: HWND) {
    unsafe {
        if SetForegroundWindow(hwnd).as_bool() {
            return;
        }
        let front = GetWindowThreadProcessId(GetForegroundWindow(), None);
        let me = GetCurrentThreadId();
        if front != 0 && front != me && AttachThreadInput(me, front, true).as_bool() {
            let _ = SetForegroundWindow(hwnd);
            let _ = AttachThreadInput(me, front, false);
        }
    }
}

/// Zona tempat file yang diseret akan dilepas.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DropZone {
    Shelf,
    Share,
}

/// Kotak ketik yang sedang dipakai.
#[derive(Clone, Copy)]
enum Field {
    Task,
    City,
    Agenda,
}

/// Tanggal hari ini.
fn today() -> Date {
    let t = clock::now();
    Date::new(t.wYear as i32, t.wMonth as u32, t.wDay as u32)
}

/// Tingkat bilah kabar volume/kecerahan, 0..1.
fn hud_level(hud: &Hud) -> Option<f32> {
    match hud {
        Hud::Volume { level, .. } | Hud::Brightness { level } => Some(*level),
        _ => None,
    }
}

/// Kabar yang tetap muncul (dan berbunyi) walau notch sedang senyap.
fn urgent(hud: &Hud) -> bool {
    matches!(hud, Hud::Power { low: true, .. } | Hud::Timer { .. })
}

/// DPI gambar notch: DPI layar dikali pilihan ukuran.
fn scaled_dpi(dpi: u32, size: Size) -> u32 {
    (dpi as f32 * size.factor()).round() as u32
}

/// Jam mengantuk maskot (23.00–04.59).
fn night(hour: u16) -> bool {
    !(5..23).contains(&hour)
}

fn monitor_dpi(monitor: HMONITOR) -> u32 {
    let (mut x, mut y) = (96, 96);
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) };
    x
}
