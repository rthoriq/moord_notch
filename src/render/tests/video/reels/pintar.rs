//! Bab 8, pintar menyesuaikan: garis tipis saat maximize, sembunyi saat full-screen, klik
//! tembus, pintasan Win+Alt+N, tiga ukuran, dan atur halaman.

use super::peraga::{desktop, filler, keycap, press, window};
use super::*;

const VIOLET: u32 = 0xBF5AF2;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Pintar menyesuaikan",
        tint: VIOLET,
        shots: vec![
            Shot { beats: 6, energy: Energy::Full, draw: slim, sound: slim_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: fullscreen, sound: fullscreen_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: click_through, sound: click_through_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: hotkey, sound: hotkey_sound, blur: &[] },
            Shot { beats: 6, energy: Energy::Full, draw: sizes, sound: sizes_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: pages, sound: pages_sound, blur: &[] },
        ],
    }
}

/// Layar laptop tiruan tempat notch menempel di tepi atasnya.
const SCREEN: D2D_RECT_F = D2D_RECT_F { left: 90.0, top: 330.0, right: 990.0, bottom: 900.0 };
const K: f32 = 1.7;

fn lerp_rect(a: D2D_RECT_F, b: D2D_RECT_F, p: f32) -> D2D_RECT_F {
    rect(mix(a.left, b.left, p), mix(a.top, b.top, p), mix(a.right, b.right, p), mix(a.bottom, b.bottom, p))
}

// ---------------------------------------------------------------- 35. garis tipis saat maximize

const MAXIMIZE: f64 = 0.6;
const HOVER: f64 = 2.0;

fn slim(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, VIOLET);
    desktop(studio, SCREEN, VIOLET, 1.0);
    let grow = ease_in_out(span(t, MAXIMIZE, 0.35));
    let small = rect(200.0, 470.0, 700.0, 800.0);
    let full = rect(SCREEN.left + 4.0, SCREEN.top + 4.0, SCREEN.right - 4.0, SCREEN.bottom - 62.0);
    let win = lerp_rect(small, full, grow);
    window(studio, win, "Browser", 1.0);
    filler(studio, win, 6, 1.0);
    let usage = Usage { microphone: vec!["Rapat".into()], ..Usage::default() };
    let peek = (View::Pill.size().0 + 12.0, View::Pill.size().1 + 4.0);
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (MAXIMIZE + 0.2, View::Slim, View::Slim.size(), 0.0),
        (HOVER, View::Pill, peek, 0.3),
        (len - 0.2, View::Pill, peek, 0.3),
    ];
    let motion = simulate(&keys, t);
    let mut data = sample();
    data.media = song();
    let mut scene = home(&data, &cast.idle, &usage, View::Pill, Page::Home, false);
    motion.apply(&mut scene);
    studio.notch(&scene, Theme::Black, 540.0, SCREEN.top, K, 1.0);
    // Kaca pembesar ke garis tipisnya, supaya titik privasinya terlihat.
    let zoom = span(t, MAXIMIZE + 0.4, 0.3) * (1.0 - span(t, HOVER - 0.3, 0.3));
    if zoom > 0.0 {
        let (lx, ly, r) = (540.0, 1010.0, 170.0);
        studio.line((540.0, SCREEN.top + 20.0), (lx, ly - r), 3.0, 0xFFFFFF, 0.5 * zoom);
        studio.circle(lx, ly, r, 0x0B1226, zoom);
        studio.world(camera(1.0, 0.0, 0.0, lx - 540.0, ly - 20.0));
        studio.notch(&scene, Theme::Black, 540.0, 0.0, 2.4, zoom);
        studio.reset();
        studio.ring(lx, ly, r, 6.0, 0xFFFFFF, 0.8 * zoom);
        studio.label("titik mikrofon tetap ada", SANS, 30.0, false, lx, ly + r + 34.0, 0xFF9F0A, zoom, 0);
    }
    let come = ease_in_out(span(t, HOVER - 0.4, 0.4));
    studio.cursor(mix(800.0, 560.0, come), mix(700.0, SCREEN.top + 18.0, come), 1.3, span(t, HOVER - 0.5, 0.2));
    caption_at(studio, t, len, 1330.0, "Garis tipis", "Saat jendela di-maximize, ruang tab tetap lega");
}

fn slim_sound(score: &mut Score) {
    score.cue(MAXIMIZE, Cue::Whoosh { len: 0.4, from: 400.0, to: 2400.0, pan: (0.0, 0.0), gain: 0.16 });
    score.cue(MAXIMIZE + 0.2, Cue::Glide { len: 0.25, from: 800.0, to: 300.0, gain: 0.14 });
    score.cue(HOVER, Cue::Glide { len: 0.2, from: 350.0, to: 700.0, gain: 0.12 });
}

// ---------------------------------------------------------------- 36. sembunyi saat full-screen

const FULL: f64 = 0.7;

fn fullscreen(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, VIOLET);
    desktop(studio, SCREEN, VIOLET, 1.0);
    let grow = ease_in_out(span(t, FULL, 0.4));
    let small = rect(250.0, 470.0, 830.0, 800.0);
    let player = lerp_rect(small, SCREEN, grow);
    studio.rounded(player, mix(22.0, 26.0, grow), 0x05070C, 1.0);
    studio.clip(player);
    let (cx, cy) = ((player.left + player.right) / 2.0, (player.top + player.bottom) / 2.0);
    studio.glow(cx - 120.0, cy + 40.0, 420.0 * mix(0.6, 1.0, grow), 260.0, MAGENTA, 0.6);
    studio.glow(cx + 160.0, cy - 60.0, 360.0 * mix(0.6, 1.0, grow), 220.0, CYAN, 0.5);
    let bar = player.bottom - 30.0;
    studio.rounded(rect(player.left + 30.0, bar - 3.0, player.right - 30.0, bar + 3.0), 3.0, 0xFFFFFF, 0.3);
    studio.rounded(rect(player.left + 30.0, bar - 3.0, mix(player.left + 30.0, player.right - 30.0, 0.35 + 0.02 * t as f32), bar + 3.0), 3.0, 0xFF453A, 0.9);
    // Notch meluncur naik keluar layar selama full-screen, lalu kembali saat keluar.
    let away = ease_in_out(span(t, FULL + 0.2, 0.35)) * (1.0 - ease_in_out(span(t, len - 0.7, 0.35)));
    let scene = cast.scene(View::Pill, Page::Home, false);
    studio.notch(&scene, Theme::Black, 540.0, SCREEN.top - 70.0 * away, K, 1.0);
    studio.unclip();
    if grow < 1.0 {
        let scene = cast.scene(View::Pill, Page::Home, false);
        studio.clip(SCREEN);
        studio.notch(&scene, Theme::Black, 540.0, SCREEN.top - 70.0 * away, K, 1.0 - grow);
        studio.unclip();
    }
    let arrow = span(t, FULL + 0.2, 0.2) * (1.0 - span(t, len - 0.9, 0.2));
    if arrow > 0.0 {
        studio.line((900.0, 300.0), (900.0, 200.0), 5.0, 0xFFFFFF, 0.7 * arrow);
        studio.polyline(&[(882.0, 220.0), (900.0, 200.0), (918.0, 220.0)], 5.0, 0xFFFFFF, 0.7 * arrow);
    }
    let keys = ease_out(span(t, 0.0, 0.2)) * (1.0 - span(t, FULL + 0.3, 0.3));
    keycap(studio, 540.0, 1050.0, 160.0, 130.0, "F", press(t, &[FULL]), VIOLET, keys);
    caption_at(studio, t, len, 1330.0, "Sembunyi saat full-screen", "Game, video, dan presentasi tanpa gangguan");
}

fn fullscreen_sound(score: &mut Score) {
    score.cue(FULL, Cue::Click { gain: 0.3 });
    score.cue(FULL + 0.05, Cue::Whoosh { len: 0.5, from: 300.0, to: 3000.0, pan: (0.0, 0.0), gain: 0.18 });
}

// ---------------------------------------------------------------- 37. klik tembus

const CLICK: f64 = 1.1;

fn click_through(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, VIOLET);
    desktop(studio, SCREEN, VIOLET, 1.0);
    // Jendela yang tab-nya berada tepat di samping notch.
    let win = rect(SCREEN.left + 4.0, SCREEN.top + 4.0, SCREEN.right - 4.0, SCREEN.bottom - 62.0);
    window(studio, win, "", 1.0);
    let clicked = t >= CLICK;
    for (i, name) in ["Beranda", "Dokumen"].into_iter().enumerate() {
        let tab = rect(win.left + 20.0 + i as f32 * 190.0, win.top + 12.0, win.left + 200.0 + i as f32 * 190.0, win.top + 56.0);
        let active = if clicked { i == 1 } else { i == 0 };
        studio.rounded(tab, 12.0, if active { 0x2A3346 } else { 0x151B29 }, 1.0);
        studio.label(name, SANS, 24.0, active, (tab.left + tab.right) / 2.0, (tab.top + tab.bottom) / 2.0, 0xF2F4F8, if active { 1.0 } else { 0.6 }, 0);
    }
    filler(studio, win, 6, 1.0);
    let scene = cast.scene(View::Pill, Page::Home, false);
    // Sinar-X: saat diklik notch tampil sebagai garis luar saja, menunjukkan klik menembus.
    let xray = span(t, CLICK, 0.1) * (1.0 - span(t, CLICK + 0.6, 0.3));
    studio.notch(&scene, Theme::Black, 540.0, SCREEN.top, K, 1.0 - 0.75 * xray);
    let (w, h) = (View::Pill.size().0 * K, View::Pill.size().1 * K);
    let shape = rect(540.0 - w / 2.0, SCREEN.top, 540.0 + w / 2.0, SCREEN.top + h);
    studio.outline(shape, h / 2.0, 3.0, CYAN, xray);
    let target = (win.left + 290.0, win.top + 34.0);
    let come = ease_in_out(span(t, 0.2, CLICK - 0.2));
    studio.cursor(mix(820.0, target.0, come), mix(760.0, target.1, come), 1.3, span(t, 0.1, 0.2));
    let ring = span(t, CLICK, 0.5);
    if ring > 0.0 && ring < 1.0 {
        studio.ring(target.0, target.1, 20.0 + 80.0 * ring, 4.0, CYAN, 1.0 - ring);
    }
    caption_at(studio, t, len, 1330.0, "Klik tembus", "Di luar bentuk notch, klik sampai ke jendela");
}

fn click_through_sound(score: &mut Score) {
    score.cue(CLICK, Cue::Click { gain: 0.32 });
    score.cue(CLICK + 0.02, Cue::Glide { len: 0.25, from: 1200.0, to: 600.0, gain: 0.1 });
}

// ---------------------------------------------------------------- 38. Win + Alt + N

const PRESS: f64 = 0.35;
const AUTO_CLOSE: f64 = 2.45;

fn hotkey(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, VIOLET);
    let keys = [
        (0.0, View::Pill, View::Pill.size(), 0.0f32),
        (PRESS + 0.05, View::Panel, View::Panel.size(), 1.0),
        (AUTO_CLOSE, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let mut scene = cast.scene(View::Panel, Page::Home, false);
    motion.apply(&mut scene);
    let (ax, ay, k) = (540.0, 420.0, 2.1);
    studio.glow(ax, ay + 140.0, 600.0, 240.0, VIOLET, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    let show = ease_out(span(t, 0.0, 0.2)) * (1.0 - span(t, PRESS + 0.6, 0.3));
    for (x, label) in [(320.0, "Win"), (540.0, "Alt"), (760.0, "N")] {
        keycap(studio, x, 1000.0, 180.0, 130.0, label, press(t, &[PRESS]), VIOLET, show);
    }
    // Hitung mundur lima detik (dipercepat) sebelum panel menutup sendiri.
    let countdown = span(t, PRESS + 0.5, AUTO_CLOSE - PRESS - 0.5);
    let ring_show = span(t, PRESS + 0.6, 0.2) * (1.0 - span(t, AUTO_CLOSE, 0.25));
    if ring_show > 0.0 {
        let (cx, cy) = (540.0, 1010.0);
        studio.ring(cx, cy, 110.0, 12.0, 0xFFFFFF, 0.15 * ring_show);
        studio.arc(cx, cy, 110.0, 12.0, countdown, 1.0 - countdown, VIOLET, ring_show);
        let seconds = (5.0 * (1.0 - countdown)).ceil() as u32;
        studio.label(&seconds.to_string(), SERIF, 110.0, false, cx, cy - 4.0, 0xFFFFFF, ring_show, 0);
        studio.label("menutup sendiri kalau tidak disentuh", SANS, 30.0, false, cx, cy + 160.0, 0xBFD8FF, ring_show, 0);
    }
    caption_at(studio, t, len, 1330.0, "Win + Alt + N", "Buka panel tanpa menyentuh mouse");
}

fn hotkey_sound(score: &mut Score) {
    score.cue(PRESS, Cue::Click { gain: 0.35 });
    score.cue(PRESS + 0.05, Cue::Glide { len: 0.3, from: 300.0, to: 900.0, gain: 0.14 });
    for i in 0..5 {
        score.cue(PRESS + 0.5 + i as f64 * (AUTO_CLOSE - PRESS - 0.5) / 5.0, Cue::Tick { hz: 2400.0, gain: 0.08 });
    }
    score.cue(AUTO_CLOSE, Cue::Glide { len: 0.25, from: 900.0, to: 350.0, gain: 0.12 });
}

// ---------------------------------------------------------------- 39. tiga ukuran

const SIZES: [(f64, f32, &str); 3] = [(0.2, 0.88, "Kecil"), (1.1, 1.0, "Sedang"), (1.9, 1.15, "Besar")];

fn sizes(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, VIOLET);
    let i = SIZES.iter().rposition(|s| t >= s.0).unwrap_or(0);
    let before = if i == 0 { SIZES[0].1 } else { SIZES[i - 1].1 };
    let scale = mix(before, SIZES[i].1, back_out(span(t, SIZES[i].0, 0.4)).min(1.1));
    let base = 3.6;
    let k = base * scale;
    let scene = cast.scene(View::Pill, Page::Home, false);
    let (ax, ay) = (540.0, 560.0);
    studio.glow(ax, ay + 60.0, 560.0, 200.0, VIOLET, 0.25);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Penggaris di bawah pil.
    let half = View::Pill.size().0 / 2.0 * k;
    let y = ay + View::Pill.size().1 * k + 70.0;
    studio.line((ax - half, y), (ax + half, y), 3.0, 0xFFFFFF, 0.6);
    for x in [ax - half, ax + half] {
        studio.line((x, y - 20.0), (x, y + 20.0), 3.0, 0xFFFFFF, 0.6);
    }
    let mut tick = ax - half;
    while tick < ax + half {
        studio.line((tick, y), (tick, y - 10.0), 2.0, 0xFFFFFF, 0.3);
        tick += 24.0 * scale;
    }
    // Tiga pilihan, yang aktif menyala.
    for (j, (_, _, name)) in SIZES.iter().enumerate() {
        let x = 300.0 + j as f32 * 240.0;
        let area = rect(x - 100.0, 960.0, x + 100.0, 1030.0);
        let on = j == i;
        studio.rounded(area, 35.0, if on { VIOLET } else { 0x1C2333 }, 1.0);
        studio.label(name, SANS, 34.0, on, x, 995.0, 0xFFFFFF, if on { 1.0 } else { 0.6 }, 0);
    }
    caption_at(studio, t, len, 1330.0, "Tiga ukuran", "Kecil, sedang, atau besar");
}

fn sizes_sound(score: &mut Score) {
    for (i, &(at, _, _)) in SIZES.iter().enumerate() {
        score.cue(at, Cue::Pop { midi: 67.0 + i as f64 * 5.0, gain: 0.2 });
    }
}

// ---------------------------------------------------------------- 40. atur halaman

const MENU: f64 = 0.55;
const MOVE: f64 = 1.35;
const HIDE: f64 = 2.3;

fn pages(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, VIOLET);
    let mut pages = Pages::default();
    if t >= MOVE + 0.3 {
        pages.move_by(Page::Water, -2);
    }
    if t >= HIDE + 0.3 {
        pages.set_shown(Page::System, false);
    }
    let mut scene = cast.scene(View::Panel, Page::Water, false);
    scene.pages = pages;
    let (ax, ay, k) = (540.0, 360.0, 2.0);
    studio.glow(ax, ay + 140.0, 600.0, 240.0, VIOLET, 0.22);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    // Menu klik kanan.
    let menu = ease_out(span(t, MENU, 0.2)) * (1.0 - span(t, HIDE + 0.4, 0.2));
    if menu > 0.0 {
        let area = rect(560.0, 600.0, 960.0, 600.0 + 250.0 * menu);
        studio.clip(area);
        studio.rounded(area, 18.0, 0x222B3D, 1.0);
        studio.outline(area, 18.0, 2.0, 0xFFFFFF, 0.12);
        let items = ["Geser ke kiri", "Geser ke kanan", "Sembunyikan halaman", "Tampilkan halaman…"];
        let active = if t < HIDE { 0 } else { 2 };
        for (i, item) in items.into_iter().enumerate() {
            let y = 640.0 + i as f32 * 56.0;
            if i == active && t > MENU + 0.4 {
                studio.rounded(rect(area.left + 10.0, y - 24.0, area.right - 10.0, y + 24.0), 12.0, VIOLET, 0.5);
            }
            studio.label(item, SANS, 28.0, false, area.left + 30.0, y, 0xF2F4F8, 1.0, -1);
        }
        studio.unclip();
    }
    // Deretan halaman: Minum air bergeser ke kiri, Sistem disembunyikan.
    let order: Vec<Page> = Page::ALL.into_iter().collect();
    let glyphs = ['\u{E80F}', '\u{E713}', '\u{E8B7}', '\u{E916}', '\u{E121}', ' ', '\u{E9D5}', '\u{E9D9}'];
    let shift = ease_in_out(span(t, MOVE + 0.3, 0.4));
    let hide = ease_in_out(span(t, HIDE + 0.3, 0.4));
    for (i, page) in order.iter().enumerate() {
        let mut slot = i as f32;
        if *page == Page::Water {
            slot -= 2.0 * shift;
        } else if (3..5).contains(&i) {
            slot += shift;
        }
        let gone = if *page == Page::System { hide } else { 0.0 };
        let x = 540.0 + (slot - 3.5) * 118.0;
        let y = 1010.0 + 40.0 * gone;
        let lifted = *page == Page::Water && shift > 0.0 && shift < 1.0;
        let tile = rect(x - 50.0, y - 50.0 - if lifted { 30.0 } else { 0.0 }, x + 50.0, y + 50.0 - if lifted { 30.0 } else { 0.0 });
        let alpha = ease_out(span(t, 0.0, 0.3)) * (1.0 - gone);
        let focus = *page == Page::Water || *page == Page::System && t >= HIDE;
        studio.rounded(tile, 20.0, if focus { 0x2E2346 } else { 0x1C2333 }, alpha);
        if focus {
            studio.outline(tile, 20.0, 3.0, VIOLET, alpha);
        }
        let cy = (tile.top + tile.bottom) / 2.0;
        if *page == Page::Water {
            // Tetes air, seperti ikon di halaman Minum air.
            studio.circle(x, cy + 8.0, 16.0, 0x0A84FF, alpha);
            studio.polygon(&[(x - 14.0, cy + 2.0), (x, cy - 24.0), (x + 14.0, cy + 2.0)], 0x0A84FF, alpha);
        } else {
            studio.icon(glyphs[i], 40.0, x, cy, 0xF2F4F8, alpha);
        }
    }
    // Kursor: klik kanan di panel, pilih "Geser ke kiri", lalu "Sembunyikan halaman".
    let path = [
        (0.1, (820.0, 1150.0)),
        (MENU, (600.0, 560.0)),
        (MOVE, (640.0, 640.0)),
        (HIDE, (640.0, 752.0)),
        (HIDE + 0.6, (900.0, 1150.0)),
    ];
    let i = path.iter().rposition(|p| p.0 <= t).unwrap_or(0).min(path.len() - 2);
    let ((t0, a), (t1, b)) = (path[i], path[i + 1]);
    let p = ease_in_out(span(t, t0, (t1 - t0).min(0.45)));
    studio.cursor(mix(a.0, b.0, p), mix(a.1, b.1, p), 1.4, span(t, 0.0, 0.2) * (1.0 - span(t, HIDE + 0.4, 0.2)));
    caption_at(studio, t, len, 1330.0, "Atur halaman", "Geser atau sembunyikan lewat klik kanan");
}

fn pages_sound(score: &mut Score) {
    score.cue(MENU, Cue::Click { gain: 0.3 });
    score.cue(MENU + 0.02, Cue::Pop { midi: 72.0, gain: 0.14 });
    score.cue(MOVE + 0.3, Cue::Click { gain: 0.26 });
    score.cue_pan(MOVE + 0.3, (0.3, -0.3), Cue::Whoosh { len: 0.4, from: 900.0, to: 2400.0, pan: (0.3, -0.3), gain: 0.12 });
    score.cue(HIDE + 0.3, Cue::Click { gain: 0.26 });
    score.cue(HIDE + 0.32, Cue::Glide { len: 0.3, from: 800.0, to: 300.0, gain: 0.12 });
}
