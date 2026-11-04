//! Bab 3, shelf dan bagikan: seret-lepas file, jendela Bagikan Windows, buka atau seret keluar
//! ke chat, dan screenshot yang langsung masuk shelf.

use super::peraga::{desktop, filler, keycap, press, screenshot_thumb, window};
use super::*;

pub(super) fn chapter() -> Chapter {
    Chapter {
        title: "Shelf & Bagikan",
        tint: CYAN,
        shots: vec![
            Shot { beats: 7, energy: Energy::Full, draw: drag, sound: drag_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: share, sound: share_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: open_out, sound: open_out_sound, blur: &[] },
            Shot { beats: 7, energy: Energy::Full, draw: screenshot, sound: screenshot_sound, blur: &[(1.3, 1.95, 6)] },
        ],
    }
}

/// Ubin ke-`slot` di halaman Shelf (koordinat DIP pusatnya).
fn tile(slot: usize) -> (f32, f32) {
    (-154.0 + 76.0 * slot as f32, 78.0)
}

fn shelf_fx(targeted: bool, share: bool, t: f64, glow: Option<f32>) -> ShelfFx {
    ShelfFx { targeted, share_targeted: share, dash: (t * 5.0 % 5.0) as f32, glow, tiles: HashMap::new(), slots: HashMap::new(), flow: None, emptied: None, undo: false, lean: 0.0 }
}

// ---------------------------------------------------------------- 14. seret dan lepas

/// Adegan seret-lepas dari teaser: file pertama ke shelf, file kedua ke zona Bagikan.
fn drag(studio: &mut Studio, cast: &Cast, t: f64, _len: f64) {
    scene_drop(studio, cast, t + 11.5);
}

fn drag_sound(score: &mut Score) {
    score.cue_pan(0.4, (0.7, -0.3), Cue::Whoosh { len: 0.95, from: 300.0, to: 1800.0, pan: (0.7, -0.3), gain: 0.24 });
    score.cue_pan(1.4, (-0.2, -0.2), Cue::Thud { gain: 0.5 });
    score.cue(1.4, Cue::Chime { notes: vec![84.0, 88.0, 91.0], gain: 0.12 });
    score.cue_pan(1.95, (-0.7, 0.6), Cue::Whoosh { len: 0.5, from: 300.0, to: 2000.0, pan: (-0.7, 0.6), gain: 0.24 });
    score.cue_pan(2.5, (0.3, 0.3), Cue::Thud { gain: 0.5 });
    score.cue_pan(2.5, (0.3, 0.3), Cue::Pop { midi: 79.0, gain: 0.2 });
}

// ---------------------------------------------------------------- 15. jendela Bagikan

const SHEET: f64 = 0.45;
const PICK: f64 = 1.5;
const SENT: f64 = 2.9;

fn share(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, CYAN);
    let mut data = sample();
    data.media = song();
    data.shelf.drop_hover = t < SHEET;
    let fx = shelf_fx(false, t < SHEET, t, None);
    let mut scene = home(&data, &cast.idle, &cast.none, View::Panel, Page::Shelf, false);
    scene.shelf_fx = &fx;
    scene.seconds = t;
    let (ax, ay, k) = (540.0, 300.0, 2.0);
    studio.glow(ax, ay + 160.0, 600.0, 240.0, CYAN, 0.2);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0 - 0.5 * span(t, SHEET, 0.3));
    // Jendela Bagikan naik dari bawah.
    let rise = ease_out(span(t, SHEET, 0.4));
    if rise > 0.0 {
        let dy = (1.0 - rise) * 300.0;
        let sheet = rect(110.0, 700.0 + dy, 970.0, 1180.0 + dy);
        window(studio, sheet, "Bagikan", rise);
        let chip = rect(sheet.left + 32.0, sheet.top + 92.0, sheet.left + 380.0, sheet.top + 152.0);
        studio.rounded(chip, 14.0, 0x222B3D, rise);
        studio.icon('\u{E8A5}', 28.0, chip.left + 36.0, (chip.top + chip.bottom) / 2.0, 0xF2F4F8, rise);
        studio.label("Laporan Q3.pdf", SANS, 28.0, true, chip.left + 66.0, (chip.top + chip.bottom) / 2.0, 0xF2F4F8, rise, -1);
        let targets = [('\u{E72D}', "Sekitar"), ('\u{E8EA}', "Ponsel"), ('\u{E715}', "Email"), ('\u{E8BD}', "WhatsApp")];
        for (i, (glyph, name)) in targets.into_iter().enumerate() {
            let x = sheet.left + 120.0 + i as f32 * 205.0;
            let y = sheet.top + 250.0;
            let pop = pop_in(t, SHEET + 0.25 + i as f64 * 0.07, 0.35).min(1.2);
            let chosen = i == 0 && t >= PICK;
            studio.circle(x, y, 62.0 * pop, if chosen { CYAN } else { 0x2A3346 }, rise);
            studio.icon(glyph, 40.0 * pop, x, y, if chosen { 0x05070C } else { 0xF2F4F8 }, rise);
            studio.label(name, SANS, 28.0, false, x, y + 96.0, 0xC9D1E0, rise * pop.min(1.0), 0);
            if chosen {
                let ripple = span(t, PICK, 0.5);
                studio.ring(x, y, 62.0 + 70.0 * ripple, 4.0, CYAN, 0.8 * (1.0 - ripple));
            }
        }
        // Perangkat di sekitar menerima file.
        let device = ease_out(span(t, PICK + 0.25, 0.3));
        if device > 0.0 {
            let row = rect(sheet.left + 32.0, sheet.bottom - 20.0 - 90.0 * device, sheet.right - 32.0, sheet.bottom - 20.0);
            studio.clip(row);
            studio.rounded(row, 16.0, 0x222B3D, 1.0);
            let ring = (row.left + 60.0, (row.top + row.bottom) / 2.0);
            studio.icon('\u{E7F8}', 30.0, ring.0, ring.1, 0xF2F4F8, 1.0);
            studio.label("Laptop Rina", SANS, 30.0, true, row.left + 110.0, ring.1, 0xF2F4F8, 1.0, -1);
            let progress = ease_in_out(span(t, PICK + 0.4, SENT - PICK - 0.4));
            let end = (row.right - 50.0, ring.1);
            studio.ring(end.0, end.1, 24.0, 5.0, 0x3A4255, 1.0);
            studio.arc(end.0, end.1, 24.0, 5.0, 0.0, progress, CYAN, 1.0);
            if t >= SENT {
                let done = pop_in(t, SENT, 0.3);
                studio.circle(end.0, end.1, 26.0 * done, 0x30D158, 1.0);
                studio.icon('\u{E73E}', 26.0 * done, end.0, end.1, 0x05070C, 1.0);
            }
            studio.unclip();
        }
        let come = ease_in_out(span(t, SHEET + 0.4, PICK - SHEET - 0.4));
        let x = mix(900.0, sheet.left + 128.0, come);
        let y = mix(1180.0, sheet.top + 262.0, come);
        studio.cursor(x, y, 1.5, rise * (1.0 - span(t, SENT, 0.3)));
    }
    caption_at(studio, t, len, 1330.0, "Bagikan", "Padanan AirDrop, lewat jendela Windows");
}

fn share_sound(score: &mut Score) {
    score.cue(SHEET, Cue::Whoosh { len: 0.45, from: 300.0, to: 1800.0, pan: (0.0, 0.0), gain: 0.18 });
    for i in 0..4 {
        score.cue(SHEET + 0.25 + i as f64 * 0.07, Cue::Pop { midi: 72.0 + i as f64 * 3.0, gain: 0.12 });
    }
    score.cue(PICK, Cue::Click { gain: 0.3 });
    score.cue(PICK + 0.4, Cue::Glide { len: SENT - PICK - 0.4, from: 400.0, to: 1200.0, gain: 0.08 });
    score.cue(SENT, Cue::Ding { gain: 0.22 });
}

// ---------------------------------------------------------------- 14b. buka dan seret keluar

const DOUBLE: [f64; 2] = [0.6, 0.75];
const GRAB: f64 = 1.75;
const LET_GO: f64 = 2.65;

fn open_out(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, CYAN);
    let mut data = sample();
    data.media = song();
    let fx = shelf_fx(false, false, t, None);
    let mut scene = home(&data, &cast.idle, &cast.none, View::Panel, Page::Shelf, false);
    scene.shelf_fx = &fx;
    let (ax, ay, k) = (540.0, 300.0, 2.0);
    let first = tile(1);
    let second = tile(2);
    let px = |p: (f32, f32)| (ax + p.0 * k, ay + p.1 * k);
    let (a, b) = (px(first), px(second));
    scene.hovered = if t < 1.2 {
        (t > 0.3).then_some(Hit::ShelfItem(2))
    } else {
        (t > 1.5 && t < GRAB + 0.1).then_some(Hit::ShelfItem(3))
    };
    studio.glow(ax, ay + 160.0, 600.0, 240.0, CYAN, 0.2);
    studio.notch(&scene, Theme::Black, ax, ay, k, 1.0);
    for &at in &DOUBLE {
        let r = span(t, at, 0.4);
        if r > 0.0 && r < 1.0 {
            studio.ring(a.0, a.1, 40.0 + 90.0 * r, 4.0, 0xFFFFFF, 0.7 * (1.0 - r));
        }
    }
    // Foto terbuka sebentar di penampil.
    let viewer = ease_out(span(t, 0.9, 0.3)) * (1.0 - span(t, 1.5, 0.25));
    if viewer > 0.0 {
        let area = rect(160.0, 700.0 + (1.0 - viewer) * 60.0, 920.0, 1160.0 + (1.0 - viewer) * 60.0);
        window(studio, area, "Foto · Corporate and Campaign.png", viewer);
        let image = rect(area.left + 30.0, area.top + 90.0, area.right - 30.0, area.bottom - 30.0);
        studio.clip(image);
        studio.rect(image, 0x2B1E5C, viewer);
        studio.glow(image.left + 200.0, image.bottom - 60.0, 420.0, 260.0, MAGENTA, 0.7 * viewer);
        studio.glow(image.right - 160.0, image.top + 60.0, 360.0, 220.0, CYAN, 0.6 * viewer);
        studio.unclip();
    }
    // Jendela chat tempat file diseret.
    let chat = ease_out(span(t, 1.55, 0.3));
    let area = rect(120.0, 720.0, 960.0, 1180.0);
    if chat > 0.0 {
        window(studio, area, "Chat · Rina", chat);
        let bubble = rect(area.left + 32.0, area.top + 100.0, area.left + 470.0, area.top + 170.0);
        studio.rounded(bubble, 22.0, 0x2A3346, chat);
        studio.label("Kirim fotonya ya", SANS, 30.0, false, bubble.left + 26.0, (bubble.top + bubble.bottom) / 2.0, 0xF2F4F8, chat, -1);
        let sent = pop_in(t, LET_GO, 0.35);
        if sent > 0.0 {
            let card = rect(area.right - 470.0, area.top + 200.0, area.right - 32.0, area.top + 330.0);
            let s = sent.min(1.0);
            studio.rounded(card, 22.0, 0x0A84FF, s);
            studio.rounded(rect(card.left + 18.0, card.top + 18.0, card.left + 112.0, card.bottom - 18.0), 12.0, 0x2B1E5C, s);
            studio.glow(card.left + 65.0, card.top + 65.0, 60.0, 50.0, MAGENTA, 0.6 * s);
            studio.label("Infuse Navy Tote", SANS, 28.0, true, card.left + 132.0, card.top + 50.0, 0xFFFFFF, s, -1);
            studio.label("Bag.png · 1,2 MB", SANS, 26.0, false, card.left + 132.0, card.top + 88.0, 0xDCE9FF, s, -1);
        }
    }
    // Kursor: klik dua kali ubin pertama, lalu seret ubin kedua ke chat.
    let drag = ease_in_out(span(t, GRAB, LET_GO - GRAB));
    let drop_at = (area.right - 250.0, area.top + 260.0);
    let (cx, cy) = if t < 1.2 {
        let come = ease_in_out(span(t, 0.0, 0.5));
        (mix(860.0, a.0, come), mix(1100.0, a.1, come))
    } else if t < GRAB {
        let to = ease_in_out(span(t, 1.2, 0.4));
        (mix(a.0, b.0, to), mix(a.1, b.1, to))
    } else {
        (mix(b.0, drop_at.0, drag), mix(b.1, drop_at.1, drag) - 120.0 * (drag * PI).sin())
    };
    if t > GRAB && t < LET_GO {
        studio.rounded(rect(cx - 70.0, cy - 60.0, cx + 70.0, cy + 60.0), 16.0, 0x2B1E5C, 0.95);
        studio.glow(cx, cy, 90.0, 70.0, MAGENTA, 0.5);
    }
    studio.cursor(cx + 6.0, cy + 6.0, 1.5, span(t, 0.0, 0.15) * (1.0 - span(t, LET_GO + 0.5, 0.3)));
    caption_at(studio, t, len, 1330.0, "Buka atau seret keluar", "Klik dua kali, atau seret ke chat");
}

fn open_out_sound(score: &mut Score) {
    for &at in &DOUBLE {
        score.cue(at, Cue::Click { gain: 0.3 });
    }
    score.cue(0.9, Cue::Pop { midi: 76.0, gain: 0.18 });
    score.cue(GRAB, Cue::Tick { hz: 2000.0, gain: 0.12 });
    score.cue_pan(GRAB, (0.0, 0.4), Cue::Whoosh { len: LET_GO - GRAB, from: 400.0, to: 1600.0, pan: (0.0, 0.4), gain: 0.18 });
    score.cue_pan(LET_GO, (0.3, 0.3), Cue::Pop { midi: 81.0, gain: 0.22 });
}

// ---------------------------------------------------------------- 16. screenshot

const KEYS: f64 = 0.3;
const SNAP: f64 = 1.2;
const LAND: f64 = 1.9;

fn screenshot(studio: &mut Studio, cast: &Cast, t: f64, len: f64) {
    backdrop(studio, t, CYAN);
    let mut data = sample();
    data.media = song();
    data.shelf.add([PathBuf::from("C:\\contoh\\Screenshot 2026-10-21 192104.png")]);
    studio.set_thumbnail(&screenshot_thumb(4));
    let hud = Hud::Screenshot { id: 4, name: "Screenshot 2026-10-21 192104.png".into(), more: 0 };
    // Kamera mendekat ke notch setelah screenshot mendarat.
    let closer = ease_in_out(span(t, LAND - 0.1, 0.45));
    let zoom = 1.0 + 0.35 * closer;
    studio.world(camera(zoom, 540.0, 300.0, 0.0, 0.0));
    let screen = rect(90.0, 300.0, 990.0, 860.0);
    desktop(studio, screen, CYAN, 1.0);
    let win = rect(170.0, 420.0, 640.0, 760.0);
    window(studio, win, "Desain", 1.0);
    filler(studio, win, 5, 1.0);
    // Setelah screenshot mendarat, desktop meredup supaya mata tertuju ke notch.
    studio.rect(screen, 0x03050C, 0.55 * closer);
    let keys = [
        (LAND - 0.05, View::Pill, View::Pill.size(), 0.0f32),
        (LAND, View::Hud, View::Hud.size(), 0.0),
        (len - 0.5, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t.max(LAND - 0.05));
    let mut scene = home(&data, &cast.idle, &cast.none, View::Pill, Page::Home, false);
    motion.apply(&mut scene);
    if t >= LAND {
        scene.hud = Some(&hud);
    }
    studio.notch(&scene, Theme::Black, 540.0, 300.0, 1.35, 1.0);
    // Area pilihan screenshot: layar di luarnya meredup.
    let select = ease_in_out(span(t, KEYS + 0.25, 0.6));
    let cut = rect(mix(240.0, 240.0, select), 470.0, mix(260.0, 840.0, select), mix(490.0, 800.0, select));
    let dim = span(t, KEYS + 0.15, 0.15) * (1.0 - span(t, SNAP, 0.1));
    if dim > 0.0 {
        for part in [
            rect(screen.left, screen.top, screen.right, cut.top),
            rect(screen.left, cut.bottom, screen.right, screen.bottom),
            rect(screen.left, cut.top, cut.left, cut.bottom),
            rect(cut.right, cut.top, screen.right, cut.bottom),
        ] {
            studio.rect(part, 0x000000, 0.45 * dim);
        }
        studio.outline(cut, 4.0, 3.0, 0xFFFFFF, dim);
        studio.cursor(cut.right - 4.0, cut.bottom - 4.0, 1.3, dim);
    }
    // Kilat rana, lalu hasilnya mengecil dan terbang ke notch.
    let flash = 1.0 - span(t, SNAP, 0.25);
    if t >= SNAP && flash > 0.0 {
        studio.rect(cut, 0xFFFFFF, 0.8 * flash);
    }
    let fly = ease_in_out(span(t, SNAP + 0.1, LAND - SNAP - 0.1));
    if t >= SNAP && t < LAND {
        let (w, h) = ((cut.right - cut.left) * mix(1.0, 0.12, fly), (cut.bottom - cut.top) * mix(1.0, 0.12, fly));
        let (cx, cy) = (mix((cut.left + cut.right) / 2.0, 540.0, fly), mix((cut.top + cut.bottom) / 2.0, 320.0, fly) - 80.0 * (fly * PI).sin());
        let card = rect(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
        studio.rounded(card, 12.0, 0x23305A, 1.0);
        studio.outline(card, 12.0, 4.0, 0xFFFFFF, 0.9);
        studio.glow(cx, cy, w, h, CYAN, 0.4);
    }
    studio.reset();
    // Pintasan Win + Shift + S.
    let show = ease_out(span(t, 0.0, 0.25)) * (1.0 - span(t, LAND - 0.2, 0.3));
    for (x, label) in [(320.0, "Win"), (540.0, "Shift"), (760.0, "S")] {
        keycap(studio, x, 1050.0, 180.0, 130.0, label, press(t, &[KEYS]), CYAN, show);
    }
    caption_at(studio, t, len, 1330.0, "Screenshot masuk shelf", "Siap diseret ke WhatsApp atau Discord");
}

fn screenshot_sound(score: &mut Score) {
    score.cue(KEYS, Cue::Click { gain: 0.35 });
    score.cue(KEYS + 0.25, Cue::Glide { len: 0.6, from: 300.0, to: 700.0, gain: 0.08 });
    score.cue(SNAP, Cue::Shutter { gain: 0.45 });
    score.cue(SNAP + 0.1, Cue::Whoosh { len: LAND - SNAP - 0.1, from: 3000.0, to: 600.0, pan: (0.0, 0.0), gain: 0.2 });
    score.cue(LAND, Cue::Pop { midi: 79.0, gain: 0.24 });
    score.cue(LAND + 0.05, Cue::Chime { notes: vec![84.0, 88.0], gain: 0.1 });
}
