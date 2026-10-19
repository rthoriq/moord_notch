use std::path::{Path, PathBuf};

use super::*;
use crate::widgets::shelf::Shelf;
use crate::widgets::tasks::Tasks;

/// Data contoh untuk semua halaman.
struct Sample {
    media: MediaView,
    shelf: Shelf,
    tasks: Tasks,
    focus: FocusView,
    water: WaterView,
    privacy: Usage,
    system: Stats,
    timer: TimerView,
    controls: ControlsView,
}

fn sample() -> Sample {
    let mut shelf = Shelf::default();
    shelf.add(["Project", "Corporate and Campaign.png", "Infuse Navy Tote Bag.png"].map(|n| PathBuf::from(format!("C:\\contoh\\{n}"))));
    let mut tasks = Tasks::default();
    for text in ["Kirim revisi desain ke klien", "Beli kopi", "Rapikan folder Downloads yang sudah penuh sekali"] {
        tasks.add(text);
    }
    tasks.add("Bayar listrik");
    tasks.toggle(1);
    Sample {
        media: MediaView {
            title: "Kehilanganmu Berat Bagiku (judul yang panjang sekali)".into(),
            artist: "Kangen Band".into(),
            source: "Spotify".into(),
            playing: true,
            progress: Some(0.38),
            elapsed: "1:06".into(),
            remaining: "-1:37".into(),
            can_previous: true,
            can_next: false,
            accent: Some(0x4FC3F7),
        },
        shelf,
        tasks,
        focus: FocusView {
            phase: Phase::Focus,
            running: true,
            active: true,
            clock: "17:42".into(),
            progress: 0.29,
            sessions: 2,
            minutes_left: 18,
        },
        water: WaterView {
            ml: 1250,
            goal: 2000,
            week: vec![(1500, 'S'), (2000, 'S'), (750, 'R'), (1750, 'K'), (2250, 'J'), (1000, 'S'), (1250, 'M')],
            next: Some("19:37".into()),
        },
        privacy: Usage { camera: vec![], microphone: vec!["Discord".into()] },
        system: Stats {
            cpu: 23.0,
            gpu: Some(64.0),
            gpu_temp: Some(71.0),
            ram_used: 10_522_669_875,
            ram_total: 16_857_853_952,
            disk_free: 120_000_000_000,
            disk_total: 512_000_000_000,
            down: 1_258_291.0,
            up: 81_920.0,
        },
        timer: idle_timer(),
        controls: ControlsView {
            volume: Some((0.62, false)),
            brightness: Some(70),
            mic: Some(false),
            quiet_minutes: None,
            city: Some("Kota: Jakarta".into()),
        },
    }
}

/// Timer yang tidak berjalan, untuk keadaan selain halaman/pil timer.
fn idle_timer() -> TimerView {
    TimerView { running: false, active: false, clock: "00:00".into(), short: String::new(), progress: 0.0, total_minutes: 0 }
}

/// Satu keadaan yang dirender ke gambar preview.
struct State<'a> {
    name: &'a str,
    dpi: u32,
    theme: Theme,
    view: View,
    page: Page,
    media: bool,
    lyric: Option<&'a str>,
    hud: Option<&'a Hud>,
    hovered: Option<Hit>,
    locked: bool,
    eyes: Eyes,
    pages: Pages,
    weather: Option<&'a str>,
    city_draft: Option<&'a str>,
}

impl<'a> State<'a> {
    fn new(name: &'a str, view: View, page: Page) -> Self {
        Self {
            name,
            dpi: 96,
            theme: Theme::Black,
            view,
            page,
            media: false,
            lyric: None,
            hud: None,
            hovered: None,
            locked: false,
            eyes: Eyes::default(),
            pages: Pages::default(),
            weather: None,
            city_draft: None,
        }
    }
}

/// Render beberapa keadaan notch ke `target/preview/*.bmp` supaya tampilannya bisa
/// dicek tanpa menjalankan aplikasi. Jalankan dengan `cargo test`.
#[test]
fn preview_states() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("preview");
    std::fs::create_dir_all(&dir).unwrap();
    let mut data = sample();
    let volume = Hud::Volume { level: 0.62, muted: false };
    let bluetooth = Hud::Bluetooth {
        id: String::new(),
        name: "Baseus Encok WM01".into(),
        connected: true,
        battery: Some(80),
        kind: DeviceKind::Audio,
    };
    let water_hud = Hud::Water { ml: 1250, goal: 2000 };
    let camera = Hud::Privacy { camera: true, app: "Zoom".into() };
    let lyric = "Tiap detik masih merindukanmu, oh-oh";
    let long_lyric = "Kau bilang aku segalanya, tapi kenyataannya kau pergi tanpa sepatah kata pun untukku";
    let brightness = Hud::Brightness { level: 0.7 };
    let caps = Hud::LockKey { caps: true, on: true };
    let num = Hud::LockKey { caps: false, on: false };
    let message = Hud::Message {
        app: "WhatsApp".into(),
        aumid: "contoh!App".into(),
        sender: "Ibu".into(),
        text: "Nak, nanti pulang jam berapa? Ibu masak sayur asem kesukaanmu".into(),
        browser: false,
    };
    let screenshot = Hud::Screenshot { id: 2, name: "Screenshot 2026-10-19 101112.png".into(), more: 0 };
    let download = Hud::Download { path: PathBuf::from("C:\\contoh\\laporan.pdf"), name: "Laporan Keuangan Q3 2026.pdf".into(), more: 2 };
    let wifi = Hud::Wifi { name: "Rumah 5G".into(), connected: true, internet: true };
    let wifi_limited = Hud::Wifi { name: "Kafe Senja".into(), connected: true, internet: false };
    let mood = |mood| Eyes { mood, ..Eyes::default() };
    let mut few_pages = Pages::default();
    few_pages.set_shown(Page::Water, false);
    few_pages.set_shown(Page::System, false);
    few_pages.move_by(Page::Tasks, -1);
    let timer_done = Hud::Timer { minutes: 5 };
    let running = TimerView {
        running: true,
        active: true,
        clock: "03:42".into(),
        short: "4m".into(),
        progress: 0.26,
        total_minutes: 5,
    };
    let muted_controls = ControlsView {
        volume: Some((0.3, true)),
        brightness: None,
        mic: Some(true),
        quiet_minutes: Some(42),
        city: None,
    };
    let message_locked =
        Hud::Message {
        app: String::new(),
        aumid: "contoh!App".into(),
        sender: "WhatsApp".into(),
        text: "Pesan baru".into(),
        browser: false,
    };

    // Maskot di pil hanya terlihat kalau pomodoro tidak berjalan.
    let idle = FocusView { running: false, active: false, ..sample().focus };
    let states = [
        State::new("pill", View::Pill, Page::Home),
        State { media: true, ..State::new("pill_music", View::Pill, Page::Home) },
        State { media: true, lyric: Some(lyric), ..State::new("pill_lyric", View::LyricPill, Page::Home) },
        State::new("slim", View::Slim, Page::Home),
        State { hud: Some(&volume), ..State::new("hud_volume", View::Hud, Page::Home) },
        State { hud: Some(&bluetooth), ..State::new("hud_bluetooth", View::Hud, Page::Home) },
        State { hud: Some(&water_hud), ..State::new("hud_water", View::Hud, Page::Home) },
        State { hud: Some(&camera), ..State::new("hud_camera", View::Hud, Page::Home) },
        State { hovered: Some(Hit::PageNext), ..State::new("page_clock", View::Panel, Page::Home) },
        State { media: true, hovered: Some(Hit::Media(Button::PlayPause)), ..State::new("page_media", View::Panel, Page::Home) },
        State { media: true, lyric: Some(lyric), ..State::new("page_media_lyric", View::Panel, Page::Home) },
        State { hovered: Some(Hit::ShelfItem(2)), ..State::new("page_shelf", View::Panel, Page::Shelf) },
        State { hovered: Some(Hit::FocusReset), ..State::new("page_focus", View::Panel, Page::Focus) },
        State { hovered: Some(Hit::WaterDrink), ..State::new("page_water", View::Panel, Page::Water) },
        State { hovered: Some(Hit::TaskToggle(0)), ..State::new("page_tasks", View::Panel, Page::Tasks) },
        State::new("page_system", View::Panel, Page::System),
        State { dpi: 144, ..State::new("page_tasks_150", View::Panel, Page::Tasks) },
        State { theme: Theme::Midnight, ..State::new("theme_midnight", View::Panel, Page::Focus) },
        State { theme: Theme::Grape, media: true, ..State::new("theme_grape", View::Panel, Page::Home) },
        State { theme: Theme::Light, hovered: Some(Hit::TaskToggle(0)), ..State::new("theme_light", View::Panel, Page::Tasks) },
        State { theme: Theme::Light, ..State::new("theme_light_pill", View::Pill, Page::Home) },
        State { media: true, lyric: Some(long_lyric), ..State::new("pill_lyric_long", View::LyricPill, Page::Home) },
        State { media: true, lyric: Some(long_lyric), ..State::new("page_media_lyric_long", View::Panel, Page::Home) },
        State { hud: Some(&brightness), ..State::new("hud_brightness", View::Hud, Page::Home) },
        State { hud: Some(&caps), ..State::new("hud_capslock", View::Hud, Page::Home) },
        State { hud: Some(&num), ..State::new("hud_numlock", View::Hud, Page::Home) },
        State { hud: Some(&message), ..State::new("hud_message", View::Hud, Page::Home) },
        State { hud: Some(&message_locked), locked: true, ..State::new("hud_message_locked", View::Hud, Page::Home) },
        State { media: true, locked: true, ..State::new("page_locked", View::Panel, Page::Home) },
        State { theme: Theme::Glass, ..State::new("theme_glass_pill", View::Pill, Page::Home) },
        State { theme: Theme::Glass, media: true, ..State::new("theme_glass", View::Panel, Page::Home) },
        State { theme: Theme::Glass, hud: Some(&message), ..State::new("theme_glass_hud", View::Hud, Page::Home) },
        State { eyes: mood(Mood::Happy), ..State::new("mood_happy", View::Pill, Page::Home) },
        State { eyes: mood(Mood::Sleepy), ..State::new("mood_sleepy", View::Pill, Page::Home) },
        State { eyes: mood(Mood::Surprised), ..State::new("mood_surprised", View::Pill, Page::Home) },
        State { eyes: Eyes { look: (1.0, 1.0), ..Eyes::default() }, ..State::new("mood_look", View::Pill, Page::Home) },
        State { eyes: mood(Mood::Happy), ..State::new("mood_happy_panel", View::Panel, Page::Home) },
        State { eyes: Eyes { look: (-1.0, 0.6), mood: Mood::Surprised, open: 1.0 }, ..State::new("mood_surprised_panel", View::Panel, Page::Home) },
        State { hud: Some(&screenshot), ..State::new("hud_screenshot", View::Hud, Page::Home) },
        State { hud: Some(&download), ..State::new("hud_download", View::Hud, Page::Home) },
        State { hud: Some(&wifi), ..State::new("hud_wifi", View::Hud, Page::Home) },
        State { hud: Some(&wifi_limited), ..State::new("hud_wifi_limited", View::Hud, Page::Home) },
        State { pages: few_pages, ..State::new("pages_custom", View::Panel, Page::Tasks) },
        State { dpi: 84, ..State::new("size_small", View::Pill, Page::Home) },
        State { dpi: 110, media: true, ..State::new("size_large", View::Panel, Page::Home) },
        State { hovered: Some(Hit::VolumeSlider), ..State::new("page_controls", View::Panel, Page::Controls) },
        State { city_draft: Some("Yogya"), ..State::new("page_controls_city", View::Panel, Page::Controls) },
        State::new("page_controls_muted", View::Panel, Page::Controls),
        State { hovered: Some(Hit::TimerPreset(5)), ..State::new("page_timer", View::Panel, Page::Timer) },
        State::new("page_timer_idle", View::Panel, Page::Timer),
        State::new("pill_timer", View::Pill, Page::Home),
        State { hud: Some(&timer_done), ..State::new("hud_timer", View::Hud, Page::Home) },
        State { weather: Some("28° Mendung · Jakarta"), ..State::new("page_clock_weather", View::Panel, Page::Home) },
    ];
    for state in states {
        let mut renderer = Renderer::new(state.dpi, state.theme).unwrap();
        if state.media {
            renderer.set_art(Some(&gradient_art()));
        }
        // Ubin kedua shelf diberi thumbnail tiruan, sisanya memakai ikon.
        renderer.set_thumbnail(&Thumbnail { id: 2, width: 64, height: 40, pixels: gradient(64, 40) });
        data.tasks.draft = (state.name == "page_tasks_150").then(|| "Siapkan materi rapat".into());
        let (width, height) = match (state.view, state.lyric) {
            (View::LyricPill, Some(line)) => renderer.lyric_size(line),
            _ => state.view.size(),
        };
        let mut alpha = [0.0; CONTENTS];
        if let Some(i) = state.view.content() {
            alpha[i] = 1.0;
        }
        let scene = Scene {
            width,
            height,
            alpha,
            eyes: state.eyes,
            time: "19:21",
            date: "Senin, 19 Oktober 2026",
            greeting: "Selamat malam",
            media: state.media.then_some(&data.media),
            lyric: state.lyric,
            hud: state.hud,
            page: state.page,
            pages: state.pages,
            page_from: None,
            shelf: &data.shelf,
            tasks: &data.tasks,
            focus: if state.name.starts_with("mood") || state.name == "size_small" { &idle } else { &data.focus },
            water: &data.water,
            privacy: &data.privacy,
            system: Some(&data.system),
            uptime: "3 j 12 m",
            hovered: state.hovered,
            seconds: 1.3,
            locked: state.locked,
            timer: if state.name.contains("timer") && !state.name.ends_with("idle") { &running } else { &data.timer },
            controls: if state.name.ends_with("muted") { &muted_controls } else { &data.controls },
            weather: state.weather,
            city_draft: state.city_draft,
        };
        renderer.draw(&scene).unwrap();
        let _ = unsafe { GdiFlush() };
        let (w, h) = (renderer.surface.width, renderer.surface.height);
        write_bmp(&dir.join(format!("{}.bmp", state.name)), w, h, renderer.surface.pixels());
    }
}

fn scene_for<'a>(data: &'a Sample, view: View, page: Page, media: bool) -> Scene<'a> {
    let (width, height) = view.size();
    let mut alpha = [0.0; CONTENTS];
    if let Some(i) = view.content() {
        alpha[i] = 1.0;
    }
    Scene {
        width,
        height,
        alpha,
        eyes: Eyes::default(),
        time: "19:21",
        date: "",
        greeting: "",
        media: media.then_some(&data.media),
        lyric: None,
        hud: None,
        page,
        pages: Pages::default(),
        page_from: None,
        shelf: &data.shelf,
        tasks: &data.tasks,
        focus: &data.focus,
        water: &data.water,
        privacy: &data.privacy,
        system: None,
        uptime: "",
        hovered: None,
        seconds: 0.0,
        locked: false,
        timer: &data.timer,
        controls: &data.controls,
        weather: None,
        city_draft: None,
    }
}

/// Gambar ulang mata saja (berkedip, melirik, ganti ekspresi) harus sama persis dengan gambar
/// penuh: tidak ada bagian mata yang tertinggal di luar kotak kecilnya.
#[test]
fn eye_patch_covers_every_expression() {
    let data = sample();
    let mut variants = vec![Eyes::default()];
    for mood in [Mood::Normal, Mood::Sleepy, Mood::Happy, Mood::Surprised] {
        for look in [(-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0), (1.0, -1.0)] {
            variants.push(Eyes { open: 1.0, look, mood });
        }
    }
    // Mata hanya tampil di pil kalau pomodoro tidak berjalan.
    let idle = FocusView { running: false, ..sample().focus };
    for dpi in [96, 144] {
        let mut renderer = Renderer::new(dpi, Theme::Black).unwrap();
        for eyes in &variants {
            let mut scene = scene_for(&data, View::Pill, Page::Home, false);
            scene.focus = &idle;
            scene.eyes = *eyes;
            renderer.draw(&scene).unwrap();
            let _ = unsafe { GdiFlush() };
            let full = renderer.surface.pixels().to_vec();

            scene.eyes = Eyes::default();
            renderer.draw(&scene).unwrap();
            renderer.draw_eyes(*eyes).unwrap();
            let _ = unsafe { GdiFlush() };
            let patched = renderer.surface.pixels();
            let worst = full
                .iter()
                .zip(patched.iter())
                .map(|(a, b)| (0..4).map(|i| ((a >> (i * 8)) & 0xFF).abs_diff((b >> (i * 8)) & 0xFF)).max().unwrap_or(0))
                .max()
                .unwrap_or(0);
            assert!(worst <= 2, "mata {eyes:?} pada {dpi} DPI keluar dari kotaknya (selisih {worst})");
        }
    }
}

/// Pil lirik melebar mengikuti barisnya, lalu jadi dua baris kalau masih tidak muat.
#[test]
fn lyric_pill_fits_line() {
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let (w, h) = renderer.lyric_size("♪");
    assert!(w >= LYRIC_MIN_W && w < LYRIC_MIN_W + 24.0 && h == View::LyricPill.size().1, "ukuran {w}x{h}");
    let (w, h) = renderer.lyric_size("Tiap detik masih merindukanmu, oh-oh, sampai pagi");
    assert!(w > LYRIC_MIN_W && w <= LYRIC_MAX_W && w % 24.0 == 0.0, "lebar {w}");
    assert_eq!(h, View::LyricPill.size().1);
    let (w, h) = renderer.lyric_size(&"panjang sekali ".repeat(8));
    assert_eq!((w, h), (LYRIC_MAX_W, LYRIC_TWO_LINES_H));
}

/// Area klik tercatat sesuai yang digambar dan bisa ditemukan lagi.
#[test]
fn hits_follow_drawing() {
    let data = sample();
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    renderer.draw(&scene_for(&data, View::Panel, Page::Water, false)).unwrap();
    let width = View::Panel.size().0;
    let left = CANVAS.0 / 2.0 - width / 2.0 + MARGIN_FOR_TEST;
    assert_eq!(renderer.hit_at(left + 20.0, 128.0), Some(Hit::WaterDrink));
    assert_eq!(renderer.hit_at(CANVAS.0 / 2.0 + width / 2.0 - 15.0, 84.0), Some(Hit::PageNext));
    assert_eq!(renderer.hit_at(2.0, 2.0), None);
}

const MARGIN_FOR_TEST: f32 = 32.0;

/// Bandingkan biaya satu frame penuh dengan gambar ulang gelombang saja.
/// Jalankan dengan `cargo test --release -- --ignored --nocapture`.
#[test]
#[ignore]
fn bench_wave() {
    let data = sample();
    let mut renderer = Renderer::new(96, Theme::Black).unwrap();
    renderer.set_art(Some(&gradient_art()));
    let scene = scene_for(&data, View::Pill, Page::Home, true);
    let runs = 500;
    let start = std::time::Instant::now();
    for _ in 0..runs {
        renderer.draw(&scene).unwrap();
    }
    let full = start.elapsed() / runs;
    let start = std::time::Instant::now();
    for i in 0..runs {
        let (w, h) = View::Pill.size();
        renderer.draw_wave(w, h, i as f64 * 0.08, None).unwrap();
    }
    let wave = start.elapsed() / runs;
    println!("frame penuh: {full:?}, gelombang saja: {wave:?}");
}

fn gradient(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&[230, (x * 255 / width) as u8, (y * 255 / height) as u8, 255]);
        }
    }
    pixels
}

/// Sampul tiruan: gradasi biru ke ungu.
fn gradient_art() -> Art {
    Art { id: 1, pixels: gradient(ART_PX, ART_PX), accent: Some(0x4FC3F7) }
}

/// BMP 32-bit; piksel premultiplied ditumpuk di atas latar abu-abu supaya notch hitam terlihat.
fn write_bmp(path: &Path, w: i32, h: i32, pixels: &[u32]) {
    const BG: u32 = 0xB4;
    let mut data = Vec::with_capacity(54 + pixels.len() * 4);
    data.extend_from_slice(b"BM");
    data.extend_from_slice(&(54 + pixels.len() as u32 * 4).to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&54u32.to_le_bytes());
    data.extend_from_slice(&40u32.to_le_bytes());
    data.extend_from_slice(&w.to_le_bytes());
    data.extend_from_slice(&(-h).to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&32u16.to_le_bytes());
    data.extend_from_slice(&[0u8; 24]);
    for &px in pixels {
        let a = px >> 24;
        let over = |c: u32| (c + BG * (255 - a) / 255).min(255) as u8;
        data.extend_from_slice(&[over(px & 0xFF), over((px >> 8) & 0xFF), over((px >> 16) & 0xFF), 255]);
    }
    std::fs::write(path, data).unwrap();
}
