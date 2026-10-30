//! Bab 1, musik: pil musik, ganti lagu, pil lirik, kolom lirik, seret bar, gestur dua jari,
//! dan lirik untuk YouTube.

use super::peraga::{finger, second_song, sunset_art, touchpad, window};
use super::*;
use super::super::super::aset::LYRICS;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Musik",
        tint: MAGENTA,
        shots: vec![
            Shot { beats: 6, energy: Energy::Full, draw: pill, sound: pill_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: switch, sound: switch_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: lyric_pill, sound: lyric_pill_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: column, sound: column_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: seek, sound: seek_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: swipe, sound: swipe_sound, blur: &[(0.55, 1.0, 6), (1.95, 2.4, 6)] },
            Shot { beats: 9, energy: Energy::Light, draw: youtube, sound: youtube_sound, blur: &[(1.55, 2.1, 6)] },
        ],
    }
}

/// Titik di notch (koordinat DIP dari tengah atasnya) dalam piksel video.
fn at(ax: f32, ay: f32, k: f32, x: f32, y: f32) -> (f32, f32) {
    (ax + x * k, ay + y * k)
}

// ---------------------------------------------------------------- 3. pil musik

const DRIPS: [f64; 3] = [0.7, 1.0, 1.3];

/// Fitur 3: tetes warna dari sampul jatuh ke gelombang di pil.
fn pill(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    let (ax, ay, k) = (540.0, 640.0, 4.2 * mix(0.9, 1.0, ease_out(span(t, 0.0, 0.4))));
    let scene = cast.scene(View::Pill, Page::Home, true);
    let cover = at(ax, ay, k, -72.0, 15.0);
    let wave = at(ax, ay, k, 70.0, 15.0);
    studio.glow(ax, ay + 70.0, 520.0, 220.0, CYAN, 0.2);
    // Gelombang menyala tiap kali tetesan mendarat.
    let landed: f32 = DRIPS.iter().map(|&d| (1.0 - span(t, d + 0.35, 0.4)) * span(t, d + 0.34, 0.01)).sum();
    studio.glow(wave.0, wave.1, 160.0, 110.0, CYAN, 0.55 * landed.min(1.0));
    studio.notch(&scene, Theme::Black, ax, ay, k, ease_out(span(t, 0.0, 0.25)));
    // Palet warna keluar dari sampul.
    let palette = [(0x5B5BFF, -120.0f32), (0xB04BFF, 0.0), (CYAN, 120.0)];
    for (i, (hex, dx)) in palette.into_iter().enumerate() {
        let p = pop_in(t, 0.25 + i as f64 * 0.08, 0.35);
        let x = mix(cover.0, cover.0 + dx * 0.9 + 60.0, p.min(1.0));
        let y = mix(cover.1, ay - 140.0, p.min(1.0));
        let fade = 1.0 - span(t, 1.7, 0.3);
        studio.circle(x, y, 30.0 * p, hex, fade);
        if hex == CYAN {
            studio.ring(x, y, 40.0 * p, 4.0, 0xFFFFFF, 0.8 * fade * span(t, 0.55, 0.15));
        }
    }
    // Tiga tetes warna terpilih melengkung dari palet ke gelombang.
    let source = (cover.0 + 120.0 * 0.9 + 60.0, ay - 140.0);
    for &d in &DRIPS {
        let p = ease_in_out(span(t, d, 0.35));
        if p <= 0.0 || p >= 1.0 {
            continue;
        }
        let x = mix(source.0, wave.0, p);
        let y = mix(source.1, wave.1, p) - 90.0 * (p * PI).sin();
        for trail in 0..6 {
            let q = (p - trail as f32 * 0.03).max(0.0);
            let tx = mix(source.0, wave.0, q);
            let ty = mix(source.1, wave.1, q) - 90.0 * (q * PI).sin();
            studio.circle(tx, ty, 14.0 - trail as f32 * 2.0, CYAN, 0.5 - trail as f32 * 0.08);
        }
        studio.glow(x, y, 50.0, 50.0, CYAN, 0.6);
        studio.circle(x, y, 14.0, 0xFFFFFF, 1.0);
    }
    caption(studio, t, len, "Musik di pil", "Gelombangnya ikut warna sampul");
}

fn pill_sound(score: &mut Score) {
    for (i, &d) in DRIPS.iter().enumerate() {
        score.cue(d + 0.35, Cue::Pop { midi: 76.0 + i as f64 * 3.0, gain: 0.24 });
    }
    score.cue(0.25, Cue::Chime { notes: vec![84.0, 88.0, 91.0], gain: 0.09 });
}

// ---------------------------------------------------------------- 4. ganti lagu

const CLICK_NEXT: f64 = 1.05;

/// Fitur 4: klik ⏭, lagu dan pendaran panel berganti warna.
fn switch(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    let change = ease_in_out(span(t, CLICK_NEXT, 0.35));
    backdrop(studio, t, if change < 0.5 { MAGENTA } else { 0xFF9F0A });
    let (ax, ay, k) = (540.0, 540.0, 2.15);
    let next = at(ax, ay, k, 48.0, 125.0);
    // Riak warna baru menyebar dari tombol ⏭.
    let ripple = ease_out(span(t, CLICK_NEXT, 0.9));
    if ripple > 0.0 {
        studio.glow(next.0, next.1, 1400.0 * ripple, 1400.0 * ripple, 0xFF9F0A, 0.3 * (1.0 - ripple * 0.6));
        studio.ring(next.0, next.1, 700.0 * ripple, 6.0, 0xFFB340, 0.6 * (1.0 - ripple));
    }
    let later = second_song();
    for (media, art, opacity) in [(&cast.data.media, None, 1.0 - change), (&later, Some(sunset_art()), change)] {
        if opacity <= 0.01 {
            continue;
        }
        match &art {
            Some(art) => studio.set_art(art),
            None => studio.set_art(&gradient_art()),
        }
        let mut scene = cast.scene(View::Panel, Page::Home, true);
        scene.media = Some(media);
        scene.shadow = 1.0;
        scene.hovered = (t > 0.75 && t < CLICK_NEXT + 0.3).then_some(Hit::Media(Button::Next));
        studio.notch(&scene, Theme::Black, ax, ay, k, opacity);
    }
    studio.set_art(&gradient_art());
    let come = ease_in_out(span(t, 0.2, 0.6));
    let press = 1.0 - (span(t, CLICK_NEXT - 0.05, 0.05) - span(t, CLICK_NEXT + 0.05, 0.1));
    studio.cursor(mix(860.0, next.0 - 6.0, come), mix(1000.0, next.1 + 6.0, come), 1.6 * mix(1.0, 0.88, 1.0 - press), span(t, 0.1, 0.15));
    caption(studio, t, len, "Ganti lagu, ganti warna", "Pendaran panel mengikuti sampul");
}

fn switch_sound(score: &mut Score) {
    score.cue(CLICK_NEXT, Cue::Click { gain: 0.35 });
    score.cue(CLICK_NEXT + 0.02, Cue::Whoosh { len: 0.6, from: 600.0, to: 3200.0, pan: (0.3, -0.3), gain: 0.16 });
    score.cue(CLICK_NEXT + 0.1, Cue::Chime { notes: vec![79.0, 84.0, 88.0, 91.0], gain: 0.1 });
}

// ---------------------------------------------------------------- 5. pil lirik

const LINES: [(f64, usize); 3] = [(0.0, 1), (1.1, 2), (2.3, 4)];

/// Fitur 5: baris lirik di pil; lebarnya memantul mengikuti panjang baris.
fn lyric_pill(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    let (ax, ay, k) = (540.0, 640.0, 2.35);
    let keys: Vec<(f64, View, (f32, f32), f32)> =
        LINES.iter().map(|&(at, i)| (at, View::LyricPill, studio.lyric_size(LYRICS[i]), 0.0)).collect();
    let motion = simulate(&keys, t);
    let line = LINES.iter().rev().find(|(at, _)| t >= *at).map_or(LINES[0].1, |l| l.1);
    let mut scene = cast.scene(View::LyricPill, Page::Home, true);
    motion.apply(&mut scene);
    scene.lyric = Some(LYRICS[line]);
    studio.glow(ax, ay + 60.0, 600.0, 200.0, MAGENTA, 0.22 + 0.1 * beat_pulse(t));
    studio.notch(&scene, Theme::Black, ax, ay, k, ease_out(span(t, 0.0, 0.25)));
    // Penggaris lebar di bawah pil.
    let half = motion.size().0 / 2.0 * k;
    let y = ay + motion.size().1 * k + 70.0;
    let show = ease_out(span(t, 0.3, 0.3)) * (1.0 - span(t, len - 0.3, 0.3));
    for x in [ax - half, ax + half] {
        studio.line((x, y - 22.0), (x, y + 22.0), 3.0, 0xFFFFFF, 0.6 * show);
    }
    studio.line((ax - half + 8.0, y), (ax + half - 8.0, y), 3.0, 0xFFFFFF, 0.6 * show);
    for (x, dir) in [(ax - half + 8.0, 1.0f32), (ax + half - 8.0, -1.0)] {
        studio.polyline(&[(x + 16.0 * dir, y - 12.0), (x, y), (x + 16.0 * dir, y + 12.0)], 3.0, 0xFFFFFF, 0.6 * show);
    }
    caption(studio, t, len, "Lirik di pil", "Lebarnya ikut panjang baris");
    badge(studio, t, 0.6, len, "opsional · pakai internet", 1530.0, MAGENTA);
}

fn lyric_pill_sound(score: &mut Score) {
    for &(at, _) in &LINES[1..] {
        score.cue(at, Cue::Glide { len: 0.3, from: 500.0, to: 800.0, gain: 0.12 });
    }
}

// ---------------------------------------------------------------- 6. kolom lirik

const TOGGLE: f64 = 0.55;

/// Fitur 6: klik tombol lirik; kalender bergeser keluar, lirik masuk dan bergulir.
fn column(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    let (ax, ay, k) = (540.0, 580.0, 1.5);
    let lyric_mix = ease_in_out(span(t, TOGGLE + 0.05, 0.55));
    let (current, scroll) = if t < 1.7 { (2, 1.0) } else if t < 2.4 { (3, span(t, 1.7, 0.38)) } else { (4, span(t, 2.4, 0.38)) };
    let lyrics = lyrics_at(current, scroll);
    let mut scene = cast.scene(View::PanelWide, Page::Home, true);
    scene.calendar = Some(&cast.calendar);
    scene.lyrics = Some(&lyrics);
    scene.lyric_mix = lyric_mix;
    scene.lyrics_button = Some(t >= TOGGLE);
    scene.hovered = (t > 0.3 && t < TOGGLE + 0.4).then_some(Hit::LyricsToggle);
    studio.glow(ax, ay + 160.0, 700.0, 300.0, MAGENTA, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k, ease_out(span(t, 0.0, 0.25)));
    let button = at(ax, ay, k, -298.0 + 10.0, 125.0);
    let come = ease_in_out(span(t, 0.05, 0.45));
    let leave = ease_in_out(span(t, 1.2, 0.5));
    let (x, y) = (mix(mix(300.0, button.0 + 4.0, come), 200.0, leave), mix(mix(950.0, button.1 + 6.0, come), 1000.0, leave));
    studio.cursor(x, y, 1.5, span(t, 0.0, 0.15) * (1.0 - span(t, 1.5, 0.2)));
    // Arah geser: kalender keluar ke kanan, lirik masuk.
    let arrow = span(t, TOGGLE, 0.2) * (1.0 - span(t, TOGGLE + 0.7, 0.3));
    if arrow > 0.0 {
        let y = ay + 156.0 * k + 60.0;
        let x0 = 640.0 + 160.0 * ease_out(span(t, TOGGLE, 0.6));
        studio.line((x0 - 120.0, y), (x0, y), 4.0, 0xFFFFFF, 0.6 * arrow);
        studio.polyline(&[(x0 - 18.0, y - 14.0), (x0, y), (x0 - 18.0, y + 14.0)], 4.0, 0xFFFFFF, 0.6 * arrow);
        studio.label("kalender", SANS, 28.0, false, x0 - 60.0, y + 34.0, 0xBFD8FF, 0.8 * arrow, 0);
    }
    caption(studio, t, len, "Kolom lirik", "Sekali klik, lirik bergulir di samping");
}

fn column_sound(score: &mut Score) {
    score.cue(TOGGLE, Cue::Click { gain: 0.32 });
    score.cue_pan(TOGGLE + 0.05, (-0.5, 0.6), Cue::Whoosh { len: 0.55, from: 900.0, to: 2600.0, pan: (-0.5, 0.6), gain: 0.15 });
    for at in [1.7, 2.4] {
        score.cue_pan(at, (0.4, 0.4), Cue::Tick { hz: 3200.0, gain: 0.08 });
    }
}

// ---------------------------------------------------------------- 7. seret bar

const GRAB: f64 = 0.75;
const DROP: f64 = 2.0;
/// Panjang lagu contoh (3:41).
const DURATION: f32 = 221.0;

fn clock(seconds: f32) -> String {
    let s = seconds.round() as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

/// Fitur 7: bar menebal saat disentuh, diseret, waktunya ikut bergulir.
fn seek(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    let (ax, ay, k) = (540.0, 400.0, 2.15);
    let drag = ease_in_out(span(t, GRAB + 0.1, DROP - GRAB - 0.1));
    let progress = mix(0.38, 0.76, drag);
    let media = MediaView {
        progress: Some(progress),
        elapsed: clock(progress * DURATION),
        remaining: format!("-{}", clock((1.0 - progress) * DURATION)),
        seeking: (GRAB..DROP).contains(&t),
        ..song()
    };
    let mut scene = cast.scene(View::Panel, Page::Home, true);
    scene.media = Some(&media);
    scene.shadow = 1.0;
    scene.hovered = (t > GRAB - 0.15 && t < DROP + 0.4).then_some(Hit::MediaSeek);
    studio.glow(ax, ay + 160.0, 600.0, 260.0, CYAN, 0.2);
    studio.notch(&scene, Theme::Black, ax, ay, k, ease_out(span(t, 0.0, 0.25)));
    let (start, end) = (-146.0, 140.0);
    let knob = at(ax, ay, k, mix(start, end, progress), 101.0);
    let come = ease_in_out(span(t, 0.1, GRAB - 0.1));
    let leave = ease_in_out(span(t, DROP + 0.3, 0.5));
    let x = mix(mix(880.0, knob.0, come), 900.0, leave);
    let y = mix(mix(1000.0, knob.1 + 4.0, come), 1050.0, leave);
    studio.cursor(x, y, 1.6, span(t, 0.0, 0.15));
    // Waktu besar yang bergulir di bawah panel.
    let show = ease_out(span(t, GRAB, 0.25)) * (1.0 - span(t, len - 0.3, 0.3));
    let digits = clock(progress * DURATION);
    let wobble = if media.seeking { (t * 40.0).sin() as f32 * 3.0 } else { 0.0 };
    studio.text(&digits, SERIF, 170.0, false, 980.0 + wobble, 0xFFFFFF, show, 1.0);
    studio.label("dari 3:41", SANS, 36.0, false, 540.0, 1100.0, 0xBFD8FF, 0.8 * show, 0);
    caption(studio, t, len, "Seret untuk lompat", "Bar menebal saat disentuh");
}

fn seek_sound(score: &mut Score) {
    score.cue(GRAB, Cue::Tick { hz: 2200.0, gain: 0.14 });
    score.cue(GRAB + 0.1, Cue::Glide { len: DROP - GRAB - 0.1, from: 500.0, to: 1400.0, gain: 0.1 });
    score.cue(DROP, Cue::Click { gain: 0.25 });
}

// ---------------------------------------------------------------- 8. gestur dua jari

const SWIPES: [(f64, f32); 2] = [(0.55, 1.0), (1.95, -1.0)];

/// Fitur 8: dua jari menggeser touchpad; pil berganti lagu tanpa membuka panel.
fn swipe(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    let (ax, ay, k) = (540.0, 380.0, 3.4);
    // Lagu berganti di tengah tiap geseran; pil terdorong ke arah geseran lalu kembali.
    let mut kick = 0.0f32;
    let mut forward = false;
    for &(at, dir) in &SWIPES {
        let p = span(t, at + 0.2, 0.45);
        kick += dir * (p * PI).sin() * (1.0 - p) * 70.0;
        if t >= at + 0.25 {
            forward = dir > 0.0;
        }
    }
    let later = second_song();
    let mut scene = cast.scene(View::Pill, Page::Home, true);
    if forward {
        studio.set_art(&sunset_art());
        scene.media = Some(&later);
    }
    let tint = if forward { 0xFF9F0A } else { CYAN };
    studio.glow(ax, ay + 60.0, 520.0, 200.0, tint, 0.25);
    studio.notch(&scene, Theme::Black, ax + kick, ay, k, ease_out(span(t, 0.0, 0.25)));
    studio.set_art(&gradient_art());

    let (px, py, pw, ph) = (540.0, 900.0, 640.0, 360.0);
    let show = ease_out(span(t, 0.0, 0.3));
    touchpad(studio, px, py, pw, ph, show);
    for &(at, dir) in &SWIPES {
        let p = ease_in_out(span(t, at, 0.45));
        let alpha = span(t, at - 0.15, 0.15) * (1.0 - span(t, at + 0.5, 0.2));
        if alpha <= 0.0 {
            continue;
        }
        let x = px - dir * 200.0 + dir * 400.0 * p;
        let trail = dir * 160.0 * (p * PI).sin();
        finger(studio, x, py - 50.0, trail, alpha);
        finger(studio, x, py + 50.0, trail, alpha);
        let text = if dir > 0.0 { "lagu berikutnya  →" } else { "←  lagu sebelumnya" };
        studio.label(text, SANS, 34.0, true, px, py + ph / 2.0 + 60.0, 0xFFFFFF, alpha, 0);
    }
    caption_at(studio, t, len, 1330.0, "Geser dua jari", "Ganti lagu tanpa membuka panel");
}

fn swipe_sound(score: &mut Score) {
    for &(at, dir) in &SWIPES {
        let pan = (-0.6 * dir as f64, 0.6 * dir as f64);
        score.cue_pan(at, pan, Cue::Whoosh { len: 0.5, from: 500.0, to: 2500.0, pan, gain: 0.22 });
        score.cue(at + 0.25, Cue::Pop { midi: if dir > 0.0 { 79.0 } else { 74.0 }, gain: 0.2 });
    }
}

// ---------------------------------------------------------------- 9. lirik YouTube

const STRIKES: [f64; 3] = [0.8, 1.0, 1.2];
const FLY: f64 = 1.55;
const SETTLE: f64 = 2.1;
const SING: f64 = 2.85;

/// Fitur 9: judul video yang berantakan dirapikan jadi judul + artis, lalu liriknya muncul.
fn youtube(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, MAGENTA);
    // Kartu video di browser.
    let appear = ease_out(span(t, 0.0, 0.35));
    let card = rect(90.0, 560.0 + (1.0 - appear) * 40.0, 990.0, 900.0 + (1.0 - appear) * 40.0);
    window(studio, card, "Browser", appear);
    let thumb = rect(card.left + 32.0, card.top + 96.0, card.left + 272.0, card.top + 231.0);
    studio.rounded(thumb, 14.0, 0x2A3346, appear);
    studio.circle((thumb.left + thumb.right) / 2.0, (thumb.top + thumb.bottom) / 2.0, 30.0, 0xFFFFFF, 0.9 * appear);
    studio.icon('\u{F5B0}', 26.0, (thumb.left + thumb.right) / 2.0 + 3.0, (thumb.top + thumb.bottom) / 2.0, 0x151B29, appear);
    // Potongan judul: (teks, sampah?). Baris 1 judul + artis, baris 2 tambahan, baris 3 channel.
    let rows: [&[(&str, bool)]; 3] = [
        &[("Ruang Senja", false), (" - ", true), ("Langit Sore", false)],
        &[("(Official Music Video)", true), (" ", true), ("[MV]", true)],
        &[("Ruang Senja", false), (" - Topic", true)],
    ];
    let left = thumb.right + 28.0;
    let mut junk = 0;
    for (r, row) in rows.iter().enumerate() {
        let (size, color) = if r == 2 { (30.0, 0x8F9BB3) } else { (40.0, 0xF2F4F8) };
        let y = card.top + 120.0 + r as f32 * 56.0;
        let mut x = left;
        for &(text, trash) in row.iter() {
            let w = studio.measure(text, SANS, size, r < 2);
            if trash && !text.trim().is_empty() {
                let strike = ease_out(span(t, STRIKES[junk.min(2)], 0.2));
                let fly = ease_in_out(span(t, FLY + junk as f64 * 0.06, 0.5));
                let dx = 60.0 * fly;
                let dy = 260.0 * fly * fly;
                let alpha = appear * (1.0 - fly);
                studio.world(camera(1.0, x + w / 2.0, y, dx, dy));
                studio.label(text, SANS, size, r < 2, x, y, 0xFF6B6B, alpha, -1);
                studio.line((x - 4.0, y + 2.0), (x - 4.0 + (w + 8.0) * strike, y + 2.0), 4.0, 0xFF453A, alpha);
                studio.reset();
                junk += 1;
            } else if !trash {
                // Judul dan artis meluncur ke kolomnya masing-masing di bawah kartu.
                let settle = ease_in_out(span(t, SETTLE, 0.5));
                let target = if r == 0 && text == "Langit Sore" { (300.0, 1040.0) } else { (300.0, 980.0) };
                let ghost = r == 2;
                let alpha = if ghost { appear * (1.0 - span(t, FLY, 0.3)) } else { appear };
                let (tx, ty) = if ghost { (x, y) } else { (mix(x, target.0, settle), mix(y, target.1, settle)) };
                studio.label(text, SANS, size, r < 2, tx, ty, color, alpha, -1);
            }
            x += w;
        }
    }
    let fields = ease_out(span(t, SETTLE + 0.2, 0.3));
    for (y, name) in [(980.0, "Artis"), (1040.0, "Judul")] {
        studio.label(name, SANS, 30.0, false, 270.0, y, 0x8F9BB3, fields, 1);
    }
    studio.icon('\u{E73E}', 34.0, 800.0, 1010.0, 0x30D158, fields);
    // Lirik muncul di pil di atas kartu.
    let sing = pop_in(t, SING, 0.45);
    if sing > 0.0 {
        let size = studio.lyric_size(LYRICS[0]);
        let keys = [(SING, View::Pill, View::Pill.size(), 0.0f32), (SING + 0.01, View::LyricPill, size, 0.0)];
        let motion = simulate(&keys, t);
        let mut scene = cast.scene(View::LyricPill, Page::Home, true);
        motion.apply(&mut scene);
        scene.lyric = Some(LYRICS[0]);
        studio.glow(540.0, 330.0, 520.0, 160.0, MAGENTA, 0.3 * sing.min(1.0));
        studio.notch(&scene, Theme::Black, 540.0, 300.0, 2.2, sing.min(1.0));
    }
    caption_at(studio, t, len, 1320.0, "Lirik YouTube juga", "Judul video dirapikan otomatis");
    badge(studio, t, SING, len, "opsional · pakai internet", 1540.0, MAGENTA);
}

fn youtube_sound(score: &mut Score) {
    for (i, &at) in STRIKES.iter().enumerate() {
        score.cue(at, Cue::Tick { hz: 1800.0 + i as f64 * 300.0, gain: 0.14 });
    }
    score.cue_pan(FLY, (0.0, 0.5), Cue::Whoosh { len: 0.6, from: 2500.0, to: 400.0, pan: (0.0, 0.5), gain: 0.2 });
    score.cue(SETTLE + 0.2, Cue::Pop { midi: 72.0, gain: 0.2 });
    score.cue(SETTLE + 0.35, Cue::Pop { midi: 76.0, gain: 0.2 });
    score.cue(SING, Cue::Chime { notes: vec![81.0, 84.0, 88.0], gain: 0.12 });
}
