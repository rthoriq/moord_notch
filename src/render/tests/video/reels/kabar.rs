//! Bab 4, kabar lain: Bluetooth, Wi-Fi, pesan masuk, unduhan, baterai lemah, dan agenda.

use super::*;

pub(super) const INDIGO: u32 = 0x5E5CE6;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Kabar lain",
        tint: INDIGO,
        shots: vec![
            Shot { beats: 6, energy: Energy::Full, draw: bluetooth, sound: bluetooth_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: wifi, sound: wifi_sound, blur: &[] },
            Shot { beats: 8, energy: Energy::Full, draw: message, sound: message_sound, blur: &[(0.25, 0.75, 6)] },
            Shot { beats: 6, energy: Energy::Full, draw: download, sound: download_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Light, draw: battery, sound: battery_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Light, draw: agenda, sound: agenda_sound, blur: &[] },
        ],
    }
}

const CARD: (f32, f32, f32) = (540.0, 470.0, 2.6);

/// Notch yang berubah jadi kartu kabar di detik `at` dan kembali jadi pil di `back`.
fn card_scene<'a>(cast: &'a Cast, t: f64, at: f64, back: f64, hud: &'a Hud) -> Scene<'a> {
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (at, View::Hud, View::Hud.size(), 0.6),
        (back, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Hud, Page::Home, false);
    motion.apply(&mut scene);
    if t >= at && t < back + 0.2 {
        scene.hud = Some(hud);
    }
    scene
}

// ---------------------------------------------------------------- 17. Bluetooth

const PAIRED: f64 = 1.0;

fn bluetooth(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, INDIGO);
    let battery = ((t - PAIRED - 0.2).max(0.0) * 120.0).min(80.0) as u8;
    let hud = Hud::Bluetooth { id: String::new(), name: "Earbuds Senja".into(), connected: true, battery: Some(battery.max(1)), kind: DeviceKind::Audio };
    let scene = card_scene(cast, t, PAIRED, len - 0.5, &hud);
    let (ax, ay, k) = CARD;
    studio.glow(ax, ay + 50.0, 560.0, 200.0, INDIGO, 0.3);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Earbuds besar dengan riak pencarian, lalu sinyal tersambung ke notch.
    let (cx, cy) = (540.0, 930.0);
    let show = ease_out(span(t, 0.0, 0.3));
    let searching = t < PAIRED;
    for ring in 0..3 {
        let phase = ((t * 1.4 + ring as f64 / 3.0) % 1.0) as f32;
        let alpha = if searching { (1.0 - phase) * 0.6 } else { (1.0 - phase) * 0.6 * (1.0 - span(t, PAIRED, 0.4)) };
        studio.ring(cx, cy, 110.0 + 180.0 * phase, 4.0, INDIGO, alpha * show);
    }
    studio.glow(cx, cy, 220.0, 220.0, INDIGO, 0.35 * show);
    studio.circle(cx, cy, 110.0, 0x1C2333, show);
    studio.icon('\u{E7F6}', 110.0, cx, cy, 0xF2F4F8, show);
    studio.circle(cx + 80.0, cy - 80.0, 30.0, if searching { 0x3A4255 } else { ELECTRIC }, show);
    studio.icon('\u{E702}', 32.0, cx + 80.0, cy - 80.0, 0xFFFFFF, show);
    let link = ease_out(span(t, PAIRED - 0.25, 0.3)) * (1.0 - span(t, PAIRED + 0.5, 0.3));
    if link > 0.0 {
        let top = ay + View::Hud.size().1 * k;
        let mut y = cy - 120.0;
        while y > top + 10.0 {
            studio.circle(cx, y, 6.0, ELECTRIC, link);
            y -= 34.0;
        }
    }
    // Baterai earbuds terisi setelah tersambung.
    let bar = rect(390.0, 1110.0, 690.0, 1140.0);
    let fill = battery as f32 / 100.0;
    let bar_show = show * span(t, PAIRED, 0.2);
    studio.rounded(bar, 15.0, 0x2A3346, bar_show);
    studio.rounded(rect(bar.left, bar.top, bar.left + (bar.right - bar.left) * fill, bar.bottom), 15.0, 0x30D158, bar_show);
    studio.label(&format!("{battery}%"), SANS, 34.0, true, 720.0, 1125.0, 0xFFFFFF, bar_show, -1);
    caption_at(studio, t, len, 1330.0, "Bluetooth", "Tersambung, lengkap dengan baterainya");
}

fn bluetooth_sound(score: &mut Score) {
    for i in 0..3 {
        score.cue(0.1 + i as f64 * 0.3, Cue::Tick { hz: 2800.0, gain: 0.06 });
    }
    score.cue(PAIRED, Cue::Chime { notes: vec![76.0, 83.0, 88.0], gain: 0.14 });
    score.cue(PAIRED, Cue::Glide { len: 0.3, from: 500.0, to: 1100.0, gain: 0.12 });
}

// ---------------------------------------------------------------- 18. Wi-Fi

const ONLINE: f64 = 0.9;
const OFFLINE: f64 = 1.9;

fn wifi(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, INDIGO);
    let lost = t >= OFFLINE;
    let hud = Hud::Wifi { name: "Rumah 5G".into(), connected: true, internet: !lost };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (ONLINE, View::Hud, View::Hud.size(), 0.6),
        (len - 0.5, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Hud, Page::Home, false);
    motion.apply(&mut scene);
    if t >= ONLINE && t < len - 0.3 {
        scene.hud = Some(&hud);
    }
    let (ax, ay, k) = CARD;
    studio.glow(ax, ay + 50.0, 560.0, 200.0, if lost { 0xFF9F0A } else { INDIGO }, 0.3);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Lengkung sinyal menyala satu per satu, lalu meredup saat internet hilang.
    let (cx, cy) = (540.0, 1130.0);
    let show = ease_out(span(t, 0.0, 0.3));
    let dim = span(t, OFFLINE, 0.3);
    for i in 0..4 {
        let on = span(t, 0.15 + i as f64 * 0.2, 0.15);
        let color = if lost { 0xFF9F0A } else { 0xFFFFFF };
        let alpha = show * mix(0.18, 1.0, on) * mix(1.0, if i == 0 { 1.0 } else { 0.25 }, dim);
        if i == 0 {
            studio.circle(cx, cy - 20.0, 22.0, color, alpha);
        } else {
            let r = 40.0 + i as f32 * 85.0;
            studio.arc(cx, cy - 20.0, r, 26.0, -0.125, 0.25, color, alpha);
            if on > 0.5 && !lost {
                studio.arc(cx, cy - 20.0, r, 60.0, -0.125, 0.25, color, 0.08 * alpha);
            }
        }
    }
    if lost {
        let bang = pop_in(t, OFFLINE, 0.35);
        studio.circle(cx + 200.0, cy - 280.0, 42.0 * bang, 0xFF9F0A, 1.0);
        studio.label("!", SANS, 56.0 * bang, true, cx + 200.0, cy - 282.0, 0x05070C, 1.0, 0);
    }
    let title = if lost { "Tanpa internet" } else { "Wi-Fi" };
    caption_at(studio, t, len, 1330.0, title, "Tersambung, terputus, atau tanpa internet");
}

fn wifi_sound(score: &mut Score) {
    for i in 0..4 {
        score.cue(0.15 + i as f64 * 0.2, Cue::Pop { midi: 72.0 + i as f64 * 4.0, gain: 0.12 });
    }
    score.cue(ONLINE, Cue::Ding { gain: 0.16 });
    score.cue(OFFLINE, Cue::Glide { len: 0.4, from: 900.0, to: 300.0, gain: 0.16 });
}

// ---------------------------------------------------------------- 19. pesan masuk

const ARRIVE: f64 = 0.75;
const HIDE: f64 = 2.15;

fn message(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, INDIGO);
    let hidden = t >= HIDE + 0.15;
    let text = if hidden { "Pesan baru" } else { "Nanti jadi nonton jam 7? Aku pesan tiketnya ya" };
    let hud = Hud::Message { app: "WhatsApp".into(), aumid: String::new(), sender: "Rina".into(), text: text.into(), browser: false };
    let scene = card_scene(cast, t, ARRIVE, len - 0.5, &hud);
    let (ax, ay, k) = CARD;
    studio.glow(ax, ay + 50.0, 560.0, 200.0, 0x30D158, 0.25 * span(t, ARRIVE, 0.2));
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Gelembung chat terbang dari bawah ke notch.
    let fly = ease_in_out(span(t, 0.1, ARRIVE - 0.1));
    if t < ARRIVE {
        let (x, y) = (mix(300.0, 540.0, fly), mix(1150.0, 560.0, fly) - 120.0 * (fly * PI).sin());
        let s = mix(1.0, 0.3, fly);
        let bubble = rect(x - 180.0 * s, y - 60.0 * s, x + 180.0 * s, y + 60.0 * s);
        studio.rounded(bubble, 40.0 * s, 0x30D158, 1.0);
        studio.polygon(&[(bubble.left + 30.0 * s, bubble.bottom - 4.0), (bubble.left + 10.0 * s, bubble.bottom + 26.0 * s), (bubble.left + 70.0 * s, bubble.bottom - 4.0)], 0x30D158, 1.0);
        for i in 0..3 {
            studio.circle(x - 50.0 * s + i as f32 * 50.0 * s, y, 12.0 * s, 0xFFFFFF, 0.9);
        }
    }
    // Sakelar "Sembunyikan isi pesan" menyala, lalu isinya tersapu kabur.
    let panel = ease_out(span(t, HIDE - 0.5, 0.3));
    if panel > 0.0 {
        let row = rect(160.0, 960.0 + (1.0 - panel) * 30.0, 920.0, 1070.0 + (1.0 - panel) * 30.0);
        studio.rounded(row, 26.0, 0x1C2333, panel);
        studio.icon('\u{E8BD}', 34.0, row.left + 50.0, (row.top + row.bottom) / 2.0, 0xC9D1E0, panel);
        studio.label("Sembunyikan isi pesan", SANS, 34.0, false, row.left + 90.0, (row.top + row.bottom) / 2.0, 0xF2F4F8, panel, -1);
        let on = ease_in_out(span(t, HIDE, 0.2));
        let switch = rect(row.right - 130.0, row.top + 33.0, row.right - 40.0, row.bottom - 33.0);
        studio.rounded(switch, 22.0, if on > 0.5 { ELECTRIC } else { 0x3A4255 }, panel);
        studio.circle(mix(switch.left + 22.0, switch.right - 22.0, on), (switch.top + switch.bottom) / 2.0, 15.0, 0xFFFFFF, panel);
        let come = ease_in_out(span(t, HIDE - 0.45, 0.4));
        studio.cursor(mix(980.0, switch.right - 30.0, come), mix(1250.0, switch.bottom - 4.0, come), 1.5, panel * (1.0 - span(t, HIDE + 0.6, 0.3)));
    }
    let sweep = span(t, HIDE, 0.3);
    if sweep > 0.0 && sweep < 1.0 {
        let left = ax - 120.0 * k;
        let x = mix(left, ax + 160.0 * k, sweep);
        studio.glow(x, ay + 28.0 * k, 120.0, 70.0, 0xFFFFFF, 0.5 * (1.0 - sweep));
    }
    caption_at(studio, t, len, 1330.0, "Pesan masuk", "Isinya bisa disembunyikan");
    badge(studio, t, ARRIVE, len, "aktifkan sekali di menu tray", 1540.0, INDIGO);
}

fn message_sound(score: &mut Score) {
    score.cue_pan(0.1, (-0.4, 0.0), Cue::Whoosh { len: ARRIVE - 0.1, from: 400.0, to: 2400.0, pan: (-0.4, 0.0), gain: 0.18 });
    score.cue(ARRIVE, Cue::Pop { midi: 81.0, gain: 0.24 });
    score.cue(HIDE, Cue::Click { gain: 0.3 });
    score.cue(HIDE + 0.02, Cue::Whoosh { len: 0.3, from: 3000.0, to: 1200.0, pan: (-0.3, 0.3), gain: 0.12 });
}

// ---------------------------------------------------------------- 20. unduhan

const FINISH: f64 = 1.6;

fn download(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, INDIGO);
    let mb = (t.min(FINISH) / FINISH).powf(1.3) * 24.6;
    let size = format!("{mb:.1} MB").replace('.', ",");
    let hud = Hud::Download { path: PathBuf::from("C:\\contoh\\Laporan Q3.pdf"), name: "Laporan Q3 2026.pdf".into(), more: 0 };
    let pill = (View::Pill.size().0 + 60.0, View::Pill.size().1);
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (0.15, View::Pill, pill, 0.0),
        (FINISH, View::Hud, View::Hud.size(), 0.6),
        (len - 0.5, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut data = sample();
    data.media = song();
    let mut scene = home(&data, &cast.idle, &cast.none, View::Pill, Page::Home, false);
    motion.apply(&mut scene);
    if t < FINISH {
        scene.download = Some(&size);
    } else if t < len - 0.3 {
        scene.hud = Some(&hud);
    }
    let (ax, ay, k) = CARD;
    studio.glow(ax, ay + 50.0, 560.0, 200.0, ELECTRIC, 0.25);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Panah unduh berjatuhan ke baki, lalu baki berubah jadi centang.
    let (cx, tray) = (540.0, 1130.0);
    let show = ease_out(span(t, 0.0, 0.3));
    if t < FINISH {
        for i in 0..3 {
            let phase = ((t * 1.6 + i as f64 / 3.0) % 1.0) as f32;
            let y = mix(760.0, tray - 40.0, phase);
            studio.icon('\u{E896}', 90.0, cx, y, 0xFFFFFF, show * (1.0 - phase) * (phase * 4.0).min(1.0));
        }
    }
    studio.line((cx - 150.0, tray), (cx + 150.0, tray), 12.0, if t < FINISH { 0xFFFFFF } else { 0x30D158 }, show);
    studio.line((cx - 150.0, tray), (cx - 150.0, tray - 50.0), 12.0, if t < FINISH { 0xFFFFFF } else { 0x30D158 }, show);
    studio.line((cx + 150.0, tray), (cx + 150.0, tray - 50.0), 12.0, if t < FINISH { 0xFFFFFF } else { 0x30D158 }, show);
    if t >= FINISH {
        let done = pop_in(t, FINISH, 0.4);
        studio.glow(cx, tray - 100.0, 160.0, 160.0, 0x30D158, 0.4);
        studio.icon('\u{E73E}', 110.0 * done, cx, tray - 90.0, 0x30D158, 1.0);
    }
    caption_at(studio, t, len, 1330.0, "Unduhan", "Ukurannya naik di pil, kabar saat selesai");
}

fn download_sound(score: &mut Score) {
    score.cue(0.15, Cue::Glide { len: 0.25, from: 400.0, to: 700.0, gain: 0.1 });
    for i in 0..5 {
        score.cue(0.3 + i as f64 * 0.33, Cue::Tick { hz: 2400.0 + i as f64 * 200.0, gain: 0.06 });
    }
    score.cue(FINISH, Cue::Ding { gain: 0.22 });
}

// ---------------------------------------------------------------- 21. baterai lemah

const LOW: f64 = 0.9;
const LOWER: f64 = 2.4;

fn battery(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, 0xFF453A);
    let percent = if t < LOW { mix(26.0, 20.0, span(t, 0.0, LOW)) } else if t < LOWER { 20.0 } else { 10.0 };
    let percent = percent.round() as u32;
    let hud = Hud::Power { plugged: false, percent: Some(percent), low: true };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (LOW, View::Hud, View::Hud.size(), 0.6),
        (LOW + 0.85, View::Pill, View::Pill.size(), 0.0),
        (LOWER, View::Hud, View::Hud.size(), 0.6),
        (len - 0.45, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Hud, Page::Home, false);
    motion.apply(&mut scene);
    let card = (LOW..LOW + 1.0).contains(&t) || (LOWER..len - 0.25).contains(&t);
    if card {
        scene.hud = Some(&hud);
    }
    // Di antara dua kabar, maskot membulat kaget.
    scene.eyes.mood = if t > LOW + 0.9 && t < LOWER { Mood::Surprised } else { Mood::Normal };
    let (ax, ay, k) = CARD;
    studio.glow(ax, ay + 50.0, 560.0, 200.0, 0xFF453A, 0.25 + 0.2 * span(t, LOWER, 0.2));
    studio.notch(&scene, Theme::Black, ax, ay, if card { k } else { k * 1.3 }, 1.0);
    // Baterai besar yang berkedip merah.
    let (cx, cy) = (540.0, 1000.0);
    let blink = if t >= LOWER && ((t * 4.0) as i32) % 2 == 0 { 0.45 } else { 1.0 };
    let body = rect(cx - 170.0, cy - 80.0, cx + 150.0, cy + 80.0);
    let show = ease_out(span(t, 0.0, 0.3));
    studio.outline(body, 26.0, 10.0, 0xFFFFFF, show);
    studio.rounded(rect(body.right + 8.0, cy - 30.0, body.right + 30.0, cy + 30.0), 8.0, 0xFFFFFF, show);
    let fill = (body.right - body.left - 36.0) * percent as f32 / 100.0;
    let color = if percent > 20 { 0x30D158 } else if percent > 10 { 0xFF9F0A } else { 0xFF453A };
    studio.rounded(rect(body.left + 18.0, body.top + 18.0, body.left + 18.0 + fill, body.bottom - 18.0), 12.0, color, show * blink);
    studio.label(&format!("{percent}%"), SERIF, 110.0, false, cx, cy + 180.0, 0xFFFFFF, show, 0);
    caption_at(studio, t, len, 1330.0, "Baterai lemah", "Diingatkan di 20% dan 10%");
}

fn battery_sound(score: &mut Score) {
    score.cue(LOW, Cue::Ding { gain: 0.22 });
    score.cue(LOW + 0.95, Cue::Pop { midi: 84.0, gain: 0.16 });
    score.cue(LOWER, Cue::Alarm { len: 0.3, gain: 0.12 });
}

// ---------------------------------------------------------------- 22. agenda sekarang

const RING: f64 = 1.3;

fn agenda(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, INDIGO);
    let hud = Hud::Agenda { text: "Rapat tim".into(), time: "09.00".into() };
    let scene = card_scene(cast, t, RING, len - 0.5, &hud);
    let (ax, ay, k) = CARD;
    studio.glow(ax, ay + 50.0, 560.0, 200.0, INDIGO, 0.3);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Jam dinding berputar ke 09.00 lalu bergoyang berdering.
    let (cx, cy) = (540.0, 980.0);
    let show = ease_out(span(t, 0.0, 0.3));
    let shake = if t >= RING { ((t - RING) * 50.0).sin() as f32 * 0.08 * (1.0 - span(t, RING, 0.8)) } else { 0.0 };
    let minutes = mix(8.0 * 60.0 + 40.0, 9.0 * 60.0, ease_in_out(span(t, 0.1, RING - 0.1)));
    studio.glow(cx, cy, 260.0, 260.0, INDIGO, 0.3 * show);
    studio.circle(cx, cy, 170.0, 0x1C2333, show);
    studio.ring(cx, cy, 170.0, 10.0, 0xFFFFFF, show);
    for i in 0..12 {
        let a = i as f32 / 12.0 * 2.0 * PI + shake;
        studio.line((cx + 140.0 * a.sin(), cy - 140.0 * a.cos()), (cx + 155.0 * a.sin(), cy - 155.0 * a.cos()), 6.0, 0xFFFFFF, 0.6 * show);
    }
    let hour = (minutes / 60.0) / 12.0 * 2.0 * PI + shake;
    let minute = (minutes % 60.0) / 60.0 * 2.0 * PI + shake;
    studio.line((cx, cy), (cx + 85.0 * hour.sin(), cy - 85.0 * hour.cos()), 14.0, 0xFFFFFF, show);
    studio.line((cx, cy), (cx + 125.0 * minute.sin(), cy - 125.0 * minute.cos()), 9.0, INDIGO, show);
    studio.circle(cx, cy, 12.0, 0xFFFFFF, show);
    if t >= RING {
        let wave = span(t, RING, 0.6);
        for side in [-1.0f32, 1.0] {
            studio.arc(cx, cy, 200.0 + 40.0 * wave, 8.0, if side < 0.0 { 0.8 } else { 0.07 }, 0.13, 0xFFD60A, 0.8 * (1.0 - wave));
        }
    }
    caption_at(studio, t, len, 1330.0, "Agenda sekarang", "Diingatkan tepat waktunya");
}

fn agenda_sound(score: &mut Score) {
    for i in 0..4 {
        score.cue(0.2 + i as f64 * 0.25, Cue::Tick { hz: 1800.0, gain: 0.08 });
    }
    score.cue(RING, Cue::Alarm { len: 0.5, gain: 0.1 });
    score.cue(RING, Cue::Ding { gain: 0.2 });
}
