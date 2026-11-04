//! Bab 6, kontrol dan sistem: kontrol cepat (slider, mikrofon, senyap), cuaca, dan monitor
//! sistem.

use super::*;

const TEAL: u32 = 0x64D2FF;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Kontrol & Sistem",
        tint: TEAL,
        shots: vec![
            Shot { beats: 9, energy: Energy::Full, draw: controls, sound: controls_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: weather, sound: weather_sound, blur: &[] },
            Shot { beats: 8, energy: Energy::Full, draw: system, sound: system_sound, blur: &[] },
        ],
    }
}

const PANEL: (f32, f32, f32) = (540.0, 470.0, 2.1);

fn spot(x: f32, y: f32) -> (f32, f32) {
    let (ax, ay, k) = PANEL;
    (ax + x * k, ay + y * k)
}

/// Posisi kenop slider untuk nilai 0..1 (jalur dari -154 sampai 140 DIP).
fn knob(value: f32, y: f32) -> (f32, f32) {
    spot(mix(-154.0, 140.0, value), y)
}

// ---------------------------------------------------------------- 28. kontrol cepat

const VOLUME: (f64, f64) = (0.45, 1.1);
const LIGHT: (f64, f64) = (1.35, 1.95);
const MIC: f64 = 2.3;
const QUIET: f64 = 2.8;

fn controls(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, TEAL);
    let volume = mix(0.62, 0.88, ease_in_out(span(t, VOLUME.0 + 0.05, VOLUME.1 - VOLUME.0)));
    let light = mix(0.7, 0.35, ease_in_out(span(t, LIGHT.0 + 0.05, LIGHT.1 - LIGHT.0)));
    let view = ControlsView {
        volume: Some((volume, false)),
        brightness: Some((light * 100.0).round() as u32),
        mic: Some(t >= MIC),
        quiet_minutes: (t >= QUIET).then_some(60),
        city: Some("Kota: Jakarta".into()),
        fx: ControlsFx::default(),
    };
    let mut scene = cast.scene(View::Panel, Page::Controls, false);
    scene.controls = &view;
    scene.hovered = if (VOLUME.0 - 0.2..VOLUME.1 + 0.1).contains(&t) {
        Some(Hit::VolumeSlider)
    } else if (LIGHT.0 - 0.2..LIGHT.1 + 0.1).contains(&t) {
        Some(Hit::BrightnessSlider)
    } else if (MIC - 0.3..MIC + 0.2).contains(&t) {
        Some(Hit::MicToggle)
    } else if (QUIET - 0.3..QUIET + 0.2).contains(&t) {
        Some(Hit::QuietToggle)
    } else {
        None
    };
    let (ax, ay, k) = PANEL;
    studio.fade(0.45 * (1.0 - light));
    studio.glow(ax, ay + 140.0, 600.0, 240.0, TEAL, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k * mix(0.95, 1.0, ease_out(span(t, 0.0, 0.3))), ease_out(span(t, 0.0, 0.25)));
    // Kursor menyeret kedua slider lalu mengklik dua tombol.
    // (detik, titik): kursor bergerak lurus di antara titik-titik ini.
    let path = [
        (0.0, (860.0, 1050.0)),
        (VOLUME.0, knob(0.62, 56.0)),
        (VOLUME.1, knob(0.88, 56.0)),
        (LIGHT.0, knob(0.7, 90.0)),
        (LIGHT.1, knob(0.35, 90.0)),
        (MIC, spot(-128.0, 128.0)),
        (QUIET, spot(0.0, 128.0)),
        (QUIET + 0.3, spot(0.0, 128.0)),
        (QUIET + 0.8, (900.0, 1150.0)),
    ];
    let i = path.iter().rposition(|p| p.0 <= t).unwrap_or(0).min(path.len() - 2);
    let ((t0, a), (t1, b)) = (path[i], path[i + 1]);
    // Saat menyeret slider gerakannya mengikuti kenop (sama mulusnya), selain itu ease.
    let dragging = i == 1 || i == 3;
    let p = if dragging { ease_in_out(span(t, t0 + 0.05, t1 - t0)) } else { ease_in_out(span(t, t0, t1 - t0)) };
    let (cx, cy) = (mix(a.0, b.0, p), mix(a.1, b.1, p));
    studio.cursor(cx + 4.0, cy + 6.0, 1.5, span(t, 0.0, 0.15) * (1.0 - span(t, QUIET + 0.6, 0.3)));
    // Senyap: perisai di sekitar notch memantulkan kabar yang datang.
    let shield = ease_out(span(t, QUIET, 0.4));
    if shield > 0.0 {
        let (sx, sy) = (540.0, ay + 160.0);
        studio.glow(sx, sy, 520.0 * shield, 330.0 * shield, TEAL, 0.15);
        studio.arc(sx, sy, 470.0 * shield, 8.0, 0.22, 0.56, TEAL, 0.8);
        for (i, start) in [3.05f64, 3.4, 3.75].into_iter().enumerate() {
            let p = span(t, start, 0.7);
            if p <= 0.0 || p >= 1.0 {
                continue;
            }
            let side = if i % 2 == 0 { -1.0 } else { 1.0 };
            let hit = 0.45;
            let (x, y) = if p < hit {
                let q = p / hit;
                (mix(540.0 + side * 380.0, 540.0 + side * 140.0, q), mix(1250.0, sy + 470.0 * 0.92, q))
            } else {
                let q = (p - hit) / (1.0 - hit);
                (mix(540.0 + side * 140.0, 540.0 + side * 520.0, q), mix(sy + 470.0 * 0.92, 1350.0, q))
            };
            let bubble = rect(x - 70.0, y - 34.0, x + 70.0, y + 34.0);
            studio.rounded(bubble, 34.0, 0x2A3346, 1.0 - p * 0.5);
            studio.icon('\u{EA8F}', 30.0, x, y, 0xF2F4F8, 1.0 - p * 0.5);
            if (p - hit).abs() < 0.08 {
                studio.glow(x, y - 30.0, 90.0, 50.0, 0xFFFFFF, 0.6);
            }
        }
    }
    caption(studio, t, len, "Kontrol cepat", "Volume, kecerahan, mikrofon, senyap");
}

fn controls_sound(score: &mut Score) {
    score.cue(VOLUME.0, Cue::Tick { hz: 2200.0, gain: 0.12 });
    score.cue(VOLUME.0 + 0.05, Cue::Glide { len: VOLUME.1 - VOLUME.0, from: 400.0, to: 900.0, gain: 0.08 });
    score.cue(LIGHT.0, Cue::Tick { hz: 2200.0, gain: 0.12 });
    score.cue(LIGHT.0 + 0.05, Cue::Glide { len: LIGHT.1 - LIGHT.0, from: 900.0, to: 450.0, gain: 0.08 });
    score.cue(MIC, Cue::Click { gain: 0.3 });
    score.cue(MIC + 0.02, Cue::Pop { midi: 67.0, gain: 0.16 });
    score.cue(QUIET, Cue::Click { gain: 0.3 });
    score.cue(QUIET + 0.02, Cue::Glide { len: 0.4, from: 300.0, to: 700.0, gain: 0.14 });
    for at in [3.05 + 0.32, 3.4 + 0.32, 3.75 + 0.32] {
        score.cue(at, Cue::Thud { gain: 0.25 });
    }
}

// ---------------------------------------------------------------- 29. cuaca

const TYPE: f64 = 0.25;
const SAVE: f64 = 1.3;
const CITY: &str = "Jakarta";

fn weather(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, TEAL);
    let typed: String = CITY.chars().take(((t - TYPE).max(0.0) / 0.11) as usize).collect();
    let saved = t >= SAVE;
    let view = ControlsView { city: Some("Kota: Jakarta".into()), ..sample().controls };
    let mut scene = cast.scene(View::Panel, if saved { Page::Home } else { Page::Controls }, false);
    scene.controls = &view;
    if !saved {
        scene.city_draft = Some(&typed);
    } else {
        scene.weather = Some("29° Cerah · Jakarta");
        let slide = span(t, SAVE, 0.4);
        if slide < 1.0 {
            scene.page_from = Some((Page::Controls, slide, -1.0));
        }
    }
    let (ax, ay, k) = PANEL;
    studio.glow(ax, ay + 140.0, 600.0, 240.0, if saved { 0xFFD60A } else { TEAL }, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k * mix(0.95, 1.0, ease_out(span(t, 0.0, 0.3))), ease_out(span(t, 0.0, 0.25)));
    // Matahari terbit di belakang awan, suhunya muncul besar.
    let rise = ease_out(span(t, SAVE, 0.6));
    let (cx, cy) = (480.0, mix(1150.0, 960.0, rise));
    if rise > 0.0 {
        for i in 0..10 {
            let a = i as f32 / 10.0 * 2.0 * PI + t as f32 * 0.6;
            studio.line((cx + 120.0 * a.cos(), cy + 120.0 * a.sin()), (cx + 170.0 * a.cos(), cy + 170.0 * a.sin()), 10.0, 0xFFD60A, 0.8 * rise);
        }
        studio.glow(cx, cy, 260.0, 260.0, 0xFFD60A, 0.4 * rise);
        studio.circle(cx, cy, 95.0, 0xFFD60A, rise);
    }
    let drift = (t as f32 * 30.0) - 40.0;
    let (kx, ky) = (620.0 + drift, 1050.0);
    let cloud = ease_out(span(t, 0.0, 0.4));
    for (dx, dy, r) in [(-90.0, 20.0, 70.0), (0.0, -20.0, 95.0), (90.0, 20.0, 70.0)] {
        studio.circle(kx + dx, ky + dy, r, 0xE9EEF6, 0.95 * cloud);
    }
    studio.rounded(rect(kx - 160.0, ky + 10.0, kx + 160.0, ky + 90.0), 40.0, 0xE9EEF6, 0.95 * cloud);
    studio.label("29°", SERIF, 120.0, false, 820.0, 900.0, 0xFFFFFF, rise, 0);
    caption(studio, t, len, "Cuaca", "Cukup ketik nama kotanya");
    badge(studio, t, SAVE, len, "opsional · pakai internet", 1530.0, TEAL);
}

fn weather_sound(score: &mut Score) {
    score.cue(TYPE, Cue::Typing { len: SAVE - TYPE - 0.25, gain: 0.13 });
    score.cue(SAVE, Cue::Click { gain: 0.3 });
    score.cue_pan(SAVE, (0.4, -0.4), Cue::Whoosh { len: 0.4, from: 700.0, to: 2600.0, pan: (0.4, -0.4), gain: 0.14 });
    score.cue(SAVE + 0.2, Cue::Chime { notes: vec![79.0, 83.0, 88.0], gain: 0.12 });
}

// ---------------------------------------------------------------- 30. monitor sistem

fn system(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, TEAL);
    let load = ease_in_out(span(t, 0.3, 2.3));
    let cpu = mix(23.0, 93.0, load);
    let gb = 1_000_000_000.0;
    let stats = Stats {
        cpu,
        gpu: Some(mix(40.0, 88.0, load)),
        gpu_temp: Some(mix(64.0, 88.0, load)),
        ram_used: (mix(8.1, 14.6, load) * gb) as u64,
        ram_total: (16.0 * gb) as u64,
        disk_free: 120_000_000_000,
        disk_total: 512_000_000_000,
        down: mix(1_200_000.0, 38_000_000.0, load) as f64,
        up: mix(80_000.0, 2_400_000.0, load) as f64,
    };
    let mut scene = cast.scene(View::Panel, Page::System, false);
    scene.system = Some(&stats);
    scene.uptime = "3 jam 12 menit";
    let (ax, ay, k) = PANEL;
    let hot = if cpu > 85.0 { 0xFF453A } else if cpu > 60.0 { 0xFF9F0A } else { TEAL };
    studio.glow(ax, ay + 140.0, 600.0, 240.0, hot, 0.25);
    studio.notch(&scene, Theme::Black, ax, ay, k * mix(0.95, 1.0, ease_out(span(t, 0.0, 0.3))), ease_out(span(t, 0.0, 0.25)));
    // Spidometer CPU besar dengan zona warna.
    let (cx, cy, r) = (540.0, 1070.0, 220.0);
    let show = ease_out(span(t, 0.1, 0.4));
    let start = 0.625;
    for (from, to, color) in [(0.0, 0.6, 0x30D158), (0.6, 0.85, 0xFF9F0A), (0.85, 1.0, 0xFF453A)] {
        studio.arc(cx, cy, r, 26.0, start + 0.75 * from, 0.75 * (to - from) - 0.006, color, 0.35 * show);
    }
    studio.arc(cx, cy, r, 26.0, start, 0.75 * cpu / 100.0, hot, show);
    let angle = (start + 0.75 * cpu / 100.0) * 2.0 * PI - PI / 2.0;
    let shake = if cpu > 85.0 { (t * 60.0).sin() as f32 * 0.015 } else { 0.0 };
    studio.line((cx, cy), (cx + (r - 50.0) * (angle + shake).cos(), cy + (r - 50.0) * (angle + shake).sin()), 10.0, 0xFFFFFF, show);
    studio.circle(cx, cy, 18.0, 0xFFFFFF, show);
    studio.label(&format!("{cpu:.0}%"), SERIF, 90.0, false, cx, cy + 120.0, hot, show, 0);
    studio.label("CPU", SANS, 30.0, true, cx, cy + 190.0, 0xBFD8FF, show, 0);
    caption_at(studio, t, len, 1400.0, "Monitor sistem", "CPU, GPU, RAM, disk, jaringan, suhu");
}

fn system_sound(score: &mut Score) {
    score.cue(0.3, Cue::Riser { len: 2.3, gain: 0.18 });
    score.cue(2.6, Cue::Alarm { len: 0.3, gain: 0.08 });
}
