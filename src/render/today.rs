//! Suasana Beranda, bertema "hari ini": pendaran sesuai waktu hari, angka jam yang bergulir saat
//! menit berganti, kalender yang meluncur, agenda yang dicentang seperti tugas, sampul dan judul
//! yang berganti halus saat lagu berganti, dan animasi masuknya. Di luar animasi tidak ada gerak
//! tambahan.

use std::f32::consts::PI;

use crate::widgets::agenda::VISIBLE;

use super::checklist::{Mark, checking, phase, redrawn, unchecking};
use super::*;

/// Animasi masuk: sapaan, jam, dan tanggal (atau judul lagu) naik bergantian; hari-hari kalender
/// muncul bergantian dari kiri dan lingkaran hari ini membesar.
pub const ENTER_SECS: f32 = 0.8;
const ENTER_FROM: f32 = 0.05;
const ENTER_GAP: f32 = 0.08;
const ENTER_IN: f32 = 0.3;
const ENTER_RISE: f32 = 8.0;
const DAY_FROM: f32 = 0.12;
const DAY_GAP: f32 = 0.05;
const DAY_IN: f32 = 0.28;
const DAY_RISE: f32 = 6.0;
const BAR_SWEEP: (f32, f32) = (0.15, 0.65);
const COVER_IN: f32 = 0.35;
/// Menit berganti: angka yang berubah bergulir naik, bergantian dari kanan seperti jam yang
/// dibalik. Paling banyak empat angka berubah sekaligus (misal 19:59 → 20:00).
const ROLL_SECS: f32 = 0.4;
const ROLL_GAP: f32 = 0.07;
pub const CLOCK_ROLL_SECS: f32 = ROLL_SECS + 3.0 * ROLL_GAP;
/// Bagian atas dan bawah kotak teks jam yang tidak ikut menampilkan angka yang bergulir.
const ROLL_BAND: (f32, f32) = (0.22, 0.14);
/// Hari lain dipilih: deretan hari meluncur dan agendanya bergeser masuk.
pub const SLIDE_SECS: f32 = 0.35;
/// Hari yang terlihat, dan hari yang disiapkan di tiap sisi hari terpilih supaya ada yang
/// meluncur masuk.
const DAYS: f32 = 5.0;
pub const DAY_SPAN: i32 = 5;
const EVENTS_SHIFT: f32 = 20.0;
/// Baris agenda: tinggi baris tampil pertama dan jaraknya.
const AGENDA_TOP: f32 = 92.0;
const AGENDA_STEP: f32 = 16.0;
/// Agenda baru naik ke barisnya (dari kotak tambah kalau ia baris terbawah); agenda yang dihapus
/// memudar ke kanan, lalu sisanya menutup celah.
const ADD_FROM: f32 = 128.0;
const NEW_RISE: (f32, f32) = (0.2, 0.5);
const NEW_DROP: f32 = 8.0;
const ADD_MOVE: (f32, f32) = (0.0, 0.25);
const REMOVE_MOVE: (f32, f32) = (0.12, 0.45);
const GONE_SECS: f32 = 0.25;
const GONE_SHIFT: f32 = 24.0;
/// Lagu berganti: judul lama keluar, judul baru masuk dari arah lompatannya, bar progres meluncur.
pub const TRACK_SECS: f32 = 0.45;
const TRACK_OUT: (f32, f32) = (0.0, 0.25);
const TRACK_IN: (f32, f32) = (0.12, 0.45);
const TRACK_SHIFT: f32 = 24.0;
/// Sampul berganti: yang lama memudar, yang baru masuk sambil membesar sedikit.
pub const ART_SECS: f32 = 0.4;

/// Pendaran Beranda saat tidak ada musik, mengikuti sapaannya (lihat `clock::greeting`): pagi
/// keemasan, siang biru langit, sore jingga, malam nila.
fn daylight(hour: u32) -> (D2D1_COLOR_F, f32) {
    match hour {
        4..=10 => (rgb(0xFFB340, 1.0), 0.55),
        11..=14 => (rgb(0x64D2FF, 1.0), 0.5),
        15..=17 => (rgb(0xFF8A4C, 1.0), 0.6),
        _ => (rgb(0x5E5CE6, 1.0), 0.55),
    }
}

/// Pendaran Beranda: warna sampul selama ada musik, selain itu warna waktu hari yang menyala
/// pelan saat halaman dibuka.
pub(super) fn glow(s: &Scene) -> Option<(D2D1_COLOR_F, f32)> {
    match s.media {
        Some(m) => m.accent.map(|c| (rgb(c, 1.0), 1.0)),
        None => {
            let (color, strength) = daylight(s.hour);
            Some((color, strength * smooth(entering(s, Page::Home) / 0.6)))
        }
    }
}

/// Kejelasan dan geser turun baris isi ke-`line` (0 = paling atas) selama animasi masuk.
pub(super) fn rise(age: f32, line: usize) -> (f32, f32) {
    let shown = ease_out((age - ENTER_FROM - line as f32 * ENTER_GAP) / ENTER_IN);
    (shown, ENTER_RISE * (1.0 - shown))
}

/// Kejelasan, geser turun, dan skala lingkaran hari di kolom `col` (0 = paling kiri) saat halaman
/// dibuka: muncul bergantian dari kiri, lingkarannya membesar sedikit melewati ukurannya.
fn day_in(age: f32, col: f32) -> (f32, f32, f32) {
    let u = ((age - DAY_FROM - col.max(0.0) * DAY_GAP) / DAY_IN).clamp(0.0, 1.0);
    let shown = ease_out(u);
    let (c1, v) = (1.70158, u - 1.0);
    (shown, DAY_RISE * (1.0 - shown), 1.0 + (c1 + 1.0) * v * v * v + c1 * v * v)
}

/// Kejelasan hari yang `pos` kolom dari tengah: hari di tepi deretan memudar saat meluncur.
fn edge(pos: f32) -> f32 {
    (3.0 - pos.abs()).clamp(0.0, 1.0)
}

/// Bagian deretan hari yang sudah meluncur (0..1) setelah `t` detik.
pub fn slide_ease(t: f32) -> f32 {
    smooth(t / SLIDE_SECS)
}

/// Geser mendatar dan kejelasan isi lama (keluar) dan isi baru (masuk) yang bertukar ke arah
/// `dir`, dengan kemajuan keluar `out` dan masuk `into` (0..1).
fn swap(dir: f32, out: f32, into: f32, distance: f32) -> ((f32, f32), (f32, f32)) {
    ((-dir * distance * ease_out(out), 1.0 - smooth(out)), (dir * distance * (1.0 - ease_out(into)), smooth(into)))
}

/// Agenda hari sebelumnya keluar dan agenda hari terpilih masuk selama peralihan hari.
fn day_swap(shift: Option<&DayShift>) -> ((f32, f32), (f32, f32)) {
    match shift {
        Some(shift) => swap(shift.dir, phase(shift.t, (0.0, 0.45)), phase(shift.t, (0.45, 1.0)), EVENTS_SHIFT),
        None => ((0.0, 0.0), (0.0, 1.0)),
    }
}

/// Judul lagu lama keluar dan judul baru masuk, `t` detik sejak lagu berganti.
pub(super) fn track_swap(dir: f32, t: f32) -> ((f32, f32), (f32, f32)) {
    swap(dir, phase(t, TRACK_OUT), phase(t, TRACK_IN), TRACK_SHIFT)
}

/// Bagian bar progres lagu yang tampil: menyapu dari kosong saat halaman dibuka, atau meluncur
/// dari posisi lagu sebelumnya saat lagu berganti.
pub(super) fn bar(progress: f32, track: Option<(&TrackFx, f32)>, age: f32) -> f32 {
    match track {
        Some((old, t)) => lerp(old.progress.unwrap_or(0.0), progress, smooth(t / TRACK_SECS)),
        None => progress * ease_out(phase(age, BAR_SWEEP)),
    }
}

/// Huruf lama dan kemajuan guliran (0..1) tiap huruf jam yang berubah dari `old` ke `new`
/// (`None` = tetap), bergantian dari kanan. `None` kalau tidak ada yang bergulir.
fn roll_chars(old: &str, new: &str, age: f32) -> Option<Vec<Option<(char, f32)>>> {
    let (old, new): (Vec<char>, Vec<char>) = (old.chars().collect(), new.chars().collect());
    if old == new || old.len() != new.len() || age >= CLOCK_ROLL_SECS {
        return None;
    }
    let mut turn = 0.0;
    let mut rolls = vec![None; new.len()];
    for i in (0..new.len()).rev() {
        if old[i] != new[i] {
            rolls[i] = Some((old[i], ((age - turn * ROLL_GAP) / ROLL_SECS).clamp(0.0, 1.0)));
            turn += 1.0;
        }
    }
    Some(rolls)
}

/// Tanggal yang ikut bergulir kalau hari berganti saat menit berganti.
pub(super) fn date_roll(clock: Option<&(String, String, f32)>, date: &str) -> Option<Roll> {
    let (_, from, age) = clock.filter(|(_, from, age)| from != date && *age < CLOCK_ROLL_SECS)?;
    Some(Roll { from: from.clone(), t: (age / ROLL_SECS).min(1.0), up: true })
}

/// Tinggi tengah baris agenda tampil ke-`row` (0 = baris teratas yang terlihat).
fn agenda_y(row: f32) -> f32 {
    AGENDA_TOP + row * AGENDA_STEP
}

/// Kapan agenda lain meluncur untuk perubahan ini.
fn agenda_move(fx: &AgendaFx) -> (f32, f32) {
    match fx.change {
        AgendaChange::Remove => REMOVE_MOVE,
        _ => ADD_MOVE,
    }
}

/// Lama animasi sebuah perubahan agenda.
pub fn agenda_fx_secs(fx: &AgendaFx) -> f32 {
    match fx.change {
        AgendaChange::Check(_) => checklist::CHECK_SECS,
        AgendaChange::Uncheck(_) => checklist::UNDO,
        AgendaChange::Add(_) => NEW_RISE.1,
        AgendaChange::Remove => REMOVE_MOVE.1,
    }
}

/// Tinggi dan kejelasan agenda `id` yang kini di baris tampil `row`: meluncur dari baris lamanya,
/// atau naik ke tempatnya kalau baru ditambahkan.
fn agenda_slide(id: u64, row: f32, fx: Option<(&AgendaFx, f32)>) -> (f32, f32) {
    let now = agenda_y(row);
    let Some((fx, t)) = fx else { return (now, 1.0) };
    if fx.change == AgendaChange::Add(id) {
        let below = if row >= (VISIBLE - 1) as f32 { ADD_FROM } else { now + NEW_DROP };
        return (lerp(below, now, ease_out(phase(t, NEW_RISE))), smooth(phase(t, (NEW_RISE.0, NEW_RISE.0 + 0.2))));
    }
    match fx.rows.iter().find(|(old, _)| *old == id) {
        Some((_, from)) => (lerp(agenda_y(*from), now, smooth(phase(t, agenda_move(fx)))), 1.0),
        None => (now, 1.0),
    }
}

/// Centang agenda `id` (baris tampil ke-`row`): tergambar saat dicentang, terhapus saat
/// dibatalkan, dan tergambar ulang saat halaman dibuka.
fn agenda_mark(id: u64, done: bool, row: f32, fx: Option<(&AgendaFx, f32)>, age: f32) -> Mark {
    match fx {
        Some((AgendaFx { change: AgendaChange::Check(i), .. }, t)) if *i == id => checking(t),
        Some((AgendaFx { change: AgendaChange::Uncheck(i), .. }, t)) if *i == id => unchecking(t),
        _ if done && age < ENTER_SECS => redrawn(age, 0.3 + row.max(0.0) * ENTER_GAP),
        _ => Mark::flat(done),
    }
}

/// Agenda yang baru dihapus: tinggi barisnya, geser ke kanan, dan kejelasannya.
fn agenda_gone(fx: Option<(&AgendaFx, f32)>) -> Option<(&AgendaItem, f32, f32, f32)> {
    let (fx, t) = fx?;
    let (item, row) = fx.gone.as_ref()?;
    let g = phase(t, (0.0, GONE_SECS));
    Some((item, agenda_y(*row), GONE_SHIFT * ease_out(g), 1.0 - smooth(g)))
}

/// Kejelasan tulisan "Tidak ada agenda" sesudah agenda terakhir dihapus.
fn agenda_settled(fx: Option<(&AgendaFx, f32)>) -> f32 {
    fx.map_or(1.0, |(fx, t)| smooth(phase(t, agenda_move(fx))))
}

impl Renderer {
    /// Jam rata kiri di `area`. Selama `roll` berisi jam sebelumnya dan umur gulirannya, angka
    /// yang berubah bergulir naik bergantian dari kanan; huruf lainnya diam.
    pub(super) fn clock_text(&self, text: &str, roll: Option<(&str, f32)>, format: &IDWriteTextFormat, area: D2D_RECT_F, color: D2D1_COLOR_F, a: f32) {
        let Some(rolls) = roll.and_then(|(old, age)| roll_chars(old, text, age)) else {
            self.text(text, format, area, color, a);
            return;
        };
        // Angka hanya terlihat di pita setinggi hurufnya, supaya potongannya tidak menimpa sapaan
        // atau tanggal di atas dan bawahnya.
        let height = area.bottom - area.top;
        let (top, bottom) = (area.top + height * ROLL_BAND.0, area.bottom - height * ROLL_BAND.1);
        let mut prefix = String::new();
        for (ch, roll) in text.chars().zip(rolls) {
            let x = area.left + self.text_width(&prefix, format);
            prefix.push(ch);
            let right = area.left + self.text_width(&prefix, format) + 2.0;
            let at = |dy: f32| rect(x, area.top + dy, right, area.bottom + dy);
            let Some((from, t)) = roll else {
                self.text(&ch.to_string(), format, at(0.0), color, a);
                continue;
            };
            let eased = ease_out(t);
            self.push_clip(rect(x - 1.0, top, right, bottom));
            self.text(&from.to_string(), format, at(-(bottom - top) * eased), color, a * (1.0 - eased));
            self.text(&ch.to_string(), format, at((bottom - top) * (1.0 - eased)), color, a * eased);
            self.pop_clip();
        }
    }

    /// Sampul lagu di pemutar: membesar masuk saat halaman dibuka; saat sampulnya berganti
    /// (`swap` = umur peralihannya), sampul lama memudar dan yang baru masuk sambil membesar.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn cover(&self, x: f32, y: f32, size: f32, radius: f32, swap: Option<f32>, age: f32, a: f32) {
        let shown = ease_out((age - ENTER_FROM) / COVER_IN);
        let swap = swap.map_or(1.0, |t| smooth(t / ART_SECS));
        if swap < 1.0 {
            self.art_from(self.art_prev.as_ref(), x, y, size, radius, a * (1.0 - swap));
        }
        let scale = (0.88 + 0.12 * shown) * (0.92 + 0.08 * swap);
        self.placed(x + size / 2.0, y + size / 2.0, scale, 0.0, || self.art(x, y, size, radius, a * shown * swap));
    }

    /// Bulan dan deretan hari: lingkaran pilihan diam di tengah dan hari-harinya meluncur di
    /// bawahnya saat hari lain dipilih. Bulan dan tahun ikut bergulir kalau peralihannya melewati
    /// batas bulan.
    pub(super) fn calendar_strip(&self, s: &Scene, cal: &CalendarView, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        let age = entering(s, Page::Home);
        let (shown, dy) = rise(age, 0);
        let roll = |from: &str, now: &str| {
            cal.shift.as_ref().filter(|_| from != now).map(|shift| Roll { from: from.to_string(), t: shift.t, up: shift.dir > 0.0 })
        };
        let (month, year) = match cal.shift.as_ref().and_then(|shift| shift.month.as_ref()) {
            Some((month, year)) => (roll(month, &cal.month), roll(year, &cal.year)),
            None => (None, None),
        };
        self.rolling_text(&cal.month, month.as_ref(), &self.fonts.title, rect(l, 38.0 + dy, l + 46.0, 58.0 + dy), p.text, a * shown);
        self.rolling_text(&cal.year, year.as_ref(), &self.fonts.caption, rect(l, 58.0 + dy, l + 46.0, 72.0 + dy), p.muted, a * shown);

        let strip = l + 46.0;
        let col = (r - strip) / DAYS;
        let mid = strip + col * DAYS / 2.0;
        self.hit(rect(strip, 34.0, r, 84.0), Hit::CalendarStrip);
        let selected = cal.days.iter().find(|d| d.selected).map_or(0, |d| d.offset);
        let pos = |day: &DayView| (day.offset - selected) as f32 - cal.center;
        // Lingkaran pilihan berubah jadi cincin di sekeliling hari ini saat hari ini ada di tengah.
        let on_today = cal.days.iter().find(|d| d.today).map_or(0.0, |d| (1.0 - pos(d).abs()).max(0.0));
        let (ring_shown, ring_dy, ring_pop) = day_in(age, 2.0);
        if on_today < 1.0 {
            self.circle(mid, 63.0 + ring_dy, 12.0 * ring_pop, self.tint(0.18), a * ring_shown * (1.0 - on_today));
        }
        self.push_clip(rect(strip - 2.0, 30.0, r + 6.0, 88.0));
        for day in &cal.days {
            let x = pos(day);
            let fade = edge(x);
            if fade <= 0.0 {
                continue;
            }
            let cx = mid + col * x;
            let (shown, dy, pop) = day_in(age, x + 2.0);
            let alpha = a * fade * shown;
            let near = (1.0 - x.abs()).max(0.0);
            let name = if day.today { p.accent } else { p.muted };
            self.text(day.name, &self.fonts.caption_center, rect(cx - col / 2.0, 36.0 + dy, cx + col / 2.0, 50.0 + dy), name, alpha);
            let hovered = near < 0.5 && s.hovered == Some(Hit::CalendarDay(day.offset));
            if day.today {
                self.circle(cx, 63.0 + dy, 12.0 * pop, p.accent, alpha);
            } else if hovered {
                self.circle(cx, 63.0 + dy, 12.0, self.tint(0.08), alpha);
            }
            let color = if day.today { WHITE } else { blend(p.soft, p.text, if hovered { 1.0 } else { near }) };
            self.text(&day.number.to_string(), &self.fonts.label_center, rect(cx - 14.0, 53.0 + dy, cx + 14.0, 73.0 + dy), color, alpha);
            if day.has_events {
                self.circle(cx, 80.0 + dy, 2.0, if day.today { p.accent } else { p.muted }, alpha);
            }
        }
        self.pop_clip();
        if on_today > 0.0 {
            self.circle_outline(mid, 63.0 + ring_dy, 14.5, 1.2, with_alpha(p.accent, 0.5), a * ring_shown * on_today);
        }
        // Area klik selalu di tempat akhir tiap hari.
        for day in &cal.days {
            let rel = (day.offset - selected) as f32;
            if rel.abs() <= DAYS / 2.0 {
                let cx = mid + col * rel;
                self.hit(rect(cx - col / 2.0, 34.0, cx + col / 2.0, 84.0), Hit::CalendarDay(day.offset));
            }
        }
    }

    /// Agenda hari terpilih. Selama peralihan hari, agenda hari sebelumnya bergeser keluar dan
    /// agenda hari terpilih masuk dari arah geserannya; agenda yang dicentang, ditambah, atau
    /// dihapus dianimasikan seperti tugas.
    pub(super) fn agenda_list(&self, s: &Scene, cal: &CalendarView, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        let age = entering(s, Page::Home);
        let fx = cal.agenda_fx.as_ref().map(|(fx, t)| (fx, *t));
        self.hit(rect(l - 4.0, 85.0, r + 4.0, 116.0), Hit::AgendaList);
        let moving = cal.shift.is_some() || fx.is_some();
        if moving {
            self.push_clip(rect(l - 8.0, 84.0, r + 8.0, 116.0));
        }
        let ((out_dx, out_a), (in_dx, in_a)) = day_swap(cal.shift.as_ref());
        if let Some(shift) = &cal.shift {
            for (row, item) in shift.events.iter().enumerate() {
                self.agenda_row(item, l + out_dx, r + out_dx, agenda_y(row as f32), Mark::flat(item.done), None, a * out_a);
            }
            if shift.events.is_empty() {
                self.text("Tidak ada agenda", &self.fonts.caption, rect(l + out_dx, 92.0, r + out_dx, 108.0), p.muted, a * out_a);
            }
        }
        if cal.events.is_empty() {
            let shown = in_a * agenda_settled(fx) * rise(age, 3).0;
            self.text("Tidak ada agenda", &self.fonts.caption, rect(l + in_dx, 92.0, r + in_dx, 108.0), p.muted, a * shown);
        }
        if let Some((item, y, dx, alpha)) = agenda_gone(fx) {
            self.agenda_row(item, l + dx, r + dx, y, Mark::flat(item.done), None, a * alpha);
        }
        let hovered = |id: u64| match s.hovered {
            Some(Hit::AgendaRow(i) | Hit::AgendaToggle(i)) if i == id => Some(false),
            Some(Hit::AgendaRemove(i)) if i == id => Some(true),
            _ => None,
        };
        for (index, item) in cal.events.iter().enumerate() {
            let row = index as f32 - cal.scroll as f32;
            if !moving && !(0.0..VISIBLE as f32).contains(&row) {
                continue;
            }
            let (y, shown) = agenda_slide(item.id, row, fx);
            if !(70.0..=130.0).contains(&y) {
                continue;
            }
            let (entry, dy) = rise(age, 3 + row.max(0.0) as usize);
            let mark = agenda_mark(item.id, item.done, row, fx, age);
            let over = hovered(item.id).filter(|_| !moving);
            self.agenda_row(item, l + in_dx, r + in_dx, y + dy, mark, over, a * in_a * shown * entry);
        }
        if moving {
            self.pop_clip();
        }
        // Area klik di tempat akhir tiap agenda yang terlihat; lingkaran dan × dicatat sesudah
        // barisnya supaya menang.
        for (row, item) in cal.events.iter().skip(cal.scroll).take(VISIBLE).enumerate() {
            let y = agenda_y(row as f32);
            self.hit(rect(l - 4.0, y - 8.0, r + 4.0, y + 8.0), Hit::AgendaRow(item.id));
            let bx = l + 6.0;
            self.hit(rect(bx - 9.0, y - 8.0, bx + 9.0, y + 8.0), Hit::AgendaToggle(item.id));
            if hovered(item.id).is_some() {
                self.hit(rect(r - 20.0, y - 8.0, r + 4.0, y + 8.0), Hit::AgendaRemove(item.id));
            }
        }
    }

    /// Satu baris agenda setinggi `y`: lingkaran centang, teks (tercoret sepanjang `mark.strike`),
    /// dan jamnya. Saat disorot, × menggantikan jamnya (`hovered` = kursor tepat di atas ×).
    #[allow(clippy::too_many_arguments)]
    fn agenda_row(&self, item: &AgendaItem, l: f32, r: f32, y: f32, mark: Mark, hovered: Option<bool>, a: f32) {
        let p = &self.pal;
        let bx = l + 6.0;
        if mark.fill < 1.0 {
            self.circle_outline(bx, y, 5.5, 1.4, if hovered.is_some() { p.text } else { p.accent }, a * (1.0 - mark.fill));
        }
        if mark.fill > 0.0 {
            // Sedikit membesar di tengah jalan, lalu kembali: "pop".
            self.circle(bx, y, 6.0 * mark.fill + 1.5 * (PI * mark.fill).sin(), p.accent, a);
        }
        if mark.tick > 0.0 {
            self.tick(bx, y, 0.75, mark.tick, a);
        }
        // Hitung mundur ("15 mnt lagi") lebih lebar dari jam biasa.
        let time = if item.time.is_empty() { 0.0 } else { self.text_width(&item.time, &self.fonts.caption) + 8.0 };
        let right = r - time.max(44.0);
        let area = rect(l + 18.0, y - 8.0, right, y + 8.0);
        self.striking(&item.text, &self.fonts.caption, area, blend(p.text, p.muted, mark.strike), mark.strike, a);
        match hovered {
            Some(over) => self.glyph(ICON_CLOSE, &self.fonts.icon_9, r - 8.0, y, if over { RED } else { p.muted }, a),
            None => {
                let color = if item.soon { p.accent } else { p.muted };
                self.text(&item.time, &self.fonts.caption_end, rect(right, y - 8.0, r, y + 8.0), color, a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fx(change: AgendaChange, rows: Vec<(u64, f32)>) -> AgendaFx {
        AgendaFx { change, rows, gone: None }
    }

    #[test]
    fn changed_digits_roll_from_the_right() {
        let rolls = roll_chars("19:59", "20:00", 0.0).unwrap();
        assert_eq!(rolls[2], None);
        assert_eq!(rolls[4], Some(('9', 0.0)));
        // Angka paling kiri baru mulai setelah tiga angka lain.
        let late = roll_chars("19:59", "20:00", 3.0 * ROLL_GAP + ROLL_SECS / 2.0).unwrap();
        assert!(late[0].is_some_and(|(from, t)| from == '1' && (t - 0.5).abs() < 0.001));
        assert_eq!(late[4], Some(('9', 1.0)));
        assert!(roll_chars("19:59", "20:00", CLOCK_ROLL_SECS).is_none());
        assert!(roll_chars("19:21", "19:21", 0.1).is_none());
        assert!(roll_chars("9:59", "10:00", 0.1).is_none());
    }

    #[test]
    fn agenda_rows_slide_and_new_ones_rise() {
        let added = fx(AgendaChange::Add(9), vec![(1, 0.0)]);
        assert_eq!(agenda_slide(1, 1.0, Some((&added, 0.0))), (agenda_y(0.0), 1.0));
        assert_eq!(agenda_slide(1, 1.0, Some((&added, ADD_MOVE.1))).0, agenda_y(1.0));
        // Agenda baru di baris terbawah naik dari kotak tambah.
        assert_eq!(agenda_slide(9, 1.0, Some((&added, 0.0))), (ADD_FROM, 0.0));
        assert_eq!(agenda_slide(9, 0.0, Some((&added, 0.0))).0, agenda_y(0.0) + NEW_DROP);
        assert_eq!(agenda_slide(9, 1.0, Some((&added, NEW_RISE.1))), (agenda_y(1.0), 1.0));
        assert_eq!(agenda_slide(9, 1.0, None), (agenda_y(1.0), 1.0));
    }

    #[test]
    fn agenda_checks_like_tasks() {
        let checked = fx(AgendaChange::Check(4), Vec::new());
        assert_eq!(agenda_mark(4, true, 0.0, Some((&checked, 0.0)), f32::MAX).fill, 0.0);
        assert_eq!(agenda_mark(4, true, 0.0, Some((&checked, agenda_fx_secs(&checked))), f32::MAX), Mark::flat(true));
        assert_eq!(agenda_mark(5, true, 0.0, Some((&checked, 0.0)), f32::MAX), Mark::flat(true));
        assert_eq!(agenda_mark(5, true, 0.0, None, 0.0).tick, 0.0);
    }

    #[test]
    fn entry_and_swaps_settle() {
        assert_eq!(rise(f32::MAX, 3), (1.0, 0.0));
        assert_eq!(rise(0.0, 0).0, 0.0);
        assert_eq!(day_in(ENTER_SECS, 4.0), (1.0, 0.0, 1.0));
        assert_eq!(day_in(0.0, 0.0).2, 0.0);
        assert_eq!(edge(2.0), 1.0);
        assert_eq!(edge(3.0), 0.0);
        assert_eq!(track_swap(1.0, TRACK_SECS), ((-TRACK_SHIFT, 0.0), (0.0, 1.0)));
        assert_eq!(track_swap(-1.0, 0.0), ((0.0, 1.0), (-TRACK_SHIFT, 0.0)));
        assert_eq!(slide_ease(SLIDE_SECS), 1.0);
        assert_eq!(bar(0.6, None, f32::MAX), 0.6);
        assert_eq!(bar(0.6, None, 0.0), 0.0);
        let old = TrackFx { title: String::new(), artist: String::new(), progress: Some(0.9), dir: 1.0 };
        assert_eq!(bar(0.0, Some((&old, 0.0)), f32::MAX), 0.9);
    }

    #[test]
    fn daylight_follows_the_greeting() {
        assert_eq!(daylight(7).0.r, rgb(0xFFB340, 1.0).r);
        assert_eq!(daylight(12).0.b, rgb(0x64D2FF, 1.0).b);
        assert_eq!(daylight(16).0.g, rgb(0xFF8A4C, 1.0).g);
        assert_eq!(daylight(2).0.b, daylight(21).0.b);
    }
}
