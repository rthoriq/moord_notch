//! Paket aset untuk video promosi: gambar PNG transparan resolusi 4× dari setiap tampilan
//! notch, dan urutan frame 60 fps untuk animasinya (buka, tutup, kolom lirik, lirik bergulir,
//! kabar volume, dan mata maskot). Lagu dan liriknya contoh karangan sendiri.
//!
//! Jalankan dengan `cargo test aset_video -- --ignored`. Hasilnya di folder `aset-video`.

use std::path::{Path, PathBuf};

use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatPng, GUID_WICPixelFormat32bppBGRA, IWICImagingFactory,
    WICBitmapEncoderNoCache,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::System::Com::StructuredStorage::IPropertyBag2;
use windows::core::HSTRING;

use super::*;
use crate::anim::Spring;

/// 4× skala layar 100%: tetap tajam walau diperbesar di video 1080×1920.
const DPI: u32 = 384;
pub(super) const FPS: f32 = 60.0;

/// Potongan gambar dalam piksel: kiri, atas, lebar, tinggi.
type Crop = (i32, i32, i32, i32);

/// Lagu contoh (karangan sendiri) untuk semua aset.
pub(super) fn song() -> MediaView {
    MediaView {
        title: "Langit Sore".into(),
        artist: "Ruang Senja".into(),
        playing: true,
        progress: Some(0.38),
        elapsed: "1:24".into(),
        remaining: "-2:17".into(),
        can_previous: true,
        can_next: true,
        can_seek: true,
        seeking: false,
        accent: Some(0x4FC3F7),
    }
}

pub(super) const LYRICS: [&str; 7] = [
    "Langit sore memudar pelan",
    "Kau tersenyum di ujung jalan",
    "Dan semua lagu terasa milik kita berdua malam ini",
    "Jangan pergi dulu, tunggu sebentar",
    "Bintang pun ikut bernyanyi",
    "Pelan-pelan saja, malam masih panjang",
    "Biar lampu kota yang bercerita",
];

/// Lima baris di sekitar baris ke-`current`, seperti yang dikirim app ke kolom lirik.
pub(super) fn lyrics_at(current: usize, scroll: f32) -> LyricsView {
    let line = |k: isize| {
        let i = current as isize + k - 2;
        if i >= 0 && (i as usize) < LYRICS.len() { LYRICS[i as usize].to_string() } else { String::new() }
    };
    LyricsView { lines: std::array::from_fn(|k| line(k as isize)), scroll, status: None, retry: false }
}

pub(super) fn idle_focus() -> FocusView {
    FocusView { running: false, active: false, ..sample().focus }
}

/// Adegan dasar tanpa pomodoro berjalan dan tanpa titik privasi.
pub(super) fn home<'a>(data: &'a Sample, idle: &'a FocusView, none: &'a Usage, view: View, page: Page, media: bool) -> Scene<'a> {
    let mut scene = scene_for(data, view, page, media);
    scene.focus = idle;
    scene.privacy = none;
    scene.shadow = if view.is_panel() { 1.0 } else { 0.0 };
    scene.battery = Some((81, false));
    scene.day = "Rab 21 Okt 2026";
    scene
}

#[test]
#[ignore]
fn aset_video() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("aset-video");
    let stills = root.join("gambar");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&stills).unwrap();
    export_stills(&stills);
    export_animations(&root.join("animasi"));
    std::fs::write(root.join("BACA-SAYA.txt"), README).unwrap();
}

/// Satu gambar: dirender, dipotong ke bentuk notch (plus bayangannya), lalu disimpan.
fn still(dir: &Path, name: &str, theme: Theme, scene: &Scene) {
    let mut renderer = Renderer::new(DPI, theme).unwrap();
    renderer.set_art(Some(&gradient_art()));
    renderer.draw(scene).unwrap();
    let _ = unsafe { GdiFlush() };
    let width = renderer.surface.width;
    let height = renderer.surface.height;
    let pixels = renderer.surface.pixels();
    let crop = visible_area(pixels, width, height, 12);
    save_png(&dir.join(format!("{name}.png")), pixels, width, crop);
}

fn export_stills(dir: &Path) {
    let mut data = sample();
    data.media = song();
    let idle = idle_focus();
    let none = Usage::default();
    let lyrics = lyrics_at(2, 1.0);
    let calendar = sample_calendar();
    let running = TimerView {
        running: true,
        active: true,
        clock: "03:42".into(),
        short: "4m".into(),
        progress: 0.26,
        total_minutes: 5,
        ending: false,
        fx: None,
    };

    // Tema hitam, tanpa titik privasi (kecuali gambar yang memang tentang itu).
    let base = |view: View, page: Page, media: bool| home(&data, &idle, &none, view, page, media);

    still(dir, "pil", Theme::Black, &base(View::Pill, Page::Home, false));
    still(dir, "pil-musik", Theme::Black, &base(View::Pill, Page::Home, true));
    let mut scene = base(View::LyricPill, Page::Home, true);
    scene.lyric = Some(LYRICS[0]);
    (scene.width, scene.height) = Renderer::new(96, Theme::Black).unwrap().lyric_size(LYRICS[0]);
    still(dir, "pil-lirik", Theme::Black, &scene);
    still(dir, "garis-tipis", Theme::Black, &base(View::Slim, Page::Home, false));
    let mut scene = base(View::Slim, Page::Home, false);
    scene.privacy = &data.privacy;
    still(dir, "garis-tipis-mikrofon", Theme::Black, &scene);

    let mut scene = base(View::Panel, Page::Home, true);
    scene.lyrics_button = Some(false);
    still(dir, "pemutar", Theme::Black, &scene);
    let mut scene = base(View::PanelWide, Page::Home, true);
    scene.lyrics = Some(&lyrics);
    scene.lyric_mix = 1.0;
    scene.lyrics_button = Some(true);
    still(dir, "pemutar-lirik", Theme::Black, &scene);
    let mut scene = base(View::PanelWide, Page::Home, true);
    scene.calendar = Some(&calendar);
    scene.lyrics_button = Some(false);
    still(dir, "pemutar-kalender", Theme::Black, &scene);
    let mut scene = base(View::PanelWide, Page::Home, false);
    scene.calendar = Some(&calendar);
    scene.weather = Some("29° Cerah");
    still(dir, "jam-kalender", Theme::Black, &scene);
    let mut scene = base(View::Panel, Page::Home, false);
    scene.weather = Some("29° Cerah · Jakarta");
    still(dir, "jam", Theme::Black, &scene);

    still(dir, "kontrol", Theme::Black, &base(View::Panel, Page::Controls, false));
    let mut scene = base(View::Panel, Page::Focus, false);
    scene.focus = &data.focus;
    still(dir, "pomodoro", Theme::Black, &scene);
    let mut scene = base(View::Panel, Page::Timer, false);
    scene.timer = &running;
    still(dir, "timer", Theme::Black, &scene);
    still(dir, "minum", Theme::Black, &base(View::Panel, Page::Water, false));
    still(dir, "tugas", Theme::Black, &base(View::Panel, Page::Tasks, false));
    let mut scene = base(View::Panel, Page::System, false);
    scene.system = Some(&data.system);
    scene.uptime = "3 j 12 m";
    still(dir, "sistem", Theme::Black, &scene);
    let mut scene = base(View::Panel, Page::Home, false);
    scene.welcome = true;
    still(dir, "sambutan", Theme::Black, &scene);
    still(dir, "shelf", Theme::Black, &base(View::Panel, Page::Shelf, false));

    // Area lepas saat file diseret: simpan di shelf, dan zona Bagikan.
    let mut dragging = sample();
    dragging.shelf.drop_hover = true;
    for (name, fx) in [
        ("shelf-seret", ShelfFx { targeted: true, dash: 1.3, ..ShelfFx::default() }),
        ("shelf-bagikan", ShelfFx { share_targeted: true, dash: 2.2, ..ShelfFx::default() }),
    ] {
        let mut scene = scene_for(&dragging, View::Panel, Page::Shelf, false);
        scene.privacy = &none;
        scene.shadow = 1.0;
        scene.shelf_fx = &fx;
        still(dir, name, Theme::Black, &scene);
    }

    // Kabar singkat: ringkas setinggi pil, dan kartu dua baris.
    let huds: [(&str, View, Hud); 9] = [
        ("kabar-volume", View::Strip, Hud::Volume { level: 0.62, muted: false }),
        ("kabar-kecerahan", View::Strip, Hud::Brightness { level: 0.7 }),
        ("kabar-capslock", View::Strip, Hud::LockKey { caps: true, on: true }),
        ("kabar-mengisi-daya", View::Strip, Hud::Power { plugged: true, percent: Some(80), low: false }),
        (
            "kabar-pesan",
            View::Hud,
            Hud::Message {
                app: "WhatsApp".into(),
                aumid: String::new(),
                sender: "Rina".into(),
                text: "Nanti malam jadi nonton? Aku udah pesan tiketnya".into(),
                browser: false,
            },
        ),
        (
            "kabar-bluetooth",
            View::Hud,
            Hud::Bluetooth { id: String::new(), name: "Earbuds".into(), connected: true, battery: Some(80), kind: DeviceKind::Audio },
        ),
        ("kabar-unduhan", View::Hud, Hud::Download { path: PathBuf::new(), name: "Laporan Akhir.pdf".into(), more: 0 }),
        ("kabar-agenda", View::Hud, Hud::Agenda { text: "Rapat tim".into(), time: "09.00".into() }),
        ("kabar-wifi", View::Hud, Hud::Wifi { name: "Rumah 5G".into(), connected: true, internet: true }),
    ];
    for (name, view, hud) in &huds {
        let mut scene = base(*view, Page::Home, false);
        scene.hud = Some(hud);
        scene.hud_compact = *view == View::Strip;
        scene.hud_fill = match hud {
            Hud::Volume { level, .. } | Hud::Brightness { level } => *level,
            _ => 0.0,
        };
        still(dir, name, Theme::Black, &scene);
    }

    // Pemutar di keenam tema.
    for theme in Theme::ALL {
        let mut scene = base(View::Panel, Page::Home, true);
        scene.lyrics_button = Some(false);
        let name = format!("tema-{}", theme.label().to_lowercase().replace(' ', "-"));
        still(dir, &name, theme, &scene);
    }

    // Mata maskot dari dekat (8×), dari halaman jam.
    let moods = [
        ("mata-biasa", Eyes::default()),
        ("mata-melirik-kiri", Eyes { look: (-1.0, -0.2), ..Eyes::default() }),
        ("mata-melirik-kanan", Eyes { look: (1.0, 0.2), ..Eyes::default() }),
        ("mata-berkedip", Eyes { open: 0.12, ..Eyes::default() }),
        ("mata-senang", Eyes { mood: Mood::Happy, ..Eyes::default() }),
        ("mata-kaget", Eyes { mood: Mood::Surprised, ..Eyes::default() }),
        ("mata-mengantuk", Eyes { mood: Mood::Sleepy, ..Eyes::default() }),
    ];
    for (name, eyes) in moods {
        let mut scene = base(View::Panel, Page::Home, false);
        scene.eyes = eyes;
        let mut renderer = Renderer::new(EYES_DPI, Theme::Black).unwrap();
        renderer.draw(&scene).unwrap();
        let _ = unsafe { GdiFlush() };
        let width = renderer.surface.width;
        save_png(&dir.join(format!("{name}.png")), renderer.surface.pixels(), width, eyes_crop());
    }
}

/// Mata dari dekat dirender 8× supaya tetap tajam saat memenuhi layar.
const EYES_DPI: u32 = 768;

/// Kotak di sekitar mata di halaman jam (panel biasa), dalam piksel 8×.
fn eyes_crop() -> Crop {
    let scale = EYES_DPI as f32 / 96.0;
    let (cx, cy) = (CANVAS.0 / 2.0 + View::Panel.size().0 / 2.0 - 32.0 - 40.0, 76.0);
    let (half_w, half_h) = (46.0, 34.0);
    (((cx - half_w) * scale) as i32, ((cy - half_h) * scale) as i32, (2.0 * half_w * scale) as i32, (2.0 * half_h * scale) as i32)
}

/// Bentuk notch yang sedang bergerak, dengan aturan yang sama seperti di app: pegas untuk
/// ukurannya, isi baru muncul setelah bentuknya hampir sampai, dan bayangan menyusul pelan.
pub(super) struct Motion {
    width: Spring,
    height: Spring,
    from: (f32, f32),
    target: (f32, f32),
    alpha: [f32; CONTENTS],
    content: Option<usize>,
    shadow: f32,
    pub(super) shadow_goal: f32,
}

impl Motion {
    pub(super) fn new(view: View, size: (f32, f32)) -> Self {
        let mut alpha = [0.0; CONTENTS];
        if let Some(i) = view.content() {
            alpha[i] = 1.0;
        }
        Self {
            width: Spring::new(size.0, 300.0, 0.74, 0.05),
            height: Spring::new(size.1, 300.0, 0.74, 0.05),
            from: size,
            target: size,
            alpha,
            content: view.content(),
            shadow: 0.0,
            shadow_goal: 0.0,
        }
    }

    /// Mulai berpindah ke `view` berukuran `size`: membuka memantul, menutup mulus. Isi yang
    /// sama (hanya ukurannya berubah, atau antara dua ukuran panel) tetap tampil.
    pub(super) fn go(&mut self, view: View, size: (f32, f32)) {
        let opening = size.0 * size.1 > self.width.value * self.height.value;
        let bounce = if opening { 0.72 } else { 1.0 };
        self.width.set_damping_ratio(bounce);
        self.height.set_damping_ratio(bounce);
        self.from = if view.content() == self.content { size } else { (self.width.value, self.height.value) };
        self.target = size;
        self.width.target = size.0;
        self.height.target = size.1;
        self.content = view.content();
    }

    fn transition(&self) -> f32 {
        let (tw, th) = self.target;
        let total = (self.from.0 - tw).hypot(self.from.1 - th);
        if total < 0.5 {
            return 1.0;
        }
        (1.0 - (self.width.value - tw).hypot(self.height.value - th) / total).clamp(0.0, 1.0)
    }

    pub(super) fn step(&mut self) {
        let dt = 1.0 / FPS;
        self.width.step(dt);
        self.height.step(dt);
        let show = self.transition() > 0.6;
        for (i, alpha) in self.alpha.iter_mut().enumerate() {
            let goal = if show && Some(i) == self.content { 1.0 } else { 0.0 };
            let tau = if goal > *alpha { 0.08 } else { 0.05 };
            *alpha += (goal - *alpha) * (1.0 - (-dt / tau).exp());
            if (goal - *alpha).abs() < 0.004 {
                *alpha = goal;
            }
        }
        self.shadow += (self.shadow_goal - self.shadow) * (1.0 - (-dt / 0.1).exp());
    }

    /// Ukuran notch saat ini.
    pub(super) fn size(&self) -> (f32, f32) {
        (self.width.value, self.height.value)
    }

    pub(super) fn apply(&self, scene: &mut Scene) {
        scene.width = self.width.value;
        scene.height = self.height.value;
        scene.alpha = self.alpha;
        scene.shadow = self.shadow;
    }
}

/// Urutan frame bernomor (0001.png, 0002.png, …) dengan potongan yang sama untuk setiap frame,
/// supaya frame-nya tidak bergeser saat diputar.
struct Sequence {
    dir: PathBuf,
    renderer: Renderer,
    crop: Crop,
    frame: u32,
}

impl Sequence {
    fn new(dir: PathBuf, dpi: u32, crop: Crop) -> Self {
        std::fs::create_dir_all(&dir).unwrap();
        let mut renderer = Renderer::new(dpi, Theme::Black).unwrap();
        renderer.set_art(Some(&gradient_art()));
        Self { dir, renderer, crop, frame: 0 }
    }

    /// Potongan selebar bentuk terbesar `widest` (DIP), setinggi kanvas.
    fn wide(dir: PathBuf, widest: f32) -> Self {
        let scale = DPI as f32 / 96.0;
        let half = widest / 2.0 + FLARE_MAX + 6.0;
        let crop = (((CANVAS.0 / 2.0 - half) * scale) as i32, 0, (2.0 * half * scale) as i32, (CANVAS.1 * scale) as i32);
        Self::new(dir, DPI, crop)
    }

    fn add(&mut self, scene: &Scene) {
        self.frame += 1;
        self.renderer.draw(scene).unwrap();
        let _ = unsafe { GdiFlush() };
        let width = self.renderer.surface.width;
        let path = self.dir.join(format!("{:04}.png", self.frame));
        save_png(&path, self.renderer.surface.pixels(), width, self.crop);
    }
}

fn export_animations(dir: &Path) {
    let mut data = sample();
    data.media = song();
    let idle = idle_focus();
    let none = Usage::default();
    let base = |view: View, media: bool| {
        let mut scene = home(&data, &idle, &none, view, Page::Home, media);
        scene.shadow = 0.0;
        scene.lyrics_button = Some(false);
        scene
    };
    let frames = |seconds: f32| (seconds * FPS).round() as u32;

    // 1. Buka: pil musik disentuh kursor (sedikit membesar), lalu membuka jadi pemutar.
    let mut out = Sequence::wide(dir.join("01-buka"), View::Panel.size().0);
    let pill = View::Pill.size();
    let mut motion = Motion::new(View::Pill, pill);
    for _ in 0..frames(0.2) {
        let mut scene = base(View::Pill, true);
        motion.apply(&mut scene);
        out.add(&scene);
    }
    motion.go(View::Pill, (pill.0 + 12.0, pill.1 + 4.0));
    motion.shadow_goal = 0.6;
    for _ in 0..frames(0.25) {
        motion.step();
        let mut scene = base(View::Pill, true);
        motion.apply(&mut scene);
        out.add(&scene);
    }
    motion.go(View::Panel, View::Panel.size());
    motion.shadow_goal = 1.0;
    for _ in 0..frames(1.2) {
        motion.step();
        let mut scene = base(View::Panel, true);
        motion.apply(&mut scene);
        out.add(&scene);
    }

    // 2. Tutup: pemutar kembali jadi pil, mulus tanpa pantulan.
    let mut out = Sequence::wide(dir.join("02-tutup"), View::Panel.size().0);
    let mut motion = Motion::new(View::Panel, View::Panel.size());
    motion.shadow = 1.0;
    for _ in 0..frames(0.2) {
        let mut scene = base(View::Panel, true);
        motion.apply(&mut scene);
        out.add(&scene);
    }
    motion.go(View::Pill, View::Pill.size());
    motion.shadow_goal = 0.0;
    for _ in 0..frames(0.9) {
        motion.step();
        let mut scene = base(View::Pill, true);
        motion.apply(&mut scene);
        out.add(&scene);
    }

    // 3. Kolom lirik masuk: notch melebar ke samping, lirik bergeser masuk.
    let mut out = Sequence::wide(dir.join("03-kolom-lirik"), View::PanelWide.size().0);
    let lyrics = lyrics_at(2, 1.0);
    let mut motion = Motion::new(View::Panel, View::Panel.size());
    motion.shadow = 1.0;
    motion.shadow_goal = 1.0;
    let mut mix = 0.0f32;
    let lyric_scene = |mix: f32, button: bool| {
        let mut scene = base(View::PanelWide, true);
        scene.lyrics = Some(&lyrics);
        scene.lyric_mix = mix;
        scene.lyrics_button = Some(button);
        scene
    };
    for _ in 0..frames(0.3) {
        let mut scene = lyric_scene(0.0, false);
        motion.apply(&mut scene);
        out.add(&scene);
    }
    motion.go(View::PanelWide, View::PanelWide.size());
    for _ in 0..frames(1.2) {
        motion.step();
        mix += (1.0 - mix) * (1.0 - (-1.0 / FPS / 0.09f32).exp());
        let mut scene = lyric_scene(if (1.0 - mix) < 0.004 { 1.0 } else { mix }, true);
        motion.apply(&mut scene);
        out.add(&scene);
    }

    // 4. Lirik bergulir ke baris berikutnya (dua kali).
    let mut out = Sequence::wide(dir.join("04-lirik-bergulir"), View::PanelWide.size().0);
    let mut lyric_frames = Vec::new();
    for current in [2usize, 3, 4] {
        let hold = if current == 2 { 0.5 } else { 0.0 };
        for _ in 0..frames(hold) {
            lyric_frames.push(lyrics_at(current, 1.0));
        }
        if current > 2 {
            let steps = frames(0.38);
            for k in 1..=steps {
                lyric_frames.push(lyrics_at(current, k as f32 / steps as f32));
            }
            for _ in 0..frames(0.8) {
                lyric_frames.push(lyrics_at(current, 1.0));
            }
        }
    }
    for lyrics in &lyric_frames {
        let mut scene = base(View::PanelWide, true);
        scene.lyrics = Some(lyrics);
        scene.lyric_mix = 1.0;
        scene.lyrics_button = Some(true);
        scene.shadow = 1.0;
        out.add(&scene);
    }

    // 5. Kabar volume ala Dynamic Island: pil melebar, angka bergulir naik, lalu kembali.
    let mut out = Sequence::wide(dir.join("05-kabar-volume"), View::Strip.size().0 + 40.0);
    let quiet = Hud::Volume { level: 0.61, muted: false };
    let loud = Hud::Volume { level: 0.62, muted: false };
    let mut motion = Motion::new(View::Pill, View::Pill.size());
    fn strip<'a>(mut scene: Scene<'a>, hud: &'a Hud, fill: f32, roll: Option<&'a Roll>) -> Scene<'a> {
        scene.hud = Some(hud);
        scene.hud_compact = true;
        scene.hud_fill = fill;
        scene.hud_roll = roll;
        scene
    }
    for _ in 0..frames(0.3) {
        let mut scene = strip(base(View::Strip, false), &quiet, 0.61, None);
        motion.apply(&mut scene);
        out.add(&scene);
    }
    motion.go(View::Strip, View::Strip.size());
    for _ in 0..frames(0.6) {
        motion.step();
        let mut scene = strip(base(View::Strip, false), &quiet, 0.61, None);
        motion.apply(&mut scene);
        out.add(&scene);
    }
    let mut fill = 0.61f32;
    for k in 0..frames(0.6) {
        motion.step();
        fill += (0.62 - fill) * (1.0 - (-1.0 / FPS / 0.07f32).exp());
        let t = (k as f32 / FPS / 0.22).min(1.0);
        let roll = Roll { from: "61".into(), t, up: true };
        let mut scene = strip(base(View::Strip, false), &loud, fill, (t < 1.0).then_some(&roll));
        motion.apply(&mut scene);
        out.add(&scene);
    }
    motion.go(View::Pill, View::Pill.size());
    for _ in 0..frames(0.8) {
        motion.step();
        let mut scene = strip(base(View::Strip, false), &loud, 0.62, None);
        motion.apply(&mut scene);
        out.add(&scene);
    }

    // 6. Mata maskot dari dekat: melirik kiri, berkedip, melirik kanan, lalu tersenyum.
    let mut out = Sequence::new(dir.join("06-mata"), EYES_DPI, eyes_crop());
    let mut look = (0.0f32, 0.0f32);
    let mut eyes_frames = Vec::new();
    let mut toward = |target: (f32, f32), seconds: f32, frames_out: &mut Vec<Eyes>| {
        for _ in 0..frames(seconds) {
            let k = 1.0 - (-1.0 / FPS / 0.09f32).exp();
            look = (look.0 + (target.0 - look.0) * k, look.1 + (target.1 - look.1) * k);
            frames_out.push(Eyes { look, ..Eyes::default() });
        }
    };
    toward((0.0, 0.0), 0.4, &mut eyes_frames);
    toward((-1.0, -0.2), 0.7, &mut eyes_frames);
    let blink = frames(0.16);
    for k in 0..=blink {
        let t = k as f32 / blink as f32;
        eyes_frames.push(Eyes { open: 1.0 - (t * std::f32::consts::PI).sin(), look: (-1.0, -0.2), ..Eyes::default() });
    }
    toward((1.0, 0.2), 0.8, &mut eyes_frames);
    toward((0.0, 0.0), 0.5, &mut eyes_frames);
    for _ in 0..frames(0.8) {
        eyes_frames.push(Eyes { mood: Mood::Happy, ..Eyes::default() });
    }
    for eyes in eyes_frames {
        let mut scene = base(View::Panel, false);
        scene.eyes = eyes;
        out.add(&scene);
    }
}

/// Kotak berisi semua piksel yang tidak transparan, ditambah `pad` piksel.
fn visible_area(pixels: &[u32], width: i32, height: i32, pad: i32) -> Crop {
    let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
    for y in 0..height {
        for x in 0..width {
            if pixels[(y * width + x) as usize] >> 24 != 0 {
                (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
            }
        }
    }
    let left = (left - pad).max(0);
    let top = (top - pad).max(0);
    let right = (right + pad).min(width - 1);
    let bottom = (bottom + pad).min(height - 1);
    (left, top, right - left + 1, bottom - top + 1)
}

/// PNG dengan transparansi lewat WIC (pengode gambar bawaan Windows). Piksel Direct2D
/// premultiplied diubah dulu ke alfa lurus seperti yang diharapkan PNG.
pub(super) fn save_png(path: &Path, pixels: &[u32], width: i32, crop: Crop) {
    let (left, top, w, h) = crop;
    let mut data = Vec::with_capacity((w * h * 4) as usize);
    for y in top..top + h {
        for x in left..left + w {
            let px = pixels[(y * width + x) as usize];
            let a = px >> 24;
            let un = |c: u32| if a == 0 { 0 } else { ((c * 255 + a / 2) / a).min(255) };
            data.extend([un(px & 0xFF) as u8, un((px >> 8) & 0xFF) as u8, un((px >> 16) & 0xFF) as u8, a as u8]);
        }
    }
    unsafe {
        let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let stream = factory.CreateStream().unwrap();
        stream.InitializeFromFilename(&HSTRING::from(path.as_os_str()), windows::Win32::Foundation::GENERIC_WRITE.0).unwrap();
        let encoder = factory.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null()).unwrap();
        encoder.Initialize(&stream, WICBitmapEncoderNoCache).unwrap();
        let mut frame = None;
        let mut options = None;
        encoder.CreateNewFrame(&mut frame, &mut options).unwrap();
        let frame = frame.unwrap();
        frame.Initialize(options.as_ref().map(|o: &IPropertyBag2| o)).unwrap();
        frame.SetSize(w as u32, h as u32).unwrap();
        let mut format = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut format).unwrap();
        frame.WritePixels(h as u32, (w * 4) as u32, &data).unwrap();
        frame.Commit().unwrap();
        encoder.Commit().unwrap();
    }
}

const README: &str = "\
Paket aset video NotchWindows
==============================

Semua gambar berlatar transparan, dirender 4x (mata maskot 8x) supaya tetap tajam saat
diperbesar di video 1080x1920. Lagu, lirik, dan pesan di dalamnya hanya contoh karangan sendiri,
jadi aman dipakai di media sosial.

gambar/
  pil, pil-musik, pil-lirik, garis-tipis(-mikrofon)   bentuk notch saat tertutup
  pemutar, pemutar-lirik, pemutar-kalender           pemutar musik
  jam, jam-kalender, kontrol, shelf, shelf-seret,
  shelf-bagikan, pomodoro, timer, minum, tugas,
  sistem, sambutan                                   halaman panel
  kabar-*                                            kabar singkat (volume, kecerahan, Caps Lock,
                                                     mengisi daya, pesan, Bluetooth, unduhan,
                                                     agenda, Wi-Fi)
  tema-*                                             pemutar di keenam tema
  mata-*                                             ekspresi maskot dari dekat

animasi/  (60 fps, satu folder = satu klip, frame 0001.png dan seterusnya)
  01-buka            pil disentuh kursor lalu membuka jadi pemutar (memantul)
  02-tutup           pemutar kembali jadi pil
  03-kolom-lirik     notch melebar dan kolom lirik bergeser masuk
  04-lirik-bergulir  lirik naik ke baris berikutnya, dua kali
  05-kabar-volume    kabar volume ala Dynamic Island, angkanya bergulir naik
  06-mata            mata melirik kiri, berkedip, melirik kanan, lalu tersenyum

Memakai urutan frame:
  - DaVinci Resolve (gratis), Premiere Pro, After Effects, Alight Motion: impor folder sebagai
    image sequence, atur 60 fps. Transparansinya ikut.
  - CapCut belum bisa mengimpor image sequence sebagai satu klip; pakai salah satu aplikasi di
    atas untuk menggabungkannya, atau pakai gambar diam dengan animasi keyframe di CapCut.
";
