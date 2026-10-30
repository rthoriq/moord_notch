//! Bab 9, gaya kamu: ekspresi maskot dan enam tema.

use super::*;

const PINK: u32 = 0xFF6482;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Gaya kamu",
        tint: PINK,
        shots: vec![
            Shot { beats: 10, energy: Energy::Light, draw: mascot, sound: mascot_sound, blur: &[] },
            Shot { beats: 5, energy: Energy::Full, draw: themes, sound: themes_sound, blur: &[(0.0, 2.5, 24)] },
        ],
    }
}

/// Babak ekspresi maskot: (mulai, nama).
const MOODS: [(f64, &str); 5] = [(0.0, "melirik kursor"), (1.1, "mengantuk jam 23.00"), (2.1, "kaget"), (2.9, "bangun setelah buka kunci"), (4.0, "senang")];

fn mascot(studio: &mut Studio, _cast: &Cast, t: f64, len: f64) {
    let stage = MOODS.iter().rposition(|m| t >= m.0).unwrap_or(0);
    let night = span(t, 1.1, 0.2) * (1.0 - span(t, 2.0, 0.2));
    backdrop(studio, t, if stage == 1 { 0x5E5CE6 } else { PINK });
    studio.fade(0.4 * night);
    let (cx, cy) = (540.0, 760.0);
    let appear = ease_out(span(t, 0.0, 0.3));
    // Pil besar tempat mata maskot tinggal.
    let pill = rect(cx - 300.0, cy - 140.0, cx + 300.0, cy + 140.0);
    studio.glow(cx, cy, 520.0, 260.0, PINK, 0.25 * appear);
    studio.rounded(pill, 140.0, 0x000000, appear);
    let mut eyes = Eyes::default();
    let mut mx = 540.0;
    let my = 1080.0;
    match stage {
        0 => {
            let path = ease_in_out(span(t, 0.1, 0.45)) - 2.0 * ease_in_out(span(t, 0.55, 0.45));
            mx = 540.0 + path * 360.0;
            eyes.look = (path.clamp(-1.0, 1.0), 0.3);
        }
        1 => {
            eyes.mood = Mood::Sleepy;
            eyes.open = 0.8 + 0.2 * ((t * 2.0).sin() as f32);
        }
        2 => eyes.mood = Mood::Surprised,
        3 => {
            // Terpejam, terbuka pelan, lalu berkedip.
            let wake = ease_in_out(span(t, 3.15, 0.5));
            eyes.open = mix(0.06, 1.0, wake) * blink(t, 3.75);
        }
        _ => eyes.mood = Mood::Happy,
    }
    studio.eyes(cx, cy, 60.0, 140.0, 120.0, eyes, appear);
    if stage == 0 {
        studio.cursor(mx, my, 2.0, appear * (1.0 - span(t, 1.0, 0.1)));
    }
    match stage {
        1 => {
            // Bulan sabit dan "z z".
            studio.circle(800.0, 520.0, 60.0, 0xFFE9A8, night);
            studio.circle(828.0, 500.0, 58.0, 0x0A0D18, night);
            for i in 0..3 {
                let p = (((t - 1.1) * 0.8 + i as f64 * 0.33) % 1.0) as f32;
                studio.label("z", SERIF, 40.0 + 20.0 * i as f32, false, 880.0 + 40.0 * p, 640.0 - 120.0 * p, 0xFFFFFF, night * (1.0 - p), 0);
            }
        }
        2 => {
            let bang = pop_in(t, 2.1, 0.3);
            studio.circle(860.0, 600.0, 46.0 * bang.min(1.2), 0xFF453A, 1.0);
            studio.label("!", SANS, 60.0 * bang.min(1.2), true, 860.0, 598.0, 0xFFFFFF, 1.0, 0);
        }
        3 => {
            let lock = ease_out(span(t, 2.9, 0.3)) * (1.0 - span(t, 3.8, 0.2));
            studio.icon('\u{E785}', 70.0, 860.0, 590.0, 0xFFFFFF, lock);
        }
        4 => {
            for i in 0..6 {
                let a = i as f32 / 6.0 * 2.0 * PI + t as f32;
                let r = 360.0 + 30.0 * ((t * 4.0) as f32 + i as f32).sin();
                let p = pop_in(t, 4.0 + i as f64 * 0.05, 0.3).min(1.0);
                studio.label("✦", SANS, 44.0 * p, false, cx + r * a.cos(), cy + r * 0.55 * a.sin(), 0xFFD60A, p, 0);
            }
        }
        _ => {}
    }
    // Nama ekspresi yang sedang tampil.
    let (start, name) = MOODS[stage];
    let p = ease_out(span(t, start, 0.25));
    let w = studio.measure(name, SANS, 38.0, true) + 60.0;
    let chip = rect(540.0 - w / 2.0, 990.0 + (1.0 - p) * 16.0, 540.0 + w / 2.0, 1062.0 + (1.0 - p) * 16.0);
    studio.rounded(chip, 36.0, 0x1C2333, p);
    studio.label(name, SANS, 38.0, true, 540.0, (chip.top + chip.bottom) / 2.0, 0xFFFFFF, p, 0);
    caption_at(studio, t, len, 1330.0, "Maskot yang hidup", "Ekspresinya ikut keadaan");
}

fn mascot_sound(score: &mut Score) {
    score.cue_pan(0.1, (0.0, 0.6), Cue::Whoosh { len: 0.45, from: 900.0, to: 1600.0, pan: (0.0, 0.6), gain: 0.07 });
    score.cue_pan(0.55, (0.6, -0.6), Cue::Whoosh { len: 0.45, from: 900.0, to: 1600.0, pan: (0.6, -0.6), gain: 0.07 });
    score.cue(1.1, Cue::Glide { len: 0.6, from: 600.0, to: 250.0, gain: 0.1 });
    score.cue(2.1, Cue::Pop { midi: 88.0, gain: 0.22 });
    score.cue(2.9, Cue::Click { gain: 0.2 });
    score.cue(3.15, Cue::Glide { len: 0.5, from: 300.0, to: 800.0, gain: 0.1 });
    score.cue(3.75, Cue::Tick { hz: 2600.0, gain: 0.1 });
    score.cue(4.0, Cue::Chime { notes: vec![84.0, 88.0, 91.0, 96.0], gain: 0.16 });
}

/// Enam tema bergeser seperti carousel (adegan dari video teaser).
fn themes(studio: &mut Studio, cast: &Cast, t: f64, _len: f64) {
    scene_themes(studio, cast, t + 21.5);
}

fn themes_sound(score: &mut Score) {
    for i in 0..6 {
        let lift = 1.15f64.powi(i);
        score.cue_pan(i as f64 * 0.4, (0.7, -0.7), Cue::Whoosh { len: 0.3, from: 450.0 * lift, to: 2600.0 * lift, pan: (0.7, -0.7), gain: 0.17 });
    }
}
