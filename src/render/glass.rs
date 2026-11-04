//! Gelas di halaman Minum air: terisi sesuai target hari ini, dengan kran yang muncul dan
//! mengalirkan air setiap kali minum dicatat. Air hanya bergerak selama dituang, sesaat setelah
//! halaman dibuka, dan sebentar sesudahnya; selebihnya diam, jadi panel yang dibiarkan terbuka
//! tidak digambar ulang terus.

use windows::Win32::Graphics::Direct2D::{
    D2D1_COMBINE_MODE_INTERSECT, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, ID2D1GeometrySink,
};

use super::*;

/// Animasi masuk halaman: gelas terisi dari kosong, lalu airnya bergoyang sampai tenang.
pub const ENTER_SECS: f32 = 1.4;
const ENTER_FILL: f32 = 0.6;
/// Satu tuangan: kran turun, air mengalir, permukaan naik sambil beriak, lalu kran pergi.
const POUR_SECS: f32 = 1.8;
/// Tuangan yang mencapai target diakhiri gelembung yang naik.
const BUBBLE_SECS: f32 = 2.4;
/// Gelas yang dibatalkan hanya turun dan beriak sedikit, tanpa kran.
const UNDO_SECS: f32 = 1.0;
const UNDO_FALL: f32 = 0.5;
/// Urutan tuangan, dalam detik sejak diklik.
const TAP_IN: f32 = 0.18;
const FLOW: (f32, f32) = (0.15, 0.85);
const RISE: (f32, f32) = (0.25, 0.95);
const TAP_OUT: (f32, f32) = (0.95, 1.2);
const BUBBLES_FROM: f32 = 0.95;
/// Kecepatan jatuh air dari mulut kran, DIP per detik.
const FALL_SPEED: f32 = 280.0;

/// Bentuk gelas: sedikit mengerucut ke bawah, sudut bawahnya membulat.
const TOP: f32 = 50.0;
const BOTTOM: f32 = 100.0;
const TOP_HALF: f32 = 19.0;
const BOTTOM_HALF: f32 = 14.5;
const CORNER: f32 = 5.0;
/// Permukaan air saat kosong dan saat target tercapai (sedikit di bawah bibir gelas).
const EMPTY_Y: f32 = BOTTOM - 2.0;
const FULL_Y: f32 = TOP + 5.0;
/// Mulut kran, sedikit di kanan tengah gelas.
const NOZZLE_DX: f32 = 7.0;
const NOZZLE_Y: f32 = 47.5;
/// Air lebih terang di permukaan dan lebih pekat di dasar.
const WATER_TOP: D2D1_COLOR_F = rgb(0x5AB0FF, 0.95);
const WATER_BOTTOM: D2D1_COLOR_F = rgb(0x0A84FF, 0.95);
const MENISCUS: D2D1_COLOR_F = rgb(0x9CD0FF, 1.0);
/// Gelembung saat target tercapai: geser dari tengah gelas, jari-jari, dan jeda mulainya.
const BUBBLES: [(f32, f32, f32); 5] = [(-8.0, 1.6, 0.0), (5.0, 1.2, 0.17), (-2.0, 2.0, 0.3), (9.0, 1.4, 0.45), (-5.0, 1.1, 0.6)];
const BUBBLE_RISE: f32 = 0.8;

/// Lama animasi sebuah tuangan dari isi `from` ke `to` (0..1 dari target).
pub fn pour_secs(from: f32, to: f32) -> f32 {
    if to < from {
        UNDO_SECS
    } else if from < 1.0 && to >= 1.0 {
        BUBBLE_SECS
    } else {
        POUR_SECS
    }
}

/// Isi gelas yang tampil (0..1): bergerak dari isi sebelumnya selama tuangan `pour` (isi awal,
/// detik berjalan), atau terisi dari kosong selama animasi masuk halaman.
pub fn shown_level(level: f32, pour: Option<(f32, f32)>, page_age: f32) -> f32 {
    match pour {
        Some((from, age)) if level < from => lerp(from, level, smooth(age / UNDO_FALL)),
        Some((from, age)) => lerp(from, level, smooth((age - RISE.0) / (RISE.1 - RISE.0))),
        None if page_age < ENTER_FILL => level * ease_out(page_age / ENTER_FILL),
        None => level,
    }
}

/// Setengah lebar bagian dalam gelas pada ketinggian `y`.
fn half_width(y: f32) -> f32 {
    BOTTOM_HALF + (TOP_HALF - BOTTOM_HALF) * ((BOTTOM - y) / (BOTTOM - TOP)).clamp(0.0, 1.0)
}

/// Gerak permukaan air: kemiringan (air bergoyang dari sisi ke sisi), tinggi riak, dan titik
/// asal riak (-1..1 dari kiri ke kanan gelas; jauh di luar gelas berarti ombak merata).
#[derive(Default)]
struct Surface {
    tilt: f32,
    ripple: f32,
    origin: f32,
    age: f32,
}

impl Surface {
    fn new(level: f32, pour: Option<(f32, f32)>, page_age: f32) -> Self {
        let fade = |t: f32, tau: f32| (-t.max(0.0) / tau).exp();
        let sway = |t: f32| (TAU * 1.6 * t).sin();
        match pour {
            Some((from, age)) if level < from => {
                Self { tilt: 0.07 * fade(age, 0.35) * sway(age), ripple: 0.9 * fade(age, 0.3), origin: -3.0, age }
            }
            Some((_, age)) => {
                // Beriak dari titik jatuhnya air selama mengalir, lalu reda sambil sedikit bergoyang.
                let stop = FLOW.1 + 0.1;
                let (ripple, tilt) = if age < stop {
                    (1.4 * smooth((age - FLOW.0 - 0.12) / 0.1), 0.0)
                } else {
                    (1.4 * fade(age - stop, 0.3), 0.05 * fade(age - stop, 0.4) * sway(age - stop))
                };
                Self { tilt, ripple, origin: NOZZLE_DX / TOP_HALF, age }
            }
            // Seperti gelas yang baru diletakkan: airnya bergoyang lalu tenang.
            None if page_age < ENTER_SECS => Self {
                tilt: 0.12 * fade(page_age, 0.4) * sway(page_age),
                ripple: 1.6 * fade(page_age, 0.4),
                origin: -3.0,
                age: page_age,
            },
            None => Self::default(),
        }
    }

    /// Selisih tinggi permukaan di posisi `u` (-1..1, kiri ke kanan gelas), dalam DIP.
    fn offset(&self, u: f32) -> f32 {
        let d = (u - self.origin).abs();
        let near = 0.5 + 0.5 * (-(d / 0.5).powi(2)).exp();
        self.tilt * u * TOP_HALF + self.ripple * near * (d * 9.0 - self.age * 15.0).sin()
    }
}

impl Renderer {
    /// Gelas berpusat di `gx` dengan isi `level` (0..1 dari target). Pipa kran keluar dari
    /// `edge`, tepi kanan panel. `pour` dan `page_age` lihat [`shown_level`].
    #[allow(clippy::too_many_arguments)]
    pub(super) fn glass(&self, gx: f32, edge: f32, level: f32, pour: Option<(f32, f32)>, page_age: f32, a: f32) {
        let shown = shown_level(level, pour, page_age);
        let surface = Surface::new(level, pour, page_age);
        let water_y = lerp(EMPTY_Y, FULL_Y, shown.clamp(0.0, 1.0));
        let Some(inside) = self.path(|sink| self.glass_figure(sink, gx, true)) else { return };
        self.fill(&inside, self.tint(0.07), a);
        // Garis takaran tiap seperempat target di dinding kiri.
        for k in 1..4 {
            let y = lerp(EMPTY_Y, FULL_Y, k as f32 / 4.0);
            let x = gx - half_width(y) + 2.5;
            self.rounded(rect(x, y - 0.5, x + 4.0, y + 0.5), 0.5, self.tint(0.22), a);
        }

        let pouring = pour.filter(|(from, _)| level >= *from);
        if let Some((_, age)) = pouring
            && age > FLOW.0
        {
            // Kepala aliran jatuh dari mulut kran sampai permukaan; ekornya menyusul saat kran
            // ditutup.
            let nx = gx + NOZZLE_DX;
            let head = (NOZZLE_Y + (age - FLOW.0) * FALL_SPEED).min(water_y + surface.offset(NOZZLE_DX / TOP_HALF) + 1.0);
            let tail = NOZZLE_Y + (age - FLOW.1).max(0.0) * FALL_SPEED;
            if head > tail + 0.5 {
                self.rounded(rect(nx - 1.6, tail, nx + 1.6, head), 1.6, WATER_TOP, a);
            }
        }

        if shown > 0.004 {
            self.water(&inside, gx, water_y, &surface, a);
        }
        if let Some((from, age)) = pouring
            && from < 1.0
            && level >= 1.0
        {
            for (dx, radius, delay) in BUBBLES {
                let t = (age - BUBBLES_FROM - delay) / BUBBLE_RISE;
                if !(0.0..1.0).contains(&t) {
                    continue;
                }
                let y = lerp(BOTTOM - 4.0, water_y + 3.0, t * t);
                let x = gx + dx + (t * 9.0 + dx).sin() * 1.2;
                let fade = (t * 5.0).min(1.0) * (1.0 - t).min(0.3) / 0.3;
                self.circle_outline(x, y, radius, 0.8, WHITE, a * 0.8 * fade);
            }
        }

        // Kaca: pantulan tipis di dinding kiri, lalu garis tepi tanpa tutup di atas.
        unsafe {
            self.brush.SetColor(&rgb(0xFFFFFF, 0.12 * a));
            let (top, bottom) = (pt(gx - TOP_HALF + 4.0, TOP + 6.0), pt(gx - BOTTOM_HALF + 3.5, BOTTOM - 10.0));
            self.target.DrawLine(top, bottom, &self.brush, 2.0, &self.round);
        }
        if let Some(rim) = self.path(|sink| self.glass_figure(sink, gx, false)) {
            unsafe {
                self.brush.SetColor(&with_alpha(self.tint(0.45), a));
                self.target.DrawGeometry(&rim, &self.brush, 1.6, &self.round);
            }
        }

        if let Some((_, age)) = pouring {
            let show = if age < TAP_OUT.0 { smooth(age / TAP_IN) } else { 1.0 - smooth((age - TAP_OUT.0) / (TAP_OUT.1 - TAP_OUT.0)) };
            if show > 0.01 {
                self.tap(gx, edge, (1.0 - show) * -6.0, a * show);
            }
        }
    }

    /// Air di dalam gelas: bidang berpermukaan bergelombang yang dipotong bentuk gelas, dengan
    /// garis permukaan yang lebih terang.
    fn water(&self, inside: &ID2D1PathGeometry, gx: f32, water_y: f32, surface: &Surface, a: f32) {
        const STEPS: usize = 20;
        let at = |u: f32| (water_y + surface.offset(u.clamp(-1.0, 1.0))).clamp(TOP - 2.0, BOTTOM + 2.0);
        let Some(wave) = self.path(|sink| unsafe {
            let wide = TOP_HALF * 1.3;
            sink.BeginFigure(pt(gx - wide, BOTTOM + 4.0), D2D1_FIGURE_BEGIN_FILLED);
            for i in 0..=STEPS {
                let u = -1.3 + 2.6 * i as f32 / STEPS as f32;
                sink.AddLine(pt(gx + u * TOP_HALF, at(u)));
            }
            sink.AddLine(pt(gx + wide, BOTTOM + 4.0));
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        }) else {
            return;
        };
        let Some(body) = self.path(|sink| unsafe {
            let _ = inside.CombineWithGeometry(&wave, D2D1_COMBINE_MODE_INTERSECT, None, 0.25, sink);
        }) else {
            return;
        };
        let stops = [
            D2D1_GRADIENT_STOP { position: 0.0, color: with_alpha(WATER_TOP, a) },
            D2D1_GRADIENT_STOP { position: 1.0, color: with_alpha(WATER_BOTTOM, a) },
        ];
        unsafe {
            let Ok(collection) = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP) else {
                return;
            };
            let props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: pt(0.0, water_y), endPoint: pt(0.0, BOTTOM) };
            let Ok(brush) = self.target.CreateLinearGradientBrush(&props, None, &collection) else { return };
            self.target.FillGeometry(&body, &brush, None::<&ID2D1Brush>);
        }
        // Garis permukaan hanya kalau air sudah melewati sudut bawah, supaya tidak menembus dinding.
        if water_y > BOTTOM - CORNER - 1.0 {
            return;
        }
        let half = half_width(water_y) - 1.2;
        if let Some(line) = self.path(|sink| unsafe {
            sink.BeginFigure(pt(gx - half, at(-1.0)), D2D1_FIGURE_BEGIN_HOLLOW);
            for i in 1..=STEPS {
                let u = -1.0 + 2.0 * i as f32 / STEPS as f32;
                sink.AddLine(pt(gx + u * half, at(u)));
            }
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
        }) {
            unsafe {
                self.brush.SetColor(&with_alpha(MENISCUS, 0.9 * a));
                self.target.DrawGeometry(&line, &self.brush, 1.4, &self.round);
            }
        }
    }

    /// Kran dari tepi kanan panel: pipa, corong yang menghadap ke bawah dengan bibir di ujungnya,
    /// dan pegangan di atas pipa.
    fn tap(&self, gx: f32, edge: f32, dy: f32, a: f32) {
        let (metal, shine) = (self.tint(0.5), self.tint(0.75));
        let nx = gx + NOZZLE_DX;
        self.rounded(rect(nx - 2.5, 38.0 + dy, edge + 4.0, 43.0 + dy), 2.5, metal, a);
        self.rounded(rect(nx + 2.0, 38.0 + dy, edge + 4.0, 39.0 + dy), 0.5, shine, a * 0.6);
        self.rounded(rect(nx - 2.5, 38.0 + dy, nx + 2.5, NOZZLE_Y - 1.0 + dy), 2.0, metal, a);
        self.rounded(rect(nx - 3.5, NOZZLE_Y - 2.5 + dy, nx + 3.5, NOZZLE_Y + dy), 1.0, shine, a);
        self.rounded(rect(gx + 17.0, 34.0 + dy, gx + 21.0, 38.5 + dy), 1.5, metal, a);
        self.rounded(rect(gx + 13.0, 32.5 + dy, gx + 25.0, 35.0 + dy), 1.25, shine, a);
    }

    /// Bentuk gelas: `closed` = bidang dalamnya (untuk latar dan potongan air), selain itu garis
    /// tepi yang terbuka di atas.
    fn glass_figure(&self, sink: &ID2D1GeometrySink, gx: f32, closed: bool) {
        let (left, right) = (gx - BOTTOM_HALF, gx + BOTTOM_HALF);
        unsafe {
            sink.BeginFigure(pt(gx - TOP_HALF, TOP), if closed { D2D1_FIGURE_BEGIN_FILLED } else { D2D1_FIGURE_BEGIN_HOLLOW });
            sink.AddLine(pt(left, BOTTOM - CORNER));
            sink.AddArc(&arc(left + CORNER, BOTTOM, CORNER, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddLine(pt(right - CORNER, BOTTOM));
            sink.AddArc(&arc(right, BOTTOM - CORNER, CORNER, D2D1_SWEEP_DIRECTION_COUNTER_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.AddLine(pt(gx + TOP_HALF, TOP));
            sink.EndFigure(if closed { D2D1_FIGURE_END_CLOSED } else { D2D1_FIGURE_END_OPEN });
        }
    }

    /// Bangun satu bentuk path; `None` kalau Direct2D gagal.
    pub(super) fn path(&self, build: impl FnOnce(&ID2D1GeometrySink)) -> Option<ID2D1PathGeometry> {
        unsafe {
            let geometry = self.factory.CreatePathGeometry().ok()?;
            let sink = geometry.Open().ok()?;
            build(&sink);
            sink.Close().ok()?;
            Some(geometry)
        }
    }

    pub(super) fn fill(&self, geometry: &ID2D1PathGeometry, color: D2D1_COLOR_F, alpha: f32) {
        unsafe {
            self.brush.SetColor(&with_alpha(color, alpha));
            self.target.FillGeometry(geometry, &self.brush, None::<&ID2D1Brush>);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pour_moves_level_from_before_to_after() {
        assert_eq!(shown_level(0.5, Some((0.375, 0.0)), f32::MAX), 0.375);
        assert_eq!(shown_level(0.5, Some((0.375, 2.0)), f32::MAX), 0.5);
        // Batal: langsung turun, tanpa menunggu kran.
        assert!(shown_level(0.25, Some((0.375, 0.2)), f32::MAX) < 0.375);
        // Masuk halaman: mulai kosong, penuh sesuai isi setelah terisi.
        assert_eq!(shown_level(0.6, None, 0.0), 0.0);
        assert_eq!(shown_level(0.6, None, ENTER_SECS), 0.6);
    }

    #[test]
    fn surface_rests_when_nothing_moves() {
        let surface = Surface::new(0.6, None, f32::MAX);
        assert!((0..=20).map(|i| -1.0 + i as f32 / 10.0).all(|u| surface.offset(u) == 0.0));
        assert_eq!(pour_secs(0.875, 1.0), BUBBLE_SECS);
        assert_eq!(pour_secs(0.5, 0.375), UNDO_SECS);
    }
}
