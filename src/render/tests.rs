use std::path::{Path, PathBuf};

use super::*;
use crate::widgets::shelf::Shelf;

mod aset;
mod video;
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
    trend: SystemTrend,
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
            fx: None,
        },
        water: WaterView {
            ml: 1250,
            goal: 2000,
            week: vec![(1500, 'S'), (2000, 'S'), (750, 'R'), (1750, 'K'), (2250, 'J'), (1000, 'S'), (1250, 'M')],
            next: Some("19:37".into()),
            pour: None,
        },
        privacy: Usage { microphone: vec!["Discord".into()], ..Usage::default() },
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
        trend: sample_trend(18.0, 30),
        controls: ControlsView {
            volume: Some((0.62, false)),
            brightness: Some(70),
            mic: Some(false),
            quiet_minutes: None,
            city: Some("Kota: Jakarta".into()),
            fx: ControlsFx::default(),
        },
    }
}

/// Riwayat CPU contoh: `count` detik terakhir yang naik turun di sekitar `base` persen.
fn sample_trend(base: f32, count: usize) -> SystemTrend {
    let cpu = (0..count).map(|i| base + 9.0 * (i as f32 * 0.7).sin() + 5.0 * (i as f32 * 1.9).cos()).collect();
    SystemTrend { cpu, beat: true, scan: None }
}

/// Shelf contoh berisi file (tiruan) dengan nama-nama ini.
fn sample_shelf(names: &[&str]) -> Shelf {
    let mut shelf = Shelf::default();
    shelf.add(names.iter().map(|n| PathBuf::from(format!("C:\\contoh\\{n}"))));
    shelf
}

/// Daftar tugas contoh: teks dan sudah selesai atau belum.
fn sample_tasks(items: &[(&str, bool)]) -> Tasks {
    let mut tasks = Tasks::default();
    for (i, (text, done)) in items.iter().enumerate() {
        tasks.add(text);
        if *done {
            tasks.toggle(i);
        }
    }
    tasks
}

/// Timer yang tidak berjalan, untuk keadaan selain halaman/pil timer.
fn idle_timer() -> TimerView {
    TimerView { running: false, active: false, clock: "00:00".into(), short: String::new(), progress: 0.0, total_minutes: 0, ending: false, fx: None }
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
    call: Option<&'a CallView>,
    prayer: Option<&'a PrayerView>,
    water: Option<&'a WaterView>,
    focus: Option<&'a FocusView>,
    stats: Option<&'a Stats>,
    trend: Option<&'a SystemTrend>,
    timer: Option<&'a TimerView>,
    tasks: Option<&'a Tasks>,
    task_fx: Option<(&'a TaskFx, f32)>,
    shelf: Option<&'a Shelf>,
    controls: Option<&'a ControlsView>,
    page_age: f32,
    /// Jam yang tampil, jamnya (untuk pendaran Beranda), dan animasi sesaat di Beranda.
    time: &'a str,
    hour: u32,
    home_fx: Option<&'a HomeFx>,
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
            call: None,
            prayer: None,
            water: None,
            focus: None,
            stats: None,
            trend: None,
            timer: None,
            tasks: None,
            task_fx: None,
            shelf: None,
            controls: None,
            page_age: f32::MAX,
            time: "19:21",
            hour: 19,
            home_fx: None,
        }
    }
}

/// Panggilan contoh di Zoom; `muted` = mikrofonnya dibisukan.
fn sample_call(muted: bool) -> CallView {
    CallView { app: "Zoom".into(), clock: "12:34".into(), muted: Some(muted), camera: !muted }
}

/// Jadwal sholat contoh (Palopo, 1 November 2026) pukul 14.09: Ashar berikutnya.
fn sample_prayer() -> PrayerView {
    let times = [("Imsak", "04:09"), ("Subuh", "04:19"), ("Terbit", "05:33"), ("Dzuhur", "11:46"), ("Ashar", "15:04"), ("Maghrib", "17:53"), ("Isya", "19:03")];
    PrayerView {
        city: "Palopo".into(),
        hijri: "21 Jumadil Awal 1448 H".into(),
        times: times.iter().enumerate().map(|(i, (name, time))| (*name, time.to_string(), i < 4)).collect(),
        next: 4,
        next_name: "Ashar",
        next_time: "15:04".into(),
        tomorrow: false,
        left: "55 menit".into(),
        current: Some("Dzuhur"),
        progress: 0.62,
        soon: None,
        sky: Sky::Noon,
    }
}

/// Catatan minum contoh dengan "ml" hari ini, sedang dituang dari isi "pour.0" sejak "pour.1" detik.
fn sample_water(ml: u32, pour: Option<(f32, f32)>) -> WaterView {
    let week = vec![(1500, 'S'), (2000, 'S'), (750, 'R'), (1750, 'K'), (2250, 'J'), (1000, 'S'), (ml, 'M')];
    WaterView { ml, goal: 2000, week, next: Some("19:37".into()), pour }
}

/// Istirahat yang baru berjalan sesudah sesi fokus ketiga hari ini selesai.
fn focus_break_like() -> FocusView {
    FocusView { phase: Phase::Break, clock: "04:59".into(), progress: 0.01, sessions: 3, ..sample().focus }
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
        retry: false,
    }
}

/// Agenda contoh.
fn agenda(id: u64, time: &str, text: &str, done: bool) -> AgendaItem {
    AgendaItem { id, time: time.into(), text: text.into(), done, soon: false }
}

/// Kalender contoh: Senin 20 Oktober 2026 dipilih dan hari ini, tiga agenda.
fn sample_calendar() -> CalendarView {
    let week = ["Sen", "Sel", "Rab", "Kam", "Jum", "Sab", "Min"];
    CalendarView {
        month: "Okt".into(),
        year: "2026".into(),
        days: (-CALENDAR_SPAN..=CALENDAR_SPAN)
            .map(|offset| DayView {
                offset,
                name: week[offset.rem_euclid(7) as usize],
                number: (20 + offset) as u32,
                today: offset == 0,
                selected: offset == 0,
                has_events: offset == 0 || offset == 2,
            })
            .collect(),
        events: vec![agenda(1, "", "Ulang tahun Ibu", false), agenda(2, "09.00", "Rapat tim", false), agenda(3, "15.30", "Beli kado", true)],
        scroll: 0,
        center: 0.0,
        shift: None,
        agenda_fx: None,
    }
}

/// Kalender contoh yang sedang meluncur ke hari sesudahnya: `center` dari -1 ke 0, `t` 0..1,
/// dan bulan sebelumnya kalau peralihannya melewati batas bulan.
fn sliding_calendar(center: f32, t: f32, month: Option<(&str, &str)>) -> CalendarView {
    let mut cal = sample_calendar();
    cal.days.iter_mut().for_each(|d| d.selected = d.offset == 1);
    let before = std::mem::replace(&mut cal.events, vec![agenda(7, "10.00", "Kelas yoga", false)]);
    if month.is_some() {
        cal.month = "Nov".into();
    }
    cal.center = center;
    let month = month.map(|(m, y)| (m.to_string(), y.to_string()));
    cal.shift = Some(DayShift { t, dir: 1.0, events: before.into_iter().take(2).collect(), month });
    cal
}

/// Kalender contoh dengan agenda `events` (gulir `scroll`) yang sedang berubah.
fn agenda_calendar(events: Vec<AgendaItem>, scroll: usize, fx: AgendaFx, t: f32) -> CalendarView {
    CalendarView { events, scroll, agenda_fx: Some((fx, t)), ..sample_calendar() }
}

/// Beranda tanpa animasi sesaat.
static STILL_HOME: HomeFx = HomeFx { clock: None, track: None, art: None };

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
        ending: false,
        fx: None,
    };
    let timer_paused = TimerView { running: false, clock: "02:10".into(), short: "3m".into(), progress: 0.57, ..running.clone() };
    let timer_ending = TimerView { clock: "00:07".into(), short: "7d".into(), progress: 0.977, ending: true, ..running.clone() };
    let timer_turn = TimerView { clock: "05:00".into(), short: "5m".into(), progress: 0.001, fx: Some((TimerFx::Flip, 0.25)), ..running.clone() };
    let timer_settle = TimerView { fx: Some((TimerFx::Flip, 0.55)), ..timer_turn.clone() };
    let timer_add = TimerView { clock: "04:42".into(), total_minutes: 6, progress: 0.22, fx: Some((TimerFx::Add(0.26), 0.16)), ..running.clone() };
    let timer_up = TimerView { total_minutes: 5, fx: Some((TimerFx::Done, 0.1)), ..idle_timer() };
    let timer_done_late = TimerView { fx: Some((TimerFx::Done, 1.85)), ..timer_up.clone() };
    let task_fx = |change, rows: &[Option<usize>], gone: &[(&str, bool, usize)], progress| TaskFx {
        change,
        rows: rows.to_vec(),
        scroll: 0,
        gone: gone.iter().map(|(text, done, row)| (text.to_string(), *done, *row)).collect(),
        progress,
    };
    // Tugas "Rapikan folder" baru dicentang: tercoret, lalu turun ke kelompok selesai.
    let tasks_checked = sample_tasks(&[("Kirim revisi desain ke klien", false), ("Beli kopi", true), ("Rapikan folder Downloads yang sudah penuh sekali", true), ("Bayar listrik", false)]);
    let checked = task_fx(TaskChange::Check { index: 2, last: false }, &[Some(0), Some(3), Some(1), Some(2)], &[], 0.25);
    // Tugas terakhir dicentang: konfeti.
    let tasks_all_done = sample_tasks(&[("Beli kopi", true), ("Bayar listrik", true)]);
    let all_done = task_fx(TaskChange::Check { index: 1, last: true }, &[Some(1), Some(0)], &[], 0.5);
    let tasks_added = sample_tasks(&[("Kirim revisi desain ke klien", false), ("Beli kopi", true), ("Telepon ibu", false)]);
    let added = task_fx(TaskChange::Add, &[Some(0), Some(1), None], &[], 0.5);
    let tasks_removed = sample_tasks(&[("Beli kopi", true), ("Rapikan folder Downloads yang sudah penuh sekali", false), ("Bayar listrik", false)]);
    let removed = task_fx(TaskChange::Remove, &[Some(3), Some(1), Some(2)], &[("Kirim revisi desain ke klien", false, 0)], 0.25);
    let tasks_cleared = sample_tasks(&[("Kirim revisi desain ke klien", false), ("Bayar listrik", false)]);
    let cleared = task_fx(TaskChange::ClearDone, &[Some(0), Some(1)], &[("Beli kopi", true, 2), ("Cuci mobil", true, 3)], 0.5);
    let tasks_mixed = sample_tasks(&[("Kirim revisi desain ke klien", false), ("Bayar listrik", false), ("Beli kopi", true)]);
    let muted_controls = ControlsView {
        volume: Some((0.3, true)),
        brightness: None,
        mic: Some(true),
        quiet_minutes: Some(42),
        city: None,
        fx: ControlsFx::default(),
    };
    let controls = |volume: (f32, bool), brightness: u32, mic: bool, quiet_minutes: Option<u32>, fx: ControlsFx| ControlsView {
        volume: Some(volume),
        brightness: Some(brightness),
        mic: Some(mic),
        quiet_minutes,
        city: Some("Kota: Jakarta".into()),
        fx,
    };
    let still = ControlsFx::default();
    let gliding = controls((0.62, false), 70, false, None, ControlsFx { volume: Some((0.3, 0.07)), brightness: Some((0.95, 0.07)), ..still });
    let muting = controls((0.62, true), 70, false, None, ControlsFx { mute: Some(0.12), ..still });
    let mic_on = controls((0.62, false), 70, true, None, ControlsFx { mic: Some(0.1), ..still });
    let mic_open = controls((0.62, false), 70, false, None, ControlsFx { mic: Some(0.12), ..still });
    let quiet = controls((0.62, false), 70, false, Some(42), still);
    let quiet_on = controls((0.62, false), 70, false, Some(60), ControlsFx { quiet: Some(0.15), ..still });
    let dim = controls((0.05, false), 12, false, Some(9), still);
    let targeted = ShelfFx { targeted: true, dash: 1.3, ..ShelfFx::default() };
    let sharing = ShelfFx { share_targeted: true, dash: 2.2, ..ShelfFx::default() };
    let waiting = ShelfFx { targeted: false, dash: 3.1, ..ShelfFx::default() };
    let dropped = ShelfFx {
        glow: Some(0.3),
        tiles: [(3u64, (0.15f32, false)), (1, (0.12, true))].into_iter().collect(),
        ..ShelfFx::default()
    };
    let no_fx = ShelfFx::default();
    let tile = |id: u64, age: f32, removing: bool| ShelfFx { tiles: [(id, (age, removing))].into_iter().collect(), ..ShelfFx::default() };
    let (falling, bouncing, lifting) = (tile(3, 0.14, false), tile(3, 0.36, false), tile(1, 0.15, true));
    let shelf_kinds = sample_shelf(&["Laporan Keuangan.pdf", "Proposal klien.docx", "Anggaran 2026.xlsx", "Arsip foto.zip", "Rekaman rapat.mp3"]);
    // "Project" baru dihapus: dua ubin sisanya merapat ke kiri.
    let mut shelf_pair = sample_shelf(&["Project", "Corporate and Campaign.png", "Infuse Navy Tote Bag.png"]);
    shelf_pair.remove(1);
    let sliding = ShelfFx { slots: [(2u64, 0.55f32), (3, 1.55)].into_iter().collect(), ..ShelfFx::default() };
    let shelf_empty = Shelf::default();
    // "Bersihkan" menyedot ubin ke tulisannya, lalu "Urungkan" memuntahkannya kembali.
    let flowing = |age: f32, back: bool| ShelfFx { flow: Some((age, back)), lean: 1.0, ..ShelfFx::default() };
    let (suck_hop, suck_fly, suck_gulp, suck_last) = (flowing(0.1, false), flowing(0.4, false), flowing(0.62, false), flowing(0.9, false));
    let (chip_wait, chip_fly) = (flowing(0.5, false), flowing(0.8, false));
    let (spit_out, spit_land) = (flowing(0.3, true), flowing(0.62, true));
    let leaning = ShelfFx { lean: 1.0, ..ShelfFx::default() };
    let emptied = ShelfFx { emptied: Some(0.2), undo: true, ..ShelfFx::default() };
    let undo = ShelfFx { undo: true, ..ShelfFx::default() };
    let shelf_full = sample_shelf(&[
        "Laporan Keuangan.pdf", "Proposal klien.docx", "Anggaran 2026.xlsx", "Arsip foto.zip", "Rekaman rapat.mp3",
        "Catatan.txt", "Kontrak.pdf", "Desain.png", "Video.mp4", "Data.csv", "Slide.pptx", "Project",
    ]);
    let roll_minute = HomeFx { clock: Some(("19:59".into(), "Senin, 19 Oktober 2026".into(), 0.3)), ..HomeFx::default() };
    let roll_late = HomeFx { clock: Some(("19:59".into(), "Senin, 19 Oktober 2026".into(), 0.5)), ..HomeFx::default() };
    let roll_midnight = HomeFx { clock: Some(("23:59".into(), "Minggu, 18 Oktober 2026".into(), 0.25)), ..HomeFx::default() };
    let previous_song = TrackFx { title: "Sial".into(), artist: "Mahalini".into(), progress: Some(0.92), dir: 1.0 };
    let track_change = HomeFx { track: Some((previous_song.clone(), 0.2)), art: Some(0.2), ..HomeFx::default() };
    let track_late = HomeFx { track: Some((previous_song, 0.34)), art: Some(0.34), ..HomeFx::default() };
    let cal_sliding = sliding_calendar(-0.5, 0.45, None);
    let cal_month = sliding_calendar(-0.7, 0.3, Some(("Okt", "2026")));
    let checked_agenda = AgendaFx { change: AgendaChange::Check(2), rows: Vec::new(), gone: None };
    let mut checked_events = sample_calendar().events;
    checked_events[1].done = true;
    let cal_check = agenda_calendar(checked_events, 0, checked_agenda, 0.22);
    // "12.00 Makan siang" baru ditambahkan di antara dua agenda; daftarnya digulir supaya terlihat.
    let added_agenda = AgendaFx { change: AgendaChange::Add(9), rows: vec![(1, 0.0), (2, 1.0), (3, 2.0)], gone: None };
    let added_events = vec![
        agenda(1, "", "Ulang tahun Ibu", false),
        agenda(2, "09.00", "Rapat tim", false),
        agenda(9, "12.00", "Makan siang", false),
        agenda(3, "15.30", "Beli kado", true),
    ];
    let cal_add = agenda_calendar(added_events, 1, added_agenda, 0.32);
    let removed_agenda =
        AgendaFx { change: AgendaChange::Remove, rows: vec![(1, 0.0), (3, 2.0)], gone: Some((agenda(2, "09.00", "Rapat tim", false), 1.0)) };
    let removed_events = vec![agenda(1, "", "Ulang tahun Ibu", false), agenda(3, "15.30", "Beli kado", true)];
    let cal_remove = agenda_calendar(removed_events, 0, removed_agenda, 0.2);
    let mut cal_soon = sample_calendar();
    cal_soon.events[1] = AgendaItem { time: "15 mnt lagi".into(), soon: true, ..agenda(2, "", "Rapat tim", false) };
    let roll = Roll { from: "61".into(), t: 0.45, up: true };
    let charging = Hud::Power { plugged: true, percent: Some(80), low: false };
    let calendar = sample_calendar();
    let mut other_day = sample_calendar();
    other_day.days.iter_mut().for_each(|d| d.selected = d.offset == 2);
    other_day.events.clear();
    let lyrics = sample_lyrics(1.0);
    let lyrics_scrolling = sample_lyrics(0.4);
    let searching = LyricsView { lines: Default::default(), scroll: 1.0, status: Some("Mencari lirik…"), retry: false };
    let busy = LyricsView { status: Some("Server lirik sibuk, mencoba lagi…"), retry: true, ..searching.clone() };
    let missing = LyricsView { status: Some("Lirik tidak tersedia"), retry: true, ..searching.clone() };
    let seeking = MediaView { seeking: true, progress: Some(0.62), elapsed: "1:46".into(), remaining: "-1:07".into(), ..sample().media };
    let agenda_due = Hud::Agenda { text: "Rapat tim".into(), time: "09.00".into() };
    let call = sample_call(false);
    let call_muted = sample_call(true);
    let call_long = CallView { clock: "1:02:03".into(), ..sample_call(false) };
    let call_locked = CallView { app: String::new(), ..sample_call(false) };
    let call_voice = CallView { camera: false, ..sample_call(false) };
    let prayer = sample_prayer();
    let prayer_soon = PrayerView { soon: Some("8m".into()), ..sample_prayer() };
    let mut prayer_tomorrow = PrayerView {
        next: 1,
        next_name: "Subuh",
        next_time: "04:19".into(),
        tomorrow: true,
        left: "8 jam 58 menit".into(),
        current: Some("Isya"),
        progress: 0.03,
        sky: Sky::Night,
        ..sample_prayer()
    };
    prayer_tomorrow.times.iter_mut().for_each(|t| t.2 = false);
    let water_tap = sample_water(1250, Some((0.5, 0.2)));
    let water_pour = sample_water(1250, Some((0.5, 0.55)));
    let water_settle = sample_water(1250, Some((0.5, 1.1)));
    let water_goal = sample_water(2000, Some((0.875, 1.45)));
    let water_undo = sample_water(1000, Some((0.625, 0.25)));
    let water_empty = sample_water(0, None);
    let water_full = sample_water(2250, None);
    let prayer_dawn = PrayerView { sky: Sky::Dawn, current: Some("Subuh"), ..sample_prayer() };
    let prayer_morning = PrayerView { sky: Sky::Morning, current: None, ..sample_prayer() };
    let prayer_afternoon = PrayerView { sky: Sky::Afternoon, current: Some("Ashar"), ..sample_prayer() };
    let prayer_dusk = PrayerView { sky: Sky::Dusk, current: Some("Maghrib"), ..sample_prayer() };
    let prayer_due = Hud::Prayer { name: "Ashar", time: "15:04".into(), city: "Palopo".into() };
    let mic_off = Hud::Mic { muted: true };
    let call_ended = Hud::CallEnded { app: "Zoom".into(), length: "42 menit".into() };
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
    let system_busy = Stats { cpu: 81.0, ..sample().system };
    let system_hot = Stats { gpu: Some(96.0), gpu_temp: Some(89.0), ..sample().system };
    let busy_trend = sample_trend(66.0, 30);
    let young_trend = SystemTrend { beat: false, ..sample_trend(20.0, 3) };
    let scanning = SystemTrend { scan: Some(0.5), ..SystemTrend::default() };
    let focus_break = FocusView { phase: Phase::Break, clock: "03:12".into(), progress: 0.36, sessions: 3, ..sample().focus };
    let focus_paused = FocusView { running: false, ..sample().focus };
    let focus_fresh = FocusView { running: false, active: false, clock: "25:00".into(), progress: 0.0, sessions: 0, ..sample().focus };
    let focus_many = FocusView { sessions: 11, ..sample().focus };
    let focus_wind = FocusView { clock: "25:00".into(), progress: 0.0, fx: Some((FocusFx::Wind, 0.3)), ..sample().focus };
    let focus_done = FocusView { fx: Some((FocusFx::Done(Phase::Focus), 0.16)), ..focus_break_like() };
    let focus_landed = FocusView { fx: Some((FocusFx::Done(Phase::Focus), 0.42)), ..focus_break_like() };
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
        State { media: true, lyrics_button: Some(true), lyrics: Some(&busy), lyric_mix: 1.0, ..State::new("player_lyrics_busy", View::PanelWide, Page::Home) },
        State { media: true, lyrics_button: Some(true), lyrics: Some(&missing), lyric_mix: 1.0, hovered: Some(Hit::LyricsRetry), ..State::new("player_lyrics_retry", View::PanelWide, Page::Home) },
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
        State { page_age: 0.22, ..State::new("controls_enter", View::Panel, Page::Controls) },
        State { page_age: 0.42, ..State::new("controls_enter_late", View::Panel, Page::Controls) },
        State { controls: Some(&gliding), ..State::new("controls_glide", View::Panel, Page::Controls) },
        State { controls: Some(&muting), hovered: Some(Hit::VolumeMute), ..State::new("controls_mute", View::Panel, Page::Controls) },
        State { controls: Some(&mic_on), hovered: Some(Hit::MicToggle), ..State::new("controls_mic_on", View::Panel, Page::Controls) },
        State { controls: Some(&mic_open), hovered: Some(Hit::MicToggle), ..State::new("controls_mic_off", View::Panel, Page::Controls) },
        State { controls: Some(&quiet), ..State::new("controls_quiet", View::Panel, Page::Controls) },
        State { controls: Some(&quiet_on), ..State::new("controls_quiet_on", View::Panel, Page::Controls) },
        State { controls: Some(&dim), ..State::new("controls_dim", View::Panel, Page::Controls) },
        State { theme: Theme::Light, ..State::new("controls_light_muted", View::Panel, Page::Controls) },
        State { hovered: Some(Hit::TimerPreset(5)), ..State::new("page_timer", View::Panel, Page::Timer) },
        State::new("page_timer_idle", View::Panel, Page::Timer),
        State { timer: Some(&timer_paused), hovered: Some(Hit::TimerToggle), ..State::new("page_timer_paused", View::Panel, Page::Timer) },
        State { timer: Some(&timer_ending), ..State::new("page_timer_ending", View::Panel, Page::Timer) },
        State { theme: Theme::Light, ..State::new("page_timer_light", View::Panel, Page::Timer) },
        State { theme: Theme::Light, ..State::new("page_timer_light_idle", View::Panel, Page::Timer) },
        State { tasks: Some(&tasks_checked), task_fx: Some((&checked, 0.26)), ..State::new("page_tasks_check", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_checked), task_fx: Some((&checked, 0.58)), ..State::new("page_tasks_check_slide", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_all_done), task_fx: Some((&all_done, 0.45)), ..State::new("page_tasks_all_done", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_added), task_fx: Some((&added, 0.3)), ..State::new("page_tasks_add", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_removed), task_fx: Some((&removed, 0.14)), ..State::new("page_tasks_remove", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_cleared), task_fx: Some((&cleared, 0.12)), ..State::new("page_tasks_clear", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_mixed), page_age: 0.3, ..State::new("page_tasks_enter", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_mixed), ..State::new("page_tasks_mixed", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_mixed), theme: Theme::Light, ..State::new("page_tasks_light", View::Panel, Page::Tasks) },
        State { tasks: Some(&tasks_all_done), ..State::new("page_tasks_all_done_settled", View::Panel, Page::Tasks) },
        State { timer: Some(&timer_turn), hovered: Some(Hit::TimerPreset(5)), ..State::new("page_timer_flip", View::Panel, Page::Timer) },
        State { timer: Some(&timer_settle), ..State::new("page_timer_flip_settle", View::Panel, Page::Timer) },
        State { timer: Some(&timer_add), hovered: Some(Hit::TimerAdd), ..State::new("page_timer_add", View::Panel, Page::Timer) },
        State { timer: Some(&timer_up), ..State::new("page_timer_done", View::Panel, Page::Timer) },
        State { timer: Some(&timer_done_late), ..State::new("page_timer_done_late", View::Panel, Page::Timer) },
        State { page_age: 0.3, ..State::new("page_timer_enter", View::Panel, Page::Timer) },
        State { page_age: 0.35, ..State::new("page_timer_enter_idle", View::Panel, Page::Timer) },
        State::new("pill_timer", View::Pill, Page::Home),
        State { hud: Some(&timer_done), ..State::new("hud_timer", View::Hud, Page::Home) },
        State { weather: Some("28° Mendung · Jakarta"), ..State::new("page_clock_weather", View::Panel, Page::Home) },
        State { size: Some((188.0, 34.0)), shadow: Some(0.6), ..State::new("pill_peek", View::Pill, Page::Home) },
        State { fx: Some(&waiting), ..State::new("drop_waiting", View::Panel, Page::Shelf) },
        State { fx: Some(&targeted), ..State::new("drop_targeted", View::Panel, Page::Shelf) },
        State { fx: Some(&dropped), ..State::new("shelf_dropped", View::Panel, Page::Shelf) },
        State { fx: Some(&falling), ..State::new("page_shelf_drop_fall", View::Panel, Page::Shelf) },
        State { fx: Some(&bouncing), ..State::new("page_shelf_drop_bounce", View::Panel, Page::Shelf) },
        State { fx: Some(&lifting), ..State::new("page_shelf_lift", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_pair), fx: Some(&sliding), ..State::new("page_shelf_slide", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), ..State::new("page_shelf_kinds", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), theme: Theme::Light, ..State::new("page_shelf_light", View::Panel, Page::Shelf) },
        State { page_age: 0.3, ..State::new("page_shelf_enter", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_empty), page_age: 0.32, ..State::new("page_shelf_empty_enter", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&leaning), hovered: Some(Hit::ShelfClear), ..State::new("shelf_clear_lean", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&suck_hop), ..State::new("shelf_vacuum_hop", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&suck_fly), ..State::new("shelf_vacuum_fly", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&suck_gulp), ..State::new("shelf_vacuum_gulp", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&suck_last), ..State::new("shelf_vacuum_last", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_full), fx: Some(&chip_wait), ..State::new("shelf_vacuum_chip", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_full), fx: Some(&chip_fly), ..State::new("shelf_vacuum_chip_fly", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_empty), fx: Some(&emptied), ..State::new("shelf_emptied", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_empty), fx: Some(&undo), hovered: Some(Hit::ShelfUndo), ..State::new("shelf_undo", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&spit_out), ..State::new("shelf_spit", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&spit_land), ..State::new("shelf_spit_land", View::Panel, Page::Shelf) },
        State { shelf: Some(&shelf_kinds), fx: Some(&suck_fly), theme: Theme::Light, ..State::new("shelf_vacuum_light", View::Panel, Page::Shelf) },
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
        State { call: Some(&call), ..State::new("pill_call", View::Pill, Page::Home) },
        State { call: Some(&call_voice), media: true, ..State::new("pill_call_music", View::Pill, Page::Home) },
        State { call: Some(&call), media: true, lyric: Some(lyric), ..State::new("pill_call_lyric", View::LyricPill, Page::Home) },
        State { call: Some(&call_muted), media: true, lyric: Some(lyric), ..State::new("pill_call_lyric_muted", View::LyricPill, Page::Home) },
        State { call: Some(&call), media: true, battery: Some((76, false)), ..State::new("home_call_player", View::Panel, Page::Home) },
        State { call: Some(&call_muted), media: true, hovered: Some(Hit::CallMute), ..State::new("home_call_player_muted", View::Panel, Page::Home) },
        State { call: Some(&call_locked), media: true, locked: true, ..State::new("home_call_player_locked", View::Panel, Page::Home) },
        State { call: Some(&call_long), media: true, lyrics_button: Some(true), lyrics: Some(&lyrics), lyric_mix: 1.0, ..State::new("home_call_lyrics", View::PanelWide, Page::Home) },
        State { call: Some(&call_muted), hovered: Some(Hit::CallMute), ..State::new("pill_call_muted", View::Pill, Page::Home) },
        State { call: Some(&call_long), theme: Theme::Light, ..State::new("pill_call_long", View::Pill, Page::Home) },
        State { call: Some(&call), hovered: Some(Hit::CallMute), ..State::new("home_call", View::Panel, Page::Home) },
        State { call: Some(&call_muted), ..State::new("home_call_muted", View::Panel, Page::Home) },
        State { call: Some(&call), media: true, calendar: Some(&calendar), ..State::new("home_call_wide", View::PanelWide, Page::Home) },
        State { call: Some(&call_locked), locked: true, ..State::new("home_call_locked", View::Panel, Page::Home) },
        State { call: Some(&call), theme: Theme::Glass, ..State::new("home_call_glass", View::Panel, Page::Home) },
        State { hud: Some(&mic_off), ..State::new("strip_mic", View::Strip, Page::Home) },
        State { hud: Some(&mic_off), ..State::new("hud_mic", View::Hud, Page::Home) },
        State { hud: Some(&call_ended), ..State::new("hud_call_ended", View::Hud, Page::Home) },
        State { prayer: Some(&prayer), ..State::new("page_prayer", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer), theme: Theme::Light, ..State::new("page_prayer_light", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_tomorrow), ..State::new("page_prayer_tomorrow", View::Panel, Page::Prayer) },
        State { hovered: Some(Hit::PrayerCity), ..State::new("page_prayer_no_city", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_soon), ..State::new("pill_prayer_soon", View::Pill, Page::Home) },
        State { prayer: Some(&prayer_soon), media: true, ..State::new("pill_prayer_music", View::Pill, Page::Home) },
        State { hud: Some(&prayer_due), ..State::new("hud_prayer", View::Hud, Page::Home) },
        State { focus: Some(&focus_break), ..State::new("page_focus_break", View::Panel, Page::Focus) },
        State { stats: Some(&system_busy), trend: Some(&busy_trend), ..State::new("page_system_busy", View::Panel, Page::System) },
        State { stats: Some(&system_hot), ..State::new("page_system_hot", View::Panel, Page::System) },
        State { trend: Some(&scanning), ..State::new("page_system_scan", View::Panel, Page::System) },
        State { trend: Some(&young_trend), ..State::new("page_system_young", View::Panel, Page::System) },
        State { theme: Theme::Light, ..State::new("page_system_light", View::Panel, Page::System) },
        State { page_age: 0.3, ..State::new("page_system_enter", View::Panel, Page::System) },
        State { focus: Some(&focus_paused), hovered: Some(Hit::FocusToggle), ..State::new("page_focus_paused", View::Panel, Page::Focus) },
        State { focus: Some(&focus_fresh), ..State::new("page_focus_fresh", View::Panel, Page::Focus) },
        State { focus: Some(&focus_many), ..State::new("page_focus_many", View::Panel, Page::Focus) },
        State { theme: Theme::Light, ..State::new("page_focus_light", View::Panel, Page::Focus) },
        State { focus: Some(&focus_wind), ..State::new("page_focus_wind", View::Panel, Page::Focus) },
        State { focus: Some(&focus_done), ..State::new("page_focus_done", View::Panel, Page::Focus) },
        State { focus: Some(&focus_landed), ..State::new("page_focus_done_landed", View::Panel, Page::Focus) },
        State { page_age: 0.28, ..State::new("page_focus_enter", View::Panel, Page::Focus) },
        State { prayer: Some(&prayer_dawn), ..State::new("page_prayer_dawn", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_morning), ..State::new("page_prayer_morning", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_afternoon), ..State::new("page_prayer_afternoon", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_dusk), ..State::new("page_prayer_dusk", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_tomorrow), theme: Theme::Light, ..State::new("page_prayer_night_light", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer), page_age: 0.3, ..State::new("page_prayer_enter", View::Panel, Page::Prayer) },
        State { prayer: Some(&prayer_tomorrow), page_age: 0.35, ..State::new("page_prayer_enter_night", View::Panel, Page::Prayer) },
        State { page_age: 0.3, ..State::new("page_water_enter", View::Panel, Page::Water) },
        State { water: Some(&water_tap), hovered: Some(Hit::WaterDrink), ..State::new("page_water_pour_tap", View::Panel, Page::Water) },
        State { water: Some(&water_pour), hovered: Some(Hit::WaterDrink), ..State::new("page_water_pour", View::Panel, Page::Water) },
        State { water: Some(&water_settle), ..State::new("page_water_pour_settle", View::Panel, Page::Water) },
        State { water: Some(&water_goal), ..State::new("page_water_goal", View::Panel, Page::Water) },
        State { water: Some(&water_undo), hovered: Some(Hit::WaterUndo), ..State::new("page_water_undo", View::Panel, Page::Water) },
        State { water: Some(&water_empty), ..State::new("page_water_empty", View::Panel, Page::Water) },
        State { water: Some(&water_full), ..State::new("page_water_full", View::Panel, Page::Water) },
        State { water: Some(&water_pour), theme: Theme::Light, ..State::new("page_water_light", View::Panel, Page::Water) },
        State { water: Some(&water_pour), theme: Theme::Glass, ..State::new("page_water_glass", View::Panel, Page::Water) },
        State { hour: 7, ..State::new("home_glow_morning", View::Panel, Page::Home) },
        State { hour: 12, ..State::new("home_glow_noon", View::Panel, Page::Home) },
        State { hour: 16, calendar: Some(&calendar), ..State::new("home_glow_evening", View::PanelWide, Page::Home) },
        State { hour: 7, theme: Theme::Light, ..State::new("home_glow_light", View::Panel, Page::Home) },
        State { time: "20:00", home_fx: Some(&roll_minute), ..State::new("home_clock_roll", View::Panel, Page::Home) },
        State { time: "20:00", home_fx: Some(&roll_late), ..State::new("home_clock_roll_late", View::Panel, Page::Home) },
        State { time: "00:00", home_fx: Some(&roll_midnight), ..State::new("home_clock_midnight", View::Panel, Page::Home) },
        State { time: "20:00", media: true, home_fx: Some(&roll_minute), ..State::new("home_clock_roll_player", View::Panel, Page::Home) },
        State { calendar: Some(&cal_sliding), ..State::new("home_calendar_slide", View::PanelWide, Page::Home) },
        State { calendar: Some(&cal_month), ..State::new("home_calendar_month", View::PanelWide, Page::Home) },
        State { calendar: Some(&cal_check), ..State::new("home_agenda_check", View::PanelWide, Page::Home) },
        State { calendar: Some(&cal_add), ..State::new("home_agenda_add", View::PanelWide, Page::Home) },
        State { calendar: Some(&cal_remove), ..State::new("home_agenda_remove", View::PanelWide, Page::Home) },
        State { calendar: Some(&cal_soon), ..State::new("home_agenda_soon", View::PanelWide, Page::Home) },
        State { media: true, home_fx: Some(&track_change), ..State::new("home_track_change", View::Panel, Page::Home) },
        State { media: true, home_fx: Some(&track_late), ..State::new("home_track_late", View::Panel, Page::Home) },
        State { calendar: Some(&calendar), page_age: 0.22, ..State::new("home_enter", View::PanelWide, Page::Home) },
        State { calendar: Some(&calendar), page_age: 0.42, ..State::new("home_enter_late", View::PanelWide, Page::Home) },
        State { media: true, page_age: 0.25, ..State::new("home_enter_media", View::Panel, Page::Home) },
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
            time: state.time,
            date: "Senin, 19 Oktober 2026",
            day: "Rab 21 Okt 2026",
            greeting: "Selamat malam",
            hour: state.hour,
            home_fx: state.home_fx.unwrap_or(&STILL_HOME),
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
            page_age: state.page_age,
            shelf: state.shelf.unwrap_or(&data.shelf),
            shelf_fx: state.fx.unwrap_or(&no_fx),
            tasks: state.tasks.unwrap_or(&data.tasks),
            task_fx: state.task_fx,
            focus: match state.focus {
                Some(focus) => focus,
                None if state.name.starts_with("mood") || state.name == "size_small" || state.name.starts_with("pill_prayer") => &idle,
                None => &data.focus,
            },
            water: state.water.unwrap_or(&data.water),
            privacy: &data.privacy,
            system: if state.name == "page_system_scan" { None } else { Some(state.stats.unwrap_or(&data.system)) },
            trend: state.trend.unwrap_or(&data.trend),
            uptime: "3 j 12 m",
            hovered: state.hovered,
            seconds: 1.3,
            locked: state.locked,
            timer: match state.timer {
                Some(timer) => timer,
                None if state.name.contains("timer") && !state.name.ends_with("idle") => &running,
                None => &data.timer,
            },
            controls: state.controls.unwrap_or(if state.name.ends_with("muted") { &muted_controls } else { &data.controls }),
            weather: state.weather,
            city_draft: state.city_draft,
            calendar: state.calendar,
            agenda_draft: state.agenda_draft,
            download: state.download,
            call: state.call,
            prayer: state.prayer,
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
        hour: 19,
        home_fx: &STILL_HOME,
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
        page_age: f32::MAX,
        shelf: &data.shelf,
        shelf_fx: &data.shelf_fx,
        tasks: &data.tasks,
        task_fx: None,
        focus: &data.focus,
        water: &data.water,
        privacy: &data.privacy,
        system: None,
        trend: &data.trend,
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
        call: None,
        prayer: None,
        welcome: false,
    }
}

/// Tanpa kota, halaman Sholat punya tombol "Pilih kota"; dengan jadwal, tombol itu hilang.
#[test]
fn prayer_city_button() {
    let data = sample();
    let prayer = sample_prayer();
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let x0 = CANVAS.0 / 2.0 - View::Panel.size().0 / 2.0;
    let mut scene = scene_for(&data, View::Panel, Page::Prayer, false);
    renderer.draw(&scene).unwrap();
    let button = x0 + MARGIN_FOR_TEST + 40.0;
    assert_eq!(renderer.hit_at(button, 128.0), Some(Hit::PrayerCity));
    scene.prayer = Some(&prayer);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(button, 128.0), None);
}

/// Tombol bisu panggilan bisa diklik di pil panggilan (di kiri, di tempat mata maskot) dan di
/// Beranda. Selama lagu diputar, pil musik dan pil lirik tetap memakai sampul dan tidak punya
/// bagian yang bisa diklik, sama seperti tanpa panggilan.
#[test]
fn call_mute_hits() {
    let data = sample();
    let call = sample_call(false);
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let play = |renderer: &Renderer| (0..400).any(|x| renderer.hit_at(x as f32 * 2.0, 125.0) == Some(Hit::Media(Button::PlayPause)));
    for media in [false, true] {
        let x0 = CANVAS.0 / 2.0 - View::Pill.size().0 / 2.0;
        let mut scene = scene_for(&data, View::Pill, Page::Home, media);
        renderer.draw(&scene).unwrap();
        assert_eq!(renderer.hit_at(x0 + 18.0, 15.0), None);
        scene.call = Some(&call);
        renderer.draw(&scene).unwrap();
        let button = if media { None } else { Some(Hit::CallMute) };
        assert_eq!(renderer.hit_at(x0 + 18.0, 15.0), button);
        assert_eq!(renderer.hit_at(x0 + 90.0, 15.0), None);
    }

    let x0 = CANVAS.0 / 2.0 - View::LyricPill.size().0 / 2.0;
    let mut scene = scene_for(&data, View::LyricPill, Page::Home, true);
    scene.lyric = Some("Baris lirik");
    scene.call = Some(&call);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(x0 + 18.0, 15.0), None);

    // Tanpa pemutar: kartu panggilan dengan tombol bisu besar.
    let x1 = CANVAS.0 / 2.0 + View::Panel.size().0 / 2.0;
    let mut scene = scene_for(&data, View::Panel, Page::Home, false);
    scene.call = Some(&call);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(x1 - MARGIN_FOR_TEST - 40.0, 70.0), Some(Hit::CallMute));
    assert!(!play(&renderer));

    // Dengan pemutar: pemutar tetap ada, tombol bisu kecil di kepalanya.
    let mut scene = scene_for(&data, View::Panel, Page::Home, true);
    scene.call = Some(&call);
    renderer.draw(&scene).unwrap();
    assert_eq!(renderer.hit_at(x1 - MARGIN_FOR_TEST - 32.0, 22.0), Some(Hit::CallMute));
    assert_ne!(renderer.hit_at(x1 - MARGIN_FOR_TEST - 40.0, 70.0), Some(Hit::CallMute));
    assert!(play(&renderer));
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

/// Garis tipis saat jendela di-maximize tetap menunjukkan titik privasi selama mikrofon dipakai.
#[test]
fn slim_line_keeps_privacy_dot() {
    let data = sample();
    let probe = |privacy: &Usage| {
        let mut renderer = Renderer::new(96, Theme::Black).unwrap();
        let mut scene = scene_for(&data, View::Slim, Page::Home, false);
        scene.privacy = privacy;
        renderer.draw(&scene).unwrap();
        let _ = unsafe { GdiFlush() };
        let width = renderer.surface.width as usize;
        let (w, h) = View::Slim.size();
        let x = (CANVAS.0 / 2.0 + w / 2.0 - 8.0) as usize;
        renderer.surface.pixels()[(h / 2.0) as usize * width + x]
    };
    let red = |px: u32| (px >> 16) & 0xFF;
    // Contoh: Discord sedang memakai mikrofon, jadi titiknya oranye.
    assert!(red(probe(&data.privacy)) > 150, "titik privasi tidak terlihat");
    assert_eq!(red(probe(&Usage::default())), 0, "tanpa kamera/mikrofon garisnya polos");
}

/// Tulis ulang `packaging/notch.ico` dari maskot ikon tray (ikon notch.exe di menu Start,
/// Explorer, dan Pengaturan > Aplikasi), beserta logo paket identitas di
/// `packaging/package/assets` (ikon jendela Pengaturan di taskbar saat pesan masuk aktif).
/// Jalankan dengan `cargo test write_app_icon -- --ignored` setelah maskotnya diubah.
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
    let packaging = Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging");
    std::fs::write(packaging.join("notch.ico"), ico).unwrap();

    // Logo paket: maskot yang sama, jadi terlihat di taskbar terang maupun gelap.
    let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
    for (name, size) in [("Square44x44Logo", 44), ("Square150x150Logo", 150), ("StoreLogo", 50)] {
        let mut surface = icon_surface(size).unwrap();
        // `save_png` menerima alfa premultiplied seperti hasil Direct2D.
        let pixels: Vec<u32> = surface
            .pixels()
            .iter()
            .map(|&px| {
                let a = px >> 24;
                let pre = |c: u32| (c * a + 127) / 255;
                (a << 24) | (pre((px >> 16) & 0xFF) << 16) | (pre((px >> 8) & 0xFF) << 8) | pre(px & 0xFF)
            })
            .collect();
        let path = packaging.join("package").join("assets").join(format!("{name}.png"));
        aset::save_png(&path, &pixels, size, (0, 0, size, size));
    }
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

/// Keterangan "Lirik tidak tersedia" bisa diklik untuk mencari lagi; "Mencari lirik…" tidak.
#[test]
fn lyrics_status_retry() {
    let data = sample();
    let renderer = Renderer::new(96, Theme::Black).unwrap();
    let x0 = CANVAS.0 / 2.0 - View::PanelWide.size().0 / 2.0;
    let status = (x0 + MARGIN_FOR_TEST + 300.0 + 24.0 + 60.0, 78.0);
    for (retry, expected) in [(true, Some(Hit::LyricsRetry)), (false, None)] {
        let lyrics = LyricsView { lines: Default::default(), scroll: 1.0, status: Some("Lirik tidak tersedia"), retry };
        let mut scene = scene_for(&data, View::PanelWide, Page::Home, true);
        scene.lyric_mix = 1.0;
        scene.lyrics = Some(&lyrics);
        renderer.draw(&scene).unwrap();
        assert_eq!(renderer.hit_at(status.0, status.1), expected);
    }
}

/// Pendaran bawah panel mengikuti warna sampul; tanpa musik, warna waktu hari.
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
    // Contoh jam 19.21: malam berwarna nila, biru paling kuat.
    assert!(channel(plain, 0) > 20 && channel(plain, 0) > channel(plain, 8), "pendaran malam {plain:08X}");
    // Sampul contoh biru muda (0x4FC3F7): biru paling kuat, merah paling lemah.
    assert!(channel(glowing, 0) > 30 && channel(glowing, 0) > channel(glowing, 16), "pendaran {glowing:08X}");
}

/// Pendaran halaman Kontrol: oranye selama mikrofon bisu (juga kalau senyap), biru selama senyap
/// saja, dan tidak ada kalau keduanya mati. Tombolnya tetap bisa diklik di tempatnya.
#[test]
fn controls_glow_follows_mic_and_quiet() {
    let data = sample();
    let (width, h) = View::Panel.size();
    let probe = |mic: bool, quiet: Option<u32>| {
        let view = ControlsView { mic: Some(mic), quiet_minutes: quiet, city: None, ..sample().controls };
        let mut scene = scene_for(&data, View::Panel, Page::Controls, false);
        scene.controls = &view;
        let mut renderer = Renderer::new(96, Theme::Black).unwrap();
        renderer.draw(&scene).unwrap();
        let _ = unsafe { GdiFlush() };
        let x0 = CANVAS.0 / 2.0 - width / 2.0;
        let hits = (renderer.hit_at(x0 + MARGIN_FOR_TEST + 60.0, 128.0), renderer.hit_at(x0 + MARGIN_FOR_TEST + 188.0, 128.0));
        assert_eq!(hits, (Some(Hit::MicToggle), Some(Hit::QuietToggle)));
        let stride = renderer.surface.width as usize;
        let x = (CANVAS.0 / 2.0 + 150.0) as usize;
        renderer.surface.pixels()[(h as usize - 6) * stride + x]
    };
    let channel = |px: u32, shift: u32| (px >> shift) & 0xFF;
    let (none, muted, quiet, both) = (probe(false, None), probe(true, None), probe(false, Some(42)), probe(true, Some(42)));
    assert!(channel(none, 16) < 8 && channel(none, 0) < 8, "tanpa pendaran {none:08X}");
    assert!(channel(muted, 16) > 30 && channel(muted, 16) > channel(muted, 0), "oranye {muted:08X}");
    assert!(channel(quiet, 0) > 30 && channel(quiet, 0) > channel(quiet, 16), "biru {quiet:08X}");
    let near = |a: u32, b: u32| [0, 8, 16].iter().all(|&shift| channel(a, shift).abs_diff(channel(b, shift)) <= 2);
    assert!(near(both, muted), "oranye menang: {both:08X} vs {muted:08X}");
}

/// Selama sedotan "Bersihkan" ubin tidak bisa diklik; sesudahnya "Urungkan" bisa diklik di
/// tempat "Bersihkan" tadi.
#[test]
fn shelf_vacuum_blocks_tiles_and_offers_undo() {
    let data = sample();
    let (width, _) = View::Panel.size();
    let x0 = CANVAS.0 / 2.0 - width / 2.0;
    let render = |shelf: &Shelf, fx: &ShelfFx| {
        let mut scene = scene_for(&data, View::Panel, Page::Shelf, false);
        scene.shelf = shelf;
        scene.shelf_fx = fx;
        let renderer = Renderer::new(96, Theme::Black).unwrap();
        renderer.draw(&scene).unwrap();
        renderer
    };
    let tile = (x0 + MARGIN_FOR_TEST + 34.0, 70.0);
    let corner = (x0 + width - MARGIN_FOR_TEST - 20.0, 22.0);
    let still = render(&data.shelf, &ShelfFx::default());
    assert!(matches!(still.hit_at(tile.0, tile.1), Some(Hit::ShelfItem(_))));
    assert_eq!(still.hit_at(corner.0, corner.1), Some(Hit::ShelfClear));
    let sucking = render(&data.shelf, &ShelfFx { flow: Some((0.3, false)), ..ShelfFx::default() });
    assert_eq!(sucking.hit_at(tile.0, tile.1), Some(Hit::ShelfStrip));
    assert_eq!(sucking.hit_at(corner.0, corner.1), None);
    let empty = Shelf::default();
    let undo = render(&empty, &ShelfFx { undo: true, ..ShelfFx::default() });
    assert_eq!(undo.hit_at(corner.0, corner.1), Some(Hit::ShelfUndo));
    assert_eq!(render(&empty, &ShelfFx::default()).hit_at(corner.0, corner.1), None);
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

/// Jendela pengaturan, setiap kategori dalam tema terang dan gelap, ke `target/preview/` sebagai
/// PNG supaya tampilannya bisa dicek tanpa membuka jendelanya.
#[test]
fn settings_window_previews() {
    use crate::settings_window::{Category, preview};
    let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("preview");
    std::fs::create_dir_all(&dir).unwrap();
    let state = crate::tray::MenuState {
        visible: true,
        autostart: true,
        settings: crate::settings::Settings::default(),
        messages_available: false,
        messages_denied: false,
        hotkey_taken: false,
        can_activate: true,
        city: Some("Jakarta".into()),
        call_apps: vec![("Discord".into(), false), ("Zoom Workplace".into(), true), ("WhatsApp".into(), true)],
    };
    for (i, category) in Category::ALL.into_iter().enumerate() {
        for (dark, name) in [(false, "terang"), (true, "gelap")] {
            let (pixels, w, h) = preview(category, dark, &state, false);
            aset::save_png(&dir.join(format!("pengaturan-{i}-{name}.png")), &pixels, w, (0, 0, w, h));
        }
        // Jendela paling sempit: keterangan panjang harus turun ke baris berikutnya.
        let (pixels, w, h) = preview(category, false, &state, true);
        aset::save_png(&dir.join(format!("pengaturan-{i}-sempit.png")), &pixels, w, (0, 0, w, h));
    }
}
