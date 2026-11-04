//! Suasana halaman Sholat: warna langit yang mengikuti waktu yang sedang berjalan, bintang di
//! malam hari, dan animasi masuknya. Di luar animasi masuk tidak ada yang bergerak.

use super::*;

/// Animasi masuk: cincin menyapu, sorotan jadwal bergeser, langit memudar masuk, bintang berkelip.
pub const ENTER_SECS: f32 = 1.2;
const RING_SWEEP: f32 = 0.7;
const SLIDE: (f32, f32) = (0.15, 0.5);
const GLOW_IN: f32 = 0.6;
/// Bintang muncul satu per satu mulai `STARS_FROM`, berjarak `STAR_GAP`.
const STARS_FROM: f32 = 0.15;
const STAR_GAP: f32 = 0.07;
/// Bintang di sela teks bagian atas panel: geser dari tepi kiri isi, tinggi, jari-jari, terang.
const STARS: [(f32, f32, f32, f32); 10] = [
    (88.0, 9.0, 0.9, 0.7),
    (122.0, 27.0, 0.6, 0.45),
    (160.0, 13.0, 1.1, 0.8),
    (196.0, 31.0, 0.7, 0.5),
    (232.0, 8.0, 0.8, 0.6),
    (210.0, 46.0, 0.9, 0.55),
    (266.0, 40.0, 0.6, 0.4),
    (300.0, 9.0, 1.0, 0.7),
    (338.0, 42.0, 0.7, 0.5),
    (356.0, 8.0, 0.6, 0.45),
];

/// Warna langit tiap waktu, dipakai cincin, sorotan jadwal, dan pendaran panel.
pub(super) fn sky_color(sky: Sky) -> D2D1_COLOR_F {
    match sky {
        Sky::Dawn => rgb(0xD17BD8, 1.0),
        Sky::Morning => rgb(0xFFC94D, 1.0),
        Sky::Noon => rgb(0x5AC8FA, 1.0),
        Sky::Afternoon => rgb(0xFF9F2E, 1.0),
        Sky::Dusk => rgb(0xFF6A3D, 1.0),
        Sky::Night => rgb(0x7B8CFF, 1.0),
    }
}

/// Kekuatan pendaran langit; memudar masuk selama animasi masuk (`age` detik). Kuning pagi
/// lebih menyala daripada warna lain, jadi dibuat lebih tipis.
pub(super) fn glow(sky: Sky, age: f32) -> f32 {
    let strength = if sky == Sky::Morning { 0.4 } else { 0.55 };
    strength * smooth(age / GLOW_IN)
}

/// Kemajuan cincin selama animasi masuk: menyapu dari kosong.
pub(super) fn ring(progress: f32, age: f32) -> f32 {
    progress * ease_out(age / RING_SWEEP)
}

/// Posisi sorotan jadwal (indeks kolom, bisa pecahan): bergeser dari waktu sebelumnya.
pub(super) fn highlight(next: usize, age: f32) -> f32 {
    let from = next.saturating_sub(1) as f32;
    lerp(from, next as f32, smooth((age - SLIDE.0) / SLIDE.1))
}

impl Renderer {
    /// Bintang malam di antara `l` dan tepi kanan isi; berkelip sekali saat halaman dibuka.
    pub(super) fn stars(&self, l: f32, age: f32, a: f32) {
        // Di tema terang bintang putih tidak terlihat, jadi memakai biru malam.
        let color = if self.pal.text.r < 0.5 { sky_color(Sky::Night) } else { WHITE };
        for (i, (dx, y, radius, light)) in STARS.into_iter().enumerate() {
            let start = STARS_FROM + i as f32 * STAR_GAP;
            let shown = smooth((age - start) / 0.12);
            if shown < 0.01 {
                continue;
            }
            let flash = (-((age - start - 0.15) / 0.1).powi(2)).exp();
            self.circle(l + dx, y, radius * (1.0 + 0.8 * flash), color, a * (light * shown * (1.0 + 0.9 * flash)).min(1.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_settles_at_final_values() {
        assert_eq!(ring(0.62, 0.0), 0.0);
        assert_eq!(ring(0.62, f32::MAX), 0.62);
        assert_eq!(highlight(4, 0.0), 3.0);
        assert_eq!(highlight(4, f32::MAX), 4.0);
        assert_eq!(highlight(0, 0.3), 0.0, "kolom pertama tidak bergeser dari mana pun");
        assert_eq!(glow(Sky::Noon, f32::MAX), 0.55);
        assert_eq!(glow(Sky::Night, 0.0), 0.0);
    }
}
