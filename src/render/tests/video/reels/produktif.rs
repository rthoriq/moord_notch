//! Bab 5, produktif: pomodoro, timer cepat, minum air, tugas, dan kalender dengan agenda.

use super::*;

const ORANGE: u32 = 0xFF9F0A;
const WATER: u32 = 0x0A84FF;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Produktif",
        tint: ORANGE,
        shots: vec![
            Shot { beats: 8, energy: Energy::Full, draw: pomodoro, sound: pomodoro_sound, blur: &[(0.0, 1.4, 4)] },
            Shot { beats: 9, energy: Energy::Full, draw: timer, sound: timer_sound, blur: &[(1.9, 2.7, 4)] },
            Shot { beats: 9, energy: Energy::Full, draw: water, sound: water_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: tasks, sound: tasks_sound, blur: &[] },
            Shot { beats: 8, energy: Energy::Light, draw: calendar, sound: calendar_sound, blur: &[] },
        ],
    }
}

const PANEL: (f32, f32, f32) = (540.0, 470.0, 2.1);

/// Titik di panel (DIP) dalam piksel video.
fn spot(x: f32, y: f32) -> (f32, f32) {
    let (ax, ay, k) = PANEL;
    (ax + x * k, ay + y * k)
}

/// Kursor yang berpindah antar titik: `path` berisi (detik tiba, titik).
fn cursor_path(studio: &mut Studio, t: f64, path: &[(f64, (f32, f32))], alpha: f32) {
    let mut from = path[0].1;
    let mut start: f64 = 0.0;
    for &(arrive, to) in path {
        if t < arrive {
            let p = ease_in_out(span(t, start.max(arrive - 0.45), 0.45));
            studio.cursor(mix(from.0, to.0, p) + 4.0, mix(from.1, to.1, p) + 6.0, 1.5, alpha);
            return;
        }
        from = to;
        start = arrive;
    }
    studio.cursor(from.0 + 4.0, from.1 + 6.0, 1.5, alpha);
}

fn clicks(t: f64, at: &[f64]) -> Option<f32> {
    at.iter().map(|&c| span(t, c, 0.4)).find(|p| *p > 0.0 && *p < 1.0)
}

/// Format mm:ss.
fn clock(seconds: f32) -> String {
    let s = seconds.max(0.0).round() as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

// ---------------------------------------------------------------- 23. pomodoro

const COLLAPSE: f64 = 1.45;
const DONE: f64 = 2.25;
const SMILE: f64 = 3.05;

fn pomodoro(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, 0xFF453A);
    // Selang waktu: 17:42 tersisa sampai hampir habis dalam satu setengah detik.
    let lapse = ease_in_out(span(t, 0.1, COLLAPSE - 0.2));
    let left = mix(17.0 * 60.0 + 42.0, 40.0, lapse);
    let running = t < DONE;
    let focus = FocusView {
        phase: Phase::Focus,
        running,
        active: running,
        clock: clock(left),
        progress: 1.0 - left / (25.0 * 60.0),
        sessions: 2,
        minutes_left: (left / 60.0).ceil() as u32,
        fx: None,
    };
    let hud = Hud::Focus { finished: Phase::Focus };
    let keys = [
        (0.0, View::Panel, View::Panel.size(), 1.0f32),
        (COLLAPSE, View::Pill, View::Pill.size(), 0.0),
        (DONE, View::Hud, View::Hud.size(), 0.6),
        (SMILE, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Panel, Page::Focus, false);
    motion.apply(&mut scene);
    scene.focus = &focus;
    if (DONE..SMILE + 0.2).contains(&t) {
        scene.hud = Some(&hud);
    }
    if t >= SMILE {
        scene.eyes.mood = Mood::Happy;
    }
    let (ax, ay, _) = PANEL;
    let k = if t < COLLAPSE { 2.1 } else { mix(2.1, 2.6, ease_out(span(t, COLLAPSE, 0.4))) };
    // Jam besar di belakang yang jarumnya berputar cepat selama selang waktu.
    let (cx, cy) = (540.0, 1010.0);
    let spin = lapse * 14.0 * 2.0 * PI;
    let clock_show = ease_out(span(t, 0.0, 0.3)) * (1.0 - span(t, COLLAPSE + 0.2, 0.4));
    studio.ring(cx, cy, 190.0, 8.0, 0xFFFFFF, 0.5 * clock_show);
    studio.arc(cx, cy, 190.0, 14.0, 0.0, focus.progress, 0xFF453A, clock_show);
    studio.line((cx, cy), (cx + 150.0 * spin.sin(), cy - 150.0 * spin.cos()), 8.0, 0xFFFFFF, clock_show);
    studio.line((cx, cy), (cx + 95.0 * (spin / 12.0).sin(), cy - 95.0 * (spin / 12.0).cos()), 12.0, 0xFFFFFF, clock_show);
    studio.circle(cx, cy, 12.0, 0xFFFFFF, clock_show);
    studio.glow(ax, ay + 140.0, 600.0, 240.0, if t >= SMILE { 0x30D158 } else { 0xFF453A }, 0.25);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    if t >= SMILE {
        confetti(studio, t - SMILE, 540.0, ay + 60.0);
    }
    caption(studio, t, len, "Pomodoro", "25 menit fokus, 5 menit istirahat");
}

/// Hujan konfeti kecil untuk perayaan.
fn confetti(studio: &mut Studio, t: f64, cx: f32, cy: f32) {
    let colors = [0xFF453A, 0xFF9F0A, 0xFFD60A, 0x30D158, 0x0A84FF, 0xBF5AF2];
    for i in 0..28 {
        let angle = i as f32 * 2.399;
        let speed = 420.0 + (i * 37 % 200) as f32;
        let t = t as f32;
        let x = cx + angle.cos() * speed * t;
        let y = cy + angle.sin().abs() * -speed * 0.5 * t + 900.0 * t * t;
        let alpha = (1.0 - t / 1.4).clamp(0.0, 1.0);
        studio.rounded(rect(x - 7.0, y - 11.0, x + 7.0, y + 11.0), 3.0, colors[i % colors.len()], alpha);
    }
}

fn pomodoro_sound(score: &mut Score) {
    let mut at = 0.1;
    while at < COLLAPSE - 0.1 {
        score.cue(at, Cue::Tick { hz: 3000.0, gain: 0.05 });
        at += 0.09;
    }
    score.cue(COLLAPSE, Cue::Glide { len: 0.3, from: 900.0, to: 400.0, gain: 0.14 });
    score.cue(DONE, Cue::Ding { gain: 0.24 });
    score.cue(SMILE, Cue::Chime { notes: vec![84.0, 88.0, 91.0, 96.0], gain: 0.16 });
}

// ---------------------------------------------------------------- 24. timer cepat

const PICK: f64 = 0.65;
const PLUS: f64 = 1.3;
const SHRINK: f64 = 1.95;
const ALARM: f64 = 3.05;

fn timer(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ORANGE);
    let total = if t >= PLUS { 6.0 } else { 5.0 };
    let elapsed = if t < SHRINK { (t - PICK).max(0.0) as f32 * 2.0 } else { mix(4.0, total * 60.0 - 12.0, ease_in_out(span(t, SHRINK, 0.7))) };
    let left = total * 60.0 - elapsed;
    let active = (PICK..ALARM).contains(&t);
    let short = if left <= 60.0 { format!("{}", left.round()) } else { format!("{}m", (left / 60.0).ceil()) };
    // Jam pasir dibalik saat durasi dipilih, dan menerima sedikit pasir saat "+1 mnt".
    let fx = if (PICK..PICK + 0.65).contains(&t) {
        Some((TimerFx::Flip, (t - PICK) as f32))
    } else if (PLUS..PLUS + 0.6).contains(&t) {
        Some((TimerFx::Add(((PLUS - PICK) * 2.0 / 300.0) as f32), (t - PLUS) as f32))
    } else {
        None
    };
    let progress = elapsed / (total * 60.0);
    let view = TimerView { running: active, active, clock: clock(left), short, progress, total_minutes: total as u32, ending: active && left <= 10.0, fx };
    let hud = Hud::Timer { minutes: 6 };
    let keys = [
        (0.0, View::Panel, View::Panel.size(), 1.0f32),
        (SHRINK, View::Pill, View::Pill.size(), 0.0),
        (ALARM, View::Hud, View::Hud.size(), 0.6),
        (len - 0.45, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Panel, Page::Timer, false);
    motion.apply(&mut scene);
    scene.timer = &view;
    scene.hovered = if t < PICK + 0.3 && t > 0.3 { Some(Hit::TimerPreset(5)) } else if t > PLUS - 0.4 && t < PLUS + 0.3 { Some(Hit::TimerAdd) } else { None };
    if t >= ALARM && t < len - 0.25 {
        scene.hud = Some(&hud);
    }
    let (ax, ay, _) = PANEL;
    let k = if t < SHRINK { 2.1 } else { mix(2.1, 2.6, ease_out(span(t, SHRINK, 0.4))) };
    studio.glow(ax, ay + 140.0, 600.0, 240.0, ORANGE, 0.25 + 0.2 * span(t, ALARM, 0.1) * (1.0 - span(t, ALARM + 0.6, 0.4)));
    // Alarm: notch bergetar sedikit.
    let buzz = if (ALARM..ALARM + 0.6).contains(&t) { ((t - ALARM) * 70.0).sin() as f32 * 6.0 } else { 0.0 };
    studio.notch(&scene, Theme::Black, ax + buzz, ay, k, 1.0);
    if t < SHRINK {
        cursor_path(studio, t, &[(0.0, (860.0, 1000.0)), (PICK, spot(-90.0, 128.0)), (PLUS, spot(114.0, 128.0)), (SHRINK - 0.05, (900.0, 1100.0))], 1.0 - span(t, SHRINK - 0.3, 0.2));
        if let Some(p) = clicks(t, &[PICK, PLUS]) {
            let at = if t < PLUS { spot(-90.0, 128.0) } else { spot(114.0, 128.0) };
            studio.ring(at.0, at.1, 30.0 + 70.0 * p, 4.0, 0xFFFFFF, 0.7 * (1.0 - p));
        }
    }
    // Hitung mundur besar selama pil berjalan.
    let big = span(t, SHRINK + 0.2, 0.3) * (1.0 - span(t, ALARM - 0.1, 0.2));
    studio.text(&view.clock, SERIF, 190.0, false, 980.0, ORANGE, big, 1.0);
    caption(studio, t, len, "Timer cepat", "1 sampai 15 menit, tambah semenit");
}

fn timer_sound(score: &mut Score) {
    score.cue(PICK, Cue::Click { gain: 0.3 });
    score.cue(PICK + 0.02, Cue::Pop { midi: 76.0, gain: 0.16 });
    score.cue(PLUS, Cue::Click { gain: 0.3 });
    score.cue(PLUS + 0.02, Cue::Pop { midi: 79.0, gain: 0.16 });
    score.cue(SHRINK, Cue::Glide { len: 0.3, from: 900.0, to: 400.0, gain: 0.12 });
    score.cue(SHRINK + 0.1, Cue::Whoosh { len: 0.7, from: 400.0, to: 3000.0, pan: (0.0, 0.0), gain: 0.12 });
    score.cue(ALARM, Cue::Alarm { len: 1.0, gain: 0.16 });
}

// ---------------------------------------------------------------- 25. minum air

const REMIND: f64 = 0.15;
const OPEN: f64 = 0.85;
const SIPS: [f64; 3] = [1.45, 2.0, 2.55];
const GOAL: f64 = 3.15;

fn water(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, WATER);
    let sips = SIPS.iter().filter(|&&s| t >= s).count() as u32;
    let ml = 1250 + 250 * sips;
    let mut week = sample().water.week;
    if let Some(today) = week.last_mut() {
        today.0 = ml;
    }
    // Tiap tegukan dituang dari kran seperti di notch.
    let pour = SIPS.iter().rev().find(|&&s| t >= s).map(|s| ((ml - 250) as f32 / 2000.0, (t - s) as f32));
    let view = WaterView { ml, goal: 2000, week, next: Some("20:15".into()), pour };
    let hud = Hud::Water { ml: 1250, goal: 2000 };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (REMIND, View::Hud, View::Hud.size(), 0.6),
        (OPEN, View::Panel, View::Panel.size(), 1.0),
        (GOAL, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Panel, Page::Water, false);
    motion.apply(&mut scene);
    scene.water = &view;
    if (REMIND..OPEN + 0.1).contains(&t) {
        scene.hud = Some(&hud);
    }
    scene.hovered = (t > OPEN + 0.3 && t < GOAL - 0.3).then_some(Hit::WaterDrink);
    if t >= GOAL {
        scene.eyes.mood = Mood::Happy;
    }
    let (ax, ay, _) = PANEL;
    let k = if t < GOAL { 2.1 } else { mix(2.1, 3.2, ease_out(span(t, GOAL, 0.4))) };
    studio.glow(ax, ay + 140.0, 600.0, 240.0, WATER, 0.25);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Gelas besar yang airnya naik bergelombang setiap kali "+ Minum" diklik.
    let level = mix((1250 + 250 * sips.saturating_sub(1)) as f32, ml as f32, ease_out(span(t, SIPS[sips.saturating_sub(1) as usize].min(t), 0.4))) / 2000.0;
    let level = if sips == 0 { 0.625 } else { level };
    glass(studio, t, 540.0, 1010.0, level, ease_out(span(t, OPEN, 0.3)));
    let button = spot(-120.0, 128.0);
    if t > OPEN && t < GOAL {
        let path = [(OPEN + 0.3, (860.0, 1150.0)), (SIPS[0], button), (GOAL - 0.1, button)];
        cursor_path(studio, t, &path, span(t, OPEN, 0.2));
        if let Some(p) = clicks(t, &SIPS) {
            studio.ring(button.0, button.1, 30.0 + 70.0 * p, 4.0, 0xFFFFFF, 0.7 * (1.0 - p));
        }
    }
    for &sip in &SIPS {
        let p = span(t, sip, 0.35);
        if p > 0.0 && p < 1.0 {
            let y = mix(button.1 + 40.0, 1010.0 - 40.0, p * p);
            studio.circle(540.0, y, 16.0, 0x64D2FF, 1.0 - p * 0.3);
        }
    }
    if t >= GOAL {
        confetti(studio, t - GOAL, 540.0, ay + 60.0);
    }
    caption(studio, t, len, "Minum air", "Target harian, grafik 7 hari, pengingat");
}

/// Gelas bening dengan air setinggi `level` (0..1) yang permukaannya bergelombang.
fn glass(studio: &mut Studio, t: f64, cx: f32, cy: f32, level: f32, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    let (top_w, bottom_w, h) = (230.0, 170.0, 240.0);
    let (top, bottom) = (cy - h / 2.0, cy + h / 2.0);
    let half = |y: f32| mix(bottom_w, top_w, (bottom - y) / h) / 2.0;
    let surface = bottom - h * level.clamp(0.0, 1.0) * 0.92;
    let mut water = Vec::new();
    for i in 0..=24 {
        let x = cx - half(surface) + 2.0 * half(surface) * i as f32 / 24.0;
        let wave = (x * 0.04 + t as f32 * 6.0).sin() * 7.0;
        water.push((x, surface + wave));
    }
    water.push((cx + half(bottom) - 6.0, bottom - 6.0));
    water.push((cx - half(bottom) + 6.0, bottom - 6.0));
    studio.glow(cx, cy, 220.0, 200.0, WATER, 0.25 * alpha);
    studio.polygon(&water, 0x0A84FF, 0.85 * alpha);
    studio.polyline(&[(cx - top_w / 2.0, top), (cx - bottom_w / 2.0, bottom), (cx + bottom_w / 2.0, bottom), (cx + top_w / 2.0, top)], 8.0, 0xFFFFFF, 0.8 * alpha);
}

fn water_sound(score: &mut Score) {
    score.cue(REMIND, Cue::Ding { gain: 0.2 });
    score.cue(OPEN, Cue::Glide { len: 0.3, from: 400.0, to: 900.0, gain: 0.12 });
    for (i, &sip) in SIPS.iter().enumerate() {
        score.cue(sip, Cue::Click { gain: 0.26 });
        score.cue(sip + 0.3, Cue::Pop { midi: 72.0 + i as f64 * 4.0, gain: 0.2 });
    }
    score.cue(GOAL, Cue::Chime { notes: vec![84.0, 88.0, 91.0, 96.0], gain: 0.16 });
}

// ---------------------------------------------------------------- 26. tugas

const FOCUS_INPUT: f64 = 0.35;
const ENTER: f64 = 1.45;
const CHECK: f64 = 2.2;
const NEW_TASK: &str = "Kirim laporan Q3";

fn tasks(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ORANGE);
    // Daftar pendek supaya tugas baru langsung terlihat: satu selesai, satu belum.
    let mut data = sample();
    data.media = song();
    data.tasks = Tasks::default();
    data.tasks.add("Beli kopi");
    data.tasks.toggle(0);
    data.tasks.add("Bayar listrik");
    if (FOCUS_INPUT..ENTER).contains(&t) {
        let typed = ((t - FOCUS_INPUT - 0.1).max(0.0) / 0.06) as usize;
        data.tasks.draft = Some(NEW_TASK.chars().take(typed).collect());
    }
    if t >= ENTER {
        data.tasks.add(NEW_TASK);
    }
    if t >= CHECK {
        data.tasks.toggle(2);
    }
    let mut scene = home(&data, &cast.idle, &cast.none, View::Panel, Page::Tasks, false);
    scene.hovered = if t < FOCUS_INPUT + 0.1 && t > 0.1 { Some(Hit::TaskInput) } else if t > CHECK - 0.4 && t < CHECK + 0.4 { Some(Hit::TaskToggle(2)) } else { None };
    let (ax, ay, k) = PANEL;
    studio.glow(ax, ay + 140.0, 600.0, 240.0, ORANGE, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k * mix(0.95, 1.0, ease_out(span(t, 0.0, 0.3))), ease_out(span(t, 0.0, 0.25)));
    let path = [(0.0, (860.0, 1050.0)), (FOCUS_INPUT, spot(-120.0, 128.0)), (CHECK, spot(-176.0, 70.0)), (len, (900.0, 1100.0))];
    cursor_path(studio, t, &path, span(t, 0.0, 0.15));
    if let Some(p) = clicks(t, &[FOCUS_INPUT, CHECK]) {
        let at = if t < CHECK { spot(-120.0, 128.0) } else { spot(-176.0, 70.0) };
        studio.ring(at.0, at.1, 24.0 + 60.0 * p, 4.0, 0xFFFFFF, 0.7 * (1.0 - p));
    }
    // Tombol Enter muncul sesaat ditekan.
    let enter = span(t, ENTER - 0.25, 0.15) * (1.0 - span(t, ENTER + 0.3, 0.2));
    if enter > 0.0 {
        super::peraga::keycap(studio, 540.0, 1000.0, 260.0, 130.0, "Enter ↵", super::peraga::press(t, &[ENTER]), ORANGE, enter);
    }
    caption(studio, t, len, "Tugas", "Ketik, Enter, lalu centang");
}

fn tasks_sound(score: &mut Score) {
    score.cue(FOCUS_INPUT, Cue::Click { gain: 0.25 });
    score.cue(FOCUS_INPUT + 0.1, Cue::Typing { len: ENTER - FOCUS_INPUT - 0.2, gain: 0.14 });
    score.cue(ENTER, Cue::Click { gain: 0.35 });
    score.cue(ENTER + 0.02, Cue::Pop { midi: 76.0, gain: 0.18 });
    score.cue(CHECK, Cue::Click { gain: 0.28 });
    score.cue(CHECK + 0.02, Cue::Chime { notes: vec![84.0, 91.0], gain: 0.12 });
}

// ---------------------------------------------------------------- 27. kalender dan agenda

const SELECT: f64 = 0.6;
const TYPE: f64 = 1.0;
const SAVE: f64 = 2.35;
const NEW_EVENT: &str = "09.00 Rapat tim";

fn calendar(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ORANGE);
    let mut cal = sample_calendar();
    let chosen = t >= SELECT;
    for day in &mut cal.days {
        day.selected = if chosen { day.offset == 1 } else { day.today };
        if day.offset == 1 {
            day.has_events = t >= SAVE;
        }
    }
    if chosen {
        // Deretan hari meluncur ke hari yang dipilih; agenda hari ini bergeser keluar.
        let slide = ((t - SELECT) as f32 / CALENDAR_SLIDE_SECS).min(1.0);
        cal.center = calendar_ease((t - SELECT) as f32) - 1.0;
        let before = std::mem::take(&mut cal.events);
        cal.shift = (slide < 1.0).then(|| DayShift { t: slide, dir: 1.0, events: before.into_iter().take(2).collect(), month: None });
        if t >= SAVE {
            cal.events = vec![AgendaItem { id: 9, time: "09.00".into(), text: "Rapat tim".into(), done: false, soon: false }];
        }
    }
    let draft: String = if (TYPE..SAVE).contains(&t) { NEW_EVENT.chars().take(((t - TYPE - 0.1).max(0.0) / 0.07) as usize).collect() } else { String::new() };
    let mut scene = cast.scene(View::PanelWide, Page::Home, false);
    scene.calendar = Some(&cal);
    scene.weather = Some("29° Cerah");
    if (TYPE..SAVE).contains(&t) {
        scene.agenda_draft = Some(&draft);
    }
    scene.hovered = (t > SELECT - 0.3 && t < SELECT + 0.2).then_some(Hit::CalendarDay(1));
    let (ax, ay, k) = (540.0, 520.0, 1.5);
    studio.glow(ax, ay + 140.0, 700.0, 260.0, ORANGE, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k * mix(0.95, 1.0, ease_out(span(t, 0.0, 0.3))), ease_out(span(t, 0.0, 0.25)));
    // Titik agenda baru melompat ke hari yang dipilih.
    let dot = pop_in(t, SAVE, 0.4);
    if dot > 0.0 && dot < 1.2 && t < SAVE + 0.6 {
        studio.glow(700.0, ay + 70.0 * k, 90.0, 90.0, ORANGE, 0.6 * (1.0 - span(t, SAVE + 0.2, 0.4)));
    }
    // Teks yang diketik ditampilkan besar di bawah panel.
    let big = span(t, TYPE, 0.2) * (1.0 - span(t, SAVE + 0.5, 0.3));
    let shown = if t >= SAVE { NEW_EVENT.to_string() } else { draft.clone() };
    studio.label(&shown, SERIF, 110.0, false, 540.0, 1000.0, 0xFFFFFF, big, 0);
    if t >= SAVE {
        studio.icon('\u{E73E}', 60.0 * pop_in(t, SAVE, 0.3).min(1.2), 540.0, 1120.0, 0x30D158, big);
    }
    caption(studio, t, len, "Kalender & agenda", "Ketik jam dan acaranya, Enter");
}

fn calendar_sound(score: &mut Score) {
    score.cue(SELECT, Cue::Click { gain: 0.25 });
    score.cue(TYPE + 0.1, Cue::Typing { len: SAVE - TYPE - 0.2, gain: 0.13 });
    score.cue(SAVE, Cue::Click { gain: 0.3 });
    score.cue(SAVE + 0.05, Cue::Pop { midi: 79.0, gain: 0.2 });
}
