//! Bab 2, kabar ala Dynamic Island: volume (dan bisu), kecerahan, Caps/Num Lock, dan charger,
//! semuanya di strip setinggi pil yang melebar ke samping.

use super::peraga::{keycap, press};
use super::*;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Kabar ringkas",
        tint: ELECTRIC,
        shots: vec![
            Shot { beats: 8, energy: Energy::Full, draw: volume, sound: volume_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: brightness, sound: brightness_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: locks, sound: locks_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: charger, sound: charger_sound, blur: &[(0.7, 1.05, 6)] },
        ],
    }
}

const STRIP: (f32, f32, f32) = (540.0, 560.0, 3.0);

/// Strip yang membuka dari pil di awal adegan dan menutup lagi di akhirnya.
fn strip<'a>(cast: &'a Cast, t: f64, len: f64, hud: &'a Hud, fill: f32, roll: Option<&'a Roll>) -> Scene<'a> {
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (0.15, View::Strip, View::Strip.size(), 0.0),
        (len - 0.45, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Strip, Page::Home, false);
    motion.apply(&mut scene);
    scene.hud = Some(hud);
    scene.hud_compact = true;
    scene.hud_fill = fill;
    scene.hud_roll = roll;
    scene
}

/// Nilai yang berubah di detik-detik `steps`: (nilai sekarang yang bergeser halus, nilai
/// sebelumnya, detik perubahan terakhir).
fn stepped(t: f64, start: f32, steps: &[(f64, f32)]) -> (f32, f32, f64) {
    let mut value = start;
    let mut before = start;
    let mut at = f64::NEG_INFINITY;
    for &(when, next) in steps {
        if t >= when {
            before = value;
            value = next;
            at = when;
        }
    }
    let smooth = mix(before, value, ease_out(span(t, at, 0.2)));
    (smooth, before, at)
}

fn percent(level: f32) -> String {
    format!("{}", (level * 100.0).round())
}

// ---------------------------------------------------------------- 10. volume dan bisu

const LOUDER: [(f64, f32); 3] = [(0.55, 0.62), (0.9, 0.68), (1.25, 0.74)];
const MUTE: f64 = 2.2;
const UNMUTE: f64 = 3.05;

fn volume(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ELECTRIC);
    let (level, before, at) = stepped(t, 0.56, &LOUDER);
    let muted = (MUTE..UNMUTE).contains(&t);
    let hud = Hud::Volume { level: LOUDER.iter().rev().find(|s| t >= s.0).map_or(0.56, |s| s.1), muted };
    let roll = Roll { from: percent(before), t: span(t, at, 0.22), up: true };
    let rolling = roll.t > 0.0 && roll.t < 1.0;
    let scene = strip(cast, t, len, &hud, level, rolling.then_some(&roll));
    let (ax, ay, k) = STRIP;
    studio.glow(ax, ay + 50.0, 560.0, 180.0, CYAN, 0.25 + 0.2 * beat_pulse(t));
    // Gelombang suara keluar dari ujung kanan strip setiap volume naik.
    for &(when, _) in &LOUDER {
        let p = span(t, when, 0.6);
        if p > 0.0 && p < 1.0 {
            for ring in 0..2 {
                let q = (p - ring as f32 * 0.2).max(0.0);
                studio.arc(ax + 300.0, ay + 45.0, 60.0 + 160.0 * q, 6.0, 0.08, 0.34, CYAN, 0.7 * (1.0 - q));
            }
        }
    }
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Tombol volume turun, naik, dan bisu.
    let show = ease_out(span(t, 0.0, 0.3));
    let ups: Vec<f64> = LOUDER.iter().map(|s| s.0).collect();
    for (x, glyph, presses) in [(310.0, '\u{E993}', vec![]), (540.0, '\u{E995}', ups), (770.0, '\u{E74F}', vec![MUTE, UNMUTE])] {
        let top = keycap(studio, x, 880.0, 190.0, 150.0, "", press(t, &presses), CYAN, show);
        studio.icon(glyph, 52.0, top.0, top.1, 0xF2F4F8, show);
    }
    let label = if muted { "Bisu" } else { "Volume" };
    caption(studio, t, len, label, "Angka bergulir, bilahnya bergeser halus");
}

fn volume_sound(score: &mut Score) {
    for (i, &(at, _)) in LOUDER.iter().enumerate() {
        score.cue(at, Cue::Click { gain: 0.3 });
        score.cue(at + 0.02, Cue::Pop { midi: 72.0 + i as f64 * 2.0, gain: 0.18 });
    }
    score.cue_pan(MUTE, (0.4, 0.4), Cue::Click { gain: 0.3 });
    score.cue(MUTE + 0.02, Cue::Glide { len: 0.25, from: 800.0, to: 250.0, gain: 0.16 });
    score.cue_pan(UNMUTE, (0.4, 0.4), Cue::Click { gain: 0.3 });
    score.cue(UNMUTE + 0.02, Cue::Glide { len: 0.25, from: 250.0, to: 800.0, gain: 0.16 });
}

// ---------------------------------------------------------------- 11. kecerahan

const BRIGHTER: [(f64, f32); 4] = [(0.5, 0.7), (0.85, 0.85), (1.2, 1.0), (2.0, 0.6)];

fn brightness(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ELECTRIC);
    let (level, before, at) = stepped(t, 0.55, &BRIGHTER);
    // Seluruh "layar" ikut terang dan redup.
    studio.glow(540.0, 400.0, 1100.0, 900.0, 0xFFFFFF, 0.25 * (level - 0.5).max(0.0));
    studio.fade(0.55 * (1.0 - level));
    let hud = Hud::Brightness { level: BRIGHTER.iter().rev().find(|s| t >= s.0).map_or(0.55, |s| s.1) };
    let roll = Roll { from: percent(before), t: span(t, at, 0.22), up: level >= before };
    let rolling = roll.t > 0.0 && roll.t < 1.0;
    let scene = strip(cast, t, len, &hud, level, rolling.then_some(&roll));
    let (ax, ay, k) = STRIP;
    // Matahari besar di belakang yang sinarnya memanjang.
    let rays = 12;
    for i in 0..rays {
        let a = i as f32 / rays as f32 * 2.0 * PI + t as f32 * 0.4;
        let (r0, r1) = (150.0, 150.0 + 90.0 * level);
        let (cx, cy) = (540.0, 940.0);
        studio.line((cx + r0 * a.cos(), cy + r0 * a.sin()), (cx + r1 * a.cos(), cy + r1 * a.sin()), 10.0, 0xFFD60A, 0.3 + 0.5 * level);
    }
    studio.glow(540.0, 940.0, 260.0 * (0.6 + level), 260.0 * (0.6 + level), 0xFFD60A, 0.35 * level);
    studio.circle(540.0, 940.0, 110.0, 0xFFD60A, 0.85);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    caption(studio, t, len, "Kecerahan", "Layar berubah, kabarnya ikut muncul");
}

fn brightness_sound(score: &mut Score) {
    for (i, &(at, _)) in BRIGHTER.iter().enumerate() {
        score.cue(at, Cue::Click { gain: 0.26 });
        let midi = if i < 3 { 76.0 + i as f64 * 2.0 } else { 71.0 };
        score.cue(at + 0.02, Cue::Pop { midi, gain: 0.16 });
    }
    score.cue(0.5, Cue::Whoosh { len: 1.0, from: 600.0, to: 5000.0, pan: (0.0, 0.0), gain: 0.1 });
}

// ---------------------------------------------------------------- 12. Caps Lock dan Num Lock

const CAPS_ON: f64 = 0.5;
const NUM_OFF: f64 = 1.75;

fn locks(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ELECTRIC);
    let num = t >= NUM_OFF;
    let hud = if num { Hud::LockKey { caps: false, on: false } } else { Hud::LockKey { caps: true, on: t >= CAPS_ON } };
    let scene = strip(cast, t, len, &hud, 0.0, None);
    let (ax, ay, k) = STRIP;
    studio.glow(ax, ay + 50.0, 560.0, 180.0, CYAN, 0.25);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    let show = ease_out(span(t, 0.0, 0.3));
    for (x, label, at, on) in [(345.0, "Caps Lock", CAPS_ON, t >= CAPS_ON), (735.0, "Num Lock", NUM_OFF, t < NUM_OFF)] {
        let top = keycap(studio, x, 840.0, 330.0, 130.0, label, press(t, &[at]), CYAN, show);
        let lamp = (top.0 - 130.0, top.1 - 38.0);
        if on {
            studio.glow(lamp.0, lamp.1, 30.0, 30.0, 0x30D158, 0.6 * show);
        }
        studio.circle(lamp.0, lamp.1, 8.0, if on { 0x30D158 } else { 0x3A4255 }, show);
    }
    // Ketikan berubah jadi huruf besar satu per satu begitu Caps Lock menyala.
    let word = "notch";
    let mut x = 540.0 - studio.measure("NOTCH", SERIF, 150.0, false) / 2.0;
    for (i, ch) in word.chars().enumerate() {
        let flip = ease_out(span(t, CAPS_ON + 0.1 + i as f64 * 0.07, 0.2));
        let upper = ch.to_ascii_uppercase().to_string();
        let w = studio.measure(&upper, SERIF, 150.0, false);
        let lower = ch.to_string();
        studio.label(&lower, SERIF, 150.0, false, x + w / 2.0, 1060.0 - 40.0 * flip, 0xBFD8FF, show * (1.0 - flip), 0);
        studio.label(&upper, SERIF, 150.0, false, x + w / 2.0, 1060.0 + 40.0 * (1.0 - flip), 0xFFFFFF, show * flip, 0);
        x += w;
    }
    caption_at(studio, t, len, 1320.0, "Caps Lock & Num Lock", "Nyala atau mati, langsung kelihatan");
}

fn locks_sound(score: &mut Score) {
    score.cue_pan(CAPS_ON, (-0.3, -0.3), Cue::Click { gain: 0.35 });
    score.cue(CAPS_ON + 0.1, Cue::Typing { len: 0.35, gain: 0.12 });
    score.cue(CAPS_ON + 0.02, Cue::Pop { midi: 79.0, gain: 0.18 });
    score.cue_pan(NUM_OFF, (0.3, 0.3), Cue::Click { gain: 0.35 });
    score.cue(NUM_OFF + 0.02, Cue::Pop { midi: 72.0, gain: 0.16 });
}

// ---------------------------------------------------------------- 13. charger

const PLUG: f64 = 0.85;

fn charger(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, ELECTRIC);
    let plugged = t >= PLUG;
    let hud = Hud::Power { plugged: true, percent: Some(81), low: false };
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (PLUG + 0.05, View::Strip, View::Strip.size(), 0.0),
        (len - 0.45, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Strip, Page::Home, false);
    motion.apply(&mut scene);
    if plugged {
        scene.hud = Some(&hud);
        scene.hud_compact = true;
        scene.hud_fill = 0.81;
    }
    let (ax, ay, k) = STRIP;
    let zap = 1.0 - span(t, PLUG, 0.45);
    studio.glow(ax, ay + 50.0, 600.0, 200.0, 0x30D158, if plugged { 0.3 * (0.5 + 0.5 * zap) } else { 0.1 });
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Laptop dilihat dari samping dengan port USB-C di ujung kanan; kabel datang dari kanan.
    let body = rect(60.0, 905.0, 720.0, 985.0);
    studio.rounded(rect(body.left + 6.0, body.top + 14.0, body.right + 6.0, body.bottom + 14.0), 22.0, 0x000000, 0.35);
    studio.rounded(body, 22.0, 0x1C2333, 1.0);
    studio.line((body.left + 30.0, body.top + 4.0), (body.right - 30.0, body.top + 4.0), 3.0, 0xFFFFFF, 0.12);
    studio.rounded(rect(body.right - 12.0, 932.0, body.right + 2.0, 958.0), 4.0, 0x05070C, 1.0);
    let come = ease_out(span(t, 0.1, PLUG - 0.1));
    let tip = mix(1150.0, body.right + 2.0, come);
    studio.rounded(rect(tip + 70.0, 937.0, 1200.0, 953.0), 8.0, 0xD9DEE8, 1.0);
    studio.rounded(rect(tip + 16.0, 920.0, tip + 96.0, 970.0), 14.0, 0xB8C0CC, 1.0);
    studio.rounded(rect(tip, 931.0, tip + 22.0, 959.0), 6.0, 0x8A93A3, 1.0);
    // Kilat menyambar dari port ke strip.
    if plugged && zap > 0.0 {
        let flicker = if ((t - PLUG) * 30.0) as i32 % 2 == 0 { 1.0 } else { 0.55 };
        let bolt = [(720.0, 905.0), (650.0, 820.0), (715.0, 780.0), (600.0, 690.0), (560.0, 660.0)];
        studio.glow(650.0, 790.0, 200.0, 220.0, 0x30D158, 0.5 * zap);
        studio.polyline(&bolt, 26.0, 0x30D158, 0.35 * zap * flicker);
        studio.polyline(&bolt, 9.0, 0xEFFFF4, zap * flicker);
        studio.flash(0.18 * zap * zap);
    }
    caption_at(studio, t, len, 1320.0, "Charger", "Dicolok? Notch langsung tahu");
}

fn charger_sound(score: &mut Score) {
    score.cue(0.1, Cue::Whoosh { len: PLUG - 0.1, from: 300.0, to: 1500.0, pan: (0.0, 0.0), gain: 0.14 });
    score.cue(PLUG, Cue::Thud { gain: 0.4 });
    score.cue(PLUG, Cue::Zap { gain: 0.4 });
    score.cue(PLUG + 0.1, Cue::Chime { notes: vec![76.0, 83.0, 88.0], gain: 0.12 });
}
