//! Suasana halaman Shelf, rak tempat file diletakkan sementara: papan rak dengan bayangan di
//! bawah tiap ubin, file yang dilepas jatuh ke raknya, ubin yang dihapus terangkat lalu sisanya
//! merapat, lencana jenis file, dan pendaran selama file diseret di atas notch. Sedotan
//! "Bersihkan" ada di `vacuum`. Di luar animasi tidak ada gerak tambahan.

use std::f32::consts::PI;

use crate::widgets::shelf::Item;

use super::*;

/// Animasi masuk: ubin diletakkan ke rak bergantian dari kiri; shelf kosong mengangguk sekali.
pub const ENTER_SECS: f32 = 0.8;
const ENTER_FROM: f32 = 0.1;
const ENTER_GAP: f32 = 0.07;
const ENTER_DROP: f32 = 10.0;
const NOD: (f32, f32) = (0.1, 0.55);
/// File yang dilepas jatuh dari atas lalu memantul kecil; beberapa file jatuh bergantian.
pub const DROP_SECS: f32 = 0.6;
pub const DROP_GAP: f32 = 0.08;
const FALL_SECS: f32 = 0.28;
const DROP_FROM: f32 = 36.0;
/// Ubin yang dihapus terangkat sambil memudar.
pub const LIFT_SECS: f32 = 0.3;
const LIFT_RISE: f32 = 12.0;
/// Ubin lain merapat ke tempat barunya, juga saat digulir.
pub const SLIDE_SECS: f32 = 0.3;
/// Jarak antarubin dan lebar ubin.
pub(super) const TILE_STEP: f32 = 76.0;
pub(super) const TILE_WIDTH: f32 = 68.0;
/// Permukaan papan rak; gambar ubin berdiri di atasnya dan namanya tertulis di bawahnya.
pub(super) const BOARD_Y: f32 = 93.0;
const THUMB: f32 = 48.0;

/// Bagian slide yang sudah dilalui (0..1) setelah `t` detik.
pub fn slide_ease(t: f32) -> f32 {
    smooth(t / SLIDE_SECS)
}

/// Tampilan sebuah ubin: geser tegak gambarnya, kejelasan gambar dan namanya, skala, besar
/// bayangannya (0..1), dan condong gambarnya ke arah "Bersihkan" (radian).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Look {
    pub dy: f32,
    pub alpha: f32,
    pub label: f32,
    pub scale: f32,
    pub shadow: f32,
    pub lean: f32,
}

pub(super) const STILL: Look = Look { dy: 0.0, alpha: 1.0, label: 1.0, scale: 1.0, shadow: 1.0, lean: 0.0 };

/// Tinggi gambar yang jatuh di atas raknya (negatif = di atas), `t` detik sejak mulai jatuh.
fn fall(t: f32) -> f32 {
    if t < FALL_SECS {
        return -DROP_FROM * (1.0 - (t / FALL_SECS).powi(2));
    }
    bounce(t - FALL_SECS)
}

/// Pantulan kecil ubin yang mendarat di rak, `b` detik sejak menyentuhnya (negatif = naik).
pub(super) fn bounce(b: f32) -> f32 {
    -4.0 * (PI * b / 0.2).sin().abs() * (-b / 0.1).exp()
}

/// Tampilan ubin di urutan `slot`: `fx` berisi umur animasinya (detik, negatif = belum giliran)
/// dan apakah ia sedang dihapus; `age` detik sejak halaman dibuka.
pub(super) fn look(fx: Option<(f32, bool)>, slot: f32, age: f32) -> Look {
    match fx {
        Some((t, true)) => {
            let u = (t / LIFT_SECS).clamp(0.0, 1.0);
            let gone = smooth(u);
            Look { dy: -LIFT_RISE * ease_out(u), alpha: 1.0 - gone, label: 1.0 - smooth(u * 2.0), scale: 1.0 - 0.08 * u, shadow: 1.0 - gone, lean: 0.0 }
        }
        Some((t, false)) if t < 0.0 => Look { alpha: 0.0, label: 0.0, shadow: 0.0, ..STILL },
        Some((t, false)) => {
            let dy = fall(t);
            let alpha = smooth(t / 0.08);
            Look { dy, alpha, label: smooth((t - FALL_SECS) / 0.15), shadow: alpha * (1.0 + dy / DROP_FROM), ..STILL }
        }
        None if age < ENTER_SECS => {
            let u = age - ENTER_FROM - slot.max(0.0) * ENTER_GAP;
            let shown = smooth(u / 0.2);
            Look { dy: -ENTER_DROP * (1.0 - ease_out(u / 0.3)), alpha: shown, label: smooth((u - 0.15) / 0.2), shadow: shown, ..STILL }
        }
        None => STILL,
    }
}

/// Anggukan ikon shelf kosong saat halaman dibuka (negatif = naik).
pub(super) fn nod(age: f32) -> f32 {
    let u = ((age - NOD.0) / (NOD.1 - NOD.0)).clamp(0.0, 1.0);
    -4.0 * (PI * u).sin()
}

/// Pendaran selama file diseret di atas notch (lebih terang tepat di atas zona lepas), lalu
/// memudar sesudah dilepas.
pub(super) fn glow(s: &Scene, accent: D2D1_COLOR_F) -> Option<(D2D1_COLOR_F, f32)> {
    let fx = s.shelf_fx;
    if s.shelf.drop_hover {
        return Some((accent, if fx.targeted || fx.share_targeted { 0.6 } else { 0.3 }));
    }
    fx.glow.map(|t| (accent, 0.6 * (1.0 - t)))
}

/// Warna lencana untuk ekstensi file; `None` untuk gambar dan video, yang sudah punya pratinjau.
fn kind_color(ext: &str) -> Option<D2D1_COLOR_F> {
    let color = match ext {
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "heic" | "tif" | "tiff" | "ico" => return None,
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "m4v" => return None,
        "pdf" => 0xFF453A,
        "doc" | "docx" | "odt" | "rtf" => 0x0A84FF,
        "xls" | "xlsx" | "csv" | "ods" => 0x30D158,
        "ppt" | "pptx" | "odp" => 0xFF6A3D,
        "zip" | "rar" | "7z" | "tar" | "gz" => 0xFF9F0A,
        "mp3" | "wav" | "flac" | "m4a" | "ogg" | "aac" => 0xFF375F,
        _ => 0x8E8E93,
    };
    Some(rgb(color, 1.0))
}

/// Lencana sebuah file: tulisan ekstensinya (paling banyak 4 huruf) dan warnanya.
fn badge(item: &Item) -> Option<(String, D2D1_COLOR_F)> {
    if item.is_dir {
        return None;
    }
    let ext = item.path.extension()?.to_str()?.to_ascii_lowercase();
    let color = kind_color(&ext)?;
    Some((ext.to_uppercase().chars().take(4).collect(), color))
}

impl Renderer {
    /// Papan rak selebar shelf.
    pub(super) fn rack(&self, l: f32, r: f32, a: f32) {
        self.rounded(rect(l - 4.0, BOARD_Y, r + 4.0, BOARD_Y + 3.0), 1.5, self.tint(0.10), a);
        self.rounded(rect(l - 4.0, BOARD_Y, r + 4.0, BOARD_Y + 0.8), 0.4, self.tint(0.22), a);
    }

    /// Satu ubin di rak, kiri ubinnya di `tx`: bayangan, gambar yang berdiri di papan (condong
    /// bertumpu di papan), lencana jenis file, dan namanya di bawah papan.
    pub(super) fn shelf_tile(&self, item: &Item, tx: f32, look: Look, hovered: bool, a: f32) {
        let cx = tx + TILE_WIDTH / 2.0;
        let scale = look.scale * if hovered { 1.05 } else { 1.0 };
        self.placed(cx, BOARD_Y, scale, 0.0, || {
            if hovered {
                self.rounded(rect(tx, 38.0, tx + TILE_WIDTH, 118.0), 10.0, self.tint(0.08), a * look.alpha);
            }
            if look.shadow > 0.01 {
                let strength = if self.pal.text.r < 0.5 { 0.18 } else { 0.5 };
                let ellipse = D2D1_ELLIPSE { point: pt(cx, BOARD_Y + 0.6), radiusX: 21.0 * (0.5 + 0.5 * look.shadow), radiusY: 2.2 };
                unsafe {
                    self.brush.SetColor(&rgb(0x000000, strength * look.shadow * a));
                    self.target.FillEllipse(&ellipse, &self.brush);
                }
            }
        });
        self.rotated(cx, BOARD_Y, look.lean, || {
            self.placed(cx, BOARD_Y, scale, look.dy, || self.standing_thumb(item, cx, a * look.alpha));
        });
        self.placed(cx, BOARD_Y, scale, 0.0, || {
            self.text(&item.name, &self.fonts.caption_center, rect(tx + 3.0, 96.0, tx + 65.0, 112.0), self.pal.soft, a * look.label);
        });
    }

    /// Gambar file yang berdiri di papan rak, dengan lencana jenisnya di sudut kanan bawah.
    pub(super) fn standing_thumb(&self, item: &Item, cx: f32, a: f32) {
        let bottom = BOARD_Y - 1.0;
        let half = match self.thumbs.get(&item.id) {
            Some((brush, w, h)) => {
                let k = (THUMB / w).min(THUMB / h);
                let (dw, dh) = (w * k, h * k);
                self.fill_bitmap(brush, rect(cx - dw / 2.0, bottom - dh, cx + dw / 2.0, bottom), k, 6.0, a);
                dw / 2.0
            }
            None => {
                let icon = if item.is_dir { ICON_FOLDER } else { ICON_DOCUMENT };
                self.glyph(icon, &self.fonts.icon_22, cx, bottom - 12.0, self.pal.soft, a);
                9.0
            }
        };
        let Some((label, color)) = badge(item) else { return };
        let width = (self.text_width(&label, &self.fonts.tiny) + 7.0).max(18.0);
        let bx = cx + half.min(20.0) - 2.0;
        let area = rect(bx - width / 2.0, bottom - 12.5, bx + width / 2.0, bottom - 1.5);
        self.rounded(area, 3.0, color, a);
        self.text(&label, &self.fonts.tiny, area, WHITE, a);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn item(name: &str) -> Item {
        Item { id: 1, path: PathBuf::from(format!("C:\\contoh\\{name}")), name: name.into(), is_dir: false }
    }

    #[test]
    fn dropped_tile_lands_on_the_rack() {
        assert_eq!(look(Some((-0.05, false)), 0.0, f32::MAX).alpha, 0.0);
        assert_eq!(fall(0.0), -DROP_FROM);
        assert_eq!(fall(FALL_SECS), 0.0);
        assert!(fall(DROP_SECS).abs() < 0.2);
        let landed = look(Some((DROP_SECS, false)), 0.0, f32::MAX);
        assert_eq!((landed.alpha, landed.label), (1.0, 1.0));
    }

    #[test]
    fn removed_tile_lifts_away() {
        assert_eq!(look(Some((0.0, true)), 0.0, f32::MAX), STILL);
        let gone = look(Some((LIFT_SECS, true)), 0.0, f32::MAX);
        assert_eq!((gone.alpha, gone.dy), (0.0, -LIFT_RISE));
        assert_eq!(look(None, 2.0, f32::MAX), STILL);
        assert_eq!(look(None, 2.0, 0.0).alpha, 0.0);
        assert!(nod(f32::MAX).abs() < 0.001);
        assert_eq!(slide_ease(SLIDE_SECS), 1.0);
    }

    #[test]
    fn badges_name_the_file_kind() {
        assert_eq!(badge(&item("Laporan.PDF")).map(|b| b.0), Some("PDF".into()));
        assert_eq!(badge(&item("Data.xlsx")).map(|b| b.1.g), Some(rgb(0x30D158, 1.0).g));
        assert_eq!(badge(&item("Arsip.backup")).map(|b| b.0), Some("BACK".into()));
        assert!(badge(&item("Foto.jpg")).is_none());
        assert!(badge(&item("Project")).is_none());
        assert!(badge(&Item { is_dir: true, ..item("Folder.zip") }).is_none());
    }
}
