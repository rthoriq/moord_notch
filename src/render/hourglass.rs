//! Suasana halaman Timer, jam pasir: pasir di atas adalah sisa waktu, yang di bawah sudah lewat.
//! Jam pasir dibalik saat durasi dipilih, "+1 mnt" menjatuhkan sedikit pasir ke atas, dan saat
//! waktu habis ia bergoyang seperti alarm berdering. Selama timer berjalan pasirnya hanya bergeser
//! saat halaman digambar ulang tiap setengah detik untuk jamnya, jadi tidak ada frame tambahan.

use std::f32::consts::PI;

use windows::Win32::Graphics::Direct2D::{D2D1_COMBINE_MODE_INTERSECT, ID2D1GeometrySink};

use super::*;

/// Animasi masuk: cincin menyapu ke kemajuannya dan pasir mengalir ke tempatnya.
pub const ENTER_SECS: f32 = 1.0;
const RING_SWEEP: f32 = 0.7;
const ENTER_FLOW: f32 = 0.8;
/// Dibalik: diputar setengah lingkaran, lalu pasirnya turun ke leher.
const TURN_SECS: f32 = 0.45;
const SETTLE_SECS: f32 = 0.2;
/// "+1 mnt": pasir di atas bertambah sambil beberapa butir jatuh ke permukaannya.
const ADD_SECS: f32 = 0.6;
const ADD_RISE: f32 = 0.45;
const GRAIN_FALL: f32 = 0.3;
/// Geser dari tengah dan jeda tiap butir.
const GRAINS: [(f32, f32); 4] = [(-3.5, 0.0), (2.5, 0.06), (-0.5, 0.12), (4.0, 0.18)];
/// Waktu habis: jam pasir bergoyang, cincin penuh merah memudar sambil berkilau, dan tulisan
/// "Waktu habis" bertahan sebentar sebelum kembali ke "Pilih durasi".
const DONE_SECS: f32 = LABEL_HOLD + 2.0 * LABEL_SWAP;
const SHAKE: f32 = 0.22;
const SHAKE_HZ: f32 = 8.0;
const RING_FADE: f32 = 1.0;
const SPARKLE_SECS: f32 = 0.8;
const LABEL_HOLD: f32 = 1.5;
/// Tulisan lama hilang dulu, baru tulisan biasa muncul, supaya keduanya tidak bertumpuk.
const LABEL_SWAP: f32 = 0.25;

/// Bentuk jam pasir: leher setinggi pusat cincin, tinggi tiap tabung dari leher, setengah lebar
/// tabung dan leher, lalu tutup dan tiangnya.
const NECK_Y: f32 = 76.0;
const BULB: f32 = 22.0;
const HALF: f32 = 10.5;
const NECK: f32 = 1.6;
const CAP_HALF: f32 = 15.0;
const CAP: f32 = 3.5;
const POST: f32 = 12.5;
/// Bagian satu tabung yang terisi saat semua pasir ada di sisinya.
const SAND_FILL: f32 = 0.7;
/// Gundukan pasir di bawah, dan cekungan di permukaan atas selama mengalir.
const MOUND: f32 = 3.0;
const DIP: f32 = 2.0;
const SAND: D2D1_COLOR_F = rgb(0xFFC15E, 1.0);
/// Di tema terang pasir kuning muda tenggelam, jadi lebih pekat.
const SAND_DEEP: D2D1_COLOR_F = rgb(0xE08A00, 1.0);

/// Lama animasi sesaat di halaman Timer.
pub fn fx_secs(fx: TimerFx) -> f32 {
    match fx {
        TimerFx::Flip => TURN_SECS + SETTLE_SECS,
        TimerFx::Add(_) => ADD_SECS,
        TimerFx::Done => DONE_SECS,
    }
}

/// Bagian yang sudah lewat yang tampil: naik dari nol saat halaman dibuka (`age` detik, selama
/// `sweep`) dan mundur pelan sesudah "+1 mnt".
fn shown(elapsed: f32, fx: Option<(TimerFx, f32)>, age: f32, sweep: f32) -> f32 {
    match fx {
        Some((TimerFx::Add(from), t)) => lerp(from, elapsed, ease_out(t / ADD_RISE)),
        _ => elapsed * ease_out(age / sweep),
    }
}

/// Kemajuan cincin yang tampil.
pub(super) fn ring(t: &TimerView, age: f32) -> f32 {
    shown(t.progress, t.fx, age, RING_SWEEP)
}

/// Pasir yang masih di atas, 0..1 dari semuanya. Timer yang diam menyimpan semuanya di bawah.
fn top_sand(t: &TimerView, age: f32) -> f32 {
    1.0 - shown(if t.active { t.progress } else { 1.0 }, t.fx, age, ENTER_FLOW)
}

/// Warna timer: oranye, merah di detik-detik terakhir.
pub(super) fn accent(t: &TimerView) -> D2D1_COLOR_F {
    if t.ending { RED } else { ORANGE }
}

/// Pendaran panel: warna timer selama aktif, merah yang memudar sesudah waktunya habis.
pub(super) fn glow(t: &TimerView) -> Option<(D2D1_COLOR_F, f32)> {
    match t.fx {
        _ if t.active => Some((accent(t), 0.7)),
        Some((TimerFx::Done, s)) => Some((RED, 0.7 * (1.0 - smooth(s / DONE_SECS)))),
        _ => None,
    }
}

/// Seberapa tampil tulisan "Waktu habis" dan tulisan biasanya ("Pilih durasi"), 0..1.
pub(super) fn labels(t: &TimerView) -> (f32, f32) {
    match t.fx {
        Some((TimerFx::Done, s)) => {
            let swap = (s - LABEL_HOLD) / LABEL_SWAP;
            (1.0 - smooth(swap), smooth(swap - 1.0))
        }
        _ => (0.0, 1.0),
    }
}

/// Putaran jam pasir (radian, searah jarum jam): setengah lingkaran saat dibalik, goyangan yang
/// reda saat waktu habis.
fn turn(fx: Option<(TimerFx, f32)>) -> f32 {
    match fx {
        Some((TimerFx::Flip, t)) => PI * smooth(t / TURN_SECS),
        Some((TimerFx::Done, t)) => SHAKE * (-t / 0.4).exp() * (TAU * SHAKE_HZ * t).sin(),
        _ => 0.0,
    }
}

/// Setengah lebar kaca sejauh `d` dari leher: menyempit tajam ke leher, tegak di dekat tutup.
fn half_width(d: f32) -> f32 {
    let u = 1.0 - (d / BULB).clamp(0.0, 1.0);
    NECK + (HALF - NECK) * (1.0 - u * u)
}

/// Luas separuh tabung dari leher sampai sejauh `d` (integral `half_width`).
fn area(d: f32) -> f32 {
    let d = d.clamp(0.0, BULB);
    let u = 1.0 - d / BULB;
    NECK * d + (HALF - NECK) * (d - BULB / 3.0 * (1.0 - u * u * u))
}

/// Jarak dari leher yang menampung bagian `fraction` dari luas satu tabung.
fn depth(fraction: f32) -> f32 {
    let goal = fraction.clamp(0.0, 1.0) * area(BULB);
    let (mut low, mut high) = (0.0, BULB);
    for _ in 0..20 {
        let mid = (low + high) / 2.0;
        if area(mid) < goal {
            low = mid;
        } else {
            high = mid;
        }
    }
    (low + high) / 2.0
}

impl Renderer {
    /// Jam pasir berpusat di `gx`, sejajar cincin halaman Timer.
    pub(super) fn hourglass(&self, gx: f32, t: &TimerView, age: f32, a: f32) {
        let Some(inside) = self.path(|sink| glass_outline(sink, gx)) else { return };
        let sand = if self.pal.text.r < 0.5 { SAND_DEEP } else { SAND };
        self.rotated(gx, NECK_Y, turn(t.fx), || {
            self.fill(&inside, self.tint(0.06), a);
            match t.fx {
                Some((TimerFx::Flip, s)) => self.flipping(&inside, gx, s, sand, a),
                _ => self.sand(&inside, gx, t, age, sand, a),
            }
            unsafe {
                self.brush.SetColor(&with_alpha(self.tint(0.3), a));
                self.target.DrawGeometry(&inside, &self.brush, 1.2, &self.round);
            }
            self.frame(gx, a);
        });
    }

    /// Saat waktu habis: cincin penuh merah yang memudar sambil berkilau melebar.
    pub(super) fn time_up(&self, cx: f32, cy: f32, radius: f32, thickness: f32, t: &TimerView, a: f32) {
        let Some((TimerFx::Done, s)) = t.fx else { return };
        let fade = 1.0 - smooth(s / RING_FADE);
        if fade > 0.0 {
            self.circle_outline(cx, cy, radius, thickness, RED, a * fade);
        }
        if s < SPARKLE_SECS {
            let e = ease_out(s / SPARKLE_SECS);
            self.circle_outline(cx, cy, radius + 9.0 * e, 3.0 * (1.0 - e) + 0.5, RED, a * 0.7 * (1.0 - e));
        }
    }

    /// Pasir di kedua tabung, aliran tipis di antaranya, dan butir "+1 mnt".
    fn sand(&self, inside: &ID2D1PathGeometry, gx: f32, t: &TimerView, age: f32, color: D2D1_COLOR_F, a: f32) {
        let top = top_sand(t, age);
        let bottom = 1.0 - top;
        // Mengalir selama berjalan, dan selagi pasir masih menuju tempatnya saat halaman dibuka.
        let settled = 1.0 - if t.active { t.progress } else { 1.0 };
        let flowing = (t.running || top > settled + 0.005) && top > 0.002;
        let top_y = NECK_Y - depth(top * SAND_FILL);
        let bottom_y = NECK_Y + depth(1.0 - bottom * SAND_FILL);
        let mound = MOUND * (bottom * 5.0).min(1.0);
        if top > 0.002 {
            let dip = if flowing { DIP * (top * 5.0).min(1.0) } else { 0.0 };
            self.sand_block(inside, gx, top_y, NECK_Y + 1.0, dip, 7.0, color, a);
        }
        if bottom > 0.002 {
            self.sand_block(inside, gx, bottom_y + mound / 2.0, NECK_Y + BULB + 2.0, -mound, HALF, color, a);
        }
        if flowing {
            let end = if bottom > 0.002 { bottom_y - mound / 2.0 } else { NECK_Y + BULB };
            unsafe {
                self.brush.SetColor(&with_alpha(color, a * 0.9));
                self.target.DrawLine(pt(gx, NECK_Y - 1.0), pt(gx, end), &self.brush, 1.2, &self.round);
            }
        }
        if let Some((TimerFx::Add(_), s)) = t.fx {
            for (dx, delay) in GRAINS {
                let g = (s - delay) / GRAIN_FALL;
                if (0.0..1.0).contains(&g) {
                    self.circle(gx + dx, lerp(NECK_Y - BULB + 2.0, top_y, g * g), 1.0, color, a);
                }
            }
        }
    }

    /// Selagi dibalik: pasir yang tadinya di bawah ikut berputar, lalu turun ke leher. Digambar
    /// sebelum diputar, jadi tabung bawah inilah yang tampil di atas sesudah setengah putaran.
    fn flipping(&self, inside: &ID2D1PathGeometry, gx: f32, s: f32, color: D2D1_COLOR_F, a: f32) {
        let settle = smooth((s - TURN_SECS) / SETTLE_SECS);
        let rest = NECK_Y + depth(1.0 - SAND_FILL) + MOUND / 2.0;
        let full = NECK_Y + depth(SAND_FILL);
        let near = lerp(rest, NECK_Y - 1.0, settle);
        let far = lerp(NECK_Y + BULB + 2.0, full, settle);
        self.sand_block(inside, gx, near, far, -MOUND * (1.0 - settle), HALF, color, a);
    }

    /// Pasir di antara tepi `edge` dan `base`, dipotong mengikuti kaca. Tengah tepinya digeser
    /// `bend` (negatif: gundukan, positif: cekungan) selebar `spread` dari tengah.
    #[allow(clippy::too_many_arguments)]
    fn sand_block(&self, inside: &ID2D1PathGeometry, gx: f32, edge: f32, base: f32, bend: f32, spread: f32, color: D2D1_COLOR_F, a: f32) {
        let wide = HALF + 3.0;
        let Some(block) = self.path(|sink| unsafe {
            sink.BeginFigure(pt(gx - wide, edge), D2D1_FIGURE_BEGIN_FILLED);
            for (x, y) in [(-spread, edge), (0.0, edge + bend), (spread, edge), (wide, edge), (wide, base), (-wide, base)] {
                sink.AddLine(pt(gx + x, y));
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
        }) else {
            return;
        };
        if let Some(body) = self.path(|sink| unsafe {
            let _ = inside.CombineWithGeometry(&block, D2D1_COMBINE_MODE_INTERSECT, None, 0.25, sink);
        }) {
            self.fill(&body, color, a);
        }
    }

    /// Tutup atas dan bawah, dan dua tiang di sisi kaca.
    fn frame(&self, gx: f32, a: f32) {
        let (top, bottom) = (NECK_Y - BULB, NECK_Y + BULB);
        let post = self.tint(0.35);
        for x in [gx - POST, gx + POST] {
            self.rounded(rect(x - 0.8, top, x + 0.8, bottom), 0.8, post, a);
        }
        let cap = self.tint(0.55);
        self.rounded(rect(gx - CAP_HALF, top - CAP, gx + CAP_HALF, top + 0.5), 1.5, cap, a);
        self.rounded(rect(gx - CAP_HALF, bottom - 0.5, gx + CAP_HALF, bottom + CAP), 1.5, cap, a);
    }
}

/// Garis kaca: turun di sisi kiri dari tutup atas lewat leher ke tutup bawah, lalu naik di kanan.
fn glass_outline(sink: &ID2D1GeometrySink, gx: f32) {
    const STEPS: usize = 10;
    let side = |sign: f32| {
        (0..=2 * STEPS).map(move |i| {
            let d = BULB * (i as f32 / STEPS as f32 - 1.0);
            pt(gx + sign * half_width(d.abs()), NECK_Y + d)
        })
    };
    unsafe {
        sink.BeginFigure(pt(gx - HALF, NECK_Y - BULB), D2D1_FIGURE_BEGIN_FILLED);
        side(-1.0).skip(1).for_each(|p| sink.AddLine(p));
        side(1.0).rev().for_each(|p| sink.AddLine(p));
        sink.EndFigure(D2D1_FIGURE_END_CLOSED);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(active: bool, progress: f32, fx: Option<(TimerFx, f32)>) -> TimerView {
        TimerView { running: active, active, clock: String::new(), short: String::new(), progress, total_minutes: 5, ending: false, fx }
    }

    #[test]
    fn sand_level_matches_its_share() {
        assert!(depth(0.0) < 0.001);
        assert!((depth(1.0) - BULB).abs() < 0.001);
        assert!((area(depth(0.3)) / area(BULB) - 0.3).abs() < 0.001);
        // Tabung melebar dari leher: separuh pasir mengisi lebih dari separuh tinggi.
        assert!(depth(0.5) > BULB / 2.0);
    }

    #[test]
    fn sand_rests_below_until_started() {
        assert_eq!(top_sand(&view(false, 0.0, None), f32::MAX), 0.0);
        assert!((top_sand(&view(true, 0.25, None), f32::MAX) - 0.75).abs() < 0.001);
        // Halaman baru dibuka: semua pasir masih di atas, lalu mengalir ke tempatnya.
        assert_eq!(top_sand(&view(false, 0.0, None), 0.0), 1.0);
        // "+1 mnt" menambah pasir di atas pelan-pelan dari kemajuan sebelumnya.
        let added = view(true, 0.2, Some((TimerFx::Add(0.25), 0.0)));
        assert!((top_sand(&added, f32::MAX) - 0.75).abs() < 0.001);
        assert_eq!(ring(&added, f32::MAX), 0.25);
    }

    #[test]
    fn flips_and_shakes_back_to_rest() {
        assert_eq!(turn(None), 0.0);
        assert_eq!(turn(Some((TimerFx::Flip, TURN_SECS))), PI);
        assert!(turn(Some((TimerFx::Done, 0.03))).abs() > 0.1);
        assert!(turn(Some((TimerFx::Done, DONE_SECS))).abs() < 0.002);
    }

    #[test]
    fn time_up_fades_back_to_idle() {
        let done = |s| view(false, 0.0, Some((TimerFx::Done, s)));
        assert_eq!(labels(&done(0.0)), (1.0, 0.0));
        assert_eq!(labels(&done(LABEL_HOLD + LABEL_SWAP)), (0.0, 0.0));
        assert_eq!(labels(&done(DONE_SECS)), (0.0, 1.0));
        assert_eq!(glow(&done(0.0)).unwrap().0.g, RED.g);
        assert_eq!(glow(&done(DONE_SECS)).unwrap().1, 0.0);
        assert!(glow(&view(false, 0.0, None)).is_none());
        let ending = TimerView { ending: true, ..view(true, 0.97, None) };
        assert_eq!(glow(&ending).unwrap().0.g, RED.g);
        assert_eq!(glow(&view(true, 0.5, None)).unwrap().0.g, ORANGE.g);
    }
}
