//! Sedotan "Bersihkan" di halaman Shelf: ubin tersedot satu per satu ke tulisan "Bersihkan",
//! yang terdekat dulu, sambil tulisannya berpusar dan menelan tiap ubin yang masuk. "Urungkan"
//! memuntahkannya kembali ke rak. Selama kursor di atas "Bersihkan", ubin condong ke arahnya.

use std::f32::consts::{PI, TAU};

use crate::widgets::shelf::{Item, VISIBLE};

use super::rack::{BOARD_Y, Look, STILL, TILE_STEP, TILE_WIDTH, bounce};
use super::*;

/// Ubin condong ke arah tulisan: sedikit saat kursor di atas "Bersihkan" (ancang-ancang), lebih
/// kuat sambil bergetar halus selama menunggu giliran disedot.
const LEAN_HOVER: f32 = 3.0 * PI / 180.0;
const LEAN_PULL: f32 = 6.0 * PI / 180.0;
const JITTER: f32 = 0.8 * PI / 180.0;
pub const LEAN_SECS: f32 = 0.15;
/// Ubin tersedot bergantian: lepas dari rak dengan sentakan kecil, lalu melaju makin cepat ke
/// tulisan sambil mengecil, meregang searah lajunya, dan berputar sedikit.
const SUCK_GAP: f32 = 0.09;
const HOP_SECS: f32 = 0.12;
const HOP_RISE: f32 = 6.0;
const FLY_SECS: f32 = 0.36;
const SHRINK: f32 = 0.92;
const STRETCH: f32 = 0.9;
const SPIN: f32 = 0.4;
/// Jarak bayang tipis di belakang ubin yang melaju, dalam bagian jalur.
const TRAIL: f32 = 0.06;
/// Tulisan menelan tiap ubin yang masuk; yang terakhir lebih besar, lalu tulisan dan papan rak
/// memudar.
const GULP_SECS: f32 = 0.22;
const GULP: f32 = 0.12;
const LAST_GULP: f32 = 0.24;
const FADE_SECS: f32 = 0.3;
/// Pusaran di sekitar tulisan: tiga busur yang berputar (radian per detik, panjang busurnya).
const SWIRL_SPEED: f32 = 9.0;
const SWIRL_SWEEP: f32 = 0.9;
/// "Urungkan" memuntahkan ubin kembali, yang terdekat dulu; ubinnya mendarat memantul.
const SPIT_GAP: f32 = 0.08;
const LAND_SECS: f32 = 0.3;
/// Tampilan shelf kosong muncul pelan sesudah sedotan, ikonnya mengangguk sekali.
pub const REVEAL_SECS: f32 = 0.6;
/// Angka di judul bergulir tiap ubin tertelan atau mendarat.
const COUNT_ROLL: f32 = 0.25;
/// Tengah gambar ubin yang berdiri di rak, dan tengah chip "+N".
const THUMB_Y: f32 = BOARD_Y - 25.0;
const CHIP_Y: f32 = BOARD_Y - 14.0;

/// Bagian condong ancang-ancang yang sudah dilalui (0..1) setelah `t` detik.
pub fn lean_ease(t: f32) -> f32 {
    smooth(t / LEAN_SECS)
}

/// Condong ubin yang diam saat kursor di atas "Bersihkan" (`hover` 0..1), dalam radian.
pub(super) fn lean(hover: f32) -> f32 {
    LEAN_HOVER * hover
}

/// Lama sedotan "Bersihkan" untuk shelf berisi `count` file yang digulir `scroll`.
pub fn vacuum_secs(count: usize, scroll: usize) -> f32 {
    Flow::new(0.0, false, count, scroll, (0.0, 0.0)).last() + FADE_SECS
}

/// Lama muntahan "Urungkan" untuk shelf berisi `count` file yang digulir `scroll`.
pub fn spit_secs(count: usize, scroll: usize) -> f32 {
    Flow::new(0.0, true, count, scroll, (0.0, 0.0)).last() + LAND_SECS
}

/// Saat ubin giliran `turn` tertelan tulisan.
fn arrival(turn: usize) -> f32 {
    turn as f32 * SUCK_GAP + HOP_SECS + FLY_SECS
}

/// Saat ubin giliran `turn` mendarat lagi di rak sesudah dimuntahkan.
fn landing(turn: usize) -> f32 {
    turn as f32 * SPIT_GAP + FLY_SECS
}

/// Benjolan 0 → 1 → 0 selama `x` berjalan 0..1.
fn bump(x: f32) -> f32 {
    if (0.0..=1.0).contains(&x) { (PI * x).sin() } else { 0.0 }
}

/// Keadaan sebuah ubin selama sedotan atau muntahan.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Motion {
    /// Di rak: condong `lean` radian ke arah tulisan, geser tegak `dy`, kejelasan nama dan
    /// bayangannya.
    Rack { lean: f32, dy: f32, label: f32, shadow: f32 },
    /// Melayang di titik `p` jalurnya (0 = di rak, 1 = di tulisan) dengan laju `speed` (0..1).
    Flight { p: f32, speed: f32 },
    /// Sudah tertelan, atau belum dimuntahkan.
    Gone,
}

/// Sedotan "Bersihkan" (atau muntahan "Urungkan", `back`) yang sedang berjalan, `age` detik
/// sejak mulai.
pub(super) struct Flow {
    age: f32,
    back: bool,
    /// Ubin yang terlihat di rak, dan file lain yang tidak terlihat.
    visible: usize,
    hidden: usize,
    /// Tengah tulisan "Bersihkan": tujuan ubin yang tersedot, asal ubin yang dimuntahkan.
    pub mouth: (f32, f32),
}

impl Flow {
    pub fn new(age: f32, back: bool, count: usize, scroll: usize, mouth: (f32, f32)) -> Self {
        let visible = count.saturating_sub(scroll).min(VISIBLE);
        Self { age, back, visible, hidden: count - visible, mouth }
    }

    /// Jumlah giliran: ubin terlihat, ditambah chip "+N" (hanya saat disedot) kalau ada file
    /// yang tidak terlihat.
    fn turns(&self) -> usize {
        self.visible + usize::from(self.hidden > 0 && !self.back)
    }

    /// Saat giliran terakhir tertelan, atau mendarat lagi di rak.
    fn last(&self) -> f32 {
        let turn = self.turns().max(1) - 1;
        if self.back { landing(turn) } else { arrival(turn) }
    }

    /// Ubin di slot `slot`: yang terdekat ke tulisan (paling kanan) mendapat giliran pertama.
    /// `hover` = condong ancang-ancang saat sedotan dimulai (0..1).
    pub fn tile(&self, slot: f32, hover: f32) -> Motion {
        let turn = (self.visible as f32 - 1.0 - slot).max(0.0) as usize;
        self.motion(turn, hover)
    }

    fn motion(&self, turn: usize, hover: f32) -> Motion {
        if self.back {
            let t = self.age - turn as f32 * SPIT_GAP;
            if t < 0.0 {
                return Motion::Gone;
            }
            if t < FLY_SECS {
                // Keluar pelan dari tulisan, lalu makin cepat sampai menghantam rak.
                let v = t / FLY_SECS;
                return Motion::Flight { p: 1.0 - v.powf(1.6), speed: v.powf(0.6) };
            }
            let b = t - FLY_SECS;
            return Motion::Rack { lean: 0.0, dy: bounce(b), label: smooth(b / 0.15), shadow: smooth(b / 0.1) };
        }
        let t = self.age - turn as f32 * SUCK_GAP;
        let pull = smooth(self.age / LEAN_SECS);
        let waiting =
            lerp(LEAN_HOVER * hover, LEAN_PULL, pull) + JITTER * pull * (self.age * 47.0 + turn as f32 * 1.9).sin();
        if t < 0.0 {
            return Motion::Rack { lean: waiting, dy: 0.0, label: 1.0, shadow: 1.0 };
        }
        if t < HOP_SECS {
            let u = t / HOP_SECS;
            let still = 1.0 - smooth(u);
            return Motion::Rack { lean: lerp(waiting, LEAN_PULL, smooth(u)), dy: -HOP_RISE * ease_out(u), label: still, shadow: still };
        }
        let u = (t - HOP_SECS) / FLY_SECS;
        if u < 1.0 {
            // Seperti tersedot betulan: lambat di awal, makin cepat menjelang tulisan.
            return Motion::Flight { p: u.powf(2.2), speed: u.powf(1.2) };
        }
        Motion::Gone
    }

    /// Condong dan geser tegak ubin di ujung rak jalur layangnya: saat lepas dari rak
    /// (sedotan) atau saat mendarat (muntahan).
    fn base(&self) -> (f32, f32) {
        if self.back { (0.0, 0.0) } else { (LEAN_PULL, -HOP_RISE) }
    }

    /// Chip "+N": seberapa jelas (muncul setelah ubin terdekat pergi) dan keadaannya.
    fn chip(&self, hover: f32) -> Option<(f32, Motion)> {
        if self.hidden == 0 || self.back {
            return None;
        }
        let shown = smooth((self.age - HOP_SECS - 0.05) / 0.15);
        Some((shown, self.motion(self.visible, hover)))
    }

    /// Kejelasan papan rak: memudar sesudah ubin terakhir tertelan.
    pub fn board(&self) -> f32 {
        if self.back { 1.0 } else { 1.0 - smooth((self.age - self.last()) / FADE_SECS) }
    }

    /// Kejelasan petunjuk di bawah rak: hilang selama sedotan, kembali di akhir muntahan.
    pub fn hint(&self) -> f32 {
        if self.back { smooth((self.age - self.last()) / LAND_SECS) } else { 1.0 - smooth(self.age / 0.2) }
    }

    /// Tulisan "Bersihkan": kekuatan warna aksen dan pusarannya, tambahan skala saat menelan
    /// (atau memuntahkan), dan kejelasannya.
    fn mouth_look(&self) -> (f32, f32, f32) {
        let turns = self.turns();
        let last = self.last();
        if self.back {
            let out = last - FLY_SECS;
            let glow = smooth(self.age / 0.1) * (1.0 - smooth((self.age - out - 0.15) / 0.25));
            let gulp = (0..turns).map(|turn| GULP * bump((self.age - turn as f32 * SPIT_GAP) / GULP_SECS)).fold(0.0, f32::max);
            return (glow, gulp, smooth(self.age / 0.15));
        }
        let glow = smooth(self.age / 0.15) * (1.0 - smooth((self.age - last) / FADE_SECS));
        let gulp = (0..turns)
            .map(|turn| {
                let size = if turn + 1 == turns { LAST_GULP } else { GULP };
                size * bump((self.age - arrival(turn)) / GULP_SECS)
            })
            .fold(0.0, f32::max);
        (glow, gulp, 1.0 - smooth((self.age - last - 0.1) / 0.2))
    }

    /// Sudut putar pusaran: searah jarum jam saat menyedot, sebaliknya saat memuntahkan.
    fn swirl(&self) -> f32 {
        if self.back { -self.age * SWIRL_SPEED } else { self.age * SWIRL_SPEED }
    }

    /// Angka di judul: jumlah file di shelf, bergulir turun tiap ubin tertelan (chip "+N"
    /// sekaligus semuanya) atau naik tiap ubin mendarat lagi.
    pub fn count(&self) -> (usize, Option<Roll>) {
        let events: Vec<(f32, usize)> = if self.back {
            (0..self.visible).map(|turn| (landing(turn), 1)).collect()
        } else {
            (0..self.turns()).map(|turn| (arrival(turn), if turn < self.visible { 1 } else { self.hidden })).collect()
        };
        let start = if self.back { self.hidden } else { self.visible + self.hidden };
        let value = |done: usize| {
            let moved: usize = events[..done].iter().map(|(_, n)| n).sum();
            if self.back { start + moved } else { start - moved }
        };
        let done = events.iter().take_while(|(at, _)| *at <= self.age).count();
        let roll = done.checked_sub(1).map(|k| Roll {
            from: value(k).to_string(),
            t: ((self.age - events[k].0) / COUNT_ROLL).min(1.0),
            up: self.back,
        });
        (value(done), roll)
    }
}

/// Jalur dari rak (`from`) ke tulisan (`to`): naik dulu lalu berbelok masuk. Titik di bagian
/// `p` jalur dan arah lajunya (radian).
fn path(from: (f32, f32), to: (f32, f32), p: f32) -> ((f32, f32), f32) {
    let bend = (lerp(from.0, to.0, 0.3), to.1 + 4.0);
    let q = 1.0 - p;
    let at = |a: f32, b: f32, c: f32| q * q * a + 2.0 * q * p * b + p * p * c;
    let slope = |a: f32, b: f32, c: f32| 2.0 * q * (b - a) + 2.0 * p * (c - b);
    let dir = slope(from.1, bend.1, to.1).atan2(slope(from.0, bend.0, to.0));
    ((at(from.0, bend.0, to.0), at(from.1, bend.1, to.1)), dir)
}

fn rotation(angle: f32) -> Matrix3x2 {
    let (sin, cos) = angle.sin_cos();
    Matrix3x2 { M11: cos, M12: sin, M21: -sin, M22: cos, M31: 0.0, M32: 0.0 }
}

fn scaling(x: f32, y: f32) -> Matrix3x2 {
    Matrix3x2 { M11: x, M12: 0.0, M21: 0.0, M22: y, M31: 0.0, M32: 0.0 }
}

/// Ubin di titik `p` jalurnya: gambar di sekitar `from` dipindah ke titik itu, diperkecil,
/// diregangkan searah lajunya sebanyak `speed` (0..1), dan diputar sedikit.
fn warp(from: (f32, f32), to: (f32, f32), p: f32, speed: f32) -> Matrix3x2 {
    let ((x, y), dir) = path(from, to, p);
    let k = 1.0 - SHRINK * p;
    let stretch = 1.0 + STRETCH * speed;
    translation(-from.0, -from.1)
        * rotation(SPIN * p)
        * rotation(-dir)
        * scaling(k * stretch, k / stretch.sqrt())
        * rotation(dir)
        * translation(x, y)
}

impl Renderer {
    /// Tengah tulisan "Bersihkan" yang rata kanan di `r`, pada ketinggian `cy`.
    pub(super) fn mouth_at(&self, r: f32, cy: f32) -> (f32, f32) {
        (r - self.text_width("Bersihkan", &self.fonts.caption_end) / 2.0, cy)
    }

    /// Judul "Shelf · N" selama sedotan atau muntahan; angkanya bergulir tiap ubin tertelan
    /// atau mendarat.
    pub(super) fn flow_title(&self, flow: &Flow, l: f32, r: f32, a: f32) {
        let font = &self.fonts.header;
        let cy = flow.mouth.1;
        let head = "Shelf · ";
        let x = l + 18.0 + self.text_width(head, font);
        let (count, roll) = flow.count();
        self.text(head, font, rect(l + 18.0, cy - 9.0, r - 180.0, cy + 9.0), self.pal.text, a);
        self.rolling_text(&count.to_string(), roll.as_ref(), font, rect(x, cy - 9.0, x + 48.0, cy + 9.0), self.pal.text, a);
    }

    /// "Urungkan" di kanan atas selama shelf baru dibersihkan; muncul pelan sesudah sedotan.
    pub(super) fn undo_link(&self, s: &Scene, r: f32, cy: f32, a: f32) {
        let shown = s.shelf_fx.emptied.map_or(1.0, |e| smooth(e / 0.3));
        let color = if s.hovered == Some(Hit::ShelfUndo) { self.pal.text } else { self.pal.soft };
        let text = "Urungkan";
        self.text(text, &self.fonts.caption_end, rect(r - 190.0, cy - 8.0, r, cy + 8.0), color, a * shown);
        let width = self.text_width(text, &self.fonts.caption_end);
        self.hit(rect(r - width - 6.0, cy - 11.0, r + 4.0, cy + 11.0), Hit::ShelfUndo);
    }

    /// Satu ubin selama sedotan atau muntahan; kiri ubinnya di `tx`, di slot `slot`.
    pub(super) fn flowing_tile(&self, flow: &Flow, item: &Item, tx: f32, slot: f32, hover: f32, a: f32) {
        let cx = tx + TILE_WIDTH / 2.0;
        match flow.tile(slot, hover) {
            Motion::Rack { lean, dy, label, shadow } => self.shelf_tile(item, tx, Look { dy, label, shadow, lean, ..STILL }, false, a),
            Motion::Flight { p, speed } => {
                let (lean, dy) = flow.base();
                self.flying(flow, (cx, THUMB_Y + dy), p, speed, |alpha| {
                    self.rotated(cx, BOARD_Y, lean, || self.placed(cx, BOARD_Y, 1.0, dy, || self.standing_thumb(item, cx, a * alpha)));
                });
            }
            Motion::Gone => {}
        }
    }

    /// Chip "+N" yang mewakili file yang tidak terlihat: muncul di slot paling kanan setelah
    /// ubinnya pergi, lalu ikut tersedot paling akhir.
    pub(super) fn leftover(&self, flow: &Flow, l: f32, hover: f32, a: f32) {
        let Some((shown, motion)) = flow.chip(hover) else { return };
        let cx = l + (VISIBLE - 1) as f32 * TILE_STEP + TILE_WIDTH / 2.0;
        let label = format!("+{}", flow.hidden);
        let half = (self.text_width(&label, &self.fonts.label_center) + 18.0) / 2.0;
        let paint = |alpha: f32| {
            let area = rect(cx - half, CHIP_Y - 11.0, cx + half, CHIP_Y + 11.0);
            self.chip(area, &label, Some(self.tint(0.14)), self.pal.soft, a * shown * alpha);
        };
        match motion {
            // Muncul sambil membesar sedikit, lalu menunggu giliran bersama ubin lain.
            Motion::Rack { lean, dy, .. } => self.placed(cx, BOARD_Y, lerp(0.7, 1.0, shown), 0.0, || {
                self.rotated(cx, BOARD_Y, lean, || self.placed(cx, BOARD_Y, 1.0, dy, || paint(1.0)));
            }),
            Motion::Flight { p, speed } => {
                let (lean, dy) = flow.base();
                self.flying(flow, (cx, CHIP_Y + dy), p, speed, |alpha| {
                    self.rotated(cx, BOARD_Y, lean, || self.placed(cx, BOARD_Y, 1.0, dy, || paint(alpha)));
                });
            }
            Motion::Gone => {}
        }
    }

    /// Tulisan "Bersihkan" selama sedotan atau muntahan: berwarna aksen dengan pusaran kecil di
    /// sekelilingnya, dan membesar sesaat tiap ubin masuk (atau keluar).
    pub(super) fn mouth(&self, flow: &Flow, r: f32, a: f32) {
        let p = &self.pal;
        let (cx, cy) = flow.mouth;
        let (glow, gulp, shown) = flow.mouth_look();
        self.vortex(flow.mouth, (r - cx + 9.0, 11.0), flow.swirl(), glow * shown, a);
        let color = blend(p.soft, p.accent, glow);
        self.placed(cx, cy, 1.0 + gulp, 0.0, || {
            self.text("Bersihkan", &self.fonts.caption_end, rect(r - 190.0, cy - 8.0, r, cy + 8.0), color, a * shown);
        });
    }

    /// Gambar ubin (`paint` menerima kejelasannya) yang melayang di titik `p` jalur antara
    /// `from` dan tulisan, memudar tepat sebelum masuk; dua bayang tipis mengikutinya saat
    /// melaju cepat.
    fn flying(&self, flow: &Flow, from: (f32, f32), p: f32, speed: f32, paint: impl Fn(f32)) {
        // Bayangnya tertinggal di belakang: lebih dekat ke rak saat disedot, ke tulisan saat
        // dimuntahkan.
        let behind = if flow.back { 1.0 } else { -1.0 };
        for (gap, ghost) in [(2.0, 0.12), (1.0, 0.28), (0.0, 1.0)] {
            let q = p + behind * gap * TRAIL * speed;
            if gap > 0.0 && (speed < 0.2 || !(0.0..=1.0).contains(&q)) {
                continue;
            }
            let strength = if gap > 0.0 { ghost * speed } else { 1.0 };
            let alpha = strength * (1.0 - smooth((q - 0.72) / 0.28));
            if alpha > 0.01 {
                self.transformed(warp(from, flow.mouth, q, speed), || paint(alpha));
            }
        }
    }

    /// Gambar `paint` di bawah transformasi `m`, di atas transformasi yang sedang berlaku.
    fn transformed(&self, m: Matrix3x2, paint: impl FnOnce()) {
        unsafe {
            let mut current = Matrix3x2::default();
            self.target.GetTransform(&mut current);
            self.target.SetTransform(&(m * current));
            paint();
            self.target.SetTransform(&current);
        }
    }

    /// Pusaran: tiga busur tipis mengelilingi `center` dengan jari-jari `radius` (mendatar,
    /// tegak), diputar `turn` radian.
    fn vortex(&self, center: (f32, f32), radius: (f32, f32), turn: f32, strength: f32, a: f32) {
        if strength < 0.01 {
            return;
        }
        let ((cx, cy), (rx, ry)) = (center, radius);
        let at = |angle: f32| pt(cx + rx * angle.cos(), cy + ry * angle.sin());
        for i in 0..3 {
            let start = turn + i as f32 * TAU / 3.0;
            unsafe {
                let Ok(geometry) = self.factory.CreatePathGeometry() else { return };
                let Ok(sink) = geometry.Open() else { return };
                sink.BeginFigure(at(start), D2D1_FIGURE_BEGIN_HOLLOW);
                sink.AddArc(&D2D1_ARC_SEGMENT {
                    point: at(start + SWIRL_SWEEP),
                    size: D2D_SIZE_F { width: rx, height: ry },
                    rotationAngle: 0.0,
                    sweepDirection: D2D1_SWEEP_DIRECTION_CLOCKWISE,
                    arcSize: D2D1_ARC_SIZE_SMALL,
                });
                sink.EndFigure(D2D1_FIGURE_END_OPEN);
                if sink.Close().is_ok() {
                    self.brush.SetColor(&with_alpha(self.pal.accent, 0.7 * strength * a));
                    self.target.DrawGeometry(&geometry, &self.brush, 1.6, &self.round);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOUTH: (f32, f32) = (380.0, 22.0);

    fn suck(age: f32, count: usize, scroll: usize) -> Flow {
        Flow::new(age, false, count, scroll, MOUTH)
    }

    fn spit(age: f32, count: usize) -> Flow {
        Flow::new(age, true, count, 0, MOUTH)
    }

    #[test]
    fn nearest_tile_goes_first_and_the_title_counts_down() {
        let early = suck(0.2, 5, 0);
        assert!(matches!(early.tile(4.0, 1.0), Motion::Flight { .. }));
        assert!(matches!(early.tile(0.0, 1.0), Motion::Rack { label, .. } if label == 1.0));
        // Ancang-ancang berlanjut tanpa lompatan saat sedotan dimulai.
        assert_eq!(suck(0.0, 5, 0).tile(2.0, 1.0), Motion::Rack { lean: LEAN_HOVER, dy: 0.0, label: 1.0, shadow: 1.0 });
        assert!(matches!(suck(0.0, 5, 0).count(), (5, None)));
        let (left, roll) = suck(arrival(0) + 0.01, 5, 0).count();
        assert_eq!(left, 4);
        assert!(roll.is_some_and(|r| r.from == "5" && !r.up));
        let end = vacuum_secs(5, 0);
        let done = suck(end, 5, 0);
        assert_eq!(done.count().0, 0);
        assert!((0..5).all(|slot| done.tile(slot as f32, 1.0) == Motion::Gone));
        assert!(done.board() < 0.001);
        assert!(done.mouth_look().2 < 0.001);
        assert!((0.9..1.3).contains(&end), "sekitar satu detik: {end}");
    }

    #[test]
    fn hidden_files_ride_a_chip() {
        let flow = suck(0.0, 12, 3);
        assert_eq!((flow.visible, flow.hidden, flow.turns()), (5, 7, 6));
        assert!(vacuum_secs(12, 3) > vacuum_secs(5, 0));
        assert!(suck(0.0, 3, 0).chip(1.0).is_none());
        // Chip muncul setelah ubin terdekat pergi, lalu tertelan paling akhir bersama semua sisanya.
        assert_eq!(flow.chip(1.0).map(|c| c.0), Some(0.0));
        assert_eq!(suck(0.5, 12, 3).chip(1.0).map(|c| c.0), Some(1.0));
        assert_eq!(suck(arrival(4) + 0.01, 12, 3).count().0, 7);
        assert_eq!(suck(arrival(5) + 0.01, 12, 3).count().0, 0);
    }

    #[test]
    fn undo_spits_tiles_back_with_a_bounce() {
        let start = spit(0.0, 5);
        assert_eq!(start.tile(4.0, 0.0), Motion::Flight { p: 1.0, speed: 0.0 });
        assert_eq!(start.tile(0.0, 0.0), Motion::Gone);
        assert!(matches!(start.count(), (0, None)));
        let end = spit(spit_secs(5, 0), 5);
        for slot in 0..5 {
            let Motion::Rack { lean, dy, label, .. } = end.tile(slot as f32, 0.0) else { panic!("ubin {slot} belum mendarat") };
            assert_eq!((lean, label), (0.0, 1.0));
            assert!(dy.abs() < 0.3);
        }
        assert_eq!(end.count().0, 5);
        assert!(end.hint() > 0.999);
        // File yang tidak terlihat langsung kembali; yang terlihat dihitung saat mendarat.
        assert_eq!(spit(0.0, 9).count().0, 4);
    }

    #[test]
    fn flight_starts_where_the_tile_stood() {
        let from = (100.0, 62.0);
        let m = warp(from, MOUTH, 0.0, 0.0);
        let near = |a: f32, b: f32| (a - b).abs() < 0.001;
        assert!(near(m.M11, 1.0) && near(m.M22, 1.0) && near(m.M12, 0.0) && near(m.M31, 0.0) && near(m.M32, 0.0));
        let ((x, y), _) = path(from, MOUTH, 1.0);
        assert!(near(x, MOUTH.0) && near(y, MOUTH.1));
        // Ubin naik lebih curam dulu sebelum berbelok ke tulisan.
        let (_, dir) = path(from, MOUTH, 0.0);
        assert!(dir < (MOUTH.1 - from.1).atan2(MOUTH.0 - from.0));
    }

    #[test]
    fn mouth_gulps_each_tile() {
        let flow = suck(arrival(1) + GULP_SECS / 2.0, 5, 0);
        let (glow, gulp, shown) = flow.mouth_look();
        assert!((gulp - GULP).abs() < 0.001 && glow == 1.0 && shown == 1.0);
        let last = suck(arrival(4) + GULP_SECS / 2.0, 5, 0).mouth_look().1;
        assert!((last - LAST_GULP).abs() < 0.001);
        assert_eq!(bump(-0.1), 0.0);
        assert_eq!(lean_ease(LEAN_SECS), 1.0);
    }
}
