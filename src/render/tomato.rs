//! Suasana halaman Pomodoro, dari timer dapur berbentuk tomat yang jadi asal namanya: cincin
//! bertangkai dan berdaun, tomat kecil untuk tiap sesi fokus hari ini, kenop yang diputar saat
//! mulai, dan tomat baru yang jatuh saat sesi selesai. Di luar animasi tidak ada gerak tambahan.

use std::f32::consts::PI;

use super::*;

/// Animasi masuk: cincin menyapu dari kosong, tomat sesi muncul satu per satu, daun bergoyang.
pub const ENTER_SECS: f32 = 1.0;
const RING_SWEEP: f32 = 0.7;
const POP_FROM: f32 = 0.15;
const POP_GAP: f32 = 0.06;
const POP_SECS: f32 = 0.18;
/// Mulai dari awal: cincin naik penuh, lalu kembali ke kemajuannya.
const WIND_SECS: f32 = 0.9;
const WIND_UP: f32 = 0.35;
/// Fase selesai: cincin berkilau; selesai fokus juga menjatuhkan tomat baru.
const DONE_SECS: f32 = 1.2;
const SPARKLE_SECS: f32 = 0.8;
const FALL_SECS: f32 = 0.3;
const FALL_FROM: f32 = 16.0;
/// Paling banyak tomat yang digambar; sisanya ditulis "+N".
const MAX_TOMATOES: u32 = 8;
const TOMATO_GAP: f32 = 13.0;
const TOMATO_RADIUS: f32 = 4.5;

const TOMATO: D2D1_COLOR_F = rgb(0xFF5640, 1.0);
const LEAF: D2D1_COLOR_F = rgb(0x34A853, 1.0);
const STEM: D2D1_COLOR_F = rgb(0x2B7A3D, 1.0);

/// Lama animasi sesaat di halaman Pomodoro.
pub fn fx_secs(fx: FocusFx) -> f32 {
    match fx {
        FocusFx::Wind => WIND_SECS,
        FocusFx::Done(_) => DONE_SECS,
    }
}

/// Kemajuan cincin yang tampil: diputar penuh lalu kembali saat mulai dari awal, atau menyapu
/// dari kosong saat halaman dibuka (`age` detik).
pub(super) fn ring(progress: f32, fx: Option<(FocusFx, f32)>, age: f32) -> f32 {
    match fx {
        Some((FocusFx::Wind, t)) if t < WIND_UP => ease_out(t / WIND_UP),
        Some((FocusFx::Wind, t)) => lerp(1.0, progress, smooth((t - WIND_UP) / (WIND_SECS - WIND_UP - 0.05))),
        _ => progress * ease_out(age / RING_SWEEP),
    }
}

/// Goyangan daun (radian) yang reda dengan sendirinya.
fn wiggle(fx: Option<(FocusFx, f32)>, age: f32) -> f32 {
    let t = match fx {
        Some((_, t)) => t,
        None if age < ENTER_SECS => age,
        None => return 0.0,
    };
    0.35 * (-t / 0.25).exp() * (TAU * 3.0 * t).sin()
}

/// Tinggi tomat baru di atas tempatnya: jatuh, lalu memantul kecil.
fn fall(t: f32) -> f32 {
    if t < FALL_SECS {
        return -FALL_FROM * (1.0 - (t / FALL_SECS).powi(2));
    }
    let b = t - FALL_SECS;
    -3.5 * (PI * b / 0.22).sin().abs() * (-b / 0.15).exp()
}

impl Renderer {
    /// Tangkai dan dua daun di puncak cincin berjari-jari `radius`.
    pub(super) fn calyx(&self, cx: f32, cy: f32, radius: f32, fx: Option<(FocusFx, f32)>, age: f32, a: f32) {
        let top = cy - radius;
        let turn = wiggle(fx, age);
        unsafe {
            self.brush.SetColor(&with_alpha(STEM, a));
            self.target.DrawLine(pt(cx, top - 1.0), pt(cx + 1.5 + turn * 3.0, top - 7.0), &self.brush, 2.2, &self.round);
        }
        self.leaf(cx, top, PI - 0.35 + turn, 9.0, 5.0, a);
        self.leaf(cx, top, 0.35 - turn, 9.0, 5.0, a);
    }

    /// Kilau cincin saat fase selesai: lingkaran yang melebar dan memudar.
    pub(super) fn sparkle(&self, cx: f32, cy: f32, radius: f32, fx: Option<(FocusFx, f32)>, a: f32) {
        let Some((FocusFx::Done(phase), t)) = fx else { return };
        if t >= SPARKLE_SECS {
            return;
        }
        let e = ease_out(t / SPARKLE_SECS);
        self.circle_outline(cx, cy, radius + 9.0 * e, 3.0 * (1.0 - e) + 0.5, phase_color(phase), a * 0.7 * (1.0 - e));
    }

    /// Tomat kecil untuk tiap sesi fokus hari ini, rata kanan di kepala halaman.
    pub(super) fn tomatoes(&self, r: f32, y: f32, sessions: u32, fx: Option<(FocusFx, f32)>, age: f32, a: f32) {
        let shown = sessions.min(MAX_TOMATOES);
        let mut right = r;
        if sessions > shown {
            let more = format!("+{}", sessions - shown);
            self.text(&more, &self.fonts.caption_end, rect(r - 40.0, y - 8.0, r, y + 8.0), self.pal.muted, a);
            right -= self.text_width(&more, &self.fonts.caption_end) + 5.0;
        }
        // Tomat baru hanya jatuh kalau ia sendiri tergambar (bukan masuk ke "+N").
        let dropping = match fx {
            Some((FocusFx::Done(Phase::Focus), t)) if sessions <= MAX_TOMATOES => Some(t),
            _ => None,
        };
        for i in 0..shown {
            let x = right - TOMATO_RADIUS - (shown - 1 - i) as f32 * TOMATO_GAP;
            let pop = ease_out((age - POP_FROM - i as f32 * POP_GAP) / POP_SECS);
            let dy = dropping.filter(|_| i + 1 == shown).map_or(0.0, fall);
            self.tomato(x, y + dy, TOMATO_RADIUS * pop, a);
        }
    }

    fn tomato(&self, x: f32, y: f32, radius: f32, a: f32) {
        if radius < 0.3 {
            return;
        }
        self.circle(x, y + 0.5, radius, TOMATO, a);
        self.circle(x - radius * 0.35, y - radius * 0.15, radius * 0.28, rgb(0xFFFFFF, 0.35), a);
        let top = y - radius + 1.2;
        self.leaf(x, top, PI - 0.45, radius * 0.95, radius * 0.6, a);
        self.leaf(x, top, 0.45, radius * 0.95, radius * 0.6, a);
    }

    /// Daun dari pangkal (`x`, `y`) ke arah `angle` (radian, searah jarum jam dari kanan).
    fn leaf(&self, x: f32, y: f32, angle: f32, length: f32, width: f32, a: f32) {
        let (dx, dy) = (angle.cos(), angle.sin());
        let (mx, my) = (x + dx * length * 0.5, y + dy * length * 0.5);
        let (px, py) = (-dy * width, dx * width);
        let tip = pt(x + dx * length, y + dy * length);
        unsafe {
            let Ok(geometry) = self.factory.CreatePathGeometry() else { return };
            let Ok(sink) = geometry.Open() else { return };
            sink.BeginFigure(pt(x, y), D2D1_FIGURE_BEGIN_FILLED);
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT { point1: pt(mx + px, my + py), point2: tip });
            sink.AddQuadraticBezier(&D2D1_QUADRATIC_BEZIER_SEGMENT { point1: pt(mx - px, my - py), point2: pt(x, y) });
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            if sink.Close().is_ok() {
                self.brush.SetColor(&with_alpha(LEAF, a));
                self.target.FillGeometry(&geometry, &self.brush, None::<&ID2D1Brush>);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_winds_then_returns() {
        assert_eq!(ring(0.0, Some((FocusFx::Wind, 0.0)), f32::MAX), 0.0);
        assert_eq!(ring(0.0, Some((FocusFx::Wind, WIND_UP)), f32::MAX), 1.0);
        assert_eq!(ring(0.0, Some((FocusFx::Wind, WIND_SECS)), f32::MAX), 0.0);
        assert_eq!(ring(0.29, None, 0.0), 0.0);
        assert_eq!(ring(0.29, None, f32::MAX), 0.29);
        assert_eq!(wiggle(None, f32::MAX), 0.0);
    }

    #[test]
    fn new_tomato_lands_in_place() {
        assert_eq!(fall(0.0), -FALL_FROM);
        assert_eq!(fall(FALL_SECS), 0.0);
        assert!(fall(DONE_SECS).abs() < 0.01);
    }
}
