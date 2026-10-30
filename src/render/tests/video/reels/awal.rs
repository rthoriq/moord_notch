//! Bab 0, pembuka: "Notch.", notch di layar laptop, lalu cara membukanya (diperlambat, dengan
//! kurva pegasnya digambar di bawah).

use super::*;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Pembuka",
        tint: MAGENTA,
        shots: vec![
            Shot { beats: 4, energy: Energy::Quiet, draw: hook, sound: hook_sound, blur: &[] },
            Shot { beats: 5, energy: Energy::Full, draw: laptop, sound: laptop_sound, blur: &[(0.0, 0.9, 4), (2.1, 2.5, 4)] },
            Shot { beats: 9, energy: Energy::Light, draw: open, sound: open_sound, blur: &[] },
        ],
    }
}

fn hook(studio: &mut Studio, cast: &Cast, t: f64, _len: f64) {
    scene_hook(studio, cast, t);
}

fn hook_sound(score: &mut Score) {
    score.cue(0.05, Cue::Whoosh { len: 0.85, from: 700.0, to: 3500.0, pan: (0.0, 0.0), gain: 0.22 });
    score.cue(0.62, Cue::Pop { midi: 69.0, gain: 0.35 });
    score.cue(0.62, Cue::Thud { gain: 0.3 });
    score.cue(1.0, Cue::Boom { gain: 0.4 });
    for midi in [69.0, 72.0, 76.0] {
        score.cue(1.0, Cue::Bell { midi, gain: 0.2 });
    }
    score.cue(1.15, Cue::Tick { hz: 2600.0, gain: 0.08 });
    score.cue_pan(1.5, (0.2, 0.2), Cue::Bell { midi: 76.0, gain: 0.22 });
}

/// Kamera mundur dari pil sampai terlihat di laptop, lalu mendorong masuk lagi ke notch.
fn laptop(studio: &mut Studio, cast: &Cast, t: f64, _len: f64) {
    scene_laptop(studio, cast, t + 2.0);
}

fn laptop_sound(score: &mut Score) {
    score.cue(0.0, Cue::Whoosh { len: 0.9, from: 4000.0, to: 250.0, pan: (0.0, 0.0), gain: 0.32 });
    score.cue(2.1, Cue::Whoosh { len: 0.4, from: 300.0, to: 5000.0, pan: (0.0, 0.0), gain: 0.32 });
}

const PEEK_AT: f64 = 0.9;
const OPEN_AT: f64 = 1.3;
/// Selama 2 detik setelah membuka, waktu berjalan 0,3× supaya pantulannya terlihat.
const SLOW: (f64, f64, f64) = (OPEN_AT, OPEN_AT + 2.0, 0.3);
const CLOSE_AT: f64 = 3.95;

/// Detik "simulasi" untuk detik video `t` (gerak lambat di tengah).
fn warp(t: f64) -> f64 {
    let (start, end, rate) = SLOW;
    if t < start {
        t
    } else if t < end {
        start + (t - start) * rate
    } else {
        start + (end - start) * rate + (t - end)
    }
}

/// Kurva tinggi notch (0 = pil, 1 = panel) sejak mulai membuka, per 1/60 detik.
fn spring_curve() -> Vec<f32> {
    let peek = (View::Pill.size().0 + 12.0, View::Pill.size().1 + 4.0);
    let mut motion = Motion::new(View::Pill, peek);
    motion.go(View::Panel, View::Panel.size());
    (0..=48)
        .map(|i| {
            if i > 0 {
                motion.step();
            }
            (motion.size().1 - peek.1) / (View::Panel.size().1 - peek.1)
        })
        .collect()
}

/// Fitur 2: sentuh → intip → buka memantul, lalu menutup mulus.
fn open(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    let sim = warp(t);
    let peek = (View::Pill.size().0 + 12.0, View::Pill.size().1 + 4.0);
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (PEEK_AT, View::Pill, peek, 0.3),
        (OPEN_AT, View::Panel, View::Panel.size(), 1.0),
        (warp(CLOSE_AT), View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, sim);
    let opened = ((motion.size().1 - View::Pill.size().1) / (View::Panel.size().1 - View::Pill.size().1)).clamp(0.0, 1.2);
    let k = mix(3.3, 2.05, opened.min(1.0));
    let (ax, ay) = (540.0, 330.0);

    // Kursor datang dari kanan bawah ke pil, lalu pergi lagi sebelum notch menutup.
    let come = ease_in_out(span(t, 0.15, 0.7));
    let go = ease_in_out(span(t, CLOSE_AT - 0.35, 0.4));
    let (cx, cy) = (mix(mix(900.0, 575.0, come), 980.0, go), mix(mix(1050.0, 372.0, come), 900.0, go));

    let mut scene = cast.scene(View::Pill, Page::Home, false);
    motion.apply(&mut scene);
    scene.eyes.look = (((cx - ax) / 400.0).clamp(-1.0, 1.0), ((cy - ay) / 600.0).clamp(-0.3, 0.6));
    studio.glow(ax, ay + 150.0, 560.0, 260.0, CYAN, 0.18 + 0.12 * opened.min(1.0));
    studio.notch(&scene, Theme::Black, ax, ay, k, ease_out(span(t, 0.0, 0.2)));
    studio.cursor(cx, cy, 1.6, span(t, 0.1, 0.15) * (1.0 - span(t, CLOSE_AT + 0.1, 0.2)));

    // Penanda gerak lambat.
    let slow = span(t, SLOW.0, 0.15) * (1.0 - span(t, SLOW.1 - 0.15, 0.15));
    if slow > 0.0 {
        let area = rect(80.0, 262.0, 300.0, 318.0);
        studio.rounded(area, 28.0, 0xFFFFFF, 0.1 * slow);
        studio.icon('\u{E916}', 26.0, 118.0, 290.0, 0xFFFFFF, slow);
        studio.label("0,3×", SANS, 32.0, true, 146.0, 290.0, 0xFFFFFF, slow, -1);
    }

    // Grafik pegas: tinggi notch terhadap waktu, digambar sambil berjalan.
    let graph = rect(150.0, 930.0, 930.0, 1150.0);
    let show = ease_out(span(t, OPEN_AT - 0.2, 0.3)) * (1.0 - span(t, len - 0.3, 0.3));
    if show > 0.0 {
        let curve = spring_curve();
        let elapsed = ((sim - OPEN_AT) * 60.0).clamp(0.0, (curve.len() - 1) as f64) as usize;
        let x = |i: usize| graph.left + (graph.right - graph.left) * i as f32 / (curve.len() - 1) as f32;
        let y = |p: f32| graph.bottom - (graph.bottom - graph.top) * p / 1.25;
        studio.line((graph.left, graph.bottom), (graph.right, graph.bottom), 2.0, 0xFFFFFF, 0.25 * show);
        let mut dash = graph.left;
        while dash < graph.right {
            studio.line((dash, y(1.0)), ((dash + 14.0).min(graph.right), y(1.0)), 2.0, 0xFFFFFF, 0.3 * show);
            dash += 26.0;
        }
        studio.label("terbuka", SANS, 26.0, false, graph.right, y(1.0) - 26.0, 0xBFD8FF, 0.8 * show, 1);
        studio.label("pegas · redaman 0,72", SANS, 26.0, false, graph.left, graph.bottom + 34.0, 0xBFD8FF, 0.8 * show, -1);
        if t >= OPEN_AT {
            let points: Vec<(f32, f32)> = (0..=elapsed).map(|i| (x(i), y(curve[i]))).collect();
            studio.polyline(&points, 10.0, MAGENTA, 0.25 * show);
            studio.polyline(&points, 4.0, 0xFFFFFF, 0.95 * show);
            let head = points[points.len() - 1];
            studio.glow(head.0, head.1, 50.0, 50.0, MAGENTA, 0.6 * show);
            studio.circle(head.0, head.1, 9.0, 0xFFFFFF, show);
        }
    }
    caption(studio, t, len, "Sentuh untuk membuka", "Mengintip dulu, lalu terbuka memantul");
}

fn open_sound(score: &mut Score) {
    score.cue(PEEK_AT, Cue::Tick { hz: 2400.0, gain: 0.12 });
    score.cue(OPEN_AT, Cue::Glide { len: 0.9, from: 300.0, to: 900.0, gain: 0.2 });
    score.cue(OPEN_AT + 0.05, Cue::Whoosh { len: 1.2, from: 300.0, to: 1500.0, pan: (0.0, 0.0), gain: 0.14 });
    score.cue(CLOSE_AT, Cue::Glide { len: 0.25, from: 900.0, to: 350.0, gain: 0.14 });
}
