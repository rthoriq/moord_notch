//! Menggambar notch dengan Direct2D mode software ke sebuah bitmap, lalu menempelkannya
//! ke jendela berlapis (layered window). Tidak memakai GPU sama sekali, jadi RTX tidak
//! ikut terbangun dan pemakaian RAM tetap kecil. Bonusnya, piksel transparan otomatis
//! tembus klik, sehingga hanya bentuk notch yang menangkap mouse.
//!
//! Modul ini berisi inti dan alat gambar dasarnya; tata letak tiap halaman ada di `pages`.

mod pages;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, TAU};

use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D_RECT_F, D2D_SIZE_F, D2D_SIZE_U, D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_BEZIER_SEGMENT,
    D2D1_COLOR_F, D2D1_FIGURE_BEGIN_FILLED, D2D1_FIGURE_BEGIN_HOLLOW, D2D1_FIGURE_END_CLOSED,
    D2D1_FIGURE_END_OPEN, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_ALIASED, D2D1_ARC_SEGMENT, D2D1_ARC_SIZE, D2D1_ARC_SIZE_LARGE,
    D2D1_ARC_SIZE_SMALL, D2D1_BITMAP_PROPERTIES, D2D1_CAP_STYLE_ROUND, D2D1_DASH_STYLE_CUSTOM,
    D2D1_DASH_STYLE_SOLID, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_ELLIPSE,
    D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_LINE_JOIN_ROUND,
    D2D1_RENDER_TARGET_PROPERTIES, D2D1_RENDER_TARGET_TYPE_SOFTWARE,
    D2D1_RENDER_TARGET_USAGE_NONE, D2D1_ROUNDED_RECT, D2D1_STROKE_STYLE_PROPERTIES,
    D2D1_SWEEP_DIRECTION, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE,
    D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE, D2D1CreateFactory, ID2D1BitmapBrush, ID2D1Brush,
    ID2D1DCRenderTarget, ID2D1Factory, ID2D1PathGeometry, ID2D1SolidColorBrush, ID2D1StrokeStyle,
};
use windows::Win32::Graphics::DirectWrite::{
    DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
    DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT, DWRITE_PARAGRAPH_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING,
    DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP, DWRITE_WORD_WRAPPING_WRAP,
    DWriteCreateFactory,
    IDWriteFactory, IDWriteTextFormat,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateBitmap,
    CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GdiFlush,
    HBITMAP, HDC, HGDIOBJ, SelectObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, HICON, ICONINFO, UPDATELAYEREDWINDOWINFO, ULW_ALPHA,
    UpdateLayeredWindowIndirect,
};
use windows::core::{BOOL, PCWSTR, Result, w};
use windows_numerics::{Matrix3x2, Vector2};

use crate::bluetooth::DeviceKind;
use crate::media::{ART_PX, Art};
use crate::privacy::Usage;
use crate::settings::Theme;
use crate::sysinfo::Stats;
use crate::thumbnail::Thumbnail;
use crate::widgets::Page;
use crate::widgets::focus::Phase;
use crate::widgets::shelf::Shelf;
use crate::widgets::tasks::Tasks;

/// Bentuk notch. Ukuran dalam DIP (1 DIP = 1 piksel pada skala layar 100%).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum View {
    /// Garis tipis saat jendela aktif di-maximize.
    Slim,
    Pill,
    /// Pil lebar berisi baris lirik yang sedang dinyanyikan.
    LyricPill,
    /// Kabar singkat: volume, daya, Bluetooth, pomodoro, minum, privasi.
    Hud,
    /// Panel terbuka berisi halaman-halaman widget.
    Panel,
}

/// Jumlah isi yang bisa memudar masuk/keluar; urutannya sesuai [`View::content`].
pub const CONTENTS: usize = 4;

impl View {
    pub const fn size(self) -> (f32, f32) {
        match self {
            View::Slim => (132.0, 7.0),
            View::Pill => (176.0, 30.0),
            View::LyricPill => (360.0, 30.0),
            View::Hud => (320.0, 42.0),
            View::Panel => (440.0, 156.0),
        }
    }

    /// Indeks isi yang tampil di bentuk ini; garis tipis tidak berisi apa-apa.
    pub const fn content(self) -> Option<usize> {
        match self {
            View::Slim => None,
            View::Pill => Some(0),
            View::LyricPill => Some(1),
            View::Hud => Some(2),
            View::Panel => Some(3),
        }
    }
}

/// Kabar singkat yang tampil sebentar di notch.
pub enum Hud {
    Volume { level: f32, muted: bool },
    Power { plugged: bool, percent: Option<u32>, low: bool },
    Bluetooth { id: String, name: String, connected: bool, battery: Option<u8>, kind: DeviceKind },
    Focus { finished: Phase },
    Water { ml: u32, goal: u32 },
    /// Aplikasi mulai memakai kamera (`camera`) atau mikrofon.
    Privacy { camera: bool, app: String },
    /// Kecerahan layar, 0..1.
    Brightness { level: f32 },
    /// Caps Lock (`caps`) atau Num Lock dinyalakan/dimatikan.
    LockKey { caps: bool, on: bool },
    /// Pesan masuk; logonya dicari lewat `aumid` (lihat [`Renderer::set_logo`]).
    Message { app: String, aumid: String, sender: String, text: String },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Button {
    Previous,
    PlayPause,
    Next,
}

/// Bagian panel yang bisa diklik/disorot. Didaftarkan sambil menggambar, jadi letaknya
/// selalu sama dengan yang terlihat.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hit {
    PagePrev,
    PageNext,
    PageDot(Page),
    Media(Button),
    /// Latar deretan ubin shelf (untuk roda mouse).
    ShelfStrip,
    ShelfItem(u64),
    ShelfRemove(u64),
    ShelfClear,
    FocusToggle,
    FocusReset,
    FocusPhase(Phase),
    WaterDrink,
    WaterUndo,
    /// Latar daftar tugas (untuk roda mouse).
    TaskList,
    TaskToggle(usize),
    TaskRemove(usize),
    TaskInput,
    TasksClearDone,
}

impl Hit {
    /// Latar belakang hanya menangkap roda mouse, bukan klik.
    pub fn clickable(self) -> bool {
        !matches!(self, Hit::ShelfStrip | Hit::TaskList)
    }
}

/// Data media yang sudah siap digambar.
pub struct MediaView {
    pub title: String,
    pub artist: String,
    pub source: String,
    pub playing: bool,
    /// 0..1, atau `None` kalau durasinya tidak diketahui (misal siaran langsung).
    pub progress: Option<f32>,
    pub elapsed: String,
    pub remaining: String,
    pub can_previous: bool,
    pub can_next: bool,
    pub accent: Option<u32>,
}

pub struct FocusView {
    pub phase: Phase,
    pub running: bool,
    /// Sudah dimulai (berjalan atau dijeda di tengah jalan).
    pub active: bool,
    /// Sisa waktu, misal "24:59".
    pub clock: String,
    pub progress: f32,
    pub sessions: u32,
    pub minutes_left: u32,
}

pub struct WaterView {
    pub ml: u32,
    pub goal: u32,
    /// Tujuh hari terakhir (ml dan huruf awal hari), hari ini paling kanan.
    pub week: Vec<(u32, char)>,
    /// Jam pengingat berikutnya, misal "19:37".
    pub next: Option<String>,
}

/// Apa yang harus digambar pada satu frame.
pub struct Scene<'a> {
    pub width: f32,
    pub height: f32,
    /// Transparansi tiap isi: pil, pil lirik, HUD, panel.
    pub alpha: [f32; CONTENTS],
    /// 1 = mata terbuka, 0 = terpejam (berkedip).
    pub eyes_open: f32,
    pub time: &'a str,
    pub date: &'a str,
    pub greeting: &'a str,
    pub media: Option<&'a MediaView>,
    /// Baris lirik yang sedang dinyanyikan, kalau lirik aktif dan ditemukan.
    pub lyric: Option<&'a str>,
    pub hud: Option<&'a Hud>,
    pub page: Page,
    /// Halaman sebelumnya selama perpindahan: (halaman, progres 0..1, arah ±1).
    pub page_from: Option<(Page, f32, f32)>,
    pub shelf: &'a Shelf,
    pub tasks: &'a Tasks,
    pub focus: &'a FocusView,
    pub water: &'a WaterView,
    pub privacy: &'a Usage,
    /// Hasil pengukuran terakhir; `None` sebelum yang pertama tiba.
    pub system: Option<&'a Stats>,
    /// Lama komputer menyala, misal "3 j 12 m".
    pub uptime: &'a str,
    /// Laptop terkunci: panel hanya menampilkan jam/musik, tanpa navigasi halaman.
    pub locked: bool,
    pub hovered: Option<Hit>,
    /// Detik berjalan, untuk animasi gelombang musik.
    pub seconds: f64,
}

/// Warna tema. Hijau/merah/oranye/biru air tetap sama di semua tema karena punya arti.
#[derive(Clone, Copy)]
struct Palette {
    /// Badan notch.
    body: D2D1_COLOR_F,
    /// Teks dan ikon utama; juga dasar warna sorotan transparan (lihat `Renderer::tint`).
    text: D2D1_COLOR_F,
    soft: D2D1_COLOR_F,
    muted: D2D1_COLOR_F,
    dim: D2D1_COLOR_F,
    badge: D2D1_COLOR_F,
    eye: D2D1_COLOR_F,
    placeholder: D2D1_COLOR_F,
    accent: D2D1_COLOR_F,
    /// Tema kaca: tepi terang tipis dan bayangan teks supaya tetap terbaca di atas wallpaper.
    glass: bool,
}

fn palette(theme: Theme) -> Palette {
    let dark = |body, soft, muted, dim, badge, placeholder, accent| Palette {
        glass: false,
        body: rgb(body, 1.0),
        text: rgb(0xFFFFFF, 1.0),
        soft: rgb(soft, 1.0),
        muted: rgb(muted, 1.0),
        dim: rgb(dim, 1.0),
        badge: rgb(badge, 1.0),
        eye: rgb(0xF5F5F7, 1.0),
        placeholder: rgb(placeholder, 1.0),
        accent: rgb(accent, 1.0),
    };
    match theme {
        Theme::Black => dark(0x000000, 0xD1D1D6, 0x8E8E93, 0x48484A, 0x3A3A3C, 0x2C2C2E, 0x0A84FF),
        Theme::Graphite => dark(0x1C1C1E, 0xD1D1D6, 0x8E8E93, 0x545458, 0x48484A, 0x3A3A3C, 0x0A84FF),
        Theme::Midnight => dark(0x0B1730, 0xD0DAE8, 0x8A97AD, 0x3A4A66, 0x2A3A56, 0x1C2A45, 0x64D2FF),
        Theme::Grape => dark(0x1A0F2E, 0xDCD3EA, 0x9A8FB0, 0x4A3D66, 0x3A2D56, 0x2C2045, 0xBF5AF2),
        Theme::Glass => Palette {
            body: rgb(0x16161C, 0.58),
            text: rgb(0xFFFFFF, 1.0),
            soft: rgb(0xE5E5EA, 1.0),
            muted: rgb(0xB4B4BE, 1.0),
            dim: rgb(0x6E6E78, 1.0),
            badge: rgb(0x5A5A64, 1.0),
            eye: rgb(0xFFFFFF, 1.0),
            placeholder: rgb(0xFFFFFF, 0.14),
            accent: rgb(0x64D2FF, 1.0),
            glass: true,
        },
        Theme::Light => Palette {
            glass: false,
            body: rgb(0xF5F5F7, 1.0),
            text: rgb(0x1C1C1E, 1.0),
            soft: rgb(0x3A3A3C, 1.0),
            muted: rgb(0x8E8E93, 1.0),
            dim: rgb(0xC7C7CC, 1.0),
            badge: rgb(0xD1D1D6, 1.0),
            eye: rgb(0x1C1C1E, 1.0),
            placeholder: rgb(0xE5E5EA, 1.0),
            accent: rgb(0x007AFF, 1.0),
        },
    }
}

/// Lengkung cekung tempat notch menyatu dengan tepi atas layar.
const FLARE_MIN: f32 = 6.0;
const FLARE_MAX: f32 = 10.0;
/// Pil lirik melebar mengikuti panjang baris sampai batas ini; kalau masih kurang, jadi dua baris.
const LYRIC_MIN_W: f32 = 300.0;
const LYRIC_MAX_W: f32 = 480.0;
const LYRIC_TWO_LINES_H: f32 = 44.0;
/// Ruang di kiri-kanan teks lirik untuk sampul dan gelombang.
const LYRIC_SIDES: f32 = 84.0;
/// Kanvas jendela: muat bentuk terbesar, lengkungannya, dan lebihan saat pegas memantul.
const CANVAS: (f32, f32) = {
    let (w, h) = View::Panel.size();
    let widest = if LYRIC_MAX_W > w { LYRIC_MAX_W } else { w };
    (widest + 2.0 * FLARE_MAX + 40.0, h + 16.0)
};

const CLEAR: D2D1_COLOR_F = rgb(0x000000, 0.0);
const WHITE: D2D1_COLOR_F = rgb(0xFFFFFF, 1.0);
const GREEN: D2D1_COLOR_F = rgb(0x30D158, 1.0);
const RED: D2D1_COLOR_F = rgb(0xFF453A, 1.0);
const ORANGE: D2D1_COLOR_F = rgb(0xFF9F0A, 1.0);
/// Biru air dan Bluetooth.
const BLUE: D2D1_COLOR_F = rgb(0x0A84FF, 1.0);
const BLUE_LIGHT: D2D1_COLOR_F = rgb(0x409CFF, 1.0);
const ICON_BG: D2D1_COLOR_F = rgb(0x1C1C1E, 1.0);
const ICON_EYE: D2D1_COLOR_F = rgb(0xF5F5F7, 1.0);

// Glyph dari font ikon bawaan Windows (Segoe Fluent Icons / Segoe MDL2 Assets).
const ICON_PLAY: char = '\u{E768}';
const ICON_PAUSE: char = '\u{E769}';
const ICON_PREVIOUS: char = '\u{E892}';
const ICON_NEXT: char = '\u{E893}';
const ICON_MUTE: char = '\u{E74F}';
const ICON_VOLUME: [char; 4] = ['\u{E992}', '\u{E993}', '\u{E994}', '\u{E995}'];
const ICON_BLUETOOTH: char = '\u{E702}';
const ICON_HEADPHONE: char = '\u{E7F6}';
const ICON_KEYBOARD: char = '\u{E765}';
const ICON_PHONE: char = '\u{E8EA}';
const ICON_MUSIC: char = '\u{EC4F}';
const ICON_FOLDER: char = '\u{E8B7}';
const ICON_DOCUMENT: char = '\u{E8A5}';
const ICON_STOPWATCH: char = '\u{E916}';
const ICON_REFRESH: char = '\u{E72C}';
const ICON_CLOSE: char = '\u{E711}';
const ICON_CHECK: char = '\u{E73E}';
const ICON_TASKS: char = '\u{E9D5}';
const ICON_SYSTEM: char = '\u{E9D9}';
const ICON_CAMERA: char = '\u{E722}';
const ICON_MICROPHONE: char = '\u{E720}';
const ICON_BRIGHTNESS: char = '\u{E706}';
const ICON_MESSAGE: char = '\u{E8BD}';
const ICON_CHEVRON_LEFT: char = '\u{E76B}';
const ICON_CHEVRON_RIGHT: char = '\u{E76C}';
const ICON_CHEVRON_UP: char = '\u{E70E}';
const ICON_CHEVRON_DOWN: char = '\u{E70D}';
/// Baterai 0..10 dan baterai mengisi 0..10 berurutan mulai dari kode ini.
const ICON_BATTERY: u32 = 0xEBA0;
const ICON_BATTERY_CHARGING: u32 = 0xEBAB;

pub struct Renderer {
    factory: ID2D1Factory,
    dwrite: IDWriteFactory,
    target: ID2D1DCRenderTarget,
    brush: ID2D1SolidColorBrush,
    /// Ujung garis membulat, untuk cincin progres.
    round: ID2D1StrokeStyle,
    /// Garis putus-putus, untuk area lepas file.
    dashed: ID2D1StrokeStyle,
    fonts: Fonts,
    surface: Surface,
    dpi: u32,
    pal: Palette,
    /// Sampul album yang sedang dipakai, beserta id-nya supaya tidak dibuat ulang.
    art: Option<(u64, ID2D1BitmapBrush)>,
    /// Thumbnail shelf per id item: kuas, lebar, tinggi.
    thumbs: HashMap<u64, (ID2D1BitmapBrush, f32, f32)>,
    /// Logo aplikasi chat per AppUserModelID.
    logos: HashMap<String, ID2D1BitmapBrush>,
    /// Area yang bisa diklik dari frame penuh terakhir.
    hits: RefCell<Vec<(D2D_RECT_F, Hit)>>,
    /// Hanya mencatat area klik saat panel sudah diam di tempatnya.
    recording: Cell<bool>,
}

struct Fonts {
    pill_time: IDWriteTextFormat,
    greeting: IDWriteTextFormat,
    big_time: IDWriteTextFormat,
    date: IDWriteTextFormat,
    big_num: IDWriteTextFormat,
    caption: IDWriteTextFormat,
    caption_end: IDWriteTextFormat,
    caption_center: IDWriteTextFormat,
    tiny: IDWriteTextFormat,
    header: IDWriteTextFormat,
    title: IDWriteTextFormat,
    body: IDWriteTextFormat,
    /// Seperti `body` tapi tanpa "…", untuk teks yang sedang diketik.
    input: IDWriteTextFormat,
    /// Baris lirik di pil (rata tengah, dipotong "…").
    lyric: IDWriteTextFormat,
    /// Lirik dua baris di pil, dan lirik di halaman musik (rata kiri).
    lyric_wrap: IDWriteTextFormat,
    lyric_block: IDWriteTextFormat,
    label: IDWriteTextFormat,
    label_end: IDWriteTextFormat,
    label_center: IDWriteTextFormat,
    icon_9: IDWriteTextFormat,
    icon_11: IDWriteTextFormat,
    icon_13: IDWriteTextFormat,
    icon_16: IDWriteTextFormat,
    icon_20: IDWriteTextFormat,
    icon_22: IDWriteTextFormat,
}

impl Fonts {
    fn new(dwrite: &IDWriteFactory) -> Result<Self> {
        let display = w!("Segoe UI Variable Display");
        let text = w!("Segoe UI Variable Text");
        let icons = icon_family(dwrite);
        let (lead, center, end) =
            (DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_TRAILING);
        let middle = DWRITE_PARAGRAPH_ALIGNMENT_CENTER;
        let (normal, semi) = (DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD);
        let f = |family, weight, size, align, trim| text_format(dwrite, family, weight, size, align, middle, trim);
        let wrapped = |size, align| -> Result<IDWriteTextFormat> {
            let format = text_format(dwrite, text, semi, size, align, middle, true)?;
            unsafe { format.SetWordWrapping(DWRITE_WORD_WRAPPING_WRAP)? };
            Ok(format)
        };
        let icon = |size| text_format(dwrite, icons, normal, size, center, middle, false);
        Ok(Self {
            pill_time: f(display, semi, 13.0, center, false)?,
            greeting: f(text, normal, 12.0, lead, false)?,
            big_time: f(display, semi, 40.0, lead, false)?,
            date: f(text, normal, 13.0, lead, false)?,
            big_num: f(display, semi, 34.0, lead, false)?,
            caption: f(text, normal, 11.5, lead, true)?,
            caption_end: f(text, normal, 11.5, end, true)?,
            caption_center: f(text, normal, 11.0, center, true)?,
            tiny: f(text, normal, 9.5, center, true)?,
            header: f(text, semi, 12.5, lead, true)?,
            title: f(text, semi, 15.0, lead, true)?,
            body: f(text, normal, 12.5, lead, true)?,
            input: f(text, normal, 12.5, lead, false)?,
            lyric: f(text, semi, 12.5, center, true)?,
            lyric_wrap: wrapped(12.0, center)?,
            lyric_block: wrapped(12.0, lead)?,
            label: f(text, semi, 13.0, lead, true)?,
            label_end: f(display, semi, 13.0, end, false)?,
            label_center: f(text, semi, 12.0, center, false)?,
            icon_9: icon(9.0)?,
            icon_11: icon(11.0)?,
            icon_13: icon(13.0)?,
            icon_16: icon(16.0)?,
            icon_20: icon(20.0)?,
            icon_22: icon(22.0)?,
        })
    }
}

impl Renderer {
    pub fn new(dpi: u32, theme: Theme) -> Result<Self> {
        unsafe {
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let target = dc_target(&factory, dpi as f32)?;
            let brush = target.CreateSolidColorBrush(&WHITE, None)?;
            let stroke = |dash_style, dashes: Option<&[f32]>| {
                let props = D2D1_STROKE_STYLE_PROPERTIES {
                    startCap: D2D1_CAP_STYLE_ROUND,
                    endCap: D2D1_CAP_STYLE_ROUND,
                    dashCap: D2D1_CAP_STYLE_ROUND,
                    lineJoin: D2D1_LINE_JOIN_ROUND,
                    miterLimit: 10.0,
                    dashStyle: dash_style,
                    dashOffset: 0.0,
                };
                factory.CreateStrokeStyle(&props, dashes)
            };
            let round = stroke(D2D1_DASH_STYLE_SOLID, None)?;
            let dashed = stroke(D2D1_DASH_STYLE_CUSTOM, Some(&[3.0, 3.0]))?;
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let (width, height) = canvas_px(dpi);
            Ok(Self {
                fonts: Fonts::new(&dwrite)?,
                factory,
                dwrite,
                target,
                brush,
                round,
                dashed,
                surface: Surface::new(width, height)?,
                dpi,
                pal: palette(theme),
                art: None,
                thumbs: HashMap::new(),
                logos: HashMap::new(),
                hits: RefCell::new(Vec::new()),
                recording: Cell::new(false),
            })
        }
    }

    pub fn set_dpi(&mut self, dpi: u32) -> Result<()> {
        if dpi == self.dpi {
            return Ok(());
        }
        let (width, height) = canvas_px(dpi);
        self.surface = Surface::new(width, height)?;
        unsafe { self.target.SetDpi(dpi as f32, dpi as f32) };
        self.dpi = dpi;
        Ok(())
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.pal = palette(theme);
    }

    /// Ukuran jendela dalam piksel fisik.
    pub fn size(&self) -> SIZE {
        SIZE { cx: self.surface.width, cy: self.surface.height }
    }

    /// Piksel fisik per DIP.
    pub fn scale(&self) -> f32 {
        self.dpi as f32 / 96.0
    }

    /// Bagian panel di titik (DIP, koordinat kanvas), dari frame penuh terakhir.
    pub fn hit_at(&self, x: f32, y: f32) -> Option<Hit> {
        self.hits
            .borrow()
            .iter()
            .rev()
            .find(|(r, _)| x >= r.left && x < r.right && y >= r.top && y < r.bottom)
            .map(|(_, hit)| *hit)
    }

    /// Ganti sampul album. Bitmap hanya dibuat ulang kalau sampulnya memang baru.
    pub fn set_art(&mut self, art: Option<&Art>) {
        let Some(art) = art else {
            self.art = None;
            return;
        };
        if self.art.as_ref().is_some_and(|(id, _)| *id == art.id) {
            return;
        }
        self.art = match self.bitmap_brush(ART_PX, ART_PX, &art.pixels) {
            Ok(brush) => Some((art.id, brush)),
            Err(e) => {
                debug_log!("sampul gagal dibuat: {e}");
                None
            }
        };
    }

    pub fn set_thumbnail(&mut self, thumb: &Thumbnail) {
        match self.bitmap_brush(thumb.width, thumb.height, &thumb.pixels) {
            Ok(brush) => {
                self.thumbs.insert(thumb.id, (brush, thumb.width as f32, thumb.height as f32));
            }
            Err(e) => debug_log!("thumbnail gagal dibuat: {e}"),
        }
    }

    /// Lepas thumbnail item yang sudah tidak ada di shelf.
    pub fn retain_thumbnails(&mut self, keep: impl Fn(u64) -> bool) {
        self.thumbs.retain(|id, _| keep(*id));
    }

    /// Simpan logo aplikasi chat (sekali per aplikasi).
    pub fn set_logo(&mut self, aumid: &str, size: u32, pixels: &[u8]) {
        if self.logos.contains_key(aumid) {
            return;
        }
        match self.bitmap_brush(size, size, pixels) {
            Ok(brush) => {
                self.logos.insert(aumid.to_string(), brush);
            }
            Err(e) => debug_log!("logo aplikasi gagal dibuat: {e}"),
        }
    }

    /// Ukuran pil lirik untuk satu baris: melebar mengikuti teks, dibulatkan per 24 DIP supaya
    /// tidak berubah-ubah sedikit tiap baris; terlalu panjang → lebar maksimum dua baris.
    pub fn lyric_size(&self, line: &str) -> (f32, f32) {
        let needed = self.text_width(line, &self.fonts.lyric) + LYRIC_SIDES;
        if needed <= LYRIC_MAX_W {
            let width = (needed.max(LYRIC_MIN_W) / 24.0).ceil() * 24.0;
            (width.min(LYRIC_MAX_W), View::LyricPill.size().1)
        } else {
            (LYRIC_MAX_W, LYRIC_TWO_LINES_H)
        }
    }

    fn bitmap_brush(&self, width: u32, height: u32, pixels: &[u8]) -> Result<ID2D1BitmapBrush> {
        let props = D2D1_BITMAP_PROPERTIES {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
        };
        unsafe {
            let bitmap = self.target.CreateBitmap(
                D2D_SIZE_U { width, height },
                Some(pixels.as_ptr().cast()),
                width * 4,
                &props,
            )?;
            self.target.CreateBitmapBrush(&bitmap, None, None)
        }
    }

    pub fn draw(&self, scene: &Scene) -> Result<()> {
        let cx = CANVAS.0 / 2.0;
        let (w, h) = (scene.width.max(24.0), scene.height.max(4.0));
        let grow = ((h - 30.0) / 98.0).clamp(0.0, 1.0);
        let flare = lerp(FLARE_MIN, FLARE_MAX, grow).min(h * 0.45);
        let radius = (h * 0.36).min(26.0).min(h - flare).min(w / 2.0 - 1.0).max(0.0);
        let (x0, x1) = (cx - w / 2.0, cx + w / 2.0);
        let shape = self.notch_path(cx, w, h, radius, flare)?;
        let [pill, lyric, hud, panel] = scene.alpha;
        self.hits.borrow_mut().clear();
        self.recording.set(panel >= 0.999 && scene.page_from.is_none());

        unsafe {
            let bounds = RECT { left: 0, top: 0, right: self.surface.width, bottom: self.surface.height };
            self.target.BindDC(self.surface.dc, &bounds)?;
            self.target.BeginDraw();
            self.target.Clear(Some(&CLEAR));
            self.brush.SetColor(&self.pal.body);
            self.target.FillGeometry(&shape, &self.brush, None::<&ID2D1Brush>);
            if self.pal.glass {
                self.brush.SetColor(&rgb(0xFFFFFF, 0.24));
                self.target.DrawGeometry(&shape, &self.brush, 1.0, None::<&ID2D1StrokeStyle>);
            }

            // Isi dipotong ke badan notch supaya tidak pernah tergambar di luar bentuknya.
            self.target.PushAxisAlignedClip(&rect(x0, 0.0, x1, h), D2D1_ANTIALIAS_MODE_ALIASED);
            if pill > 0.01 {
                self.draw_pill(scene, x0, x1, h, pill);
            }
            if lyric > 0.01 {
                self.draw_lyric_pill(scene, x0, x1, h, lyric);
            }
            if hud > 0.01
                && let Some(data) = scene.hud
            {
                self.draw_hud(data, x0, x1, hud);
            }
            if panel > 0.01 {
                self.draw_panel(scene, x0, x1, panel);
            }
            self.target.PopAxisAlignedClip();
            self.target.EndDraw(None, None)
        }
    }

    /// Gambar ulang hanya gelombang musik di pil (biasa atau lirik, berukuran `w`×`h`) yang sedang
    /// diam. Dipanggil ~12 kali per detik selama musik diputar, jadi cukup menyentuh kotak kecil
    /// ini, bukan seluruh kanvas. Mengembalikan area (piksel) yang berubah untuk [`Renderer::present`].
    pub fn draw_wave(&self, w: f32, h: f32, seconds: f64, color: Option<u32>) -> Result<RECT> {
        let (cx, cy) = (CANVAS.0 / 2.0 + w / 2.0 - 22.0, h / 2.0);
        self.patch(cx, cy, 10.0, 8.0, || self.wave(cx, cy, seconds, accent(color, self.pal.text), 1.0))
    }

    /// Gambar ulang hanya mata maskot di pil yang sedang diam, untuk animasi berkedip.
    pub fn draw_eyes(&self, open: f32) -> Result<RECT> {
        let (w, h) = View::Pill.size();
        let (cx, cy) = (CANVAS.0 / 2.0 - w / 2.0 + 18.0, h / 2.0);
        self.patch(cx, cy, 7.0, 6.0, || self.eyes(cx, cy, 3.4, 9.0, 7.5, open, 1.0))
    }

    /// Gambar ulang kotak kecil berpusat di (`cx`, `cy`) yang seluruhnya berada di dalam badan
    /// notch: dihapus dengan warna badan lalu diisi `paint`. Mengembalikan area pikselnya.
    fn patch(&self, cx: f32, cy: f32, half_w: f32, half_h: f32, paint: impl FnOnce()) -> Result<RECT> {
        let s = self.scale();
        let dirty = RECT {
            left: ((cx - half_w) * s).floor() as i32,
            top: ((cy - half_h) * s).floor() as i32,
            right: ((cx + half_w) * s).ceil() as i32,
            bottom: ((cy + half_h) * s).ceil() as i32,
        };
        let (dx, dy) = (dirty.left as f32 / s, dirty.top as f32 / s);
        unsafe {
            self.target.BindDC(self.surface.dc, &dirty)?;
            self.target.BeginDraw();
            self.target.SetTransform(&translation(-dx, -dy));
            self.target.Clear(Some(&self.pal.body));
            paint();
            self.target.SetTransform(&translation(0.0, 0.0));
            self.target.EndDraw(None, None)?;
        }
        Ok(dirty)
    }

    /// Tempel bitmap ke jendela. Posisi dan ukuran ikut diatur sekaligus; `dirty` membatasi
    /// salinan ke area yang berubah saja.
    pub fn present(&self, hwnd: HWND, pos: POINT, dirty: Option<&RECT>) -> Result<()> {
        let size = self.size();
        let source = POINT::default();
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let info = UPDATELAYEREDWINDOWINFO {
            cbSize: size_of::<UPDATELAYEREDWINDOWINFO>() as u32,
            hdcDst: HDC::default(),
            pptDst: &pos,
            psize: &size,
            hdcSrc: self.surface.dc,
            pptSrc: &source,
            crKey: COLORREF(0),
            pblend: &blend,
            dwFlags: ULW_ALPHA,
            prcDirty: dirty.map_or(std::ptr::null(), |r| r as *const RECT),
        };
        unsafe { UpdateLayeredWindowIndirect(hwnd, &info) }.ok()
    }

    /// Badan notch: atas rata menempel ke tepi layar, sudut bawah membulat, dan
    /// lengkung cekung di kiri-kanan atas seperti notch MacBook.
    fn notch_path(&self, cx: f32, w: f32, h: f32, radius: f32, flare: f32) -> Result<ID2D1PathGeometry> {
        let (x0, x1) = (cx - w / 2.0, cx + w / 2.0);
        unsafe {
            let geometry = self.factory.CreatePathGeometry()?;
            let sink = geometry.Open()?;
            sink.BeginFigure(pt(x0 - flare, 0.0), D2D1_FIGURE_BEGIN_FILLED);
            sink.AddArc(&arc(x0, flare, flare, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddLine(pt(x0, h - radius));
            sink.AddArc(&arc(x0 + radius, h, radius, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddLine(pt(x1 - radius, h));
            sink.AddArc(&arc(x1, h - radius, radius, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddLine(pt(x1, flare));
            sink.AddArc(&arc(x1 + flare, 0.0, flare, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close()?;
            Ok(geometry)
        }
    }

    /// Warna teks utama yang ditipiskan, untuk sorotan dan jalur bilah. Berlaku untuk tema
    /// gelap (putih tipis) maupun terang (hitam tipis).
    fn tint(&self, alpha: f32) -> D2D1_COLOR_F {
        with_alpha(self.pal.text, alpha)
    }

    /// Catat area yang bisa diklik (hanya saat panel diam, lihat `recording`).
    fn hit(&self, area: D2D_RECT_F, hit: Hit) {
        if self.recording.get() {
            self.hits.borrow_mut().push((area, hit));
        }
    }

    /// Sepasang mata kapsul; `open` mengecilkan tingginya saat berkedip.
    fn eyes(&self, cx: f32, cy: f32, eye_w: f32, eye_h: f32, gap: f32, open: f32, alpha: f32) {
        let h = (eye_h * open).max(eye_w * 0.45);
        for x in [cx - gap / 2.0, cx + gap / 2.0] {
            self.rounded(rect(x - eye_w / 2.0, cy - h / 2.0, x + eye_w / 2.0, cy + h / 2.0), eye_w.min(h) / 2.0, self.pal.eye, alpha);
        }
    }

    /// Sampul album bersudut bulat; kotak dengan not balok kalau belum ada.
    fn art(&self, x: f32, y: f32, size: f32, radius: f32, alpha: f32) {
        let area = rect(x, y, x + size, y + size);
        match &self.art {
            Some((_, brush)) => self.fill_bitmap(brush, area, size / ART_PX as f32, radius, alpha),
            None => {
                self.rounded(area, radius, self.pal.placeholder, alpha);
                let font = if size > 30.0 { &self.fonts.icon_22 } else { &self.fonts.icon_13 };
                self.glyph(ICON_MUSIC, font, x + size / 2.0, y + size / 2.0, self.pal.muted, alpha);
            }
        }
    }

    /// Logo aplikasi chat bersudut bulat; ikon pesan kalau logonya belum ada.
    fn logo(&self, aumid: &str, cx: f32, cy: f32, size: f32, alpha: f32) {
        let area = rect(cx - size / 2.0, cy - size / 2.0, cx + size / 2.0, cy + size / 2.0);
        match self.logos.get(aumid) {
            Some(brush) => self.fill_bitmap(brush, area, size / crate::messages::LOGO_PX as f32, size * 0.24, alpha),
            None => {
                self.circle(cx, cy, size / 2.0, GREEN, alpha);
                self.glyph(ICON_MESSAGE, &self.fonts.icon_13, cx, cy, WHITE, alpha);
            }
        }
    }

    /// Thumbnail file di dalam kotak `size`, rasio aslinya dipertahankan.
    fn thumb(&self, id: u64, is_dir: bool, cx: f32, top: f32, size: f32, alpha: f32) {
        match self.thumbs.get(&id) {
            Some((brush, w, h)) => {
                let k = (size / w).min(size / h);
                let (dw, dh) = (w * k, h * k);
                let (x, y) = (cx - dw / 2.0, top + (size - dh) / 2.0);
                self.fill_bitmap(brush, rect(x, y, x + dw, y + dh), k, 6.0, alpha);
            }
            None => {
                let icon = if is_dir { ICON_FOLDER } else { ICON_DOCUMENT };
                self.glyph(icon, &self.fonts.icon_22, cx, top + size / 2.0, self.pal.soft, alpha);
            }
        }
    }

    fn fill_bitmap(&self, brush: &ID2D1BitmapBrush, area: D2D_RECT_F, scale: f32, radius: f32, alpha: f32) {
        unsafe {
            brush.SetTransform(&Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: area.left, M32: area.top });
            brush.SetOpacity(alpha);
            self.target.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: area, radiusX: radius, radiusY: radius }, brush);
        }
    }

    /// Empat batang yang naik-turun selama musik diputar.
    fn wave(&self, cx: f32, cy: f32, seconds: f64, color: D2D1_COLOR_F, alpha: f32) {
        const SPEED: [f64; 4] = [7.3, 9.1, 6.4, 8.2];
        const PHASE: [f64; 4] = [0.0, 1.9, 3.3, 4.6];
        for i in 0..4 {
            let level = (0.5 + 0.5 * (seconds * SPEED[i] + PHASE[i]).sin()) as f32;
            let (x, h) = (cx - 7.5 + i as f32 * 5.0, 3.0 + 9.0 * level);
            self.rounded(rect(x - 1.3, cy - h / 2.0, x + 1.3, cy + h / 2.0), 1.3, color, alpha);
        }
    }

    /// Titik privasi: hijau kalau kamera dipakai, oranye kalau hanya mikrofon.
    fn privacy_dot(&self, privacy: &Usage, x: f32, y: f32, alpha: f32) {
        if privacy.active() {
            let color = if privacy.camera.is_empty() { ORANGE } else { GREEN };
            self.circle(x, y, 3.0, color, alpha);
        }
    }

    /// Bilah progres/volume: jalur redup dengan isi sepanjang `fraction`.
    fn bar(&self, x0: f32, x1: f32, cy: f32, fraction: f32, color: D2D1_COLOR_F, alpha: f32) {
        self.rounded(rect(x0, cy - 2.0, x1, cy + 2.0), 2.0, self.tint(0.22), alpha);
        let fill = x0 + (x1 - x0) * fraction.clamp(0.0, 1.0);
        if fill > x0 + 0.5 {
            self.rounded(rect(x0, cy - 2.0, fill.max(x0 + 4.0), cy + 2.0), 2.0, color, alpha);
        }
    }

    /// Cincin progres yang terisi searah jarum jam dari atas.
    fn ring(&self, cx: f32, cy: f32, radius: f32, thickness: f32, progress: f32, color: D2D1_COLOR_F, alpha: f32) {
        self.circle_outline(cx, cy, radius, thickness, self.tint(0.22), alpha);
        let p = progress.clamp(0.0, 1.0);
        if p >= 0.999 {
            self.circle_outline(cx, cy, radius, thickness, color, alpha);
            return;
        }
        if p <= 0.001 {
            return;
        }
        let angle = p * TAU - FRAC_PI_2;
        let size = if p > 0.5 { D2D1_ARC_SIZE_LARGE } else { D2D1_ARC_SIZE_SMALL };
        let end = (cx + radius * angle.cos(), cy + radius * angle.sin());
        unsafe {
            let Ok(geometry) = self.factory.CreatePathGeometry() else { return };
            let Ok(sink) = geometry.Open() else { return };
            sink.BeginFigure(pt(cx, cy - radius), D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddArc(&arc(end.0, end.1, radius, D2D1_SWEEP_DIRECTION_CLOCKWISE, size));
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            if sink.Close().is_ok() {
                self.brush.SetColor(&with_alpha(color, alpha));
                self.target.DrawGeometry(&geometry, &self.brush, thickness, &self.round);
            }
        }
    }

    /// Ikon tetes air.
    fn drop_icon(&self, cx: f32, cy: f32, size: f32, color: D2D1_COLOR_F, alpha: f32) {
        let r = size * 0.3;
        let (bx, by) = (cx, cy + size * 0.16);
        let top = pt(cx, cy - size * 0.5);
        unsafe {
            let Ok(geometry) = self.factory.CreatePathGeometry() else { return };
            let Ok(sink) = geometry.Open() else { return };
            sink.BeginFigure(top, D2D1_FIGURE_BEGIN_FILLED);
            sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                point1: pt(cx + size * 0.06, cy - size * 0.3),
                point2: pt(bx + r, by - r),
                point3: pt(bx + r, by),
            });
            sink.AddArc(&arc(bx, by + r, r, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddArc(&arc(bx - r, by, r, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddBezier(&D2D1_BEZIER_SEGMENT {
                point1: pt(bx - r, by - r),
                point2: pt(cx - size * 0.06, cy - size * 0.3),
                point3: top,
            });
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            if sink.Close().is_ok() {
                self.brush.SetColor(&with_alpha(color, alpha));
                self.target.FillGeometry(&geometry, &self.brush, None::<&ID2D1Brush>);
            }
        }
    }

    /// Tombol kapsul berisi teks, misal "+ Minum 250 ml".
    fn chip(&self, area: D2D_RECT_F, text: &str, fill: Option<D2D1_COLOR_F>, color: D2D1_COLOR_F, alpha: f32) {
        if let Some(fill) = fill {
            self.rounded(area, (area.bottom - area.top) / 2.0, fill, alpha);
        }
        self.text(text, &self.fonts.label_center, area, color, alpha);
    }

    fn rounded(&self, area: D2D_RECT_F, radius: f32, color: D2D1_COLOR_F, alpha: f32) {
        unsafe {
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT { rect: area, radiusX: radius, radiusY: radius },
                &self.brush,
            );
        }
    }

    fn outline(&self, area: D2D_RECT_F, radius: f32, width: f32, dashed: bool, color: D2D1_COLOR_F, alpha: f32) {
        let style = if dashed { &self.dashed } else { &self.round };
        unsafe {
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.DrawRoundedRectangle(
                &D2D1_ROUNDED_RECT { rect: area, radiusX: radius, radiusY: radius },
                &self.brush,
                width,
                style,
            );
        }
    }

    fn circle(&self, cx: f32, cy: f32, radius: f32, color: D2D1_COLOR_F, alpha: f32) {
        unsafe {
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.FillEllipse(
                &D2D1_ELLIPSE { point: pt(cx, cy), radiusX: radius, radiusY: radius },
                &self.brush,
            );
        }
    }

    fn circle_outline(&self, cx: f32, cy: f32, radius: f32, width: f32, color: D2D1_COLOR_F, alpha: f32) {
        unsafe {
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.DrawEllipse(
                &D2D1_ELLIPSE { point: pt(cx, cy), radiusX: radius, radiusY: radius },
                &self.brush,
                width,
                None::<&ID2D1StrokeStyle>,
            );
        }
    }

    fn glyph(&self, icon: char, format: &IDWriteTextFormat, cx: f32, cy: f32, color: D2D1_COLOR_F, alpha: f32) {
        let mut buf = [0u16; 2];
        let encoded = icon.encode_utf16(&mut buf);
        self.text_utf16(encoded, format, rect(cx - 16.0, cy - 16.0, cx + 16.0, cy + 16.0), color, alpha);
    }

    fn text(&self, s: &str, format: &IDWriteTextFormat, area: D2D_RECT_F, color: D2D1_COLOR_F, alpha: f32) {
        let wide: Vec<u16> = s.encode_utf16().collect();
        self.text_utf16(&wide, format, area, color, alpha);
    }

    fn text_utf16(&self, s: &[u16], format: &IDWriteTextFormat, area: D2D_RECT_F, color: D2D1_COLOR_F, alpha: f32) {
        unsafe {
            if self.pal.glass {
                // Bayangan tipis supaya teks tetap terbaca di atas wallpaper terang.
                let shadow = rect(area.left, area.top + 0.8, area.right, area.bottom + 0.8);
                self.brush.SetColor(&rgb(0x000000, 0.45 * alpha));
                self.target.DrawText(s, format, &shadow, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
            }
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.DrawText(
                s,
                format,
                &area,
                &self.brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
    }

    /// Teks dicoret, untuk tugas yang sudah selesai.
    fn text_struck(&self, s: &str, format: &IDWriteTextFormat, area: D2D_RECT_F, color: D2D1_COLOR_F, alpha: f32) {
        let wide: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            let Ok(layout) = self.dwrite.CreateTextLayout(&wide, format, area.right - area.left, area.bottom - area.top)
            else {
                return;
            };
            let _ = layout.SetStrikethrough(true, DWRITE_TEXT_RANGE { startPosition: 0, length: wide.len() as u32 });
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.DrawTextLayout(pt(area.left, area.top), &layout, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
        }
    }

    /// Lebar teks dalam DIP.
    fn text_width(&self, s: &str, format: &IDWriteTextFormat) -> f32 {
        let wide: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            let Ok(layout) = self.dwrite.CreateTextLayout(&wide, format, 10_000.0, 100.0) else { return 0.0 };
            let mut metrics = DWRITE_TEXT_METRICS::default();
            match layout.GetMetrics(&mut metrics) {
                Ok(()) => metrics.widthIncludingTrailingWhitespace,
                Err(_) => 0.0,
            }
        }
    }

    fn set_offset(&self, dx: f32, dy: f32) {
        unsafe { self.target.SetTransform(&translation(dx, dy)) };
    }

    fn push_clip(&self, area: D2D_RECT_F) {
        unsafe { self.target.PushAxisAlignedClip(&area, D2D1_ANTIALIAS_MODE_ALIASED) };
    }

    fn pop_clip(&self) {
        unsafe { self.target.PopAxisAlignedClip() };
    }
}

/// Ikon tray: kotak gelap dengan sepasang mata, maskot yang sama dengan di notch.
pub fn make_icon(size: i32) -> Result<HICON> {
    let s = size as f32;
    unsafe {
        let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
        let target = dc_target(&factory, 96.0)?;
        let mut surface = Surface::new(size, size)?;
        target.BindDC(surface.dc, &RECT { left: 0, top: 0, right: size, bottom: size })?;
        target.BeginDraw();
        target.Clear(Some(&CLEAR));
        let brush = target.CreateSolidColorBrush(&ICON_BG, None)?;
        let corner = s * 0.26;
        target.FillRoundedRectangle(
            &D2D1_ROUNDED_RECT { rect: rect(0.0, 0.0, s, s), radiusX: corner, radiusY: corner },
            &brush,
        );
        brush.SetColor(&ICON_EYE);
        let (eye_w, eye_h, gap) = (s * 0.17, s * 0.40, s * 0.30);
        for x in [s / 2.0 - gap / 2.0, s / 2.0 + gap / 2.0] {
            target.FillRoundedRectangle(
                &D2D1_ROUNDED_RECT {
                    rect: rect(x - eye_w / 2.0, s / 2.0 - eye_h / 2.0, x + eye_w / 2.0, s / 2.0 + eye_h / 2.0),
                    radiusX: eye_w / 2.0,
                    radiusY: eye_w / 2.0,
                },
                &brush,
            );
        }
        target.EndDraw(None, None)?;
        let _ = GdiFlush();

        // Ikon Windows memakai alfa lurus, sedangkan Direct2D menghasilkan alfa premultiplied.
        for px in surface.pixels() {
            let a = *px >> 24;
            if a != 0 && a != 255 {
                let un = |c: u32| ((c * 255 + a / 2) / a).min(255);
                *px = (a << 24) | (un((*px >> 16) & 0xFF) << 16) | (un((*px >> 8) & 0xFF) << 8) | un(*px & 0xFF);
            }
        }

        // Bitmap harus dilepas dari DC sebelum dipakai membuat ikon.
        SelectObject(surface.dc, surface.old);
        let mask = CreateBitmap(size, size, 1, 1, None);
        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: surface.bitmap,
        };
        let icon = CreateIconIndirect(&info);
        let _ = DeleteObject(mask.into());
        icon
    }
}

/// Bitmap 32-bit (BGRA) di memori yang menjadi kanvas gambar jendela.
struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    width: i32,
    height: i32,
}

impl Surface {
    fn new(width: i32, height: i32) -> Result<Self> {
        unsafe {
            let dc = CreateCompatibleDC(None);
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height, // negatif = baris pertama di atas
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(bitmap) => bitmap,
                Err(e) => {
                    let _ = DeleteDC(dc);
                    return Err(e);
                }
            };
            let old = SelectObject(dc, bitmap.into());
            Ok(Self { dc, bitmap, old, bits: bits.cast(), width, height })
        }
    }

    fn pixels(&mut self) -> &mut [u32] {
        unsafe { std::slice::from_raw_parts_mut(self.bits, (self.width * self.height) as usize) }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}

fn dc_target(factory: &ID2D1Factory, dpi: f32) -> Result<ID2D1DCRenderTarget> {
    let props = D2D1_RENDER_TARGET_PROPERTIES {
        // Software: tidak memuat driver GPU, cukup untuk bentuk sekecil ini.
        r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: dpi,
        dpiY: dpi,
        usage: D2D1_RENDER_TARGET_USAGE_NONE,
        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
    };
    unsafe {
        let target = factory.CreateDCRenderTarget(&props)?;
        // ClearType tidak bisa dipakai di atas latar transparan.
        target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
        Ok(target)
    }
}

/// Windows 11 punya Segoe Fluent Icons; Windows 10 memakai Segoe MDL2 Assets dengan kode yang sama.
fn icon_family(dwrite: &IDWriteFactory) -> PCWSTR {
    let fluent = w!("Segoe Fluent Icons");
    unsafe {
        let mut collection = None;
        if dwrite.GetSystemFontCollection(&mut collection, false).is_ok()
            && let Some(collection) = collection
        {
            let (mut index, mut exists) = (0u32, BOOL(0));
            if collection.FindFamilyName(fluent, &mut index, &mut exists).is_ok() && exists.as_bool() {
                return fluent;
            }
        }
    }
    w!("Segoe MDL2 Assets")
}

fn text_format(
    dwrite: &IDWriteFactory,
    family: PCWSTR,
    weight: DWRITE_FONT_WEIGHT,
    size: f32,
    align: DWRITE_TEXT_ALIGNMENT,
    paragraph: DWRITE_PARAGRAPH_ALIGNMENT,
    trim: bool,
) -> Result<IDWriteTextFormat> {
    unsafe {
        let format = dwrite.CreateTextFormat(
            family,
            None,
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            w!("id-ID"),
        )?;
        format.SetTextAlignment(align)?;
        format.SetParagraphAlignment(paragraph)?;
        format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
        if trim {
            // Teks yang terlalu panjang dipotong dengan "…".
            let sign = dwrite.CreateEllipsisTrimmingSign(&format)?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            format.SetTrimming(&trimming, &sign)?;
        }
        Ok(format)
    }
}

fn canvas_px(dpi: u32) -> (i32, i32) {
    let scale = dpi as f32 / 96.0;
    ((CANVAS.0 * scale).ceil() as i32, (CANVAS.1 * scale).ceil() as i32)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn translation(x: f32, y: f32) -> Matrix3x2 {
    Matrix3x2 { M11: 1.0, M12: 0.0, M21: 0.0, M22: 1.0, M31: x, M32: y }
}

fn pt(x: f32, y: f32) -> Vector2 {
    Vector2 { X: x, Y: y }
}

fn rect(left: f32, top: f32, right: f32, bottom: f32) -> D2D_RECT_F {
    D2D_RECT_F { left, top, right, bottom }
}

fn arc(x: f32, y: f32, radius: f32, sweep: D2D1_SWEEP_DIRECTION, size: D2D1_ARC_SIZE) -> D2D1_ARC_SEGMENT {
    D2D1_ARC_SEGMENT {
        point: pt(x, y),
        size: D2D_SIZE_F { width: radius, height: radius },
        rotationAngle: 0.0,
        sweepDirection: sweep,
        arcSize: size,
    }
}

const fn rgb(hex: u32, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a,
    }
}

fn with_alpha(color: D2D1_COLOR_F, alpha: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F { a: color.a * alpha, ..color }
}

/// Warna aksen sampul album, atau `fallback` kalau sampulnya tidak berwarna.
fn accent(color: Option<u32>, fallback: D2D1_COLOR_F) -> D2D1_COLOR_F {
    color.map_or(fallback, |hex| rgb(hex, 1.0))
}

fn phase_color(phase: Phase) -> D2D1_COLOR_F {
    match phase {
        Phase::Focus => RED,
        Phase::Break => GREEN,
    }
}

/// Warna beban: aksen saat santai, oranye saat sibuk, merah saat hampir penuh.
fn load_color(percent: f32, calm: D2D1_COLOR_F) -> D2D1_COLOR_F {
    if percent >= 85.0 {
        RED
    } else if percent >= 60.0 {
        ORANGE
    } else {
        calm
    }
}

#[cfg(test)]
mod tests;
