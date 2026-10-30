//! Peraga di sekitar notch: tombol keyboard, touchpad, jendela, dan desktop tiruan. Semuanya
//! digambar bergaya (bukan tangkapan layar), supaya tajam dan tanpa data pribadi.

use super::*;

/// Tombol keyboard 3D; `press` 0..1 menekannya ke bawah dan menyalakan warnanya. Mengembalikan
/// titik tengah permukaan tombol (untuk ikon atau lampu di atasnya).
#[allow(clippy::too_many_arguments)]
pub(super) fn keycap(studio: &mut Studio, cx: f32, cy: f32, w: f32, h: f32, label: &str, press: f32, tint: u32, alpha: f32) -> (f32, f32) {
    if alpha <= 0.01 {
        return (cx, cy);
    }
    let depth = 14.0;
    let sink = depth * 0.75 * press;
    let base = rect(cx - w / 2.0, cy - h / 2.0 + depth, cx + w / 2.0, cy + h / 2.0 + depth);
    studio.rounded(rect(base.left + 4.0, base.top + 12.0, base.right + 4.0, base.bottom + 12.0), 22.0, 0x000000, 0.35 * alpha);
    studio.rounded(base, 22.0, 0x0B0F1A, alpha);
    let top = rect(cx - w / 2.0, cy - h / 2.0 + sink, cx + w / 2.0, cy + h / 2.0 + sink);
    studio.rounded(top, 22.0, 0x1C2333, alpha);
    studio.rounded(top, 22.0, tint, 0.35 * press * alpha);
    studio.outline(top, 22.0, 2.0, 0xFFFFFF, (0.12 + 0.4 * press) * alpha);
    if press > 0.05 {
        studio.glow(cx, cy + sink, w * 0.9, h * 0.9, tint, 0.3 * press * alpha);
    }
    let size = if label.chars().count() > 3 { h * 0.26 } else { h * 0.34 };
    studio.label(label, SANS, size, true, cx, cy + sink, 0xF2F4F8, alpha, 0);
    (cx, cy + sink)
}

/// Tekanan tombol yang ditekan tepat di detik-detik `presses`: turun cepat, naik pelan.
pub(super) fn press(t: f64, presses: &[f64]) -> f32 {
    presses
        .iter()
        .map(|&at| span(t, at - 0.06, 0.06) * (1.0 - span(t, at + 0.08, 0.18)))
        .fold(0.0, f32::max)
}

/// Thumbnail tiruan sebuah screenshot desktop (BGRA premultiplied).
pub(super) fn screenshot_thumb(id: u64) -> Thumbnail {
    let (w, h) = (160u32, 100u32);
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let window = (20..110).contains(&x) && (14..70).contains(&y);
            let bar = y >= 90;
            let (b, g, r) = if bar {
                (52, 30, 22)
            } else if window {
                (70, 46, 34)
            } else {
                ((120 + y / 2) as u8 as u32, (60 + x / 4) as u32, (40 + x / 3) as u32)
            };
            pixels.extend_from_slice(&[b as u8, g as u8, r as u8, 255]);
        }
    }
    Thumbnail { id, width: w, height: h, pixels }
}

/// Touchpad laptop dilihat dari atas.
pub(super) fn touchpad(studio: &mut Studio, cx: f32, cy: f32, w: f32, h: f32, alpha: f32) {
    let area = rect(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
    studio.rounded(rect(area.left + 6.0, area.top + 14.0, area.right + 6.0, area.bottom + 14.0), 36.0, 0x000000, 0.35 * alpha);
    studio.rounded(area, 36.0, 0x161C2B, 0.95 * alpha);
    studio.outline(area, 36.0, 2.5, 0xFFFFFF, 0.14 * alpha);
    studio.glow(cx, cy - h * 0.2, w * 0.6, h * 0.5, 0xFFFFFF, 0.04 * alpha);
}

/// Ujung jari di touchpad: lingkaran terang dengan jejak di belakangnya (`trail` piksel ke arah
/// datangnya, negatif = dari kanan).
pub(super) fn finger(studio: &mut Studio, x: f32, y: f32, trail: f32, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    for i in 0..8 {
        let k = i as f32 / 8.0;
        studio.circle(x - trail * k, y, 26.0 * (1.0 - k * 0.5), 0xFFFFFF, 0.12 * (1.0 - k) * alpha);
    }
    studio.glow(x, y, 70.0, 70.0, 0xFFFFFF, 0.25 * alpha);
    studio.circle(x, y, 26.0, 0xFFFFFF, 0.85 * alpha);
}

/// Jendela aplikasi tiruan ala Windows 11: bilah judul dengan tombol kecilkan, perbesar, tutup.
pub(super) fn window(studio: &mut Studio, area: D2D_RECT_F, title: &str, alpha: f32) {
    if alpha <= 0.01 {
        return;
    }
    studio.rounded(rect(area.left + 8.0, area.top + 18.0, area.right + 8.0, area.bottom + 18.0), 22.0, 0x000000, 0.35 * alpha);
    studio.rounded(area, 22.0, 0x151B29, 0.97 * alpha);
    studio.outline(area, 22.0, 2.0, 0xFFFFFF, 0.1 * alpha);
    let bar = area.top + 34.0;
    studio.label(title, SANS, 26.0, false, area.left + 28.0, bar, 0xC9D1E0, alpha, -1);
    for (i, glyph) in ['\u{E921}', '\u{E922}', '\u{E8BB}'].into_iter().enumerate() {
        studio.icon(glyph, 18.0, area.right - 36.0 - (2 - i) as f32 * 58.0, bar, 0xC9D1E0, alpha);
    }
    studio.line((area.left, area.top + 68.0), (area.right, area.top + 68.0), 1.5, 0xFFFFFF, 0.06 * alpha);
}

/// Isi jendela: baris-baris abu-abu seperti teks yang samar.
pub(super) fn filler(studio: &mut Studio, area: D2D_RECT_F, rows: usize, alpha: f32) {
    for i in 0..rows {
        let y = area.top + 100.0 + i as f32 * 46.0;
        if y > area.bottom - 30.0 {
            break;
        }
        let w = (area.right - area.left - 64.0) * [0.92, 0.7, 0.84, 0.55, 0.78][i % 5];
        studio.rounded(rect(area.left + 32.0, y, area.left + 32.0 + w, y + 18.0), 9.0, 0x2A3346, alpha);
    }
}

/// Desktop tiruan: wallpaper gradasi dan taskbar Windows 11 dengan ikon di tengah.
pub(super) fn desktop(studio: &mut Studio, area: D2D_RECT_F, tint: u32, alpha: f32) {
    studio.rounded(area, 26.0, 0x0B1226, alpha);
    studio.clip(area);
    let (cx, cy) = ((area.left + area.right) / 2.0, (area.top + area.bottom) / 2.0);
    studio.glow(cx - 180.0, cy + 120.0, 520.0, 360.0, tint, 0.35 * alpha);
    studio.glow(cx + 240.0, cy - 160.0, 420.0, 300.0, ELECTRIC, 0.25 * alpha);
    studio.unclip();
    let bar = rect(area.left + 10.0, area.bottom - 58.0, area.right - 10.0, area.bottom - 10.0);
    studio.rounded(bar, 16.0, 0x141C33, 0.95 * alpha);
    for i in 0..6 {
        let x = cx - 125.0 + i as f32 * 50.0;
        let color = [ELECTRIC, 0xF2F4F8, 0xFFB340, CYAN, MAGENTA, 0x30D158][i];
        studio.rounded(rect(x - 15.0, bar.top + 9.0, x + 15.0, bar.top + 39.0), 8.0, color, 0.85 * alpha);
    }
    studio.outline(area, 26.0, 3.0, CYAN, 0.45 * alpha);
}

/// Sampul tiruan kedua: gradasi oranye ke merah muda.
pub(super) fn sunset_art() -> Art {
    let mut pixels = Vec::with_capacity((ART_PX * ART_PX * 4) as usize);
    for y in 0..ART_PX {
        for x in 0..ART_PX {
            pixels.extend_from_slice(&[(60 + y * 120 / ART_PX) as u8, (150 - x * 90 / ART_PX) as u8, 255, 255]);
        }
    }
    Art { id: 2, pixels, accent: Some(0xFF9F0A) }
}

/// Lagu kedua untuk adegan ganti lagu.
pub(super) fn second_song() -> MediaView {
    MediaView {
        title: "Jalan Pulang".into(),
        artist: "Kopi Pagi".into(),
        progress: Some(0.08),
        elapsed: "0:15".into(),
        remaining: "-3:02".into(),
        accent: Some(0xFF9F0A),
        ..song()
    }
}
