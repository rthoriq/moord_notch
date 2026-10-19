//! Jendela notch: status, input mouse/keyboard, timer, dan loop pesan.

use std::cell::RefCell;
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
use windows::Win32::System::Threading::Sleep;
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, SetFocus, TME_LEAVE, TRACKMOUSEEVENT, TrackMouseEvent,
    VK_CAPITAL, VK_NUMLOCK,
};
use windows::Win32::UI::Input::{
    GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK,
    RIDEV_REMOVE, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
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
use crate::dragdrop::{self, DRAG_DROP, DRAG_ENTER, DRAG_LEAVE, DropRegistration};
use crate::lyrics::{self, Lyrics};
use crate::media::{self, Snapshot};
use crate::power::{self, Power};
use crate::privacy::Usage;
use crate::render::{self, Button, CONTENTS, FocusView, Hit, Hud, MediaView, Renderer, Scene, View, WaterView};
use crate::services::{self, Event, Services, Task};
use crate::settings::{Messages, Settings};
use crate::sound::{self, Sound};
use crate::messages;
use crate::sysinfo::Stats;
use crate::tray::{self, Command, MenuState, Tray};
use crate::widgets::Page;
use crate::widgets::focus::{self, Focus};
use crate::widgets::shelf::{self, Shelf};
use crate::widgets::tasks::{self, Tasks};
use crate::widgets::water::{GOAL_ML, Water};
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

const POLL_MS: u32 = 1_000;
/// Jeda sebelum membuka, supaya tidak terbuka saat kursor hanya lewat menuju tab browser.
const OPEN_DELAY_MS: u32 = 140;
const CLOSE_DELAY_MS: u32 = 380;
/// Setelah file dilepas, panel tetap terbuka sebentar supaya hasilnya terlihat.
const DROP_CLOSE_MS: u32 = 2_500;
/// Tunggu jendela berhenti bergerak sebelum mengecek full-screen/maximize.
const DESKTOP_SETTLE_MS: u32 = 60;
const BLINK_SECS: f64 = 0.16;
const PAGE_ANIM_SECS: f64 = 0.24;
/// Selisih ukuran (DIP) yang sudah tidak terlihat; di bawahnya animasi bentuk dianggap selesai.
const SHAPE_PRECISION: f32 = 0.05;

const VOLUME_HUD_MS: u32 = 1_600;
const POWER_HUD_MS: u32 = 3_000;
const BLUETOOTH_HUD_MS: u32 = 3_500;
const FOCUS_HUD_MS: u32 = 6_000;
const WATER_HUD_MS: u32 = 6_000;
const PRIVACY_HUD_MS: u32 = 3_500;
const BRIGHTNESS_HUD_MS: u32 = 1_600;
const LOCK_KEY_HUD_MS: u32 = 1_400;
const MESSAGE_HUD_MS: u32 = 5_000;
/// Gelombang di pil digambar ulang ~12 kali per detik: cukup hidup, nyaris tanpa beban CPU.
const WAVE_MS: u32 = 80;
const PROGRESS_MS: u32 = 1_000;
/// Halaman musik dengan lirik diperbarui lebih rapat supaya pergantian baris tepat waktu.
const LYRIC_MS: u32 = 250;
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
        let renderer = Renderer::new(monitor_dpi(monitor), settings.theme)?;
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
            system: None,
            system_watch: false,
            target: (w, h),
            locked: false,
            brightness: None,
            lock_keys: lock_key_states(),
            raw_keyboard: false,
            messages_available: false,
            messages_denied: false,
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
        if app.messages_available && app.settings.messages != Messages::Off {
            messages::request_access(app.services.notifier());
        }
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
            WM_INPUT => {
                self.on_raw_input(lparam);
                // Windows perlu membersihkan data input mentahnya sendiri.
                return None;
            }
            WM_WTSSESSION_CHANGE => self.on_session_change(wparam.0 as u32),
            dragdrop::WM_DRAG => self.on_drag(wparam.0),
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
            }
        }
    }

    fn on_leave(&mut self) {
        self.tracking_leave = false;
        self.hovering = false;
        self.kill_timer(TIMER_OPEN);
        // Selama mengetik, panel tetap terbuka walau kursor pergi.
        if self.expanded && self.tasks.draft.is_none() {
            self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
        }
    }

    fn on_press(&mut self, lparam: LPARAM) {
        if self.expanded
            && let Some(Hit::ShelfItem(id)) = self.hit_at(lparam)
        {
            let (x, y) = point(lparam);
            self.press = Some((id, POINT { x, y }));
            unsafe { SetCapture(self.hwnd) };
        }
    }

    fn on_click(&mut self, lparam: LPARAM) {
        // Klik biasa pada ubin shelf tidak melakukan apa-apa; buka dengan klik dua kali.
        if self.press.take().is_some() {
            let _ = unsafe { ReleaseCapture() };
            return;
        }
        self.kill_timer(TIMER_OPEN);
        if !self.expanded {
            // Mengklik kabar pesan membuka aplikasinya (tidak dari lock screen).
            if self.hud_visible
                && !self.locked
                && let Some(Hud::Message { aumid, .. }) = &self.hud
            {
                dragdrop::open_app(self.hwnd, aumid);
                self.hud_visible = false;
                self.refresh();
                return;
            }
            // Mengklik kabar singkat lain membuka halaman yang bersangkutan.
            if self.hud_visible
                && let Some(hud) = &self.hud
            {
                self.page = match hud {
                    Hud::Focus { .. } => Page::Focus,
                    Hud::Water { .. } => Page::Water,
                    Hud::Privacy { .. } => Page::System,
                    _ => Page::Home,
                };
                self.page_anim = None;
            }
            if self.locked {
                self.page = Page::Home;
            }
            self.set_expanded(true);
            return;
        }
        let hit = self.hit_at(lparam);
        if self.tasks.draft.is_some() && hit != Some(Hit::TaskInput) {
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
            Hit::PagePrev => self.go_page(self.page.step(-1), -1.0),
            Hit::PageNext => self.go_page(self.page.step(1), 1.0),
            Hit::PageDot(page) => {
                let dir = if page.index() > self.page.index() { 1.0 } else { -1.0 };
                self.go_page(page, dir);
            }
            Hit::Media(button) => self.media_command(button),
            Hit::ShelfClear => {
                self.shelf.clear();
                self.shelf_changed();
            }
            Hit::ShelfRemove(id) => {
                self.shelf.remove(id);
                self.shelf_changed();
            }
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
                self.water.drink(&self.day);
                self.water.save();
                self.water_due = now + WATER_EVERY;
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
            Hit::TaskInput => self.begin_edit(),
            Hit::TasksClearDone => {
                self.tasks.clear_done();
                self.tasks.save();
            }
            Hit::ShelfItem(_) | Hit::ShelfStrip | Hit::TaskList => {}
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
            _ => self.go_page(self.page.step(step), step as f32),
        }
        self.hovered = None;
        self.refresh();
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

    fn shelf_changed(&mut self) {
        self.shelf.save();
        let shelf = &self.shelf;
        self.renderer.retain_thumbnails(|id| shelf.get(id).is_some());
    }

    fn on_drag(&mut self, kind: usize) {
        match kind {
            DRAG_ENTER => {
                self.shelf.drop_hover = true;
                self.kill_timer(TIMER_CLOSE);
                if self.expanded {
                    let dir = if Page::Shelf.index() > self.page.index() { 1.0 } else { -1.0 };
                    self.go_page(Page::Shelf, dir);
                } else {
                    self.page = Page::Shelf;
                    self.page_anim = None;
                    self.set_expanded(true);
                }
            }
            DRAG_LEAVE => {
                self.shelf.drop_hover = false;
                if self.expanded && !self.hovering {
                    self.set_timer(TIMER_CLOSE, CLOSE_DELAY_MS);
                }
            }
            DRAG_DROP => {
                self.shelf.drop_hover = false;
                let added = self.shelf.add(dragdrop::take_dropped());
                if !added.is_empty() {
                    self.shelf.save();
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

    /// Mulai mengetik tugas baru. Notch sementara boleh menerima fokus keyboard.
    fn begin_edit(&mut self) {
        if self.tasks.draft.is_some() {
            return;
        }
        self.tasks.draft = Some(String::new());
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
        let Some(draft) = self.tasks.draft.take() else { return };
        if commit && self.tasks.add(&draft) {
            self.tasks.reveal(self.tasks.items.len() - 1);
            self.tasks.save();
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
        if self.tasks.draft.is_none() {
            return;
        }
        match unit {
            0x08 => {
                if let Some(draft) = &mut self.tasks.draft {
                    draft.pop();
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
        if let Some(draft) = &mut self.tasks.draft {
            let room = tasks::MAX_LEN.saturating_sub(draft.chars().count());
            draft.extend(text.chars().filter(|c| !c.is_control()).take(room));
        }
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
                if !self.hovering && self.tasks.draft.is_none() && !self.shelf.drop_hover {
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
                }
                self.schedule_focus();
                self.refresh();
            }
            TIMER_LOCK_KEYS => self.on_lock_keys(),
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
        // Pil yang diam cukup diperbarui gelombangnya saja; gambar penuh kalau baris lirik
        // berganti, dan sesekali supaya jam dan cincin pomodoro ikut terbarui.
        let pill = matches!(self.view, View::Pill | View::LyricPill);
        let settled = self.view.content().is_some_and(|i| self.alpha[i] == 1.0);
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
        if !self.animating() {
            self.redraw();
        }
        self.schedule_clock();
    }

    /// Ingatkan minum kalau sudah waktunya, hanya di jam aktif dan selama target belum tercapai.
    fn check_water(&mut self) {
        let now = Instant::now();
        if now < self.water_due {
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
                Event::Volume { level, muted } => self.show_hud(Hud::Volume { level, muted }, VOLUME_HUD_MS),
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
                    self.messages_denied = !allowed;
                    self.services.send(Task::WatchMessages(allowed && self.settings.messages != Messages::Off));
                }
                Event::Message(message) => self.on_message(message),
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
        }
    }

    /// Pesan masuk dari aplikasi chat, disaring sesuai pengaturan. Di lock screen atau kalau
    /// diminta, isi pesannya tidak ditampilkan.
    fn on_message(&mut self, message: messages::Message) {
        let wanted = match self.settings.messages {
            Messages::Off => false,
            Messages::WhatsApp => message.is_whatsapp(),
            Messages::AllChat => true,
        };
        if !wanted {
            return;
        }
        if let Some(logo) = &message.logo {
            self.renderer.set_logo(&message.aumid, messages::LOGO_PX, logo);
        }
        let messages::Message { app, aumid, sender, text, .. } = message;
        // Di lock screen siapa pun bisa melihat layar, jadi pengirimnya pun tidak ditampilkan.
        let (app, sender) = if self.locked { (String::new(), app) } else { (app, sender) };
        let text = if self.settings.hide_content || self.locked || text.is_empty() { "Pesan baru".to_string() } else { text };
        let hud = Hud::Message { app, aumid, sender, text };
        self.alert(hud, MESSAGE_HUD_MS, Sound::Message);
    }

    /// Kabar singkat yang disertai bunyi (kalau suara notifikasi aktif). Bunyi tetap diputar
    /// walau panel sedang terbuka, karena kabarnya penting.
    fn alert(&mut self, hud: Hud, ms: u32, sound: Sound) {
        if self.settings.sound {
            sound::play(sound);
        }
        self.show_hud(hud, ms);
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
        if !self.shown || self.expanded {
            return;
        }
        self.hud = Some(hud);
        self.hud_visible = true;
        self.set_timer(TIMER_HUD, ms);
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
        }
        self.refresh();
    }

    fn wanted_view(&self) -> View {
        // Titik privasi lebih penting daripada ruang tab: selama kamera/mikrofon dipakai,
        // notch tidak menciut jadi garis tipis.
        let slim = self.mode == Mode::Maximized && !self.privacy.active() && !self.locked;
        if self.expanded {
            View::Panel
        } else if self.hud_visible {
            View::Hud
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
            if view != self.view {
                debug_log!("bentuk: {:?} -> {view:?}", self.view);
                self.from = (self.width.value, self.height.value);
            } else if self.transition() >= 1.0 {
                // Hanya ukurannya yang berubah (baris lirik baru): isinya tidak perlu memudar.
                self.from = target;
            }
            self.view = view;
            self.target = target;
            self.width.target = target.0;
            self.height.target = target.1;
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
        self.alpha = self.alpha_goals();
    }

    /// Ukuran bentuk; pil lirik melebar, atau jadi dua baris, mengikuti baris liriknya.
    fn view_size(&self, view: View) -> (f32, f32) {
        match (view, self.lyric_line()) {
            (View::LyricPill, Some((_, line))) => self.renderer.lyric_size(&line),
            _ => view.size(),
        }
    }

    /// Timer untuk isi yang bergerak sendiri, hanya selama memang ada yang berjalan.
    fn update_ambient(&mut self) {
        let playing = self.playing();
        let focusing = self.focus.running();
        let lyric = self.lyric_line().is_some();
        let interval = match self.view {
            _ if !self.shown => None,
            View::Pill | View::LyricPill if playing => Some(WAVE_MS),
            View::Pill if focusing => Some(PILL_FOCUS_MS),
            View::Panel if self.page == Page::Home && playing => Some(if lyric { LYRIC_MS } else { PROGRESS_MS }),
            View::Panel if self.page == Page::Focus && focusing => Some(FOCUS_TICK_MS),
            _ => None,
        };
        if interval != self.ambient {
            self.ambient = interval;
            match interval {
                Some(ms) => self.set_timer(TIMER_AMBIENT, ms),
                None => self.kill_timer(TIMER_AMBIENT),
            }
        }
    }

    fn start_blink(&mut self) {
        let playing = self.media.as_ref().is_some_and(|m| m.playing);
        let eyes_visible = match self.view {
            View::Pill => !playing && !self.focus.running(),
            View::Panel => self.page == Page::Home && self.media.is_none(),
            _ => false,
        };
        if self.shown && eyes_visible {
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
            for timer in [TIMER_OPEN, TIMER_CLOSE, TIMER_BLINK, TIMER_HUD, TIMER_AMBIENT] {
                self.kill_timer(timer);
            }
        }
    }

    /// Posisikan ulang di monitor utama, misalnya setelah resolusi atau skala layar berubah.
    fn place(&mut self) {
        let (monitor, rect) = primary_monitor();
        self.monitor = monitor;
        self.monitor_rect = rect;
        if let Err(e) = self.renderer.set_dpi(monitor_dpi(monitor)) {
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
            && self.transition() > 0.6
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
                || self.page_anim.is_some())
    }

    /// Panggil sebelum mengubah target animasi, supaya frame pertama tidak melompat.
    fn start_motion(&mut self) {
        if !self.animating() {
            self.last_frame = self.now();
        }
    }

    fn tick(&mut self) {
        FRAMES.fetch_add(1, Ordering::Relaxed);
        // Pil yang diam dan hanya sedang berkedip cukup digambar ulang di sekitar matanya.
        let blink_only = self.view == View::Pill
            && self.alpha[0] == 1.0
            && !self.width.is_moving()
            && !self.height.is_moving()
            && self.alpha == self.alpha_goals()
            && self.page_anim.is_none()
            && !self.playing()
            && !self.focus.running();
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
        if blink_only && self.shown {
            let result = self
                .renderer
                .draw_eyes(self.eyes_open())
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
        let media = self.media.as_ref().map(media_view);
        let focus = self.focus_view();
        let water = self.water_view();
        let lyric = self.lyric_line();
        let uptime = clock::uptime();
        let now = self.now();
        let scene = Scene {
            width: self.width.value,
            height: self.height.value,
            alpha: self.alpha,
            eyes_open: self.eyes_open(),
            time: &time,
            date: &date,
            greeting: clock::greeting(&t),
            media: media.as_ref(),
            lyric: lyric.as_ref().map(|(_, text)| text.as_str()),
            hud: self.hud.as_ref(),
            page: self.page,
            page_from: self
                .page_anim
                .map(|(from, start, dir)| (from, ((now - start) / PAGE_ANIM_SECS).clamp(0.0, 1.0) as f32, dir)),
            shelf: &self.shelf,
            tasks: &self.tasks,
            focus: &focus,
            water: &water,
            privacy: &self.privacy,
            system: self.system.as_ref(),
            uptime: &uptime,
            hovered: self.hovered,
            seconds: now,
            locked: self.locked,
        };
        let result = self.renderer.draw(&scene).and_then(|_| self.renderer.present(self.hwnd, self.window_pos(), None));
        if let Err(e) = result {
            debug_log!("gagal menggambar: {e}");
        }
        self.last_full = now;
        self.lyric_shown = lyric.and_then(|(index, _)| index);
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
        let _ = unsafe { WTSUnRegisterSessionNotification(self.hwnd) };
    }
}

/// Ubah data media mentah jadi teks yang siap digambar, dengan posisi yang terus berjalan.
fn media_view(s: &Snapshot) -> MediaView {
    let position = s.position_now();
    MediaView {
        title: s.title.clone(),
        artist: s.artist.clone(),
        source: s.source.clone(),
        playing: s.playing,
        progress: (s.duration > 1.0).then(|| (position / s.duration) as f32),
        elapsed: clock::duration(position),
        remaining: format!("-{}", clock::duration(s.duration - position)),
        can_previous: s.can_previous,
        can_next: s.can_next,
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

fn monitor_dpi(monitor: HMONITOR) -> u32 {
    let (mut x, mut y) = (96, 96);
    let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut x, &mut y) };
    x
}
