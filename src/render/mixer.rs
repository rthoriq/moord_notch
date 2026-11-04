//! Suasana halaman Kontrol, seperti meja mixer kecil: slider yang meluncur ke nilai barunya
//! dengan angka bergulir, matahari kecerahan yang membesar dan berputar mengikuti nilainya,
//! speaker yang membal saat bisu berganti, tombol sakelar yang warnanya menyapu masuk, cincin
//! sisa waktu senyap, dan pendaran oranye (mikrofon bisu) atau biru (senyap). Di luar animasi
//! tidak ada gerak tambahan; cincin senyap ikut menyusut saat jam digambar ulang tiap menit.

use std::f32::consts::PI;

use super::*;

/// Animasi masuk: kedua bilah menyapu dari kosong ke nilainya sambil matahari berputar ke
/// tempatnya dan angkanya muncul pelan, lalu tombol di bawah naik bergantian dari kiri.
pub const ENTER_SECS: f32 = 0.7;
/// Mulai dan lama sapuan bilah volume, lalu kecerahan.
const SWEEP: [(f32, f32); 2] = [(0.05, 0.45), (0.12, 0.45)];
const LABEL_IN: (f32, f32) = (0.25, 0.3);
const BUTTON_FROM: f32 = 0.22;
const BUTTON_GAP: f32 = 0.07;
const BUTTON_IN: f32 = 0.28;
const BUTTON_RISE: f32 = 8.0;
/// Slider meluncur ke nilai baru (klik di jalur, roda, tombol volume) dan angkanya bergulir.
pub const GLIDE_SECS: f32 = 0.22;
/// Bisu speaker berganti: ikonnya membal, bilahnya memudar ke abu-abu atau kembali putih, dan
/// angkanya bergulir dari/ke "Bisu".
pub const MUTE_SECS: f32 = 0.3;
const MUTE_POP: f32 = 0.28;
/// Tombol sakelar: warnanya menyapu dari kiri (atau surut ke kiri), tulisannya berganti pelan,
/// dan tombolnya sedikit membal.
pub const SWITCH_SECS: f32 = 0.35;
const SWITCH_SWEEP: f32 = 0.3;
const SWITCH_POP: f32 = 0.06;
/// Matahari kecerahan: ukurannya di 0% dan 100%, dan putarannya dari 0% sampai 100%.
const SUN_SCALE: (f32, f32) = (0.8, 1.15);
const SUN_TURN: f32 = PI / 2.0;
/// Lama mode senyap (sama dengan `QUIET` di app.rs), untuk cincin sisa waktunya.
const QUIET_MINUTES: f32 = 60.0;
/// Pendaran mikrofon bisu (lebih penting selama panggilan) dan senyap.
const MIC_GLOW: f32 = 0.6;
const QUIET_GLOW: f32 = 0.45;
/// Bulan sabit "jangan ganggu", sama dengan ikon sholat.
pub(super) const ICON_QUIET: char = ICON_PRAYER;

/// Nilai slider yang tampil `age` detik setelah mulai meluncur dari `from` ke `to`.
pub fn glide(from: f32, to: f32, age: f32) -> f32 {
    lerp(from, to, ease_out(age / GLIDE_SECS))
}

/// Seberapa menyala sebuah sakelar (0..1): `on` keadaannya sekarang, `age` umur pergantiannya.
fn switched(on: bool, age: Option<f32>, secs: f32) -> f32 {
    let k = age.map_or(1.0, |t| smooth(t / secs));
    if on { k } else { 1.0 - k }
}

/// Skala yang membal sekali ke 1 + `size` lalu kembali, selama `secs` detik.
fn pop(age: Option<f32>, secs: f32, size: f32) -> f32 {
    age.map_or(1.0, |t| 1.0 + size * (PI * (t / secs).clamp(0.0, 1.0)).sin())
}

/// Isi bilah ke-`row` (0 volume, 1 kecerahan) yang tampil: meluncur dari `glide` (nilai awal dan
/// umurnya) dan, saat halaman dibuka (`age`), menyapu dari kosong.
pub(super) fn shown(value: f32, glide: Option<(f32, f32)>, row: usize, age: f32) -> f32 {
    let value = glide.map_or(value, |(from, t)| self::glide(from, value, t));
    let (start, secs) = SWEEP[row];
    value * ease_out((age - start) / secs)
}

fn level_text(value: f32, muted: bool) -> String {
    if muted { "Bisu".to_string() } else { format!("{}", (value * 100.0).round()) }
}

/// Angka slider dan gulirannya: dari angka awal selama slidernya meluncur, dan dari/ke "Bisu"
/// saat bisu speaker berganti (umur `mute`).
pub(super) fn label(value: f32, muted: bool, glide: Option<(f32, f32)>, mute: Option<f32>) -> (String, Option<Roll>) {
    let text = level_text(value, muted);
    let roll = match (mute, glide) {
        // "Bisu" turun dari atas; saat bersuara lagi angkanya naik dari bawah.
        (Some(t), _) => Some(Roll { from: level_text(value, !muted), t: (t / MUTE_SECS).min(1.0), up: !muted }),
        (None, Some((from, t))) => {
            let old = level_text(from, muted);
            (old != text).then(|| Roll { from: old, t: (t / GLIDE_SECS).min(1.0), up: value >= from })
        }
        (None, None) => None,
    };
    (text, roll)
}

/// Kejelasan angka slider saat halaman dibuka.
pub(super) fn label_in(age: f32) -> f32 {
    smooth((age - LABEL_IN.0) / LABEL_IN.1)
}

/// Bilah volume memudar ke abu-abu saat dibisukan: 0 = putih, 1 = abu-abu.
pub(super) fn muted_tone(muted: bool, age: Option<f32>) -> f32 {
    switched(muted, age, MUTE_SECS)
}

/// Skala ikon speaker yang membal saat bisu berganti.
pub(super) fn speaker_pop(age: Option<f32>) -> f32 {
    pop(age, MUTE_SECS, MUTE_POP)
}

/// Skala dan putaran matahari untuk kecerahan yang tampil (0..1).
pub(super) fn sun(value: f32) -> (f32, f32) {
    (lerp(SUN_SCALE.0, SUN_SCALE.1, value), value * SUN_TURN)
}

/// Tombol ke-`index` dari kiri naik saat halaman dibuka: kejelasan dan geser tegaknya.
pub(super) fn rise(age: f32, index: usize) -> (f32, f32) {
    let u = age - BUTTON_FROM - index as f32 * BUTTON_GAP;
    (smooth(u / (BUTTON_IN * 0.7)), BUTTON_RISE * (1.0 - ease_out(u / BUTTON_IN)))
}

/// Bagian cincin senyap yang tersisa.
pub(super) fn quiet_left(minutes: u32) -> f32 {
    (minutes as f32 / QUIET_MINUTES).clamp(0.0, 1.0)
}

/// Oranye lembut selama mikrofon bisu, biru tenang (`accent`) selama senyap; kalau keduanya,
/// oranye yang menang. Ikut menyala/padam bersama sapuan tombolnya.
pub(super) fn glow(s: &Scene, accent: D2D1_COLOR_F) -> Option<(D2D1_COLOR_F, f32)> {
    let c = s.controls;
    let mic = switched(c.mic == Some(true), c.fx.mic, SWITCH_SECS);
    let quiet = switched(c.quiet_minutes.is_some(), c.fx.quiet, SWITCH_SECS);
    let strength = lerp(QUIET_GLOW * quiet, MIC_GLOW, mic) * smooth(entering(s, Page::Controls) / 0.6);
    let color = if quiet > 0.001 { blend(accent, ORANGE, mic) } else { ORANGE };
    (strength > 0.01).then_some((color, strength))
}

/// Tombol sakelar di halaman Kontrol.
pub(super) struct Switch<'a> {
    pub area: D2D_RECT_F,
    /// Keadaan sekarang dan umur pergantiannya.
    pub on: bool,
    pub age: Option<f32>,
    /// Ikon dan tulisan saat menyala, lalu saat mati.
    pub icon: (char, char),
    pub text: (&'a str, &'a str),
    /// Warna saat menyala.
    pub color: D2D1_COLOR_F,
    pub hovered: bool,
}

impl Renderer {
    /// Tombol sakelar, digeser tegak `dy`: warnanya menyapu masuk dari kiri saat menyala dan
    /// surut ke kiri saat mati, tulisannya berganti pelan, dan tombolnya sedikit membal.
    pub(super) fn switch(&self, sw: &Switch, dy: f32, a: f32) {
        let p = &self.pal;
        let area = rect(sw.area.left, sw.area.top + dy, sw.area.right, sw.area.bottom + dy);
        let (cx, cy) = ((area.left + area.right) / 2.0, (area.top + area.bottom) / 2.0);
        let radius = (area.bottom - area.top) / 2.0;
        let lit = switched(sw.on, sw.age, SWITCH_SWEEP);
        let swap = sw.age.map_or(1.0, |t| (t / SWITCH_SECS).min(1.0));
        self.placed(cx, cy, pop(sw.age, SWITCH_SECS, SWITCH_POP), 0.0, || {
            self.rounded(area, radius, self.tint(if sw.hovered { 0.16 } else { 0.08 }), a);
            // Kapsul warna tumbuh dari bulatan di kiri; yang hampir habis memudar.
            if lit > 0.001 {
                let edge = lerp(area.left + 2.0 * radius, area.right, ease_out(lit));
                self.rounded(rect(area.left, area.top, edge, area.bottom), radius, sw.color, a * smooth(lit / 0.2));
            }
            // Ikon dan tulisan keadaan lama memudar dulu, baru yang baru muncul (masing-masing di
            // tengah, jadi keduanya tidak boleh lama bertumpuk).
            for (on, alpha) in [(!sw.on, 1.0 - smooth(swap / 0.6)), (sw.on, smooth((swap - 0.4) / 0.6))] {
                if alpha <= 0.001 {
                    continue;
                }
                let color = if on { WHITE } else if sw.hovered { p.text } else { p.soft };
                let (icon, text) = if on { (sw.icon.0, sw.text.0) } else { (sw.icon.1, sw.text.1) };
                let width = self.text_width(text, &self.fonts.label_center);
                let left = cx - (16.0 + width) / 2.0;
                self.glyph(icon, &self.fonts.icon_11, left + 5.5, cy, color, a * alpha);
                self.text(text, &self.fonts.label_center, rect(left + 16.0, area.top, left + 16.0 + width, area.bottom), color, a * alpha);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sliders_glide_and_numbers_roll() {
        assert_eq!(glide(0.2, 0.8, 0.0), 0.2);
        assert!((glide(0.2, 0.8, GLIDE_SECS) - 0.8).abs() < 1e-6);
        let half = glide(0.2, 0.8, GLIDE_SECS / 2.0);
        assert!(half > 0.5 && half < 0.8, "ease out: lebih dari separuh jalan di tengah waktu");
        let (text, roll) = label(0.8, false, Some((0.2, 0.0)), None);
        let roll = roll.unwrap();
        assert_eq!((text.as_str(), roll.from.as_str(), roll.up), ("80", "20", true));
        // Angka yang sama tidak bergulir.
        assert!(label(0.8, false, Some((0.801, 0.0)), None).1.is_none());
        // Bisu: "Bisu" turun menggantikan angkanya.
        let (text, roll) = label(0.3, true, None, Some(0.1));
        let roll = roll.unwrap();
        assert_eq!((text.as_str(), roll.from.as_str(), roll.up), ("Bisu", "30", false));
    }

    #[test]
    fn entry_sweeps_bars_and_raises_buttons() {
        assert_eq!(shown(0.6, None, 0, 0.0), 0.0);
        assert!((shown(0.6, None, 1, ENTER_SECS) - 0.6).abs() < 1e-6);
        assert_eq!(shown(0.6, None, 0, f32::MAX), 0.6);
        assert!(shown(0.6, None, 0, 0.2) > shown(0.6, None, 1, 0.2), "volume menyapu lebih dulu");
        assert_eq!(rise(0.0, 0), (0.0, BUTTON_RISE));
        for index in 0..3 {
            assert_eq!(rise(ENTER_SECS, index), (1.0, 0.0));
        }
        assert!(rise(0.4, 0).0 > rise(0.4, 2).0, "tombol kiri naik lebih dulu");
        assert_eq!(label_in(0.0), 0.0);
        assert_eq!(label_in(ENTER_SECS), 1.0);
    }

    #[test]
    fn switches_sweep_and_settle() {
        assert_eq!(switched(true, None, SWITCH_SECS), 1.0);
        assert_eq!(switched(false, None, SWITCH_SECS), 0.0);
        assert_eq!(switched(true, Some(0.0), SWITCH_SECS), 0.0);
        assert_eq!(switched(false, Some(SWITCH_SECS), SWITCH_SECS), 0.0);
        assert_eq!(pop(None, SWITCH_SECS, SWITCH_POP), 1.0);
        assert!(pop(Some(SWITCH_SECS / 2.0), SWITCH_SECS, SWITCH_POP) > 1.05);
        assert!((pop(Some(SWITCH_SECS), SWITCH_SECS, SWITCH_POP) - 1.0).abs() < 1e-6);
        assert_eq!(muted_tone(true, None), 1.0);
        assert!(speaker_pop(Some(MUTE_SECS / 2.0)) > 1.2);
        assert!((speaker_pop(Some(MUTE_SECS)) - 1.0).abs() < 1e-6);
        assert_eq!(sun(0.0), (SUN_SCALE.0, 0.0));
        assert!((quiet_left(42) - 0.7).abs() < 1e-6);
        assert_eq!(quiet_left(90), 1.0);
    }
}
