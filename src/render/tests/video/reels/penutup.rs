//! Bab 10, penutup: pasang tanpa admin, lalu nama, ajakan mengunduh, dan alamat GitHub.

use super::peraga::window;
use super::*;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Coba sekarang",
        tint: ELECTRIC,
        shots: vec![
            Shot { beats: 8, energy: Energy::Light, draw: install, sound: install_sound, blur: &[] },
            Shot { beats: 16, energy: Energy::Full, draw: outro, sound: outro_sound, blur: &[] },
        ],
    }
}

const OPEN: [f64; 2] = [0.45, 0.58];
/// Pukulan terakhir lagu jatuh di detik ini dari awal penutup (lagu "Julia" yang diperpanjang).
const FINAL_HIT: f64 = 6.0;
const DIALOG: f64 = 0.8;
const ACCEPT: f64 = 1.55;
const START: f64 = 2.35;
const WELCOME: f64 = 2.7;

/// Ikon aplikasi: pil hitam dengan dua mata.
fn app_icon(studio: &mut Studio, cx: f32, cy: f32, size: f32, alpha: f32) {
    studio.rounded(rect(cx - size, cy - size, cx + size, cy + size), size * 0.45, 0x1C2333, alpha);
    studio.rounded(rect(cx - size * 0.8, cy - size * 0.32, cx + size * 0.8, cy + size * 0.32), size * 0.32, 0x000000, alpha);
    for dx in [-0.22, 0.22] {
        studio.rounded(rect(cx + size * dx - size * 0.07, cy - size * 0.17, cx + size * dx + size * 0.07, cy + size * 0.17), size * 0.07, 0xF5F5F7, alpha);
    }
}

fn install(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ELECTRIC);
    // File Explorer, folder Unduhan.
    let explorer = ease_out(span(t, 0.0, 0.3)) * (1.0 - span(t, START, 0.2));
    let area = rect(110.0, 520.0, 970.0, 960.0);
    window(studio, area, "Unduhan", explorer);
    let files = [("notch.exe", true), ("Laporan Q3.pdf", false), ("foto liburan.jpg", false)];
    for (i, (name, app)) in files.into_iter().enumerate() {
        let x = area.left + 150.0 + i as f32 * 280.0;
        let y = area.top + 200.0;
        if app {
            app_icon(studio, x, y, 70.0, explorer);
        } else {
            studio.icon(if i == 1 { '\u{E8A5}' } else { '\u{EB9F}' }, 110.0, x, y, 0xC9D1E0, explorer);
        }
        studio.label(name, SANS, 30.0, app, x, y + 120.0, 0xF2F4F8, explorer, 0);
    }
    let icon = (area.left + 150.0, area.top + 200.0);
    for &at in &OPEN {
        let r = span(t, at, 0.4);
        if r > 0.0 && r < 1.0 {
            studio.ring(icon.0, icon.1, 80.0 + 80.0 * r, 4.0, 0xFFFFFF, 0.7 * (1.0 - r));
        }
    }
    // Dialog pemasangan.
    let dialog = pop_in(t, DIALOG, 0.35).min(1.0) * (1.0 - span(t, START - 0.2, 0.25));
    let button = (680.0, 860.0);
    if dialog > 0.0 {
        let d = rect(170.0, 610.0, 910.0, 920.0);
        studio.rounded(rect(d.left + 8.0, d.top + 18.0, d.right + 8.0, d.bottom + 18.0), 26.0, 0x000000, 0.4 * dialog);
        studio.rounded(d, 26.0, 0x1C2333, dialog);
        app_icon(studio, d.left + 80.0, d.top + 80.0, 40.0, dialog);
        studio.label("Pasang NotchWindows?", SANS, 40.0, true, d.left + 140.0, d.top + 70.0, 0xFFFFFF, dialog, -1);
        studio.label("Tanpa hak Administrator. Bisa dicabut", SANS, 30.0, false, d.left + 140.0, d.top + 125.0, 0xC9D1E0, dialog, -1);
        studio.label("dari Pengaturan > Aplikasi.", SANS, 30.0, false, d.left + 140.0, d.top + 165.0, 0xC9D1E0, dialog, -1);
        let progress = ease_in_out(span(t, ACCEPT + 0.1, START - ACCEPT - 0.4));
        if t < ACCEPT + 0.05 {
            studio.rounded(rect(button.0 - 90.0, button.1 - 32.0, button.0 + 90.0, button.1 + 32.0), 32.0, ELECTRIC, dialog);
            studio.label("Pasang", SANS, 32.0, true, button.0, button.1, 0xFFFFFF, dialog, 0);
            studio.rounded(rect(button.0 - 300.0, button.1 - 32.0, button.0 - 120.0, button.1 + 32.0), 32.0, 0x2A3346, dialog);
            studio.label("Nanti", SANS, 32.0, false, button.0 - 210.0, button.1, 0xF2F4F8, dialog, 0);
        } else {
            let bar = rect(d.left + 60.0, button.1 - 8.0, d.right - 60.0, button.1 + 8.0);
            studio.rounded(bar, 8.0, 0x2A3346, dialog);
            studio.rounded(rect(bar.left, bar.top, bar.left + (bar.right - bar.left) * progress, bar.bottom), 8.0, ELECTRIC, dialog);
        }
    }
    let path = [(0.0, (860.0, 1150.0)), (OPEN[0], icon), (ACCEPT, button), (START, (900.0, 1200.0))];
    let i = path.iter().rposition(|p| p.0 <= t).unwrap_or(0).min(path.len() - 2);
    let ((t0, a), (t1, b)) = (path[i], path[i + 1]);
    let p = ease_in_out(span(t, t0.max(t1 - 0.45), 0.45));
    studio.cursor(mix(a.0, b.0, p), mix(a.1, b.1, p), 1.5, span(t, 0.0, 0.2) * (1.0 - span(t, START, 0.2)));
    // Menu Start dengan NotchWindows, dan notch yang menyapa.
    let start = ease_out(span(t, START + 0.2, 0.4));
    if start > 0.0 {
        let menu = rect(170.0, 760.0 + (1.0 - start) * 200.0, 910.0, 1180.0 + (1.0 - start) * 200.0);
        studio.rounded(menu, 26.0, 0x1C2333, start);
        studio.rounded(rect(menu.left + 30.0, menu.top + 30.0, menu.right - 30.0, menu.top + 90.0), 30.0, 0x2A3346, start);
        studio.label("Cari aplikasi", SANS, 28.0, false, menu.left + 70.0, menu.top + 60.0, 0x8F9BB3, start, -1);
        for j in 0..8 {
            let (col, row) = ((j % 4) as f32, (j / 4) as f32);
            let x = menu.left + 110.0 + col * 175.0;
            let y = menu.top + 180.0 + row * 140.0;
            if j == 0 {
                studio.rounded(rect(x - 80.0, y - 60.0, x + 80.0, y + 80.0), 18.0, ELECTRIC, 0.25 * start);
                app_icon(studio, x, y - 5.0, 36.0, start);
                studio.label("NotchWindows", SANS, 22.0, true, x, y + 55.0, 0xFFFFFF, start, 0);
            } else {
                let color = [CYAN, MAGENTA, 0xFF9F0A, 0x30D158, 0xFFD60A, 0xFF453A, 0x5E5CE6][j - 1];
                studio.rounded(rect(x - 34.0, y - 39.0, x + 34.0, y + 29.0), 14.0, color, 0.5 * start);
            }
        }
    }
    let keys = [
        (WELCOME - 0.3, View::Pill, View::Pill.size(), 0.0f32),
        (WELCOME, View::Panel, View::Panel.size(), 1.0),
    ];
    let motion = simulate(&keys, t.max(WELCOME - 0.3));
    let mut scene = cast.scene(View::Panel, Page::Home, false);
    motion.apply(&mut scene);
    scene.welcome = t >= WELCOME;
    studio.notch(&scene, Theme::Black, 540.0, 330.0, 2.0, ease_out(span(t, START - 0.1, 0.3)));
    caption_at(studio, t, len, 1330.0, "Pasang tanpa admin", "Langsung ada di menu Start");
}

fn install_sound(score: &mut Score) {
    for &at in &OPEN {
        score.cue(at, Cue::Click { gain: 0.28 });
    }
    score.cue(DIALOG, Cue::Pop { midi: 72.0, gain: 0.18 });
    score.cue(ACCEPT, Cue::Click { gain: 0.3 });
    score.cue(ACCEPT + 0.1, Cue::Glide { len: START - ACCEPT - 0.4, from: 400.0, to: 1100.0, gain: 0.08 });
    score.cue(START, Cue::Whoosh { len: 0.45, from: 300.0, to: 2000.0, pan: (0.0, 0.0), gain: 0.16 });
    score.cue(WELCOME, Cue::Chime { notes: vec![79.0, 84.0, 88.0], gain: 0.14 });
}

/// Penutup: pita cahaya menyatu ke pil, nama, ajakan mengunduh, dan alamat GitHub; layar memudar
/// di detik-detik terakhir bersama ekor lagu.
fn outro(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    let converge = ease_in_out(span(t, 0.0, 1.2));
    studio.background(t + 24.0, 0.85, converge * 0.85, MAGENTA);
    // Denyut cahaya tepat di pukulan terakhir lagu.
    let hit = 1.0 - span(t, FINAL_HIT, 0.8);
    let hit = if t >= FINAL_HIT { hit } else { 0.0 };
    studio.glow(540.0, 760.0, 560.0 + 200.0 * hit, 260.0 + 80.0 * hit, CYAN, 0.3 + 0.25 * converge + 0.35 * hit);
    let appear = span(t, 0.1, 0.5);
    let mut scene = cast.scene(View::Pill, Page::Home, false);
    scene.eyes.open = blink(t, 2.4) * blink(t, 4.6);
    if t >= 2.9 {
        scene.eyes.mood = Mood::Happy;
    }
    let k = 3.4 * mix(0.4, 1.0, back_out(appear)) * (1.0 + 0.04 * hit);
    studio.notch(&scene, Theme::Black, 540.0, 720.0 - 15.0 * k, k, ease_out(appear));
    studio.reveal(t, 0.5, 0.4, "NotchWindows", SERIF, 150.0, false, 1000.0, 0xFFFFFF, None);
    studio.reveal(t, 1.0, 0.35, "Gratis · Open source · Tanpa akun", SANS, 46.0, false, 1120.0, 0xBFD8FF, None);
    let button = ease_out(span(t, 1.5, 0.35));
    if button > 0.0 {
        let area = rect(290.0, 1380.0 - 46.0, 790.0, 1380.0 + 46.0);
        studio.glow(540.0, 1380.0, 320.0, 90.0, CYAN, 0.25 * button);
        studio.rounded(area, 46.0, 0x0A84FF, 0.9 * button);
        studio.text("Unduh di GitHub", SANS, 44.0, true, 1380.0, 0xFFFFFF, button, 1.0);
    }
    let url = ease_out(span(t, 1.75, 0.35));
    studio.label("github.com/rthoriq/moord_notch", SANS, 38.0, false, 540.0, 1480.0, 0xBFD8FF, url, 0);
    studio.flash(0.25 * hit * hit);
    studio.fade(ease_in_out(span(t, len - 1.6, 1.4)));
}

fn outro_sound(score: &mut Score) {
    score.cue_pan(0.0, (-0.5, 0.5), Cue::Whoosh { len: 1.2, from: 6000.0, to: 800.0, pan: (-0.5, 0.5), gain: 0.16 });
    score.cue(0.1, Cue::Pop { midi: 72.0, gain: 0.3 });
    score.cue(1.5, Cue::Pop { midi: 84.0, gain: 0.22 });
    score.cue(2.4, Cue::Tick { hz: 2600.0, gain: 0.08 });
    score.cue(2.9, Cue::Chime { notes: vec![84.0, 88.0, 91.0, 96.0], gain: 0.16 });
}
