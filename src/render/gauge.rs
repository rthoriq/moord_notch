//! Suasana halaman Sistem, "mesin yang hidup": pendaran yang mengikuti beban tertinggi, riwayat
//! CPU 30 detik, titik yang berdetak tiap data baru, cincin yang memindai selagi menunggu data
//! pertama, dan cincin yang menghitung naik saat data pertama tiba. Cincin tidak dianimasikan
//! tiap pembaruan, supaya halaman yang mengukur CPU tidak ikut membebani CPU.

use super::*;

/// Animasi masuk sesudah data pertama tiba: cincin menyapu bergantian dari kiri.
pub const ENTER_SECS: f32 = 1.0;
const SWEEP: f32 = 0.6;
const STAGGER: f32 = 0.08;
/// Paling lama memindai sebelum menyerah menunggu data pertama.
pub const SCAN_SECS: f32 = 3.0;
/// Panjang busur pindai (bagian lingkaran) dan putarannya per detik.
const SCAN_ARC: f32 = 0.22;
const SCAN_TURNS: f32 = 1.2;
/// Batas pendaran: oranye/merah untuk beban (persen) dan suhu GPU (°C).
const BUSY: f32 = 75.0;
const HEAVY: f32 = 90.0;
const WARM: f32 = 78.0;
const HOT: f32 = 87.0;
/// Grafik riwayat CPU di bawah tulisan CPU.
const SPARK_WIDTH: f32 = 56.0;
const SPARK_TOP: f32 = 105.0;
const SPARK_BOTTOM: f32 = 115.0;

/// Bagian nilai meter ke-`i` yang tampil selama animasi masuk (`age` detik).
pub(super) fn count_up(i: usize, age: f32) -> f32 {
    ease_out((age - i as f32 * STAGGER) / SWEEP)
}

/// Warna dan kekuatan pendaran dari beban tertinggi CPU, GPU, RAM, dan suhu GPU.
pub(super) fn load_glow(st: &Stats) -> (D2D1_COLOR_F, f32) {
    let ram = if st.ram_total > 0 { st.ram_used as f32 / st.ram_total as f32 * 100.0 } else { 0.0 };
    let load = st.cpu.max(st.gpu.unwrap_or(0.0)).max(ram);
    let temp = st.gpu_temp.unwrap_or(0.0);
    if load >= HEAVY || temp >= HOT {
        (RED, 0.6)
    } else if load >= BUSY || temp >= WARM {
        (ORANGE, 0.6)
    } else {
        (TEAL, 0.35)
    }
}

impl Renderer {
    /// Busur pendek yang berputar mengelilingi cincin kosong selagi menunggu data pertama.
    pub(super) fn scan(&self, i: usize, cx: f32, cy: f32, radius: f32, thickness: f32, t: f32, a: f32) {
        let start = (t * SCAN_TURNS + i as f32 * 0.12) * TAU - FRAC_PI_2;
        let end = start + SCAN_ARC * TAU;
        // Muncul pelan di awal dan memudar menjelang menyerah.
        let fade = smooth(t / 0.2) * smooth((SCAN_SECS - t) / 0.4);
        unsafe {
            let Ok(geometry) = self.factory.CreatePathGeometry() else { return };
            let Ok(sink) = geometry.Open() else { return };
            sink.BeginFigure(pt(cx + radius * start.cos(), cy + radius * start.sin()), D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddArc(&arc(cx + radius * end.cos(), cy + radius * end.sin(), radius, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            if sink.Close().is_ok() {
                self.brush.SetColor(&with_alpha(self.pal.accent, a * fade));
                self.target.DrawGeometry(&geometry, &self.brush, thickness, &self.round);
            }
        }
    }

    /// Riwayat CPU (persen, terlama dulu) di bawah tulisan CPU; yang terbaru di kanan.
    pub(super) fn sparkline(&self, cx: f32, cpu: &[f32], color: D2D1_COLOR_F, a: f32) {
        if cpu.len() < 2 {
            return;
        }
        let step = SPARK_WIDTH / (CPU_HISTORY - 1) as f32;
        let right = cx + SPARK_WIDTH / 2.0;
        let point = |i: usize, value: f32| {
            let x = right - (cpu.len() - 1 - i) as f32 * step;
            pt(x, lerp(SPARK_BOTTOM, SPARK_TOP, (value / 100.0).clamp(0.0, 1.0)))
        };
        let figure = |filled: bool| unsafe {
            let geometry = self.factory.CreatePathGeometry().ok()?;
            let sink = geometry.Open().ok()?;
            let first = point(0, cpu[0]);
            if filled {
                sink.BeginFigure(pt(first.X, SPARK_BOTTOM), D2D1_FIGURE_BEGIN_FILLED);
                sink.AddLine(first);
            } else {
                sink.BeginFigure(first, D2D1_FIGURE_BEGIN_HOLLOW);
            }
            for (i, value) in cpu.iter().enumerate().skip(1) {
                sink.AddLine(point(i, *value));
            }
            if filled {
                sink.AddLine(pt(right, SPARK_BOTTOM));
            }
            sink.EndFigure(if filled { D2D1_FIGURE_END_CLOSED } else { D2D1_FIGURE_END_OPEN });
            sink.Close().ok()?;
            Some(geometry)
        };
        unsafe {
            if let Some(area) = figure(true) {
                self.brush.SetColor(&with_alpha(color, a * 0.18));
                self.target.FillGeometry(&area, &self.brush, None::<&ID2D1Brush>);
            }
            if let Some(line) = figure(false) {
                self.brush.SetColor(&with_alpha(color, a * 0.9));
                self.target.DrawGeometry(&line, &self.brush, 1.2, &self.round);
            }
        }
    }

    /// Titik "langsung" di kiri tulisan kepala halaman (yang rata kanan di `r`); berganti terang
    /// dan redup tiap data baru tiba.
    pub(super) fn beat(&self, r: f32, y: f32, right: &str, beat: bool, a: f32) {
        let x = r - self.text_width(right, &self.fonts.caption_end) - 8.0;
        self.circle(x, y, 2.5, GREEN, a * if beat { 1.0 } else { 0.35 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glow_follows_heaviest_load() {
        let calm = Stats { cpu: 23.0, gpu: Some(64.0), gpu_temp: Some(71.0), ram_used: 6, ram_total: 10, ..Stats::default() };
        assert_eq!(load_glow(&calm).1, 0.35);
        let busy = Stats { cpu: 80.0, ..calm.clone() };
        assert_eq!(load_glow(&busy).0.g, ORANGE.g);
        let hot = Stats { gpu_temp: Some(88.0), ..calm.clone() };
        assert_eq!(load_glow(&hot).0.g, RED.g);
        let full = Stats { ram_used: 95, ram_total: 100, ..calm };
        assert_eq!(load_glow(&full).0.g, RED.g);
    }

    #[test]
    fn meters_count_up_in_turn() {
        assert_eq!(count_up(0, 0.0), 0.0);
        assert!(count_up(0, 0.3) > count_up(3, 0.3));
        assert_eq!(count_up(3, f32::MAX), 1.0);
    }
}
