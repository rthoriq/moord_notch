//! Suasana halaman Tugas, daftar centang yang memuaskan: lingkaran yang terisi, tanda centang
//! yang tergambar seperti ditulis tangan, coretan yang menyapu teks, baris yang meluncur ke tempat
//! barunya, cincin kemajuan hari ini, dan konfeti saat semuanya selesai. Di luar animasi tidak
//! ada gerak tambahan.

use std::f32::consts::PI;

use super::*;

/// Animasi masuk: baris muncul bergantian dari atas, centang tugas selesai tergambar ulang, dan
/// cincin kemajuan terisi.
pub const ENTER_SECS: f32 = 0.8;
const ROW_GAP: f32 = 0.07;
const ROW_IN: f32 = 0.25;
const ROW_DROP: f32 = 6.0;
const RING_SWEEP: f32 = 0.6;
/// Baris pertama daftar dan jarak antarbaris.
pub(super) const ROW_TOP: f32 = 46.0;
pub(super) const ROW_STEP: f32 = 24.0;
/// Tugas baru: celahnya terbuka dulu, lalu tugasnya naik ke celah itu sambil muncul. Tugas di
/// baris terakhir naik dari kotak tambah di bawah daftar; di atas tugas lain cukup naik pendek.
const NEW_RISE: (f32, f32) = (0.2, 0.5);
const NEW_DROP: f32 = 10.0;
const ADD_FROM: f32 = 128.0;
/// Dicentang: lingkaran terisi, tanda centang tergambar, lalu coretan menyapu teks.
const FILL: (f32, f32) = (0.0, 0.18);
const TICK: (f32, f32) = (0.08, 0.3);
const STRIKE: (f32, f32) = (0.15, 0.4);
pub(super) const UNDO: f32 = 0.2;
/// Lama centang tergambar sampai coretannya selesai.
pub(super) const CHECK_SECS: f32 = STRIKE.1;
/// Kapan baris meluncur ke tempat barunya, per jenis perubahan.
const CHECK_MOVE: (f32, f32) = (0.4, 0.75);
const UNCHECK_MOVE: (f32, f32) = (0.12, 0.47);
const ADD_MOVE: (f32, f32) = (0.0, 0.25);
const REMOVE_MOVE: (f32, f32) = (0.12, 0.45);
const MOVE_SECS: f32 = 0.33;
/// Tugas yang dihapus bergeser ke kanan sambil memudar; "Hapus selesai" bergantian.
const GONE_SECS: f32 = 0.25;
const GONE_SHIFT: f32 = 24.0;
const GONE_GAP: f32 = 0.05;
/// Semua selesai: konfeti meletup dari centangnya, dan "Semua selesai" berkilau sekali.
const CONFETTI_SECS: f32 = 1.3;
const CONFETTI_FROM: f32 = 0.12;
const GRAVITY: f32 = 300.0;
const SHINE: (f32, f32) = (0.35, 1.1);
/// Tiap konfeti: arah (derajat, negatif = ke atas), kecepatan, warna, dan setengah panjangnya.
const CONFETTI: [(f32, f32, usize, f32); 14] = [
    (-150.0, 100.0, 0, 3.2),
    (-135.0, 143.8, 1, 2.8),
    (-120.0, 118.8, 2, 3.4),
    (-108.0, 162.5, 3, 2.6),
    (-96.0, 131.2, 4, 3.1),
    (-84.0, 156.2, 0, 2.8),
    (-74.0, 112.5, 1, 3.3),
    (-64.0, 168.8, 2, 2.7),
    (-54.0, 137.5, 3, 3.2),
    (-44.0, 175.0, 4, 2.9),
    (-35.0, 125.0, 0, 2.6),
    (-26.0, 156.2, 1, 3.1),
    (-18.0, 106.2, 2, 2.8),
    (-10.0, 143.8, 3, 3.3),
];
const CONFETTI_COLORS: [D2D1_COLOR_F; 5] =
    [rgb(0x30D158, 1.0), rgb(0xFF9F0A, 1.0), rgb(0x0A84FF, 1.0), rgb(0xFF375F, 1.0), rgb(0xFFD60A, 1.0)];

/// Bagian `t` di antara `from` dan `to`, 0..1.
pub(super) fn phase(t: f32, (from, to): (f32, f32)) -> f32 {
    ((t - from) / (to - from)).clamp(0.0, 1.0)
}

/// Kapan baris meluncur untuk perubahan ini.
fn move_span(fx: &TaskFx) -> (f32, f32) {
    match fx.change {
        TaskChange::Check { .. } => CHECK_MOVE,
        TaskChange::Uncheck(_) => UNCHECK_MOVE,
        TaskChange::Add => ADD_MOVE,
        TaskChange::Remove => REMOVE_MOVE,
        // Tugas selesai yang terakhir disapu keluar dulu, baru sisanya merapat.
        TaskChange::ClearDone => {
            let start = REMOVE_MOVE.0 + GONE_GAP * fx.gone.len().saturating_sub(1).min(4) as f32;
            (start, start + MOVE_SECS)
        }
    }
}

/// Lama animasi sebuah perubahan di halaman Tugas.
pub fn fx_secs(fx: &TaskFx) -> f32 {
    match fx.change {
        TaskChange::Check { last: true, .. } => CONFETTI_SECS,
        TaskChange::Add => NEW_RISE.1,
        _ => move_span(fx).1,
    }
}

/// Tinggi tengah baris tampil ke-`row` (0 = baris teratas yang terlihat).
pub(super) fn row_y(row: f32) -> f32 {
    ROW_TOP + row * ROW_STEP
}

/// Tinggi baris tugas `index` yang kini di urutan `pos`: meluncur dari baris lamanya, atau naik
/// ke tempatnya kalau tugasnya baru (dari kotak tambah kalau ia baris `last`).
pub(super) fn slide(index: usize, pos: usize, last: bool, scroll: usize, fx: Option<(&TaskFx, f32)>) -> f32 {
    let now = row_y(pos as f32 - scroll as f32);
    let Some((fx, t)) = fx else { return now };
    let from = match fx.rows.get(index) {
        Some(Some(row)) => row_y(*row as f32 - fx.scroll as f32),
        Some(None) => {
            let below = if last { ADD_FROM.max(now + NEW_DROP) } else { now + NEW_DROP };
            return lerp(below, now, ease_out(phase(t, NEW_RISE)));
        }
        None => return now,
    };
    lerp(from, now, smooth(phase(t, move_span(fx))))
}

/// Kejelasan dan geser turun baris ke-`row` yang terlihat: muncul bergantian saat halaman dibuka,
/// atau memudar masuk kalau tugasnya baru ditambahkan.
pub(super) fn arrive(index: usize, row: f32, fx: Option<(&TaskFx, f32)>, age: f32) -> (f32, f32) {
    if let Some((fx, t)) = fx
        && fx.rows.get(index) == Some(&None)
    {
        return (smooth(phase(t, (NEW_RISE.0, NEW_RISE.0 + 0.2))), 0.0);
    }
    let shown = ease_out((age - row.max(0.0) * ROW_GAP) / ROW_IN);
    (shown, -ROW_DROP * (1.0 - shown))
}

/// Tugas yang baru dicentang atau dibatalkan, dan seberapa ia terangkat seperti kartu selagi
/// meluncur melewati baris lain (0..1), supaya teksnya tidak saling menimpa.
pub(super) fn lifted(fx: Option<(&TaskFx, f32)>) -> Option<(usize, f32)> {
    let (fx, t) = fx?;
    let index = match fx.change {
        TaskChange::Check { index, .. } | TaskChange::Uncheck(index) => index,
        _ => return None,
    };
    let (from, to) = move_span(fx);
    let lift = smooth(phase(t, (from - 0.06, from + 0.06))) * (1.0 - smooth(phase(t, (to - 0.08, to))));
    Some((index, lift))
}

/// Tampilan centang sebuah baris, masing-masing 0..1.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Mark {
    pub fill: f32,
    pub tick: f32,
    pub strike: f32,
}

impl Mark {
    pub(super) fn flat(done: bool) -> Self {
        let v = if done { 1.0 } else { 0.0 };
        Mark { fill: v, tick: v, strike: v }
    }
}

/// Centang yang sedang tergambar, `t` detik sejak dicentang: lingkaran terisi, tanda centang
/// tergambar, lalu garis coret menyapu teksnya. Juga dipakai agenda di Beranda.
pub(super) fn checking(t: f32) -> Mark {
    Mark { fill: ease_out(phase(t, FILL)), tick: ease_out(phase(t, TICK)), strike: smooth(phase(t, STRIKE)) }
}

/// Centang yang dibatalkan, `t` detik sejak dibatalkan: semuanya terhapus bersamaan.
pub(super) fn unchecking(t: f32) -> Mark {
    let back = 1.0 - smooth(t / UNDO);
    Mark { fill: back, tick: back, strike: back }
}

/// Centang yang sudah selesai digambar ulang saat halaman dibuka, mulai `start` detik.
pub(super) fn redrawn(age: f32, start: f32) -> Mark {
    Mark { fill: 1.0, tick: ease_out((age - start) / 0.25), strike: smooth((age - start - 0.1) / 0.3) }
}

/// Centang baris tugas `index` (baris tampil ke-`row`) yang sudah atau belum `done`.
pub(super) fn mark(index: usize, done: bool, row: f32, fx: Option<(&TaskFx, f32)>, age: f32) -> Mark {
    match fx {
        Some((TaskFx { change: TaskChange::Check { index: i, .. }, .. }, t)) if *i == index => checking(t),
        Some((TaskFx { change: TaskChange::Uncheck(i), .. }, t)) if *i == index => unchecking(t),
        // Halaman baru dibuka: centang tugas yang sudah selesai tergambar ulang.
        _ if done && age < ENTER_SECS => redrawn(age, 0.15 + row.max(0.0) * ROW_GAP),
        _ => Mark::flat(done),
    }
}

/// Tugas yang baru dihapus: teks, sudah selesai, tinggi barisnya, geser ke kanan, dan kejelasan.
pub(super) fn gone(fx: Option<(&TaskFx, f32)>) -> Vec<(&str, bool, f32, f32, f32)> {
    let Some((fx, t)) = fx else { return Vec::new() };
    fx.gone
        .iter()
        .enumerate()
        .map(|(i, (text, done, row))| {
            let start = i.min(4) as f32 * GONE_GAP;
            let g = phase(t, (start, start + GONE_SECS));
            (text.as_str(), *done, row_y(*row as f32 - fx.scroll as f32), GONE_SHIFT * ease_out(g), 1.0 - smooth(g))
        })
        .collect()
}

/// Kejelasan tulisan "Belum ada tugas" sesudah tugas terakhir dihapus.
pub(super) fn settled(fx: Option<(&TaskFx, f32)>) -> f32 {
    fx.map_or(1.0, |(fx, t)| smooth(phase(t, move_span(fx))))
}

/// Bagian tugas yang selesai yang tampil di cincin kemajuan: bergeser dari sebelum berubah, atau
/// terisi dari kosong saat halaman dibuka.
pub(super) fn progress(tasks: &Tasks, fx: Option<(&TaskFx, f32)>, age: f32) -> f32 {
    let now = tasks.done_share();
    match fx {
        Some((fx, t)) => lerp(fx.progress, now, ease_out(t / 0.5)),
        None => now * ease_out(age / RING_SWEEP),
    }
}

/// Pendaran hijau yang makin terang makin banyak tugas selesai.
pub(super) fn glow(s: &Scene) -> Option<(D2D1_COLOR_F, f32)> {
    let done = progress(s.tasks, s.task_fx, entering(s, Page::Tasks));
    (done > 0.0).then_some((GREEN, 0.55 * done))
}

impl Renderer {
    /// Satu baris tugas setinggi `y`: lingkaran centang dan teksnya, dengan coretan sepanjang
    /// `mark.strike`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn task_row(&self, l: f32, r: f32, y: f32, text: &str, mark: Mark, hovered: bool, a: f32) {
        let p = &self.pal;
        let cx = l + 9.0;
        if mark.fill < 1.0 {
            self.circle_outline(cx, y, 7.0, 1.5, if hovered { p.text } else { p.muted }, a * (1.0 - mark.fill));
        }
        if mark.fill > 0.0 {
            // Sedikit membesar di tengah jalan, lalu kembali: "pop".
            self.circle(cx, y, 8.0 * mark.fill + 2.0 * (PI * mark.fill).sin(), p.accent, a);
        }
        if mark.tick > 0.0 {
            self.tick(cx, y, 1.0, mark.tick, a);
        }
        let area = rect(l + 26.0, y - 10.0, r - 26.0, y + 10.0);
        self.striking(text, &self.fonts.body, area, blend(p.text, p.muted, mark.strike), mark.strike, a);
    }

    /// Teks yang tercoret dari kiri sepanjang `strike` (0..1).
    pub(super) fn striking(&self, text: &str, font: &IDWriteTextFormat, area: D2D_RECT_F, color: D2D1_COLOR_F, strike: f32, a: f32) {
        if strike <= 0.0 {
            self.text(text, font, area, color, a);
        } else if strike >= 1.0 {
            self.text_struck(text, font, area, color, a);
        } else {
            // Bagian kiri sudah tercoret, bagian kanan belum.
            let width = self.text_width(text, font).min(area.right - area.left);
            let split = area.left + width * strike;
            self.push_clip(rect(area.left - 2.0, area.top, split, area.bottom));
            self.text_struck(text, font, area, color, a);
            self.pop_clip();
            self.push_clip(rect(split, area.top, area.right, area.bottom));
            self.text(text, font, area, color, a);
            self.pop_clip();
        }
    }

    /// Tanda centang `size` kali ukuran centang tugas, tergambar sepanjang `part` (0..1) dari
    /// ujung kiri.
    pub(super) fn tick(&self, cx: f32, y: f32, size: f32, part: f32, a: f32) {
        let at = |dx: f32, dy: f32| pt(cx + dx * size, y + dy * size);
        let points = [at(-3.6, 0.1), at(-1.1, 2.7), at(3.7, -2.6)];
        let length = |p: Vector2, q: Vector2| (q.X - p.X).hypot(q.Y - p.Y);
        let (first, second) = (length(points[0], points[1]), length(points[1], points[2]));
        let drawn = part.clamp(0.0, 1.0) * (first + second);
        let along = |p: Vector2, q: Vector2, t: f32| pt(lerp(p.X, q.X, t), lerp(p.Y, q.Y, t));
        let Some(path) = self.path(|sink| unsafe {
            sink.BeginFigure(points[0], D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddLine(along(points[0], points[1], (drawn / first).min(1.0)));
            if drawn > first {
                sink.AddLine(along(points[1], points[2], (drawn - first) / second));
            }
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
        }) else {
            return;
        };
        unsafe {
            self.brush.SetColor(&with_alpha(WHITE, a));
            self.target.DrawGeometry(&path, &self.brush, 1.7 * size.max(0.8), &self.round);
        }
    }

    /// Cincin kecil di kiri tulisan kepala halaman (yang rata kanan di `r`): bagian tugas yang
    /// selesai.
    pub(super) fn progress_ring(&self, r: f32, y: f32, right: &str, progress: f32, a: f32) {
        let x = r - self.text_width(right, &self.fonts.caption_end) - 12.0;
        self.ring(x, y, 5.0, 2.0, progress, GREEN, a);
    }

    /// Tulisan "Semua selesai" yang berkilau hijau sekali sesudah tugas terakhir dicentang.
    pub(super) fn shine(&self, r: f32, y: f32, right: &str, fx: Option<(&TaskFx, f32)>, a: f32) {
        let Some((TaskFx { change: TaskChange::Check { last: true, .. }, .. }, t)) = fx else { return };
        let glint = (PI * phase(t, SHINE)).sin();
        if glint > 0.0 {
            self.text(right, &self.fonts.caption_end, rect(r - 190.0, y - 8.0, r, y + 8.0), GREEN, a * glint);
        }
    }

    /// Konfeti yang meletup dari centang di (`x`, `y`) saat tugas terakhir dicentang.
    pub(super) fn confetti(&self, x: f32, y: f32, fx: Option<(&TaskFx, f32)>, a: f32) {
        let Some((TaskFx { change: TaskChange::Check { last: true, .. }, .. }, t)) = fx else { return };
        let u = t - CONFETTI_FROM;
        if u <= 0.0 {
            return;
        }
        let fade = 1.0 - smooth((t - (CONFETTI_SECS - 0.4)) / 0.4);
        for (i, (angle, speed, color, half)) in CONFETTI.into_iter().enumerate() {
            let (sin, cos) = angle.to_radians().sin_cos();
            let px = x + cos * speed * u;
            let py = y + sin * speed * u + 0.5 * GRAVITY * u * u;
            let spin = u * (5.0 + i as f32 * 0.7) * if i % 2 == 0 { 1.0 } else { -1.0 };
            self.rotated(px, py, spin, || {
                self.rounded(rect(px - half, py - half * 0.45, px + half, py + half * 0.45), 0.6, CONFETTI_COLORS[color], a * fade);
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(change: TaskChange, rows: Vec<Option<usize>>) -> TaskFx {
        TaskFx { change, rows, scroll: 0, gone: Vec::new(), progress: 0.0 }
    }

    #[test]
    fn checked_row_ticks_then_slides_down() {
        // Tugas 0 dicentang: dari baris 0 turun ke baris 2 (setelah dua tugas yang belum selesai).
        let checked = fx(TaskChange::Check { index: 0, last: false }, vec![Some(0), Some(1), Some(2)]);
        assert_eq!(slide(0, 2, true, 0, Some((&checked, CHECK_MOVE.0))), row_y(0.0));
        assert_eq!(slide(0, 2, true, 0, Some((&checked, CHECK_MOVE.1))), row_y(2.0));
        assert_eq!(mark(0, true, 0.0, Some((&checked, 0.0)), f32::MAX), Mark::flat(false));
        assert_eq!(mark(0, true, 0.0, Some((&checked, STRIKE.1)), f32::MAX), Mark::flat(true));
        // Tugas lain tidak ikut dicentang ulang.
        assert_eq!(mark(1, true, 1.0, Some((&checked, 0.0)), f32::MAX), Mark::flat(true));
        assert_eq!(fx_secs(&checked), CHECK_MOVE.1);
        // Terangkat hanya selagi meluncur.
        assert_eq!(lifted(Some((&checked, 0.0))), Some((0, 0.0)));
        assert_eq!(lifted(Some((&checked, 0.58))), Some((0, 1.0)));
        assert_eq!(lifted(Some((&checked, CHECK_MOVE.1))), Some((0, 0.0)));
    }

    #[test]
    fn new_task_rises_from_the_input() {
        let added = fx(TaskChange::Add, vec![Some(0), None]);
        assert_eq!(slide(1, 1, false, 0, Some((&added, 0.0))), row_y(1.0) + NEW_DROP);
        assert_eq!(slide(1, 1, true, 0, Some((&added, 0.0))), ADD_FROM);
        assert_eq!(slide(1, 1, true, 0, Some((&added, NEW_RISE.1))), row_y(1.0));
        assert_eq!(arrive(1, 1.0, Some((&added, 0.0)), f32::MAX).0, 0.0);
        assert_eq!(fx_secs(&added), NEW_RISE.1);
        assert!(lifted(Some((&added, 0.2))).is_none());
        assert_eq!(arrive(0, 0.0, Some((&added, 0.0)), f32::MAX), (1.0, 0.0));
    }

    #[test]
    fn removed_tasks_fade_out_in_turn() {
        let cleared = TaskFx { gone: vec![("a".into(), true, 1), ("b".into(), true, 2)], ..fx(TaskChange::ClearDone, vec![Some(0)]) };
        let start = gone(Some((&cleared, 0.0)));
        assert_eq!((start[0].4, start[1].4), (1.0, 1.0));
        let later = gone(Some((&cleared, GONE_SECS)));
        assert_eq!(later[0].4, 0.0);
        assert!(later[1].4 > 0.0);
        assert!(fx_secs(&cleared) > REMOVE_MOVE.1);
    }

    #[test]
    fn ring_and_entry_settle() {
        let mut tasks = Tasks::default();
        tasks.add("a");
        tasks.add("b");
        tasks.toggle(0);
        assert_eq!(progress(&tasks, None, f32::MAX), 0.5);
        assert_eq!(progress(&tasks, None, 0.0), 0.0);
        let checked = TaskFx { progress: 0.0, ..fx(TaskChange::Check { index: 0, last: false }, vec![Some(0), Some(1)]) };
        assert_eq!(progress(&tasks, Some((&checked, 0.0)), f32::MAX), 0.0);
        assert_eq!(arrive(0, 2.0, None, f32::MAX), (1.0, 0.0));
        assert_eq!(arrive(0, 2.0, None, 0.0).0, 0.0);
        let last = fx(TaskChange::Check { index: 0, last: true }, vec![Some(0)]);
        assert_eq!(fx_secs(&last), CONFETTI_SECS);
    }
}
