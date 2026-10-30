//! Bab 7, privasi: titik kamera/mikrofon, mode rapat, layar kunci, dan data yang tetap lokal.

use super::peraga::{desktop, keycap, press, window};
use super::*;

const GREEN: u32 = 0x30D158;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Privasi",
        tint: GREEN,
        shots: vec![
            Shot { beats: 7, energy: Energy::Light, draw: dots, sound: dots_sound, blur: &[] },
            Shot { beats: 8, energy: Energy::Light, draw: meeting, sound: meeting_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Break, draw: locked, sound: locked_sound, blur: &[(0.45, 0.8, 4)] },
            Shot { beats: 8, energy: Energy::Light, draw: local, sound: local_sound, blur: &[] },
        ],
    }
}

// ---------------------------------------------------------------- 31. titik kamera dan mikrofon

const CAMERA_ON: f64 = 0.6;
const MIC_ON: f64 = 2.1;

fn dots(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, GREEN);
    let usage = Usage {
        camera: if t >= CAMERA_ON { vec!["Rapat".into()] } else { vec![] },
        microphone: if t >= MIC_ON { vec!["Rapat".into()] } else { vec![] },
        ..Usage::default()
    };
    let hud = Hud::Privacy { camera: true, app: "Rapat".into() };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (CAMERA_ON, View::Hud, View::Hud.size(), 0.6),
        (CAMERA_ON + 0.9, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut data = sample();
    data.media = song();
    let mut scene = home(&data, &cast.idle, &usage, View::Pill, Page::Home, false);
    motion.apply(&mut scene);
    let card = (CAMERA_ON..CAMERA_ON + 1.0).contains(&t);
    if card {
        scene.hud = Some(&hud);
    }
    if (CAMERA_ON + 0.9..CAMERA_ON + 1.6).contains(&t) {
        scene.eyes.mood = Mood::Surprised;
    }
    let (ax, ay) = (540.0, 470.0);
    let k = if card { 2.6 } else { 3.4 };
    studio.glow(ax, ay + 60.0, 560.0, 200.0, GREEN, 0.15 + 0.2 * span(t, CAMERA_ON, 0.2));
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Lensa kamera dan mikrofon besar dengan lampunya.
    let show = ease_out(span(t, 0.0, 0.3));
    let (cx, cy) = (380.0, 990.0);
    studio.circle(cx, cy, 120.0, 0x10151F, show);
    studio.ring(cx, cy, 120.0, 6.0, 0x3A4255, show);
    studio.circle(cx, cy, 70.0, 0x05070C, show);
    studio.ring(cx, cy, 70.0, 10.0, 0x1C2333, show);
    studio.circle(cx - 22.0, cy - 22.0, 16.0, 0xFFFFFF, 0.5 * show);
    let cam = span(t, CAMERA_ON, 0.1);
    studio.glow(cx + 150.0, cy - 90.0, 50.0, 50.0, GREEN, 0.7 * cam);
    studio.circle(cx + 150.0, cy - 90.0, 12.0, if cam > 0.0 { GREEN } else { 0x3A4255 }, show);
    let (mx, my) = (730.0, 990.0);
    let mic = span(t, MIC_ON, 0.1);
    studio.circle(mx, my, 100.0, if mic > 0.0 { 0x3A2410 } else { 0x10151F }, show);
    studio.icon('\u{E720}', 90.0, mx, my, if mic > 0.0 { 0xFF9F0A } else { 0x8F9BB3 }, show);
    for ring in 0..2 {
        let p = (((t - MIC_ON) * 1.2 + ring as f64 * 0.5) % 1.0) as f32;
        if t >= MIC_ON {
            studio.ring(mx, my, 100.0 + 90.0 * p, 4.0, 0xFF9F0A, 0.6 * (1.0 - p));
        }
    }
    caption_at(studio, t, len, 1330.0, "Kamera & mikrofon", "Titik hijau dan oranye selama dipakai");
}

fn dots_sound(score: &mut Score) {
    score.cue(CAMERA_ON, Cue::Ding { gain: 0.2 });
    score.cue(CAMERA_ON + 0.9, Cue::Pop { midi: 84.0, gain: 0.16 });
    score.cue(MIC_ON, Cue::Pop { midi: 76.0, gain: 0.18 });
}

// ---------------------------------------------------------------- 32. mode rapat

const MESSAGE: f64 = 1.1;

fn meeting(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, GREEN);
    let usage = Usage { camera: vec!["Rapat".into()], microphone: vec!["Rapat".into()], ..Usage::default() };
    // Saat rapat pengirim dan isi pesan tidak ditampilkan, seperti di app.
    let hud = Hud::Message { app: String::new(), aumid: String::new(), sender: "WhatsApp".into(), text: "Pesan baru".into(), browser: false };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (MESSAGE, View::Hud, View::Hud.size(), 0.6),
        (len - 0.5, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut data = sample();
    data.media = song();
    let mut scene = home(&data, &cast.idle, &usage, View::Pill, Page::Home, false);
    motion.apply(&mut scene);
    if t >= MESSAGE && t < len - 0.3 {
        scene.hud = Some(&hud);
    }
    let (ax, ay, k) = (540.0, 420.0, 2.6);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Jendela rapat dengan empat peserta.
    let show = ease_out(span(t, 0.0, 0.35));
    let area = rect(110.0, 700.0, 970.0, 1180.0);
    window(studio, area, "Rapat · 4 peserta", show);
    let colors = [MAGENTA, CYAN, 0xFF9F0A, GREEN];
    for (i, (initial, color)) in ["A", "B", "C", "D"].into_iter().zip(colors).enumerate() {
        let (col, row) = ((i % 2) as f32, (i / 2) as f32);
        let tile = rect(area.left + 24.0 + col * 418.0, area.top + 88.0 + row * 184.0, area.left + 418.0 + col * 418.0, area.top + 256.0 + row * 184.0);
        studio.rounded(tile, 18.0, 0x1C2333, show);
        studio.clip(tile);
        studio.glow((tile.left + tile.right) / 2.0, tile.bottom, 220.0, 120.0, color, 0.4 * show);
        studio.unclip();
        let speaking = i == 1 && ((t * 3.0) as i32) % 2 == 0;
        if speaking {
            studio.outline(tile, 18.0, 4.0, GREEN, show);
        }
        studio.circle((tile.left + tile.right) / 2.0, (tile.top + tile.bottom) / 2.0, 46.0, color, show);
        studio.label(initial, SANS, 40.0, true, (tile.left + tile.right) / 2.0, (tile.top + tile.bottom) / 2.0, 0x05070C, show, 0);
    }
    // Isi pesan asli yang tertahan: dicoret dan memudar, bunyinya dibisukan.
    let ghost = span(t, MESSAGE, 0.2) * (1.0 - span(t, MESSAGE + 1.2, 0.3));
    if ghost > 0.0 {
        let y = ay + View::Hud.size().1 * k + 70.0;
        let text = "Rina: Nanti jadi nonton jam 7?";
        let w = studio.measure(text, SANS, 36.0, false);
        studio.label(text, SANS, 36.0, false, 540.0, y, 0xBFD8FF, 0.6 * ghost, 0);
        let strike = ease_out(span(t, MESSAGE + 0.2, 0.3));
        studio.line((540.0 - w / 2.0 - 6.0, y + 2.0), (540.0 - w / 2.0 - 6.0 + (w + 12.0) * strike, y + 2.0), 4.0, 0xFF453A, ghost);
    }
    let mute = pop_in(t, MESSAGE + 0.35, 0.35);
    if mute > 0.0 {
        let (mx, my) = (870.0, ay + 50.0);
        studio.circle(mx, my, 44.0 * mute.min(1.2), 0x3A2410, 1.0);
        studio.icon('\u{E74F}', 40.0 * mute.min(1.2), mx, my, 0xFF9F0A, 1.0);
    }
    caption_at(studio, t, len, 1330.0, "Mode rapat", "Pengirim dan isi pesan disembunyikan");
}

fn meeting_sound(score: &mut Score) {
    score.cue(MESSAGE, Cue::Pop { midi: 72.0, gain: 0.16 });
    score.cue(MESSAGE + 0.2, Cue::Whoosh { len: 0.35, from: 3000.0, to: 1200.0, pan: (-0.3, 0.3), gain: 0.1 });
}

// ---------------------------------------------------------------- 33. layar kunci

const LOCK: f64 = 0.35;
const PING: f64 = 1.9;

fn locked(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, GREEN);
    let dark = ease_in_out(span(t, LOCK + 0.1, 0.35));
    // Layar kunci: wallpaper, jam besar, dan tanggal.
    if dark > 0.0 {
        let screen = rect(0.0, 0.0, W as f32, H as f32);
        studio.rect(screen, 0x050814, dark);
        desktop(studio, rect(-40.0, -40.0, W as f32 + 40.0, 1240.0), GREEN, dark * 0.9);
        studio.rect(rect(0.0, 1180.0, W as f32, 1240.0), 0x050814, dark);
        studio.label("19:21", SANS, 190.0, true, 540.0, 830.0, 0xFFFFFF, dark, 0);
        studio.label("Rabu, 21 Oktober", SANS, 48.0, false, 540.0, 960.0, 0xDCE9FF, dark, 0);
    }
    let hud = Hud::Message { app: String::new(), aumid: String::new(), sender: "WhatsApp".into(), text: "Pesan baru".into(), browser: false };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (PING, View::Hud, View::Hud.size(), 0.6),
        (len - 0.45, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Pill, Page::Home, true);
    motion.apply(&mut scene);
    scene.locked = t >= LOCK + 0.3;
    if t >= PING && t < len - 0.25 {
        scene.hud = Some(&hud);
    }
    studio.notch(&scene, Theme::Black, 540.0, 380.0, 2.8, 1.0);
    let keys_show = ease_out(span(t, 0.0, 0.2)) * (1.0 - span(t, LOCK + 0.3, 0.3));
    for (x, label) in [(420.0, "Win"), (660.0, "L")] {
        keycap(studio, x, 1000.0, 200.0, 140.0, label, press(t, &[LOCK]), GREEN, keys_show);
    }
    caption_at(studio, t, len, 1330.0, "Layar kunci", "Hanya jam dan musik, pesan disamarkan");
}

fn locked_sound(score: &mut Score) {
    score.cue(LOCK, Cue::Click { gain: 0.35 });
    score.cue(LOCK + 0.1, Cue::Glide { len: 0.35, from: 700.0, to: 250.0, gain: 0.14 });
    score.cue(PING, Cue::Pop { midi: 76.0, gain: 0.16 });
}

// ---------------------------------------------------------------- 34. data lokal

const FILES: [&str; 5] = ["shelf.txt", "todo.txt", "water.txt", "agenda.txt", "settings.txt"];
const SWITCHES: f64 = 2.1;

fn local(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, GREEN);
    let scene = cast.scene(View::Pill, Page::Home, false);
    let (ax, ay, k) = (540.0, 380.0, 2.8);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Folder di laptop tempat semua data disimpan.
    let (fx, fy) = (540.0, 820.0);
    let folder = ease_out(span(t, 0.0, 0.3)) * (1.0 - span(t, SWITCHES - 0.2, 0.3));
    let gulp = FILES.iter().enumerate().map(|(i, _)| span(t, 0.75 + i as f64 * 0.2, 0.1) * (1.0 - span(t, 0.85 + i as f64 * 0.2, 0.15))).fold(0.0f32, f32::max);
    studio.glow(fx, fy, 260.0, 200.0, GREEN, 0.3 * folder);
    studio.icon('\u{E8B7}', 240.0 * (1.0 + 0.06 * gulp), fx, fy, 0xFFD60A, folder);
    studio.label("%APPDATA%\\NotchWindows", SANS, 36.0, true, fx, fy + 170.0, 0xDCE9FF, folder, 0);
    for (i, name) in FILES.into_iter().enumerate() {
        let p = ease_in_out(span(t, 0.35 + i as f64 * 0.2, 0.5));
        if p <= 0.0 || p >= 1.0 {
            continue;
        }
        let from = (540.0 + (i as f32 - 2.0) * 160.0, ay + 120.0);
        let (x, y) = (mix(from.0, fx, p), mix(from.1, fy, p) - 60.0 * (p * PI).sin());
        let s = mix(1.0, 0.4, p);
        let card = rect(x - 80.0 * s, y - 30.0 * s, x + 80.0 * s, y + 30.0 * s);
        studio.rounded(card, 10.0 * s, 0xF2F4F8, 1.0 - p * 0.3);
        studio.label(name, SANS, 24.0 * s, true, x, y, 0x1C2333, 1.0 - p * 0.3, 0);
    }
    // Fitur yang memakai internet mati secara bawaan.
    let rows = ease_out(span(t, SWITCHES, 0.35));
    if rows > 0.0 {
        for (i, label) in ["Lirik lagu (pakai internet)", "Cuaca (pakai internet)"].into_iter().enumerate() {
            let y = 760.0 + i as f32 * 130.0 + (1.0 - rows) * 30.0;
            let row = rect(140.0, y - 50.0, 940.0, y + 50.0);
            studio.rounded(row, 26.0, 0x1C2333, rows);
            studio.icon('\u{E753}', 34.0, row.left + 50.0, y, 0x8F9BB3, rows);
            studio.label(label, SANS, 34.0, false, row.left + 90.0, y, 0xF2F4F8, rows, -1);
            let switch = rect(row.right - 130.0, y - 22.0, row.right - 40.0, y + 22.0);
            studio.rounded(switch, 22.0, 0x3A4255, rows);
            studio.circle(switch.left + 22.0, y, 15.0, 0xFFFFFF, rows);
            studio.label("Mati", SANS, 28.0, true, switch.left - 20.0, y, 0x8F9BB3, rows, 1);
        }
    }
    caption_at(studio, t, len, 1330.0, "Data tetap di laptopmu", "File teks biasa, tanpa akun");
    badge(studio, t, SWITCHES + 0.3, len, "fitur internet mati secara bawaan", 1540.0, GREEN);
}

fn local_sound(score: &mut Score) {
    for i in 0..FILES.len() {
        score.cue(0.35 + i as f64 * 0.2, Cue::Whoosh { len: 0.45, from: 2200.0, to: 700.0, pan: (0.0, 0.0), gain: 0.07 });
        score.cue(0.85 + i as f64 * 0.2, Cue::Pop { midi: 72.0 + i as f64 * 2.0, gain: 0.14 });
    }
    score.cue(SWITCHES, Cue::Glide { len: 0.3, from: 400.0, to: 800.0, gain: 0.1 });
}
