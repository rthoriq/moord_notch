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
    shelf_fx: ShelfFx,
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
            playing: true,
            progress: Some(0.38),
            elapsed: "1:06".into(),
            remaining: "-1:37".into(),
            can_previous: true,
            can_next: false,
            can_seek: true,
            seeking: false,
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
        shelf_fx: ShelfFx::default(),
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
    /// Ukuran lain dari ukuran bawaan bentuknya (misal pil yang disentuh kursor).
    size: Option<(f32, f32)>,
    shadow: Option<f32>,
    fx: Option<&'a ShelfFx>,
    roll: Option<&'a Roll>,
    calendar: Option<&'a CalendarView>,
    agenda_draft: Option<&'a str>,
    download: Option<&'a str>,
    battery: Option<(u32, bool)>,
    lyrics: Option<&'a LyricsView>,
    lyric_mix: f32,
    lyrics_button: Option<bool>,
    welcome: bool,
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
            size: None,
            shadow: None,
            fx: None,
            roll: None,
            calendar: None,
            agenda_draft: None,
            download: None,
            battery: Some((81, false)),
            lyrics: None,
            lyric_mix: 0.0,
            lyrics_button: None,
            welcome: false,
        }
    }
}

/// Lirik contoh (karangan sendiri) dengan baris ketiga sedang dinyanyikan.
fn sample_lyrics(scroll: f32) -> LyricsView {
    LyricsView {
        lines: [
            "Langit sore memudar pelan".into(),
            "Kau tersenyum di ujung jalan".into(),
            "Dan semua lagu terasa milik kita berdua malam ini".into(),
            "Jangan pergi dulu, tunggu sebentar".into(),
            "Bintang pun ikut bernyanyi".into(),
        ],
        scroll,
        status: None,
    }
}

/// Kalender contoh: Selasa 20 Oktober 2026 dipilih dan hari ini, tiga agenda.
fn sample_calendar() -> CalendarView {
    let names = ["Sab", "Min", "Sen", "Sel", "Rab"];
    CalendarView {
        month: "Okt".into(),
        year: "2026".into(),
        days: (0..5)
            .map(|i| DayView {
                offset: i - 2,
                name: names[i as usize],
                number: 18 + i as u32,
                today: i == 2,
                selected: i == 2,
                has_events: i == 2 || i == 4,
            })
            .collect(),
        events: vec![
            AgendaItem { id: 1, time: String::new(), text: "Ulang tahun Ibu".into(), done: false },
            AgendaItem { id: 2, time: "09.00".into(), text: "Rapat tim".into(), done: false },
            AgendaItem { id: 3, time: "15.30".into(), text: "Beli kado".into(), done: true },
        ],
        scroll: 0,
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
    let lyric = "Langit sore memudar di ujung jalan";
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
    let targeted = ShelfFx { targeted: true, dash: 1.3, ..ShelfFx::default() };
    let sharing = ShelfFx { share_targeted: true, dash: 2.2, ..ShelfFx::default() };
    let waiting = ShelfFx { targeted: false, dash: 3.1, ..ShelfFx::default() };
    let dropped = ShelfFx {
        glow: Some(0.3),
        tiles: [(3u64, (0.8f32, 0.7f32)), (1, (0.75, 0.5))].into_iter().collect(),
        ..ShelfFx::default()
    };
    let no_fx = ShelfFx::default();
    let roll = Roll { from: "61".into(), t: 0.45, up: true };
    let charging = Hud::Power { plugged: true, percent: Some(80), low: false };
    let calendar = sample_calendar();
    let mut other_day = sample_calendar();
    other_day.days.iter_mut().for_each(|d| d.selected = d.offset == 2);
    other_day.events.clear();
    let lyrics = sample_lyrics(1.0);
    let lyrics_scrolling = sample_lyrics(0.4);
    let searching = LyricsView { lines: Default::default(), scroll: 1.0, status: Some("Mencari lirik…") };
    let seeking = MediaView { seeking: true, progress: Some(0.62), elapsed: "1:46".into(), remaining: "-1:07".into(), ..sample().media };
    let agenda_due = Hud::Agenda { text: "Rapat tim".into(), time: "09.00".into() };
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
        State { media: true, lyrics_button: Some(false), hovered: Some(Hit::MediaSeek), ..State::new("player_seek_hover", View::Panel, Page::Home) },
        State { media: true, lyrics_button: Some(false), hovered: Some(Hit::LyricsToggle), ..State::new("player_lyrics_button", View::Panel, Page::Home) },
        State { media: true, lyrics_button: Some(true), lyrics: Some(&lyrics), lyric_mix: 1.0, ..State::new("player_lyrics", View::PanelWide, Page::Home) },
        State { media: true, lyrics_button: Some(true), lyrics: Some(&lyrics), lyric_mix: 0.5, size: Some((560.0, 156.0)), ..State::new("player_lyrics_growing", View::PanelWide, Page::Home) },
        State { media: true, lyrics_button: Some(true), lyrics: Some(&lyrics_scrolling), lyric_mix: 1.0, ..State::new("player_lyrics_scroll", View::PanelWide, Page::Home) },
        State { media: true, lyrics_button: Some(true), lyrics: Some(&searching), lyric_mix: 1.0, ..State::new("player_lyrics_searching", View::PanelWide, Page::Home) },
        State { media: true, lyrics_button: Some(true), calendar: Some(&calendar), lyrics: Some(&lyrics), lyric_mix: 1.0, weather: Some("29° Cerah"), ..State::new("player_lyrics_calendar", View::PanelWide, Page::Home) },
        State { media: true, lyrics_button: Some(true), calendar: Some(&calendar), lyrics: Some(&lyrics), lyric_mix: 0.45, ..State::new("player_lyrics_swap", View::PanelWide, Page::Home) },
        State { theme: Theme::Glass, media: true, lyrics_button: Some(true), lyrics: Some(&lyrics), lyric_mix: 1.0, ..State::new("player_lyrics_glass", View::PanelWide, Page::Home) },
        State { theme: Theme::Light, media: true, lyrics_button: Some(true), lyrics: Some(&lyrics), lyric_mix: 1.0, ..State::new("player_lyrics_light", View::PanelWide, Page::Home) },
        State { media: true, battery: Some((64, true)), ..State::new("player_charging", View::Panel, Page::Home) },
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
        State { size: Some((188.0, 34.0)), shadow: Some(0.6), ..State::new("pill_peek", View::Pill, Page::Home) },
        State { fx: Some(&waiting), ..State::new("drop_waiting", View::Panel, Page::Shelf) },
        State { fx: Some(&targeted), ..State::new("drop_targeted", View::Panel, Page::Shelf) },
        State { fx: Some(&dropped), ..State::new("shelf_dropped", View::Panel, Page::Shelf) },
        State { hud: Some(&volume), roll: Some(&roll), ..State::new("strip_volume", View::Strip, Page::Home) },
        State { hud: Some(&brightness), ..State::new("strip_brightness", View::Strip, Page::Home) },
        State { hud: Some(&caps), ..State::new("strip_capslock", View::Strip, Page::Home) },
        State { hud: Some(&charging), ..State::new("strip_power", View::Strip, Page::Home) },
        State { hud: Some(&volume), roll: Some(&roll), ..State::new("hud_volume_roll", View::Hud, Page::Home) },
        State { media: true, calendar: Some(&calendar), weather: Some("29° Cerah"), ..State::new("home_wide_media", View::PanelWide, Page::Home) },
        State { calendar: Some(&calendar), hovered: Some(Hit::AgendaRow(2)), ..State::new("home_wide_clock", View::PanelWide, Page::Home) },
        State { calendar: Some(&other_day), agenda_draft: Some("09.00 Rap"), battery: Some((34, true)), ..State::new("home_wide_typing", View::PanelWide, Page::Home) },
        State { hud: Some(&agenda_due), ..State::new("hud_agenda", View::Hud, Page::Home) },
        State { download: Some("12,4 MB"), size: Some((236.0, 30.0)), ..State::new("pill_download", View::Pill, Page::Home) },
        State { fx: Some(&sharing), ..State::new("drop_share", View::Panel, Page::Shelf) },
        State { media: true, lyrics_button: Some(false), ..State::new("player_seeking", View::Panel, Page::Home) },
        State { welcome: true, hovered: Some(Hit::WelcomeDone), ..State::new("welcome", View::Panel, Page::Home) },
        State { welcome: true, theme: Theme::Light, ..State::new("welcome_light", View::Panel, Page::Home) },
    ];
    for state in states {
        let mut renderer = Renderer::new(state.dpi, state.theme).unwrap();
        if state.media {
            renderer.set_art(Some(&gradient_art()));
        }
        // Ubin kedua shelf diberi thumbnail tiruan, sisanya memakai ikon.
        renderer.set_thumbnail(&Thumbnail { id: 2, width: 64, height: 40, pixels: gradient(64, 40) });
        data.tasks.draft = (state.name == "page_tasks_150").then(|| "Siapkan materi rapat".into());
        let (width, height) = match (state.view, state.lyric, state.size) {
            (_, _, Some(size)) => size,
            (View::LyricPill, Some(line), _) => renderer.lyric_size(line),
            _ => state.view.size(),
        };
        data.shelf.drop_hover = state.name.starts_with("drop");
        let fill = match state.hud {
            Some(Hud::Volume { level, .. } | Hud::Brightness { level }) => *level,
            _ => 0.0,
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
            shadow: state.shadow.unwrap_or(if state.view.is_panel() { 1.0 } else { 0.0 }),
            time: "19:21",
            date: "Senin, 19 Oktober 2026",
            day: "Rab 21 Okt 2026",
            greeting: "Selamat malam",
            battery: state.battery,
            media: state.media.then_some(if state.name == "player_seeking" { &seeking } else { &data.media }),
            lyric: state.lyric,
            lyrics: state.lyrics,
            lyric_mix: state.lyric_mix,
            lyrics_button: state.lyrics_button,
            hud: state.hud,
            hud_compact: state.view == View::Strip,
            hud_roll: state.roll,
            hud_fill: fill,
            page: state.page,
            pages: state.pages,
            page_from: None,
            shelf: &data.shelf,
            shelf_fx: state.fx.unwrap_or(&no_fx),
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
            calendar: state.calendar,
            agenda_draft: state.agenda_draft,
            download: state.download,
            welcome: state.welcome,
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
        shadow: 0.0,
        time: "19:21",
        date: "",
        day: "Rab 21 Okt 2026",
        greeting: "",
        battery: None,
        media: media.then_some(&data.media),
        lyric: None,
        lyrics: None,
        lyric_mix: 0.0,
        lyrics_button: None,
        hud: None,
        hud_compact: false,
        hud_roll: None,
        hud_fill: 0.0,
        page,
        pages: Pages::default(),
        page_from: None,
        shelf: &data.shelf,
        shelf_fx: &data.shelf_fx,
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
        calendar: None,
        agenda_draft: None,
        download: None,
        welcome: false,
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
            renderer.draw_eyes(176.0, 30.0, *eyes).unwrap();
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

/// Bayangan hanya jatuh di bawah notch: di samping notch (di bawah lengkung atasnya) tidak boleh
/// ada pita gelap, karena terlihat seperti kotak di balik notch.
#[test]
fn shadow_only_below_notch() {
    let data = sample();
    let mut renderer = Renderer::new(96, Theme::Black).unwrap();
    let mut scene = scene_for(&data, View::Panel, Page::Home, false);
    scene.shadow = 1.0;
    renderer.draw(&scene).unwrap();
    let _ = unsafe { GdiFlush() };
    let width = renderer.surface.width as usize;
    let (w, h) = View::Panel.size();
    let x0 = (CANVAS.0 / 2.0 - w / 2.0) as usize;
    let pixels = renderer.surface.pixels();
    let alpha = |x: usize, y: usize| pixels[y * width + x] >> 24;
    for y in (FLARE_MAX as usize + 2)..(h as usize - 2) {
        for x in (x0 - 12)..(x0 - 1) {
            assert_eq!(alpha(x, y), 0, "piksel ({x}, {y}) di samping notch tidak transparan");
        }
    }
    assert!(alpha((CANVAS.0 / 2.0) as usize, h as usize + 4) > 0, "bayangan di bawah notch ada");

    // Badan tema kaca tembus pandang: bayangan tidak boleh terlihat di dalam panel.
    let mut glass = Renderer::new(96, Theme::Glass).unwrap();
    let mut frame = |shadow: f32| {
        let mut scene = scene_for(&data, View::Panel, Page::Home, false);
        scene.shadow = shadow;
        glass.draw(&scene).unwrap();
        let _ = unsafe { GdiFlush() };
        glass.surface.pixels().to_vec()
    };
    let (plain, shaded) = (frame(0.0), frame(1.0));
    let inside = (0..h as usize).flat_map(|y| (0..width).map(move |x| y * width + x));
    assert!(inside.clone().all(|i| plain[i] == shaded[i]), "bayangan terlihat di balik badan kaca");
    let below = h as usize * width + width / 2;
    assert_ne!(plain[below + 4 * width], shaded[below + 4 * width], "bayangan di bawah notch kaca ada");
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

/// Tulis ulang `packaging/notch.ico` dari maskot ikon tray (ikon notch.exe di menu Start,
/// Explorer, dan Pengaturan > Aplikasi). Jalankan dengan
/// `cargo test write_app_icon -- --ignored` setelah maskotnya diubah.
#[test]
#[ignore]
fn write_app_icon() {
    const SIZES: [i32; 8] = [16, 20, 24, 32, 40, 48, 64, 128];
    let images: Vec<Vec<u8>> = SIZES
        .iter()
        .map(|&size| {
            let mut surface = icon_surface(size).unwrap();
            let pixels = surface.pixels().to_vec();
            // Gambar ikon: BITMAPINFOHEADER, piksel BGRA dari baris bawah ke atas, lalu topeng AND
            // (semua nol; transparansinya dari alfa).
            let mut image = Vec::new();
            for value in [40u32, size as u32, size as u32 * 2] {
                image.extend(value.to_le_bytes());
            }
            image.extend(1u16.to_le_bytes());
            image.extend(32u16.to_le_bytes());
            image.extend([0u8; 24]);
            for row in (0..size as usize).rev() {
                for px in &pixels[row * size as usize..(row + 1) * size as usize] {
                    image.extend(px.to_le_bytes());
                }
            }
            let mask_row = (size as usize).div_ceil(32) * 4;
            image.extend(vec![0u8; mask_row * size as usize]);
            image
        })
        .collect();
    let mut ico = Vec::new();
    ico.extend([0u16, 1, SIZES.len() as u16].iter().flat_map(|v| v.to_le_bytes()));
    let mut offset = 6 + 16 * SIZES.len() as u32;
    for (&size, image) in SIZES.iter().zip(&images) {
        ico.extend([size as u8, size as u8, 0, 0]);
        ico.extend(1u16.to_le_bytes());
        ico.extend(32u16.to_le_bytes());
        ico.extend((image.len() as u32).to_le_bytes());
        ico.extend(offset.to_le_bytes());
        offset += image.len() as u32;
    }
    for image in &images {
        ico.extend(image);
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging").join("notch.ico");
    std::fs::write(&path, ico).unwrap();
}

/// Tombol lirik, bar lagu, dan tombol putar bisa diklik di tempat yang terlihat; bar lagu
/// mengubah posisi klik jadi posisi lagu.
#[test]
fn player_hits() {
    let data = sample();
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let width = View::Panel.size().0;
    let x0 = CANVAS.0 / 2.0 - width / 2.0;
    let (l, r) = (x0 + MARGIN_FOR_TEST, x0 + width - MARGIN_FOR_TEST);
    let mut scene = scene_for(&data, View::Panel, Page::Home, true);
    scene.lyrics_button = Some(false);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(l + 10.0, 125.0), Some(Hit::LyricsToggle));
    assert_eq!(renderer.hit_at((l + r) / 2.0, 125.0), Some(Hit::Media(Button::PlayPause)));
    assert_eq!(renderer.hit_at((l + r) / 2.0, 101.0), Some(Hit::MediaSeek));
    let start = renderer.slider_value(Hit::MediaSeek, l + 42.0).unwrap();
    let end = renderer.slider_value(Hit::MediaSeek, r - 48.0).unwrap();
    assert!(start < 0.01 && end > 0.99, "bar lagu {start}..{end}");

    // Fitur lirik mati: tombolnya tidak ada. Aplikasi yang tidak bisa lompat: bar tidak bisa diklik.
    let fixed = MediaView { can_seek: false, ..sample().media };
    let mut scene = scene_for(&data, View::Panel, Page::Home, true);
    scene.media = Some(&fixed);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(l + 10.0, 125.0), None);
    assert_eq!(renderer.hit_at((l + r) / 2.0, 101.0), None);
}

/// Lirik menggantikan kalender: setelah bergeser penuh, kalender tidak bisa diklik lagi;
/// selama bergeser, kolom kanan tidak bisa diklik sama sekali.
#[test]
fn lyrics_replace_calendar() {
    let data = sample();
    let calendar = sample_calendar();
    let lyrics = sample_lyrics(1.0);
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let width = View::PanelWide.size().0;
    let x1 = CANVAS.0 / 2.0 + width / 2.0;
    // Hari ketiga deretan kalender (di tengah kolom kanan), dan baris agenda pertama.
    let day = (x1 - MARGIN_FOR_TEST - 100.0, 63.0);
    let mut scene = scene_for(&data, View::PanelWide, Page::Home, true);
    scene.calendar = Some(&calendar);
    scene.lyrics = Some(&lyrics);
    for (mix, expected) in [(0.0, true), (0.5, false), (1.0, false)] {
        scene.lyric_mix = mix;
        renderer.draw(&scene).unwrap();
        let hit = renderer.hit_at(day.0, day.1);
        assert_eq!(matches!(hit, Some(Hit::CalendarDay(_))), expected, "geser {mix}: {hit:?}");
    }
}

/// Pendaran bawah panel mengikuti warna sampul, dan tidak ada tanpa musik.
#[test]
fn glow_follows_album_colour() {
    let data = sample();
    let (_, h) = View::Panel.size();
    let probe = |media: bool| {
        let mut renderer = Renderer::new(96, Theme::Black).unwrap();
        renderer.draw(&scene_for(&data, View::Panel, Page::Home, media)).unwrap();
        let _ = unsafe { GdiFlush() };
        let width = renderer.surface.width as usize;
        // Sedikit di atas tepi bawah, di kiri tengah (bukan di atas tombol atau titik halaman).
        let x = (CANVAS.0 / 2.0 - 120.0) as usize;
        renderer.surface.pixels()[(h as usize - 6) * width + x]
    };
    let channel = |px: u32, shift: u32| (px >> shift) & 0xFF;
    let (plain, glowing) = (probe(false), probe(true));
    assert_eq!(channel(plain, 0), 0, "tanpa musik badan tetap hitam");
    // Sampul contoh biru muda (0x4FC3F7): biru paling kuat, merah paling lemah.
    assert!(channel(glowing, 0) > 30 && channel(glowing, 0) > channel(glowing, 16), "pendaran {glowing:08X}");
}

/// Tombol × agenda harus bisa dicapai: kursor yang bergerak dari lingkaran melewati teks baris
/// tetap menyorot baris itu, sehingga × di ujung kanan tidak hilang sebelum diklik.
#[test]
fn agenda_remove_reachable() {
    let data = sample();
    let calendar = sample_calendar();
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let width = View::PanelWide.size().0;
    let x0 = CANVAS.0 / 2.0 - width / 2.0;
    // Kolom kalender: sesudah kolom kiri Beranda (356) dan jarak 24.
    let (l, r) = (x0 + MARGIN_FOR_TEST + 356.0 + 24.0, x0 + width - MARGIN_FOR_TEST);
    let y = 92.0;
    let mut scene = scene_for(&data, View::PanelWide, Page::Home, false);
    scene.calendar = Some(&calendar);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(l + 6.0, y), Some(Hit::AgendaToggle(1)));
    assert_eq!(renderer.hit_at(l + 80.0, y), Some(Hit::AgendaRow(1)));
    assert_eq!(renderer.hit_at(r - 8.0, y), Some(Hit::AgendaRow(1)));
    assert_eq!(renderer.hit_at(r - 8.0, y + 16.0), Some(Hit::AgendaRow(2)));
    // Baris disorot: × muncul di tempat jam dan bisa diklik, baris lain tidak terpengaruh.
    scene.hovered = Some(Hit::AgendaRow(1));
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(r - 8.0, y), Some(Hit::AgendaRemove(1)));
    assert_eq!(renderer.hit_at(l + 80.0, y), Some(Hit::AgendaRow(1)));
    assert_eq!(renderer.hit_at(r - 8.0, y + 16.0), Some(Hit::AgendaRow(2)));
}

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
