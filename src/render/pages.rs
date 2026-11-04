//! Tata letak isi notch: pil, HUD, dan halaman-halaman panel. Semua koordinat dalam DIP
//! relatif terhadap kanvas; `x0`/`x1` adalah tepi kiri/kanan badan notch saat ini.

use super::*;
use crate::sysinfo::{gigabytes, rate};
use crate::widgets::shelf::VISIBLE as SHELF_VISIBLE;
use crate::widgets::tasks::VISIBLE as TASKS_VISIBLE;
use crate::widgets::water::{GLASS_ML, liters};

/// Jarak isi panel dari tepinya; sisanya untuk panah pindah halaman.
const MARGIN: f32 = 32.0;
/// Lebar kolom kiri (musik/jam) di Beranda lebar; kalender di kanannya.
const HOME_LEFT: f32 = 356.0;
/// Lebar kolom pemutar saat kolom lirik tampil; lirik mendapat sisanya.
const LYRIC_LEFT: f32 = 300.0;
/// Baris yang sedang dinyanyikan di kolom lirik, dan jarak antarbarisnya.
const LYRIC_CENTER: f32 = 84.0;
const LYRIC_GAP: f32 = 36.0;
/// Jarak geser kolom kanan saat berganti antara kalender dan lirik.
const COLUMN_SLIDE: f32 = 36.0;
/// Pemutar musik: sampul, bar progres, dan baris tombol.
const ART_TOP: f32 = 34.0;
const ART_SIZE: f32 = 52.0;
const PROGRESS_Y: f32 = 101.0;
const CONTROLS_Y: f32 = 125.0;
const HEADER_Y: f32 = 22.0;
const ARROW_Y: f32 = 84.0;
const DOTS_Y: f32 = 146.0;
/// Baris tombol di bagian bawah halaman.
const ROW_Y: f32 = 128.0;
/// Jarak geser isi saat berpindah halaman.
const PAGE_SLIDE: f32 = 28.0;

impl Renderer {
    pub(super) fn draw_pill(&self, s: &Scene, x0: f32, x1: f32, h: f32, a: f32) {
        let p = &self.pal;
        let cy = h / 2.0;
        let playing = s.media.filter(|m| m.playing);
        if let Some(call) = s.call
            && playing.is_none()
        {
            self.call_pill(s, call, x0, x1, h, a);
            return;
        }
        let focus = Some(s.focus).filter(|f| f.running);
        let timer = Some(s.timer).filter(|t| t.running);
        let download = s.download.filter(|_| playing.is_none() && timer.is_none());
        // Hitung mundur sholat hanya kalau kedua sisi pil sedang kosong.
        let prayer = s
            .prayer
            .and_then(|v| v.soon.as_deref())
            .filter(|_| playing.is_none() && timer.is_none() && focus.is_none() && download.is_none());
        // Kiri: cincin timer atau pomodoro, sampul lagu, bulan sabit, atau mata maskot.
        if let Some(t) = timer {
            self.ring(x0 + 18.0, cy, 6.5, 2.2, t.progress, ORANGE, a);
        } else if let Some(f) = focus {
            self.ring(x0 + 18.0, cy, 6.5, 2.2, f.progress, phase_color(f.phase), a);
        } else if playing.is_some() {
            self.art(x0 + 9.0, cy - 9.0, 18.0, 5.0, a);
        } else if prayer.is_some() {
            self.glyph(ICON_PRAYER, &self.fonts.icon_13, x0 + 18.0, cy, TEAL, a);
        } else {
            self.eyes(x0 + 18.0, cy, 3.4, 9.0, 7.5, s.eyes, a);
        }
        // Kanan: gelombang musik, sisa waktu timer, unduhan yang berjalan, sisa menit pomodoro,
        // atau sisa menit ke waktu sholat; paling kanan titik privasi. Pil melebar saat ada
        // unduhan, jam bergeser ke kiri.
        match (playing, timer, download, focus) {
            (Some(m), ..) => self.wave(x1 - 22.0, cy, s.seconds, accent(m.accent, p.text), a),
            (None, Some(t), ..) => {
                self.text(&t.short, &self.fonts.caption_end, rect(x1 - 52.0, 0.0, x1 - 13.0, h), ORANGE, a);
            }
            (None, None, Some(size), _) => {
                self.glyph(ICON_DOWNLOAD, &self.fonts.icon_11, x1 - 20.0, cy, p.accent, a);
                self.text(size, &self.fonts.caption_end, rect(x1 - 100.0, 0.0, x1 - 31.0, h), p.soft, a);
            }
            (None, None, None, Some(f)) => {
                let left = format!("{}m", f.minutes_left);
                self.text(&left, &self.fonts.caption_end, rect(x1 - 52.0, 0.0, x1 - 13.0, h), phase_color(f.phase), a);
            }
            (None, None, None, None) => {
                if let Some(soon) = prayer {
                    self.text(soon, &self.fonts.caption_end, rect(x1 - 52.0, 0.0, x1 - 13.0, h), TEAL, a);
                }
            }
        }
        self.pill_dot(s, x1 - 8.0, cy, a);
        let time_right = if download.is_some() { x1 - 100.0 } else { x1 - 34.0 };
        self.text(s.time, &self.fonts.pill_time, rect(x0 + 34.0, 0.0, time_right, h), p.text, a);
    }

    /// Pil selama panggilan tanpa lagu yang diputar: tombol mikrofon di kiri, jam, dan lama
    /// panggilan di kanan.
    fn call_pill(&self, s: &Scene, call: &CallView, x0: f32, x1: f32, h: f32, a: f32) {
        let p = &self.pal;
        let cy = h / 2.0;
        self.call_button(s, call, x0, h, a);
        self.text(&call.clock, &self.fonts.caption_end, rect(x1 - 64.0, 0.0, x1 - 13.0, h), GREEN, a);
        // Mikrofon sudah terlihat dari ikonnya, jadi titiknya hanya untuk kamera.
        if call.camera {
            self.circle(x1 - 8.0, cy, 3.0, GREEN, a);
        }
        // Kotak jam simetris supaya jamnya tetap di tengah pil, sama seperti pil biasa.
        self.text(s.time, &self.fonts.pill_time, rect(x0 + 34.0, 0.0, x1 - 34.0, h), p.text, a);
    }

    /// Tombol bisu panggilan di kiri pil, di tempat mata maskot: mikrofon hijau, atau merah
    /// dicoret saat bisu.
    fn call_button(&self, s: &Scene, call: &CallView, x0: f32, h: f32, a: f32) {
        let cy = h / 2.0;
        if s.hovered == Some(Hit::CallMute) {
            self.circle(x0 + 18.0, cy, 11.0, self.tint(0.16), a);
        }
        let (icon, color) = if call.muted == Some(true) { (ICON_MIC_OFF, RED) } else { (ICON_MICROPHONE, GREEN) };
        self.glyph(icon, &self.fonts.icon_13, x0 + 18.0, cy, color, a);
        self.hit(rect(x0, 0.0, x0 + 34.0, h), Hit::CallMute);
    }

    /// Titik di ujung kanan pil. Selama panggilan sambil memutar lagu, titik ini satu-satunya
    /// tanda panggilan: hijau kalau kamera menyala, merah kalau mikrofon dibisukan, selain itu
    /// oranye seperti mikrofon yang sedang dipakai.
    fn pill_dot(&self, s: &Scene, cx: f32, cy: f32, a: f32) {
        let color = match s.call {
            Some(call) if call.camera => GREEN,
            Some(call) if call.muted == Some(true) => RED,
            Some(_) => ORANGE,
            None => return self.privacy_dot(s.privacy, cx, cy, a),
        };
        self.circle(cx, cy, 3.0, color, a);
    }

    /// Pil lebar saat lirik aktif: sampul, baris lirik, dan gelombang.
    pub(super) fn draw_lyric_pill(&self, s: &Scene, x0: f32, x1: f32, h: f32, a: f32) {
        let p = &self.pal;
        let cy = h / 2.0;
        self.art(x0 + 9.0, cy - 9.0, 18.0, 5.0, a);
        if let Some(line) = s.lyric {
            // Pil yang lebih tinggi berarti barisnya terlalu panjang untuk satu baris.
            if h > View::LyricPill.size().1 + 4.0 {
                self.text(line, &self.fonts.lyric_wrap, rect(x0 + 40.0, 3.0, x1 - 40.0, h - 3.0), p.text, a);
            } else {
                self.text(line, &self.fonts.lyric, rect(x0 + 36.0, 0.0, x1 - 36.0, h), p.text, a);
            }
        }
        let color = s.media.and_then(|m| m.accent);
        self.wave(x1 - 22.0, cy, s.seconds, accent(color, p.text), a);
        self.pill_dot(s, x1 - 8.0, cy, a);
    }

    pub(super) fn draw_hud(&self, s: &Scene, hud: &Hud, x0: f32, x1: f32, h: f32, a: f32) {
        // Isi kabar muncul sedikit membesar dari atas, seperti panel.
        self.begin_zoom(0.9 + 0.1 * a, (x0 + x1) / 2.0);
        if s.hud_compact {
            self.draw_hud_compact(s, hud, x0, x1, h, a);
        } else {
            self.draw_hud_card(s, hud, x0, x1, a);
        }
        self.end_zoom();
    }

    /// Kabar ringkas setinggi pil: ikon dan nama di kiri, bilah/nilai di kanan.
    fn draw_hud_compact(&self, s: &Scene, hud: &Hud, x0: f32, x1: f32, h: f32, a: f32) {
        let p = &self.pal;
        let cy = h / 2.0;
        let row = |left: f32, right: f32| rect(left, cy - 9.0, right, cy + 9.0);
        let value = row(x1 - 56.0, x1 - 14.0);
        match hud {
            Hud::Volume { level, muted } => {
                let icon = if *muted { ICON_MUTE } else { volume_icon(*level) };
                self.glyph(icon, &self.fonts.icon_13, x0 + 22.0, cy, p.text, a);
                self.text("Volume", &self.fonts.label, row(x0 + 38.0, x1 - 140.0), p.text, a);
                self.bar(x1 - 132.0, x1 - 62.0, cy, s.hud_fill, if *muted { p.muted } else { p.text }, a);
                let text = hud_value(hud).unwrap_or_default();
                self.rolling_text(&text, s.hud_roll, &self.fonts.label_end, value, p.text, a);
            }
            Hud::Brightness { .. } => {
                self.glyph(ICON_BRIGHTNESS, &self.fonts.icon_13, x0 + 22.0, cy, p.text, a);
                self.text("Kecerahan", &self.fonts.label, row(x0 + 38.0, x1 - 140.0), p.text, a);
                self.bar(x1 - 132.0, x1 - 62.0, cy, s.hud_fill, p.text, a);
                let text = hud_value(hud).unwrap_or_default();
                self.rolling_text(&text, s.hud_roll, &self.fonts.label_end, value, p.text, a);
            }
            Hud::LockKey { caps, on } => {
                let key = rect(x0 + 13.0, cy - 8.5, x0 + 31.0, cy + 8.5);
                if *on {
                    self.rounded(key, 4.5, p.accent, a);
                } else {
                    self.outline(key, 4.5, 1.2, false, p.muted, a);
                }
                self.text(if *caps { "A" } else { "1" }, &self.fonts.label_center, key, if *on { WHITE } else { p.muted }, a);
                let name = if *caps { "Caps Lock" } else { "Num Lock" };
                self.text(name, &self.fonts.label, row(x0 + 40.0, x1 - 70.0), p.text, a);
                let state = if *on { "Nyala" } else { "Mati" };
                self.text(state, &self.fonts.label_end, row(x1 - 70.0, x1 - 14.0), if *on { p.accent } else { p.muted }, a);
            }
            Hud::Power { plugged, percent, low } => {
                let (icon, color, label) = power_look(*plugged, *percent, *low, p.text);
                self.glyph(icon, &self.fonts.icon_16, x0 + 24.0, cy, color, a);
                self.text(label, &self.fonts.label, row(x0 + 42.0, x1 - 60.0), p.text, a);
                if let Some(percent) = percent {
                    self.text(&format!("{percent}%"), &self.fonts.label_end, row(x1 - 60.0, x1 - 14.0), color, a);
                }
            }
            Hud::Mic { muted } => {
                let (icon, color) = if *muted { (ICON_MIC_OFF, RED) } else { (ICON_MICROPHONE, GREEN) };
                self.glyph(icon, &self.fonts.icon_13, x0 + 22.0, cy, color, a);
                self.text("Mikrofon", &self.fonts.label, row(x0 + 38.0, x1 - 90.0), p.text, a);
                let state = if *muted { "Bisu" } else { "Menyala" };
                self.text(state, &self.fonts.label_end, row(x1 - 90.0, x1 - 14.0), color, a);
            }
            // Kabar lain selalu memakai bentuk kartu.
            _ => self.draw_hud_card(s, hud, x0, x1, a),
        }
    }

    fn draw_hud_card(&self, s: &Scene, hud: &Hud, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let cy = 22.0;
        let row = |left: f32, right: f32| rect(left, cy - 10.0, right, cy + 10.0);
        match hud {
            Hud::Volume { level, muted } => {
                let icon = if *muted { ICON_MUTE } else { volume_icon(*level) };
                self.glyph(icon, &self.fonts.icon_16, x0 + 27.0, cy, p.text, a);
                self.bar(x0 + 50.0, x1 - 62.0, cy, s.hud_fill, if *muted { p.muted } else { p.text }, a);
                let value = hud_value(hud).unwrap_or_default();
                self.rolling_text(&value, s.hud_roll, &self.fonts.label_end, row(x1 - 70.0, x1 - 20.0), p.text, a);
            }
            Hud::Power { plugged, percent, low } => {
                let (icon, color, label) = power_look(*plugged, *percent, *low, p.text);
                self.glyph(icon, &self.fonts.icon_20, x0 + 28.0, cy, color, a);
                self.text(label, &self.fonts.label, row(x0 + 52.0, x1 - 80.0), p.text, a);
                if let Some(percent) = percent {
                    self.text(&format!("{percent}%"), &self.fonts.label_end, row(x1 - 80.0, x1 - 20.0), color, a);
                }
            }
            Hud::Bluetooth { name, connected, battery, kind, .. } => {
                self.circle(x0 + 28.0, cy, 13.0, if *connected { BLUE } else { p.dim }, a);
                self.glyph(ICON_BLUETOOTH, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text(name, &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 52.0, 22.0), p.text, a);
                let status = match (*connected, battery) {
                    (true, Some(b)) => format!("Tersambung · {b}%"),
                    (true, None) => "Tersambung".to_string(),
                    (false, _) => "Terputus".to_string(),
                };
                self.text(&status, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 52.0, 37.0), p.muted, a);
                let device = match kind {
                    DeviceKind::Audio => ICON_HEADPHONE,
                    DeviceKind::Input => ICON_KEYBOARD,
                    DeviceKind::Phone => ICON_PHONE,
                    DeviceKind::Other => ICON_BLUETOOTH,
                };
                self.glyph(device, &self.fonts.icon_16, x1 - 28.0, cy, p.muted, a);
            }
            Hud::Focus { finished } => {
                // Warna menandai fase yang dimulai berikutnya.
                let (color, label, value) = match finished {
                    Phase::Focus => (GREEN, "Fokus selesai, istirahat dulu", "5 mnt"),
                    Phase::Break => (RED, "Istirahat selesai", "Siap fokus"),
                };
                self.glyph(ICON_STOPWATCH, &self.fonts.icon_16, x0 + 27.0, cy, color, a);
                self.text(label, &self.fonts.label, row(x0 + 50.0, x1 - 90.0), p.text, a);
                self.text(value, &self.fonts.caption_end, row(x1 - 100.0, x1 - 20.0), color, a);
            }
            Hud::Water { ml, goal } => {
                self.drop_icon(x0 + 27.0, cy, 16.0, BLUE, a);
                self.text("Waktunya minum air", &self.fonts.label, row(x0 + 50.0, x1 - 100.0), p.text, a);
                let value = format!("{} / {} L", liters(*ml), liters(*goal));
                self.text(&value, &self.fonts.caption_end, row(x1 - 110.0, x1 - 20.0), p.muted, a);
            }
            Hud::Privacy { camera, app } => {
                let (icon, color, label) = if *camera {
                    (ICON_CAMERA, GREEN, "Kamera sedang dipakai")
                } else {
                    (ICON_MICROPHONE, ORANGE, "Mikrofon sedang dipakai")
                };
                self.circle(x0 + 28.0, cy, 13.0, color, a);
                self.glyph(icon, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text(label, &self.fonts.label, row(x0 + 50.0, x1 - 110.0), p.text, a);
                self.text(app, &self.fonts.caption_end, row(x1 - 120.0, x1 - 20.0), p.muted, a);
            }
            Hud::Brightness { .. } => {
                self.glyph(ICON_BRIGHTNESS, &self.fonts.icon_16, x0 + 27.0, cy, p.text, a);
                self.bar(x0 + 50.0, x1 - 62.0, cy, s.hud_fill, p.text, a);
                let value = hud_value(hud).unwrap_or_default();
                self.rolling_text(&value, s.hud_roll, &self.fonts.label_end, row(x1 - 70.0, x1 - 20.0), p.text, a);
            }
            Hud::LockKey { caps, on } => {
                // Tutup tombol kecil berisi "A" (Caps Lock) atau "1" (Num Lock), menyala saat aktif.
                let key = rect(x0 + 17.0, cy - 10.0, x0 + 39.0, cy + 10.0);
                if *on {
                    self.rounded(key, 5.0, p.accent, a);
                } else {
                    self.outline(key, 5.0, 1.3, false, p.muted, a);
                }
                self.text(if *caps { "A" } else { "1" }, &self.fonts.label_center, key, if *on { WHITE } else { p.muted }, a);
                let name = if *caps { "Caps Lock" } else { "Num Lock" };
                self.text(name, &self.fonts.label, row(x0 + 52.0, x1 - 80.0), p.text, a);
                let state = if *on { "Nyala" } else { "Mati" };
                self.text(state, &self.fonts.label_end, row(x1 - 80.0, x1 - 20.0), if *on { p.accent } else { p.muted }, a);
            }
            Hud::Message { app, aumid, sender, text, .. } => {
                self.logo(aumid, x0 + 28.0, cy, 26.0, a);
                self.text(sender, &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 70.0, 22.0), p.text, a);
                self.text(app, &self.fonts.caption_end, rect(x1 - 90.0, 6.0, x1 - 20.0, 22.0), p.muted, a);
                self.text(text, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), p.soft, a);
            }
            Hud::Screenshot { id, name, more } => {
                self.thumb(*id, false, x0 + 30.0, cy - 15.0, 30.0, a);
                let title = if *more > 0 { format!("{} screenshot masuk shelf", more + 1) } else { "Screenshot masuk shelf".to_string() };
                self.text(&title, &self.fonts.label, rect(x0 + 54.0, 6.0, x1 - 20.0, 22.0), p.text, a);
                self.text(name, &self.fonts.caption, rect(x0 + 54.0, 22.0, x1 - 20.0, 37.0), p.muted, a);
            }
            Hud::Download { name, more, .. } => {
                self.circle(x0 + 28.0, cy, 13.0, BLUE, a);
                self.glyph(ICON_DOWNLOAD, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text("Unduhan selesai", &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 20.0, 22.0), p.text, a);
                let detail = if *more > 0 { format!("{name} +{more} lainnya") } else { name.clone() };
                self.text(&detail, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), p.muted, a);
            }
            Hud::Wifi { name, connected, internet } => {
                self.circle(x0 + 28.0, cy, 13.0, if *connected { BLUE } else { p.dim }, a);
                self.glyph(ICON_WIFI, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                let title = if name.is_empty() { "Wi-Fi" } else { name.as_str() };
                self.text(title, &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 20.0, 22.0), p.text, a);
                let (status, color) = match (*connected, *internet) {
                    (true, true) => ("Tersambung", p.muted),
                    (true, false) => ("Tersambung · tanpa akses internet", ORANGE),
                    (false, _) => ("Terputus", p.muted),
                };
                self.text(status, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), color, a);
            }
            Hud::Agenda { text, time } => {
                self.circle(x0 + 28.0, cy, 13.0, p.accent, a);
                self.glyph(ICON_CALENDAR, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text(text, &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 76.0, 22.0), p.text, a);
                self.text("Agenda sekarang", &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), p.muted, a);
                self.text(time, &self.fonts.label_end, rect(x1 - 76.0, 6.0, x1 - 20.0, 22.0), p.accent, a);
            }
            Hud::Timer { minutes } => {
                self.glyph(ICON_TIMER, &self.fonts.icon_16, x0 + 27.0, cy, ORANGE, a);
                self.text("Timer selesai", &self.fonts.label, row(x0 + 50.0, x1 - 100.0), p.text, a);
                self.text(&format!("{minutes} menit"), &self.fonts.caption_end, row(x1 - 100.0, x1 - 20.0), ORANGE, a);
            }
            Hud::Mic { muted } => {
                let (icon, color, label) =
                    if *muted { (ICON_MIC_OFF, RED, "Mikrofon dibisukan") } else { (ICON_MICROPHONE, GREEN, "Mikrofon menyala") };
                self.circle(x0 + 28.0, cy, 13.0, color, a);
                self.glyph(icon, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text(label, &self.fonts.label, row(x0 + 50.0, x1 - 20.0), p.text, a);
            }
            Hud::CallEnded { app, length } => {
                self.circle(x0 + 28.0, cy, 13.0, p.dim, a);
                self.glyph(ICON_PHONE, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text("Panggilan selesai", &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 90.0, 22.0), p.text, a);
                self.text(length, &self.fonts.label_end, rect(x1 - 100.0, 6.0, x1 - 20.0, 22.0), GREEN, a);
                let detail = if app.is_empty() { "Mikrofon sudah dilepas".to_string() } else { format!("{app} sudah melepas mikrofon") };
                self.text(&detail, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), p.muted, a);
            }
            Hud::Prayer { name, time, city } => {
                self.circle(x0 + 28.0, cy, 13.0, TEAL, a);
                self.glyph(ICON_PRAYER, &self.fonts.icon_13, x0 + 28.0, cy, WHITE, a);
                self.text(&format!("Waktunya sholat {name}"), &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 76.0, 22.0), p.text, a);
                self.text(city, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), p.muted, a);
                self.text(time, &self.fonts.label_end, rect(x1 - 76.0, 6.0, x1 - 20.0, 22.0), TEAL, a);
            }
        }
    }

    pub(super) fn draw_panel(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        // Seperti di Mac: isi panel muncul membesar dari 80% dengan titik tumpu di atas, sambil
        // sedikit turun.
        self.begin_zoom(0.8 + 0.2 * a, (x0 + x1) / 2.0);
        let dy = (1.0 - a) * -6.0;
        if s.welcome {
            self.set_offset(0.0, dy);
            self.welcome_page(s, x0, x1, a);
            self.end_zoom();
            return;
        }
        match s.page_from {
            Some((from, t, dir)) => {
                let eased = 1.0 - (1.0 - t).powi(3);
                self.page(s, from, x0, x1, a * (1.0 - eased), -dir * PAGE_SLIDE * eased, dy);
                self.page(s, s.page, x0, x1, a * eased, dir * PAGE_SLIDE * (1.0 - eased), dy);
            }
            None => self.page(s, s.page, x0, x1, a, 0.0, dy),
        }
        // Di lock screen hanya beranda (jam/musik) yang boleh dibuka.
        if !s.locked {
            self.navigation(s, x0, x1, a, dy);
        }
        self.end_zoom();
    }

    fn page(&self, s: &Scene, page: Page, x0: f32, x1: f32, a: f32, dx: f32, dy: f32) {
        if a < 0.01 {
            return;
        }
        self.set_offset(dx, dy);
        match page {
            Page::Home => self.home_page(s, x0, x1, a, dx, dy),
            Page::Controls => self.controls_page(s, x0, x1, a),
            Page::Shelf => self.shelf_page(s, x0, x1, a),
            Page::Focus => self.focus_page(s, x0, x1, a),
            Page::Timer => self.timer_page(s, x0, x1, a),
            Page::Water => self.water_page(s, x0, x1, a),
            Page::Tasks => self.tasks_page(s, x0, x1, a),
            Page::System => self.system_page(s, x0, x1, a),
            Page::Prayer => self.prayer_page(s, x0, x1, a),
        }
        self.set_offset(0.0, 0.0);
    }

    /// Beranda: pemutar musik atau jam di kiri. Kolom kanan berisi kalender, atau lirik yang
    /// bergeser masuk dari samping saat tombol lirik ditekan (kalender bergeser keluar). Tanpa
    /// kalender, notch sendiri yang melebar untuk kolom lirik.
    fn home_page(&self, s: &Scene, x0: f32, x1: f32, a: f32, dx: f32, dy: f32) {
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let mix = if s.lyrics.is_some() { s.lyric_mix.clamp(0.0, 1.0) } else { 0.0 };
        let full = if s.calendar.is_some() { HOME_LEFT } else { r - l };
        let right = l + lerp(full, LYRIC_LEFT, mix).min(r - l);
        // Selama panggilan, pemutar tetap tampil (panggilannya pindah ke kepala pemutar); tanpa
        // pemutar, kartu panggilan menggantikan jam.
        match (s.call, s.media) {
            (_, Some(m)) => {
                self.player_header(s, m, x0, x1, a);
                self.media_page(s, m, l, right, a);
            }
            (Some(call), None) => self.call_page(s, call, l, right, a),
            (None, None) => self.clock_page(s, l, right, a),
        }
        let divider = if s.calendar.is_some() { 1.0 } else { mix };
        if divider > 0.01 && r - right > 40.0 {
            self.rounded(rect(right + 11.0, 34.0, right + 12.0, 132.0), 0.5, self.tint(0.12), a * divider);
        }
        // Selama bergeser, isi kolom kanan tidak bisa diklik (letaknya belum tetap).
        let recording = self.recording.get();
        if mix > 0.0 && mix < 1.0 {
            self.recording.set(false);
        }
        self.push_clip(rect(right + 12.0, 0.0, r + 8.0, View::PanelWide.size().1));
        let (cl, cr) = (right + 24.0, r);
        // Kalender bergeser keluar dulu, baru lirik masuk; tanpa kalender, lirik masuk sambil
        // notch melebar.
        let leave = (mix * 2.0).min(1.0);
        let enter = if s.calendar.is_some() { (mix * 2.0 - 1.0).max(0.0) } else { mix };
        if let Some(cal) = s.calendar
            && leave < 0.999
        {
            self.set_offset(dx + COLUMN_SLIDE * leave, dy);
            self.calendar(s, cal, cl, cr, a * (1.0 - leave));
        }
        if let Some(lyrics) = s.lyrics
            && enter > 0.001
        {
            self.set_offset(dx + COLUMN_SLIDE * (1.0 - enter), dy);
            // Ditata selebar kolom akhirnya dan hanya terpotong selama notch masih melebar, supaya
            // barisnya tidak dibungkus ulang tiap frame.
            let width = View::PanelWide.size().0 - 2.0 * MARGIN - LYRIC_LEFT - 24.0;
            self.lyrics_column(s, lyrics, cl, cr.max(cl + width), a * enter);
        }
        self.set_offset(dx, dy);
        self.pop_clip();
        self.recording.set(recording);
    }

    /// Kepala pemutar: tanggal dan jam di tengah dengan cincin baterai, cuaca di kiri, dan
    /// gelombang musik di kanan. Selama panggilan, tanggal dan cuaca memberi tempat untuk
    /// panggilannya di kiri dan tombol bisu kecil di kanan.
    fn player_header(&self, s: &Scene, m: &MediaView, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let day = if s.call.is_some() { "" } else { s.day };
        // Tanpa tanggal, jarak sesudahnya juga hilang.
        let date_w = if day.is_empty() { -6.0 } else { self.text_width(day, &self.fonts.caption) };
        let time_w = self.text_width(s.time, &self.fonts.label);
        let ring = if s.battery.is_some() { 18.0 } else { 0.0 };
        let total = ring + date_w + 6.0 + time_w;
        let start = (x0 + x1) / 2.0 - total / 2.0;
        let mut x = start;
        if let Some((percent, plugged)) = s.battery {
            self.battery_ring(percent, plugged, x + 5.5, a);
            x += ring;
        }
        if !day.is_empty() {
            self.text(day, &self.fonts.caption, rect(x, HEADER_Y - 8.0, x + date_w + 2.0, HEADER_Y + 8.0), p.soft, a);
        }
        x += date_w + 6.0;
        let roll = s.home_fx.clock.as_ref().map(|(time, _, t)| (time.as_str(), *t));
        self.clock_text(s.time, roll, &self.fonts.label, rect(x, HEADER_Y - 9.0, x + time_w + 2.0, HEADER_Y + 9.0), p.text, a);
        if let Some(call) = s.call {
            self.header_call(s, call, l, start - 12.0, r - 32.0, a);
        } else if let Some(weather) = s.weather {
            self.text(weather, &self.fonts.caption, rect(l, HEADER_Y - 8.0, start - 12.0, HEADER_Y + 8.0), p.muted, a);
        }
        if m.playing {
            self.wave(r - 8.0, HEADER_Y, s.seconds, accent(m.accent, p.text), a);
        }
    }

    /// Panggilan di kepala pemutar: titik hijau, aplikasi, dan lama panggilan di antara `l` dan
    /// `r`, plus tombol bisu kecil berpusat di `button`.
    fn header_call(&self, s: &Scene, call: &CallView, l: f32, r: f32, button: f32, a: f32) {
        let p = &self.pal;
        self.circle(l + 3.5, HEADER_Y, 3.5, GREEN, a);
        let app = if call.app.is_empty() { "Panggilan" } else { call.app.as_str() };
        let label = format!("{app} · ");
        // Nama aplikasi yang panjang dipotong; lama panggilannya selalu utuh.
        let clock_w = self.text_width(&call.clock, &self.fonts.caption) + 2.0;
        let clock_l = (l + 12.0 + self.text_width(&label, &self.fonts.caption)).min(r - clock_w);
        self.text(&label, &self.fonts.caption, rect(l + 12.0, HEADER_Y - 8.0, clock_l, HEADER_Y + 8.0), p.soft, a);
        self.text(&call.clock, &self.fonts.caption, rect(clock_l, HEADER_Y - 8.0, clock_l + clock_w, HEADER_Y + 8.0), GREEN, a);

        let muted = call.muted == Some(true);
        let over = s.hovered == Some(Hit::CallMute);
        let fill = if muted { RED } else { self.tint(if over { 0.2 } else { 0.12 }) };
        self.circle(button, HEADER_Y, 10.0, fill, a);
        if muted && over {
            self.circle(button, HEADER_Y, 10.0, rgb(0xFFFFFF, 0.15), a);
        }
        let icon = if muted { ICON_MIC_OFF } else { ICON_MICROPHONE };
        self.glyph(icon, &self.fonts.icon_11, button, HEADER_Y, if muted { WHITE } else { p.text }, a);
        self.hit(rect(button - 13.0, HEADER_Y - 13.0, button + 13.0, HEADER_Y + 13.0), Hit::CallMute);
    }

    /// Cincin baterai kecil di kepala halaman: hijau saat mengisi, merah saat hampir habis.
    fn battery_ring(&self, percent: u32, plugged: bool, cx: f32, a: f32) {
        let color = if plugged {
            GREEN
        } else if percent <= 20 {
            RED
        } else {
            self.pal.soft
        };
        self.ring(cx, HEADER_Y, 5.0, 2.0, percent as f32 / 100.0, color, a);
    }

    /// Gambar ulang hanya gelombang musik di kepala pemutar, pada panel selebar `w` yang sedang
    /// diam. Mengembalikan area (piksel) yang berubah untuk [`Renderer::present`].
    pub fn draw_header_wave(&self, w: f32, seconds: f64, color: Option<u32>) -> Result<RECT> {
        let (cx, cy) = (CANVAS.0 / 2.0 + w / 2.0 - MARGIN - 8.0, HEADER_Y);
        self.patch(cx, cy, 10.0, 8.0, || self.wave(cx, cy, seconds, accent(color, self.pal.text), 1.0))
    }

    /// Kolom lirik: baris yang sedang dinyanyikan terang di tengah, baris sebelum dan sesudahnya
    /// redup. Saat baris berganti, semuanya bergulir naik satu baris.
    fn lyrics_column(&self, s: &Scene, lyrics: &LyricsView, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        if let Some(status) = lyrics.status {
            if !lyrics.retry {
                self.glyph(ICON_MUSIC, &self.fonts.icon_16, l + 8.0, LYRIC_CENTER, p.muted, a);
                self.text(status, &self.fonts.body, rect(l + 26.0, LYRIC_CENTER - 10.0, r, LYRIC_CENTER + 10.0), p.muted, a);
                return;
            }
            // Bisa diklik: ikon muat ulang dan ajakan mencoba lagi di bawahnya.
            let over = s.hovered == Some(Hit::LyricsRetry);
            let (y, color) = (LYRIC_CENTER - 8.0, if over { p.text } else { p.muted });
            self.glyph(ICON_REFRESH, &self.fonts.icon_16, l + 8.0, y, color, a);
            self.text(status, &self.fonts.body, rect(l + 26.0, y - 10.0, r, y + 10.0), color, a);
            let hint = if over { p.soft } else { p.dim };
            self.text("Klik untuk mencoba lagi", &self.fonts.caption, rect(l + 26.0, y + 10.0, r, y + 26.0), hint, a);
            let width = self.text_width(status, &self.fonts.body).max(self.text_width("Klik untuk mencoba lagi", &self.fonts.caption));
            self.hit(rect(l - 4.0, y - 16.0, (l + 30.0 + width).min(r), y + 28.0), Hit::LyricsRetry);
            return;
        }
        let eased = 1.0 - (1.0 - lyrics.scroll.clamp(0.0, 1.0)).powi(3);
        // Baris yang bergulir keluar tidak boleh menimpa kepala pemutar atau titik halaman.
        self.push_clip(rect(l, 32.0, r, 142.0));
        for (i, line) in lyrics.lines.iter().enumerate() {
            // Posisi terhadap baris tengah; selama bergulir, semuanya masih sedikit lebih rendah.
            let pos = i as f32 - 2.0 + (1.0 - eased);
            let visible = ((1.6 - pos.abs()) / 0.6).clamp(0.0, 1.0);
            if line.is_empty() || visible < 0.01 {
                continue;
            }
            let focus = (1.0 - pos.abs()).clamp(0.0, 1.0);
            let y = LYRIC_CENTER + pos * LYRIC_GAP;
            let color = blend(p.muted, p.text, focus);
            // Setinggi dua baris: baris yang panjang dibungkus, sisanya dipotong "…".
            self.text(line, &self.fonts.lyric_line, rect(l, y - 19.0, r, y + 19.0), color, a * visible);
        }
        self.pop_clip();
    }

    /// Sambutan saat notch pertama kali dijalankan: cara memakai notch dalam empat baris.
    fn welcome_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        self.text("Halo! Ini notch-mu", &self.fonts.title, rect(l, HEADER_Y - 10.0, r - 60.0, HEADER_Y + 10.0), p.text, a);
        self.eyes(r - 18.0, HEADER_Y + 2.0, 4.2, 12.0, 10.0, s.eyes, a);
        let tips = [
            "Sentuh notch dengan kursor untuk membukanya",
            "Roda mouse atau panah di tepi untuk pindah halaman",
            "Pengaturan: klik ikon maskot di system tray",
            "Lirik, cuaca, dan pesan masuk mati sampai kamu nyalakan",
        ];
        for (i, tip) in tips.iter().enumerate() {
            let y = 48.0 + i as f32 * 19.0;
            self.circle(l + 4.0, y, 2.5, p.accent, a);
            self.text(tip, &self.fonts.caption, rect(l + 14.0, y - 9.0, r, y + 9.0), p.soft, a);
        }
        let button = rect(r - 110.0, ROW_Y - 12.0, r, ROW_Y + 12.0);
        let over = s.hovered == Some(Hit::WelcomeDone);
        self.chip(button, "Mengerti", Some(if over { BLUE_LIGHT } else { p.accent }), WHITE, a);
        self.hit(button, Hit::WelcomeDone);
    }

    /// Panah kiri/kanan dan titik penanda halaman yang tampil.
    fn navigation(&self, s: &Scene, x0: f32, x1: f32, a: f32, dy: f32) {
        let p = &self.pal;
        let pages: Vec<Page> = s.pages.visible().collect();
        if pages.len() < 2 {
            return;
        }
        for (hit, icon, x) in [(Hit::PagePrev, ICON_CHEVRON_LEFT, x0 + 15.0), (Hit::PageNext, ICON_CHEVRON_RIGHT, x1 - 15.0)] {
            let color = if s.hovered == Some(hit) { p.text } else { p.muted };
            self.glyph(icon, &self.fonts.icon_11, x, ARROW_Y + dy, color, a);
            self.hit(rect(x - 13.0, ARROW_Y - 26.0, x + 13.0, ARROW_Y + 26.0), hit);
        }
        let cx = (x0 + x1) / 2.0;
        let spacing = 12.0;
        let first = cx - (pages.len() - 1) as f32 * spacing / 2.0;
        for (i, page) in pages.iter().enumerate() {
            let x = first + i as f32 * spacing;
            let (width, color) = if *page == s.page {
                (12.0, p.text)
            } else if s.hovered == Some(Hit::PageDot(*page)) {
                (5.0, p.soft)
            } else {
                (5.0, p.dim)
            };
            let y = DOTS_Y + dy;
            self.rounded(rect(x - width / 2.0, y - 2.5, x + width / 2.0, y + 2.5), 2.5, color, a);
            self.hit(rect(x - 6.0, DOTS_Y - 9.0, x + 6.0, DOTS_Y + 9.0), Hit::PageDot(*page));
        }
    }

    /// Judul halaman di kiri atas dan keterangan di kanan atas (bisa diklik kalau `right_hit` ada).
    fn header(&self, s: &Scene, icon: Option<char>, title: &str, right: &str, right_hit: Option<Hit>, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        if let Some(icon) = icon {
            self.glyph(icon, &self.fonts.icon_13, l + 6.0, HEADER_Y, p.muted, a);
        }
        self.text(title, &self.fonts.header, rect(l + 18.0, HEADER_Y - 9.0, r - 180.0, HEADER_Y + 9.0), p.text, a);
        let color = match right_hit {
            Some(hit) if s.hovered == Some(hit) => p.text,
            Some(_) => p.soft,
            None => p.muted,
        };
        self.text(right, &self.fonts.caption_end, rect(r - 190.0, HEADER_Y - 8.0, r, HEADER_Y + 8.0), color, a);
        if let Some(hit) = right_hit {
            let width = self.text_width(right, &self.fonts.caption_end);
            self.hit(rect(r - width - 6.0, HEADER_Y - 11.0, r + 4.0, HEADER_Y + 11.0), hit);
        }
    }

    /// Jam di Beranda, di antara `l` dan `r`.
    fn clock_page(&self, s: &Scene, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        // Sapaan, jam, dan tanggal naik bergantian saat halaman dibuka; angka jam yang berubah
        // bergulir saat menit berganti.
        let age = entering(s, Page::Home);
        let (shown, dy) = today::rise(age, 0);
        self.text(s.greeting, &self.fonts.greeting, rect(l, 34.0 + dy, r - 90.0, 52.0 + dy), p.muted, a * shown);
        let (shown, dy) = today::rise(age, 1);
        let roll = s.home_fx.clock.as_ref().map(|(time, _, t)| (time.as_str(), *t));
        self.clock_text(s.time, roll, &self.fonts.big_time, rect(l - 2.0, 50.0 + dy, r - 90.0, 102.0 + dy), p.text, a * shown);
        let (shown, dy) = today::rise(age, 2);
        let date = today::date_roll(s.home_fx.clock.as_ref(), s.date);
        self.rolling_text(s.date, date.as_ref(), &self.fonts.date, rect(l, 98.0 + dy, r - 90.0, 116.0 + dy), p.soft, a * shown);
        self.eyes(r - 40.0, 76.0, 13.0, 30.0, 25.0, s.eyes, a);
        // Kepala seperti di pemutar: cuaca di kiri, baterai di kanan. Di Beranda lebar keduanya
        // tampil di kepala kolom kalender.
        if s.calendar.is_some() {
            return;
        }
        if let Some(weather) = s.weather {
            self.text(weather, &self.fonts.caption, rect(l, HEADER_Y - 8.0, r - 80.0, HEADER_Y + 8.0), p.muted, a);
        }
        if let Some((percent, plugged)) = s.battery {
            self.battery_ring(percent, plugged, r - 6.0, a);
            self.text(&format!("{percent}%"), &self.fonts.caption_end, rect(r - 70.0, HEADER_Y - 8.0, r - 16.0, HEADER_Y + 8.0), p.soft, a);
        }
    }

    /// Kartu panggilan di Beranda, di antara `l` dan `r`: aplikasi, lama panggilan, status
    /// mikrofon dan kamera, dan tombol bisu besar di tempat mata maskot.
    fn call_page(&self, s: &Scene, call: &CallView, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        let age = entering(s, Page::Home);
        let (shown, dy) = today::rise(age, 0);
        self.circle(l + 4.0, 43.0 + dy, 3.5, GREEN, a * shown);
        let title = if call.app.is_empty() { "Sedang dalam panggilan".to_string() } else { format!("Panggilan di {}", call.app) };
        self.text(&title, &self.fonts.greeting, rect(l + 14.0, 34.0 + dy, r - 90.0, 52.0 + dy), p.muted, a * shown);
        let (shown, dy) = today::rise(age, 1);
        self.text(&call.clock, &self.fonts.big_time, rect(l - 2.0, 50.0 + dy, r - 90.0, 102.0 + dy), p.text, a * shown);
        let muted = call.muted == Some(true);
        let (mic, mic_color) = if muted { ("Mikrofon bisu", RED) } else { ("Mikrofon menyala", p.soft) };
        let camera = if call.camera { " · Kamera menyala" } else { " · Kamera mati" };
        let mic_w = self.text_width(mic, &self.fonts.date);
        let (shown, dy) = today::rise(age, 2);
        self.text(mic, &self.fonts.date, rect(l, 98.0 + dy, l + mic_w + 2.0, 116.0 + dy), mic_color, a * shown);
        self.text(camera, &self.fonts.date, rect(l + mic_w, 98.0 + dy, r - 90.0, 116.0 + dy), p.soft, a * shown);

        let (cx, cy) = (r - 40.0, 70.0);
        let over = s.hovered == Some(Hit::CallMute);
        let fill = if muted { RED } else { self.tint(if over { 0.2 } else { 0.12 }) };
        self.circle(cx, cy, 24.0, fill, a);
        if muted && over {
            self.circle(cx, cy, 24.0, rgb(0xFFFFFF, 0.15), a);
        }
        let icon = if muted { ICON_MIC_OFF } else { ICON_MICROPHONE };
        self.glyph(icon, &self.fonts.icon_20, cx, cy, if muted { WHITE } else { p.text }, a);
        let label = if muted { "Nyalakan" } else { "Bisukan" };
        self.text(label, &self.fonts.caption_center, rect(cx - 40.0, 98.0, cx + 40.0, 116.0), if over { p.text } else { p.soft }, a);
        self.hit(rect(cx - 30.0, cy - 30.0, cx + 30.0, 116.0), Hit::CallMute);

        // Kepala seperti jam: cuaca di kiri, baterai di kanan (di Beranda lebar keduanya ada di
        // kepala kolom kalender).
        if s.calendar.is_some() {
            return;
        }
        if let Some(weather) = s.weather {
            self.text(weather, &self.fonts.caption, rect(l, HEADER_Y - 8.0, r - 80.0, HEADER_Y + 8.0), p.muted, a);
        }
        if let Some((percent, plugged)) = s.battery {
            self.battery_ring(percent, plugged, r - 6.0, a);
            self.text(&format!("{percent}%"), &self.fonts.caption_end, rect(r - 70.0, HEADER_Y - 8.0, r - 16.0, HEADER_Y + 8.0), p.soft, a);
        }
    }

    /// Kolom kalender: bulan, lima hari yang meluncur, agenda hari terpilih (lihat `today.rs`),
    /// dan kotak tambah agenda.
    fn calendar(&self, s: &Scene, cal: &CalendarView, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        // Kepala: cuaca dan baterai di kanan, kecuali saat musik diputar (sudah ada di kepala
        // pemutar).
        let mut right = r;
        if let Some((percent, plugged)) = s.battery.filter(|_| s.media.is_none()) {
            self.battery_ring(percent, plugged, right - 6.0, a);
            self.text(&format!("{percent}%"), &self.fonts.caption_end, rect(right - 70.0, HEADER_Y - 8.0, right - 16.0, HEADER_Y + 8.0), p.soft, a);
            right -= 76.0;
        }
        if let Some(weather) = s.weather.filter(|_| s.media.is_none()) {
            self.text(weather, &self.fonts.caption_end, rect(l, HEADER_Y - 8.0, right, HEADER_Y + 8.0), p.muted, a);
        }

        self.calendar_strip(s, cal, l, r, a);
        self.agenda_list(s, cal, l, r, a);
        // Kotak tambah agenda; jumlah agenda yang tidak muat ditulis di ujung kanannya.
        let input = rect(l, ROW_Y - 10.0, r, ROW_Y + 10.0);
        let over = s.hovered == Some(Hit::AgendaInput);
        match s.agenda_draft {
            Some(draft) => self.typing_box(input, draft, "09.00 Rapat, Enter", a),
            None => {
                self.rounded(input, 10.0, self.tint(if over { 0.10 } else { 0.07 }), a);
                let inner = rect(input.left + 10.0, input.top, input.right - 10.0, input.bottom);
                self.text("+  Tambah agenda", &self.fonts.caption, inner, if over { p.soft } else { p.muted }, a);
                let hidden = cal.events.len().saturating_sub(cal.scroll + crate::widgets::agenda::VISIBLE);
                if hidden > 0 {
                    self.text(&format!("{hidden} lagi"), &self.fonts.caption_end, inner, p.muted, a);
                }
            }
        }
        self.hit(input, Hit::AgendaInput);
    }

    /// Volume, kecerahan, mikrofon, senyap, dan kota cuaca.
    fn controls_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let c = s.controls;
        let age = entering(s, Page::Controls);
        let right = c.quiet_minutes.map(|m| format!("Senyap {m} mnt lagi")).unwrap_or_default();
        // Tanpa hitung mundur senyap, kepala halaman berisi tombol ke jendela pengaturan.
        let (right, hit) = if right.is_empty() { ("Pengaturan".to_string(), Some(Hit::OpenSettings)) } else { (right, None) };
        self.header(s, Some(ICON_CONTROLS), "Kontrol cepat", &right, hit, x0, x1, a);
        // Sisa waktu senyap juga tampil sebagai cincin kecil yang menyusut tiap menit.
        if let Some(minutes) = c.quiet_minutes {
            let x = r - self.text_width(&right, &self.fonts.caption_end) - 10.0;
            self.ring(x, HEADER_Y, 4.5, 2.0, mixer::quiet_left(minutes), p.accent, a);
        }

        // Nilai baru diluncurkan, bukan dilompati, dan angkanya bergulir. Bilah volume memudar ke
        // abu-abu saat dibisukan sambil speakernya membal; matahari kecerahan membesar dan
        // berputar mengikuti nilainya.
        let label_alpha = mixer::label_in(age);
        let muted = c.volume.is_some_and(|(_, muted)| muted);
        let level = c.volume.map(|(level, _)| mixer::shown(level, c.fx.volume, 0, age));
        let (label, roll) = c.volume.map_or((String::new(), None), |(level, _)| mixer::label(level, muted, c.fx.volume, c.fx.mute));
        self.slider(s, l, r, a, SliderRow {
            y: 56.0,
            icon: if muted { ICON_MUTE } else { volume_icon(level.unwrap_or(0.67)) },
            icon_hit: Some(Hit::VolumeMute),
            icon_look: (mixer::speaker_pop(c.fx.mute), 0.0),
            hit: Hit::VolumeSlider,
            fill: level,
            color: blend(p.text, p.muted, mixer::muted_tone(muted, c.fx.mute)),
            label: &label,
            roll: roll.as_ref(),
            label_alpha,
        });
        let light = c.brightness.map(|b| mixer::shown(b as f32 / 100.0, c.fx.brightness, 1, age));
        let (label, roll) = c.brightness.map_or((String::new(), None), |b| mixer::label(b as f32 / 100.0, false, c.fx.brightness, None));
        self.slider(s, l, r, a, SliderRow {
            y: 90.0,
            icon: ICON_BRIGHTNESS,
            icon_hit: None,
            icon_look: light.map_or((1.0, 0.0), mixer::sun),
            hit: Hit::BrightnessSlider,
            fill: light,
            color: p.text,
            label: &label,
            roll: roll.as_ref(),
            label_alpha,
        });

        if let Some(draft) = s.city_draft {
            let area = rect(l, ROW_Y - 11.0, r, ROW_Y + 11.0);
            self.typing_box(area, draft, "Ketik nama kota, Enter untuk simpan", a);
            self.hit(area, Hit::CityInput);
            return;
        }
        // Tombol naik bergantian dari kiri saat halaman dibuka.
        let width = 120.0;
        let slot = |index: usize| {
            let x = l + index as f32 * (width + 8.0);
            let (shown, dy) = mixer::rise(age, index);
            (rect(x, ROW_Y - 11.0, x + width, ROW_Y + 11.0), a * shown, dy)
        };
        let lifted = |area: D2D_RECT_F, dy: f32| rect(area.left, area.top + dy, area.right, area.bottom + dy);
        let (area, alpha, dy) = slot(0);
        match c.mic {
            Some(muted) => {
                let hovered = s.hovered == Some(Hit::MicToggle);
                let icon = (ICON_MIC_OFF, ICON_MICROPHONE);
                let text = ("Mikrofon bisu", "Mikrofon nyala");
                self.switch(&mixer::Switch { area, on: muted, age: c.fx.mic, icon, text, color: ORANGE, hovered }, dy, alpha);
                self.hit(area, Hit::MicToggle);
            }
            None => self.chip(lifted(area, dy), "Tanpa mikrofon", None, p.dim, alpha),
        }
        let (area, alpha, dy) = slot(1);
        let hovered = s.hovered == Some(Hit::QuietToggle);
        let icon = (mixer::ICON_QUIET, mixer::ICON_QUIET);
        let text = ("Senyap aktif", "Senyap 1 jam");
        let on = c.quiet_minutes.is_some();
        self.switch(&mixer::Switch { area, on, age: c.fx.quiet, icon, text, color: p.accent, hovered }, dy, alpha);
        self.hit(area, Hit::QuietToggle);
        if let Some(city) = &c.city {
            let (area, alpha, dy) = slot(2);
            let over = s.hovered == Some(Hit::CityInput);
            let fill = self.tint(if over { 0.16 } else { 0.08 });
            self.chip(lifted(area, dy), city, Some(fill), if over { p.text } else { p.soft }, alpha);
            self.hit(area, Hit::CityInput);
        }
    }

    /// Baris slider: ikon (bisa diklik kalau ada `icon_hit`), jalur tanpa kenop, dan angkanya.
    /// Tanpa `fill`, barisnya hanya bertuliskan "tidak tersedia".
    fn slider(&self, s: &Scene, l: f32, r: f32, a: f32, row: SliderRow) {
        let p = &self.pal;
        let y = row.y;
        let over_icon = row.icon_hit.is_some() && s.hovered == row.icon_hit;
        if over_icon {
            self.circle(l + 8.0, y, 13.0, self.tint(0.12), a);
        }
        let (scale, turn) = row.icon_look;
        let color = if over_icon { p.text } else { p.soft };
        self.placed(l + 8.0, y, scale, 0.0, || {
            self.rotated(l + 8.0, y, turn, || self.glyph(row.icon, &self.fonts.icon_16, l + 8.0, y, color, a));
        });
        if let Some(icon_hit) = row.icon_hit {
            self.hit(rect(l - 6.0, y - 14.0, l + 22.0, y + 14.0), icon_hit);
        }
        let (start, end) = (l + 34.0, r - 48.0);
        let Some(fill) = row.fill else {
            self.text("tidak tersedia", &self.fonts.caption, rect(start, y - 8.0, r, y + 8.0), p.muted, a);
            return;
        };
        // Seperti bar lagu: tanpa kenop, menebal saat disorot atau diseret.
        let thick = if s.hovered == Some(row.hit) { 10.0 } else { 6.0 };
        self.bar_sized(start, end, y, thick, fill, row.color, a);
        let area = rect(r - 46.0, y - 10.0, r, y + 10.0);
        self.rolling_text(row.label, row.roll, &self.fonts.label_end, area, p.text, a * row.label_alpha);
        // Area klik selebar jalurnya persis, supaya posisi klik = nilai.
        self.hit(rect(start, y - 13.0, end, y + 13.0), row.hit);
    }

    /// Timer cepat: cincin sisa waktu, jam pasir, pilihan durasi, jeda, dan batal.
    fn timer_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let t = s.timer;
        let age = entering(s, Page::Timer);
        let right = if t.active { format!("Timer {} menit", t.total_minutes) } else { String::new() };
        self.header(s, Some(ICON_TIMER), "Timer", &right, None, x0, x1, a);

        let (cx, cy) = (l + 34.0, 76.0);
        // Jam pasir di kiri tombol batal; tulisan berhenti sebelum menabraknya.
        let gx = r - 66.0;
        let text_r = gx - 24.0;
        if t.active {
            if s.hovered == Some(Hit::TimerToggle) {
                self.circle(cx, cy, 24.0, self.tint(0.14), a);
            }
            self.ring(cx, cy, 30.0, 5.0, hourglass::ring(t, age), hourglass::accent(t), a);
            self.glyph(if t.running { ICON_PAUSE } else { ICON_PLAY }, &self.fonts.icon_20, cx, cy, p.text, a);
            self.hit(rect(cx - 34.0, cy - 34.0, cx + 34.0, cy + 34.0), Hit::TimerToggle);
            let clock = if t.ending { RED } else { p.text };
            self.text(&t.clock, &self.fonts.big_num, rect(l + 82.0, 52.0, text_r, 92.0), clock, a);
            let state = if t.running { "Berjalan" } else { "Dijeda" };
            self.text(state, &self.fonts.caption, rect(l + 84.0, 90.0, text_r, 106.0), p.muted, a);
            let (bx, by) = (r - 14.0, 72.0);
            let over = s.hovered == Some(Hit::TimerCancel);
            self.circle(bx, by, 15.0, self.tint(if over { 0.14 } else { 0.08 }), a);
            self.glyph(ICON_CLOSE, &self.fonts.icon_13, bx, by, if over { p.text } else { p.soft }, a);
            self.hit(rect(bx - 16.0, by - 16.0, bx + 16.0, by + 16.0), Hit::TimerCancel);
        } else {
            self.ring(cx, cy, 30.0, 5.0, 0.0, ORANGE, a);
            self.time_up(cx, cy, 30.0, 5.0, t, a);
            self.glyph(ICON_TIMER, &self.fonts.icon_20, cx, cy, p.soft, a);
            let (title, caption) = (rect(l + 82.0, 58.0, text_r, 78.0), rect(l + 82.0, 80.0, text_r, 96.0));
            // Sesaat setelah waktu habis, halamannya sendiri yang mengabarkan.
            let (done, usual) = hourglass::labels(t);
            if done > 0.0 {
                self.text("Waktu habis", &self.fonts.title, title, p.text, a * done);
                let total = format!("Timer {} menit selesai", t.total_minutes);
                self.text(&total, &self.fonts.caption, caption, p.muted, a * done);
            }
            if usual > 0.0 {
                self.text("Pilih durasi", &self.fonts.title, title, p.text, a * usual);
                self.text("Kabar dan alarm saat waktu habis", &self.fonts.caption, caption, p.muted, a * usual);
            }
        }
        self.hourglass(gx, t, age, a);

        let mut x = l;
        let presets = crate::widgets::timer::PRESETS.map(|m| (format!("{m} mnt"), Hit::TimerPreset(m)));
        let add = t.active.then(|| ("+1 mnt".to_string(), Hit::TimerAdd));
        for (label, hit) in presets.into_iter().chain(add) {
            let area = rect(x, ROW_Y - 11.0, x + 60.0, ROW_Y + 11.0);
            let over = s.hovered == Some(hit);
            let fill = self.tint(if over { 0.16 } else { 0.08 });
            self.chip(area, &label, Some(fill), if over { p.text } else { p.soft }, a);
            self.hit(area, hit);
            x += 68.0;
        }
    }

    /// Kotak ketik yang sedang aktif: teks digeser supaya ujung yang diketik tetap terlihat.
    fn typing_box(&self, area: D2D_RECT_F, draft: &str, placeholder: &str, a: f32) {
        let p = &self.pal;
        self.rounded(area, 11.0, self.tint(0.14), a);
        self.outline(area, 11.0, 1.0, false, p.accent, a);
        let inner = rect(area.left + 12.0, area.top, area.right - 12.0, area.bottom);
        let width = self.text_width(draft, &self.fonts.input);
        let start = if width > inner.right - inner.left { inner.right - width } else { inner.left };
        let cy = (area.top + area.bottom) / 2.0;
        self.push_clip(inner);
        self.text(draft, &self.fonts.input, rect(start, inner.top, start + width + 4.0, inner.bottom), p.text, a);
        let caret = start + width + 1.0;
        self.rounded(rect(caret, cy - 7.0, caret + 1.5, cy + 7.0), 0.75, p.text, a);
        if draft.is_empty() {
            self.text(placeholder, &self.fonts.input, rect(caret + 6.0, inner.top, inner.right, inner.bottom), p.muted, a);
        }
        self.pop_clip();
    }

    /// Pemutar musik di Beranda, di antara `l` dan `r`: sampul, judul, dan artis di atas; bar
    /// progres di tengah; tombol lirik dan tombol putar di bawah.
    fn media_page(&self, s: &Scene, m: &MediaView, l: f32, r: f32, a: f32) {
        let p = &self.pal;
        let age = entering(s, Page::Home);
        let track = s.home_fx.track.as_ref().map(|(old, t)| (old, *t)).filter(|(_, t)| *t < TRACK_SECS);
        self.cover(l, ART_TOP, ART_SIZE, 11.0, s.home_fx.art, age, a);
        // Judul dan artis naik saat halaman dibuka. Saat lagu berganti, yang lama keluar dan yang
        // baru masuk dari arah lompatannya. Tanpa artis (misal video), judul diletakkan di tengah
        // sampul.
        let text_l = l + ART_SIZE + 12.0;
        let titles = |title: &str, artist: &str, dx: f32, a: f32| {
            let (title_y, artist_y) = if artist.is_empty() { (ART_TOP + ART_SIZE / 2.0, 0.0) } else { (51.0, 70.0) };
            let (shown, dy) = today::rise(age, 1);
            self.text(title, &self.fonts.title, rect(text_l + dx, title_y - 10.0 + dy, r + dx, title_y + 10.0 + dy), p.text, a * shown);
            if !artist.is_empty() {
                let (shown, dy) = today::rise(age, 2);
                self.text(artist, &self.fonts.body, rect(text_l + dx, artist_y - 9.0 + dy, r + dx, artist_y + 9.0 + dy), p.muted, a * shown);
            }
        };
        match track {
            Some((old, t)) => {
                let ((out_dx, out_a), (in_dx, in_a)) = today::track_swap(old.dir, t);
                self.push_clip(rect(text_l - 2.0, ART_TOP, r + 4.0, ART_TOP + ART_SIZE));
                titles(&old.title, &old.artist, out_dx, a * out_a);
                titles(&m.title, &m.artist, in_dx, a * in_a);
                self.pop_clip();
            }
            None => titles(&m.title, &m.artist, 0.0, a),
        }

        // Waktu berjalan, bar, dan sisa waktu. Bar menebal saat disorot kalau bisa diseret.
        if let Some(progress) = m.progress {
            let y = PROGRESS_Y;
            self.text(&m.elapsed, &self.fonts.caption, rect(l, y - 8.0, l + 40.0, y + 8.0), p.soft, a);
            self.text(&m.remaining, &self.fonts.caption_end, rect(r - 44.0, y - 8.0, r, y + 8.0), p.soft, a);
            let (start, end) = (l + 42.0, r - 48.0);
            let active = m.can_seek && (m.seeking || s.hovered == Some(Hit::MediaSeek));
            // Menyapu dari kosong saat halaman dibuka, meluncur dari posisi lama saat lagu berganti.
            let shown = today::bar(progress, track, age);
            self.bar_sized(start, end, y, if active { 7.0 } else { 4.0 }, shown, p.text, a);
            if m.can_seek {
                // Area klik selebar bar persis, supaya posisi klik = posisi lagu.
                self.hit(rect(start, y - 9.0, end, y + 9.0), Hit::MediaSeek);
            }
        }

        // Tombol lirik di kiri (di bawah waktu berjalan), tombol putar di tengah.
        let y = CONTROLS_Y;
        if let Some(on) = s.lyrics_button {
            let (bx, hit) = (l + 10.0, Hit::LyricsToggle);
            let over = s.hovered == Some(hit);
            if on || over {
                self.circle(bx, y, 14.0, self.tint(if on { 0.16 } else { 0.10 }), a);
            }
            let icon = if on { ICON_LYRICS_ON } else { ICON_LYRICS };
            self.glyph(icon, &self.fonts.icon_13, bx, y + 0.5, if on || over { p.text } else { p.muted }, a);
            self.hit(rect(bx - 14.0, y - 14.0, bx + 14.0, y + 14.0), hit);
        }
        let cx = (l + r) / 2.0;
        for (button, bx) in [(Button::Previous, cx - 48.0), (Button::PlayPause, cx), (Button::Next, cx + 48.0)] {
            let enabled = match button {
                Button::Previous => m.can_previous,
                Button::Next => m.can_next,
                Button::PlayPause => true,
            };
            let hit = Hit::Media(button);
            if enabled && s.hovered == Some(hit) {
                self.circle(bx, y, 16.0, self.tint(0.14), a);
            }
            let (icon, font) = match button {
                Button::Previous => (ICON_PREVIOUS, &self.fonts.icon_16),
                Button::Next => (ICON_NEXT, &self.fonts.icon_16),
                Button::PlayPause => (if m.playing { ICON_PAUSE } else { ICON_PLAY }, &self.fonts.icon_20),
            };
            self.glyph(icon, font, bx, y, if enabled { p.text } else { p.dim }, a);
            if enabled {
                self.hit(rect(bx - 16.0, y - 16.0, bx + 16.0, y + 16.0), hit);
            }
        }
    }

    fn shelf_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let shelf = s.shelf;
        let count = shelf.items.len();
        let fx = s.shelf_fx;
        // Selama "Bersihkan" menyedot atau "Urungkan" memuntahkan, judul dan tulisan kanan
        // digambar sedotannya sendiri, dan ubinnya tidak bisa diklik.
        let flow = fx.flow.map(|(age, back)| vacuum::Flow::new(age, back, count, shelf.scroll, self.mouth_at(r, HEADER_Y)));
        let title = if count > SHELF_VISIBLE {
            let last = (shelf.scroll + SHELF_VISIBLE).min(count);
            format!("Shelf · {}–{last} dari {count}", shelf.scroll + 1)
        } else if count > 0 {
            format!("Shelf · {count}")
        } else {
            "Shelf".to_string()
        };
        match &flow {
            Some(flow) => {
                self.header(s, Some(ICON_FOLDER), "", "", None, x0, x1, a);
                self.flow_title(flow, l, r, a);
            }
            None if count > 0 => self.header(s, Some(ICON_FOLDER), &title, "Bersihkan", Some(Hit::ShelfClear), x0, x1, a),
            None => {
                self.header(s, Some(ICON_FOLDER), &title, "", None, x0, x1, a);
                if fx.undo {
                    self.undo_link(s, r, HEADER_Y, a);
                }
            }
        }

        let strip = rect(l, 38.0, r, 118.0);
        self.hit(strip, Hit::ShelfStrip);
        let age = entering(s, Page::Shelf);
        if count == 0 && !shelf.drop_hover {
            // Sesudah dibersihkan, tampilan kosong muncul pelan dan ikonnya mengangguk sekali.
            let (shown, nod) = fx.emptied.map_or((1.0, age), |e| (smooth(e / 0.25), e));
            let caption = fx.emptied.map_or(1.0, |e| smooth((e - 0.1) / 0.25));
            self.outline(strip, 12.0, 1.2, true, p.dim, a * shown);
            self.glyph(ICON_DOWNLOAD, &self.fonts.icon_20, (l + r) / 2.0, 66.0 + rack::nod(nod), p.muted, a * shown);
            let text = "Seret file ke notch untuk disimpan sementara";
            self.text(text, &self.fonts.caption_center, rect(l, 86.0, r, 102.0), p.muted, a * caption);
            self.drop_glow(strip, fx, a);
            return;
        }

        // Ubin berdiri di papan rak, yang memudar setelah semua ubin tersedot. Yang sedang
        // merapat atau digulir bisa lewat di luar deretan yang terlihat, jadi dipotong selama itu.
        self.rack(l, r, a * flow.as_ref().map_or(1.0, |f| f.board()));
        let sliding = !fx.slots.is_empty();
        if sliding {
            self.push_clip(rect(l - 6.0, 0.0, r + 6.0, 124.0));
        }
        for (index, item) in shelf.items.iter().enumerate() {
            let target = index as f32 - shelf.scroll as f32;
            let slot = fx.slots.get(&item.id).copied().unwrap_or(target);
            if slot <= -1.0 || slot >= SHELF_VISIBLE as f32 {
                continue;
            }
            let tx = l + slot * rack::TILE_STEP;
            if let Some(flow) = &flow {
                self.flowing_tile(flow, item, tx, target, fx.lean, a);
                continue;
            }
            let visible = (0.0..SHELF_VISIBLE as f32).contains(&target);
            let hovered = visible && matches!(s.hovered, Some(Hit::ShelfItem(id) | Hit::ShelfRemove(id)) if id == item.id);
            // Ubin baru jatuh ke rak, ubin yang dihapus terangkat, ubin yang disorot membesar 5%,
            // dan semuanya condong sedikit ke "Bersihkan" selama kursor di atasnya.
            let look = rack::Look { lean: vacuum::lean(fx.lean), ..rack::look(fx.tiles.get(&item.id).copied(), target, age) };
            self.shelf_tile(item, tx, look, hovered, a);
            if !visible {
                continue;
            }
            // Area klik selalu di tempat akhirnya, walau ubinnya masih merapat.
            let hx = l + target * rack::TILE_STEP;
            self.hit(rect(hx, 38.0, hx + 68.0, 118.0), Hit::ShelfItem(item.id));
            if hovered {
                let (bx, by) = (tx + 60.0, 45.0);
                let over = s.hovered == Some(Hit::ShelfRemove(item.id));
                self.circle(bx, by, 8.5, if over { RED } else { p.badge }, a);
                self.glyph(ICON_CLOSE, &self.fonts.icon_9, bx, by, if over { WHITE } else { p.text }, a);
                self.hit(rect(hx + 50.5, by - 9.5, hx + 69.5, by + 9.5), Hit::ShelfRemove(item.id));
            }
        }
        if sliding {
            self.pop_clip();
        }
        // Tulisan "Bersihkan" digambar sesudah ubin supaya ubin yang tersedot masuk ke dalamnya.
        if let Some(flow) = &flow {
            self.leftover(flow, l, fx.lean, a);
            self.mouth(flow, r, a);
        }
        if shelf.drop_hover {
            self.drop_zone(s, strip, count > 0, a);
        } else {
            let hint = "Klik 2x buka · klik kanan lihat di folder · seret keluar";
            let shown = flow.as_ref().map_or(1.0, |f| f.hint());
            self.text(hint, &self.fonts.caption_center, rect(l, ROW_Y - 8.0, r, ROW_Y + 8.0), p.muted, a * shown);
        }
        self.drop_glow(strip, fx, a);
    }

    /// Area lepas selama file diseret ke notch: garis putus-putus yang berjalan, menyala warna
    /// aksen saat file tepat di atasnya, dengan ikon yang mengangguk mengajak dilepas.
    fn drop_zone(&self, s: &Scene, area: D2D_RECT_F, adding: bool, a: f32) {
        let p = &self.pal;
        let fx = s.shelf_fx;
        if adding {
            // Ubin yang sudah ada diredupkan supaya tulisan ajakan terbaca.
            self.rounded(area, 12.0, with_alpha(p.body, 0.85), a);
        }
        // Dua zona seperti NotchNook: simpan di shelf (kiri) atau bagikan (kanan).
        let shelf = rect(area.left, area.top, area.right - 120.0, area.bottom);
        let share = rect(area.right - 112.0, area.top, area.right, area.bottom);
        let text = if adding { "Lepaskan untuk menambahkan" } else { "Lepaskan untuk menyimpan di shelf" };
        self.drop_target(s, shelf, fx.targeted, ICON_DOWNLOAD, text, a);
        self.drop_target(s, share, fx.share_targeted, ICON_SHARE, "Bagikan", a);
        self.hit(share, Hit::DropShare);
    }

    /// Satu zona lepas: garis putus-putus berjalan, menyala dan ikonnya mengangguk saat file
    /// tepat di atasnya.
    fn drop_target(&self, s: &Scene, area: D2D_RECT_F, targeted: bool, icon: char, text: &str, a: f32) {
        let p = &self.pal;
        let (fill, line) = if targeted { (0.22, 1.0) } else { (0.08, 0.55) };
        self.rounded(area, 12.0, with_alpha(p.accent, fill), a);
        self.marching(area, 12.0, 2.0, s.shelf_fx.dash, with_alpha(p.accent, line), a);
        let bob = if targeted { (s.seconds * 7.0).sin() as f32 * 2.5 } else { 0.0 };
        let color = if targeted { p.text } else { p.soft };
        self.glyph(icon, &self.fonts.icon_20, (area.left + area.right) / 2.0, 66.0 + bob, color, a);
        self.text(text, &self.fonts.caption_center, rect(area.left + 4.0, 86.0, area.right - 4.0, 102.0), color, a);
    }

    /// Pendaran singkat sesudah file dilepas: cincin aksen yang melebar sambil memudar.
    fn drop_glow(&self, area: D2D_RECT_F, fx: &ShelfFx, a: f32) {
        let Some(t) = fx.glow else { return };
        let (fade, grow) = (1.0 - t, 3.0 * t);
        let ring = rect(area.left - grow, area.top - grow, area.right + grow, area.bottom + grow);
        self.rounded(area, 12.0, with_alpha(self.pal.accent, 0.16 * fade), a);
        self.outline(ring, 12.0 + grow, 2.0, false, with_alpha(self.pal.accent, fade), a);
    }

    fn focus_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let f = s.focus;
        // Seperti timer dapur berbentuk tomat: cincin bertangkai dan berdaun, satu tomat kecil
        // untuk tiap sesi fokus hari ini.
        let age = entering(s, Page::Focus);
        self.header(s, Some(ICON_STOPWATCH), "Pomodoro", "", None, x0, x1, a);
        self.tomatoes(r, HEADER_Y, f.sessions, f.fx, age, a);

        // Cincin progres sekaligus tombol mulai/jeda.
        let (cx, cy) = (l + 34.0, 76.0);
        if s.hovered == Some(Hit::FocusToggle) {
            self.circle(cx, cy, 24.0, self.tint(0.14), a);
        }
        let progress = tomato::ring(if f.active { f.progress } else { 0.0 }, f.fx, age);
        self.ring(cx, cy, 30.0, 5.0, progress, phase_color(f.phase), a);
        self.sparkle(cx, cy, 30.0, f.fx, a);
        self.calyx(cx, cy, 30.0, f.fx, age, a);
        self.glyph(if f.running { ICON_PAUSE } else { ICON_PLAY }, &self.fonts.icon_20, cx, cy, p.text, a);
        self.hit(rect(cx - 34.0, cy - 34.0, cx + 34.0, cy + 34.0), Hit::FocusToggle);

        self.text(&f.clock, &self.fonts.big_num, rect(l + 82.0, 52.0, r - 44.0, 92.0), p.text, a);
        let phase = match f.phase {
            Phase::Focus => "Waktu fokus",
            Phase::Break => "Istirahat",
        };
        let state = if f.running {
            "berjalan"
        } else if f.active {
            "dijeda"
        } else {
            "siap dimulai"
        };
        self.text(&format!("{phase} · {state}"), &self.fonts.caption, rect(l + 84.0, 90.0, r - 44.0, 106.0), p.muted, a);

        if f.active {
            let (bx, by) = (r - 14.0, 72.0);
            let over = s.hovered == Some(Hit::FocusReset);
            self.circle(bx, by, 15.0, self.tint(if over { 0.14 } else { 0.08 }), a);
            self.glyph(ICON_REFRESH, &self.fonts.icon_13, bx, by, if over { p.text } else { p.soft }, a);
            self.hit(rect(bx - 16.0, by - 16.0, bx + 16.0, by + 16.0), Hit::FocusReset);
        }

        for (phase, label, left, right) in
            [(Phase::Focus, "Fokus 25 mnt", l + 82.0, l + 182.0), (Phase::Break, "Istirahat 5 mnt", l + 190.0, l + 302.0)]
        {
            let area = rect(left, ROW_Y - 11.0, right, ROW_Y + 11.0);
            let hit = Hit::FocusPhase(phase);
            let (fill, color) = if f.phase == phase {
                (Some(self.tint(0.16)), p.text)
            } else if s.hovered == Some(hit) {
                (Some(self.tint(0.08)), p.soft)
            } else {
                (None, p.muted)
            };
            self.chip(area, label, fill, color, a);
            self.hit(area, hit);
        }
    }

    fn water_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let w = s.water;
        let right = if w.ml >= w.goal {
            "Target hari ini tercapai".to_string()
        } else {
            w.next.as_ref().map(|t| format!("Pengingat berikutnya {t}")).unwrap_or_default()
        };
        self.drop_icon(l + 6.0, HEADER_Y, 13.0, BLUE, a);
        self.header(s, None, "Minum air", &right, None, x0, x1, a);

        self.text(&format!("{} L", liters(w.ml)), &self.fonts.big_num, rect(l, 42.0, l + 140.0, 80.0), p.text, a);
        let sub = format!("dari {} L · {} gelas", liters(w.goal), w.ml / GLASS_ML);
        self.text(&sub, &self.fonts.caption, rect(l + 2.0, 80.0, l + 146.0, 96.0), p.muted, a);

        // Grafik tujuh hari terakhir; hari ini paling kanan dan berwarna.
        let base = 92.0;
        let goal = w.goal.max(1) as f32;
        for (i, (ml, initial)) in w.week.iter().enumerate() {
            let x = l + 150.0 + i as f32 * 16.0;
            let today = i + 1 == w.week.len();
            let height = 4.0 + 36.0 * (*ml as f32 / goal).min(1.0);
            self.rounded(rect(x, base - height, x + 8.0, base), 3.0, if today { BLUE } else { self.tint(0.28) }, a);
            self.text(&initial.to_string(), &self.fonts.tiny, rect(x - 4.0, 97.0, x + 12.0, 109.0), if today { p.soft } else { p.muted }, a);
        }

        // Gelas yang terisi menggantikan cincin; animasi masuknya hanya untuk halaman yang
        // sedang dibuka, bukan yang sedang bergeser keluar.
        let done = (w.ml as f32 / goal).min(1.0);
        self.glass(r - 30.0, x1, done, w.pour, entering(s, Page::Water), a);
        self.text(&format!("{}%", (done * 100.0).round()), &self.fonts.label_center, rect(r - 60.0, 103.0, r, 117.0), p.text, a);

        let drink = rect(l, ROW_Y - 11.0, l + 136.0, ROW_Y + 11.0);
        let over = s.hovered == Some(Hit::WaterDrink);
        self.chip(drink, "+ Minum 250 ml", Some(if over { BLUE_LIGHT } else { BLUE }), WHITE, a);
        self.hit(drink, Hit::WaterDrink);
        if w.ml > 0 {
            let undo = rect(l + 144.0, ROW_Y - 11.0, l + 206.0, ROW_Y + 11.0);
            let over = s.hovered == Some(Hit::WaterUndo);
            self.chip(undo, "Batal", over.then(|| self.tint(0.10)), if over { p.text } else { p.muted }, a);
            self.hit(undo, Hit::WaterUndo);
        }
    }

    /// Jadwal sholat: waktu berikutnya dengan cincin kemajuan sejak waktu sebelumnya, lalu
    /// ketujuh waktu sehari dengan waktu berikutnya disorot.
    fn prayer_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let (cx, cy) = (l + 28.0, 66.0);
        let Some(v) = s.prayer else {
            self.header(s, Some(ICON_PRAYER), "Sholat", "", None, x0, x1, a);
            self.ring(cx, cy, 26.0, 5.0, 0.0, TEAL, a);
            self.glyph(ICON_PRAYER, &self.fonts.icon_20, cx, cy, p.soft, a);
            self.text("Pilih kota dulu", &self.fonts.title, rect(l + 70.0, 48.0, r, 68.0), p.text, a);
            let hint = "Jadwal dihitung di laptop ini dari kota cuaca";
            self.text(hint, &self.fonts.caption, rect(l + 70.0, 70.0, r, 86.0), p.muted, a);
            let area = rect(l, ROW_Y - 11.0, l + 96.0, ROW_Y + 11.0);
            let over = s.hovered == Some(Hit::PrayerCity);
            self.chip(area, "Pilih kota", Some(self.tint(if over { 0.16 } else { 0.08 })), if over { p.text } else { p.soft }, a);
            self.hit(area, Hit::PrayerCity);
            return;
        };
        // Warna, ikon, dan bintang mengikuti langit saat ini; cincin menyapu dan sorotan jadwal
        // bergeser saat halaman dibuka.
        let age = entering(s, Page::Prayer);
        let color = sky::sky_color(v.sky);
        if v.sky == Sky::Night {
            self.stars(l, age, a);
        }
        self.header(s, Some(ICON_PRAYER), "Sholat", &v.hijri, None, x0, x1, a);
        self.ring(cx, cy, 26.0, 5.0, sky::ring(v.progress, age), color, a);
        let icon = if v.sky.daylight() { ICON_BRIGHTNESS } else { ICON_PRAYER };
        self.glyph(icon, &self.fonts.icon_20, cx, cy, color, a);
        self.text(&v.next_time, &self.fonts.big_num, rect(l + 68.0, 38.0, r - 110.0, 78.0), p.text, a);
        let name = if v.tomorrow { format!("{} besok", v.next_name) } else { v.next_name.to_string() };
        self.text(&format!("{name} · {} lagi", v.left), &self.fonts.caption, rect(l + 70.0, 76.0, r - 96.0, 92.0), p.muted, a);
        self.text(&v.city, &self.fonts.caption_end, rect(r - 120.0, 50.0, r, 66.0), p.soft, a);
        if let Some(current) = v.current {
            self.text(&format!("Waktu {current}"), &self.fonts.caption_end, rect(r - 120.0, 66.0, r, 82.0), p.muted, a);
        }

        let w = (r - l) / v.times.len().max(1) as f32;
        let spot = l + w * sky::highlight(v.next, age);
        self.rounded(rect(spot + 2.0, 102.0, spot + w - 2.0, 138.0), 10.0, D2D1_COLOR_F { a: 0.22, ..color }, a);
        for (i, (name, time, passed)) in v.times.iter().enumerate() {
            let x = l + w * i as f32;
            let (name_color, time_color) = if i == v.next {
                (p.text, p.text)
            } else if *passed {
                (p.dim, p.muted)
            } else {
                (p.muted, p.soft)
            };
            self.text(name, &self.fonts.tiny, rect(x, 105.0, x + w, 117.0), name_color, a);
            self.text(time, &self.fonts.label_center, rect(x, 118.0, x + w, 134.0), time_color, a);
        }
    }

    /// Daftar tugas: centang yang tergambar, baris yang meluncur ke tempat barunya, cincin
    /// kemajuan hari ini, dan kotak tambah tugas.
    fn tasks_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let tasks = s.tasks;
        let fx = s.task_fx;
        let age = entering(s, Page::Tasks);
        let remaining = tasks.remaining();
        let right = if tasks.items.is_empty() {
            String::new()
        } else if remaining == 0 {
            "Semua selesai".to_string()
        } else {
            format!("{remaining} tersisa")
        };
        self.header(s, Some(ICON_TASKS), "Tugas", &right, None, x0, x1, a);
        if !tasks.items.is_empty() {
            self.progress_ring(r, HEADER_Y, &right, checklist::progress(tasks, fx, age), a);
            self.shine(r, HEADER_Y, &right, fx, a);
        }

        self.hit(rect(l - 6.0, 34.0, r + 6.0, 108.0), Hit::TaskList);
        let order = tasks.order();
        if order.is_empty() {
            let shown = a * checklist::settled(fx);
            self.text("Belum ada tugas. Tambahkan di bawah.", &self.fonts.caption_center, rect(l, 62.0, r, 80.0), p.muted, shown);
        }
        // Selama ada yang meluncur, baris yang lewat di luar daftar dipotong.
        if fx.is_some() {
            self.push_clip(rect(l - 8.0, 33.0, r + 8.0, 110.0));
        }
        // Tugas yang baru dicentang meluncur di atas baris lain seperti kartu yang diangkat, jadi
        // digambar paling akhir.
        let lifted = checklist::lifted(fx);
        let draw_row = |pos: usize, index: usize, card: f32| {
            let task = &tasks.items[index];
            let row = pos as f32 - tasks.scroll as f32;
            let visible = (tasks.scroll..tasks.scroll + TASKS_VISIBLE).contains(&pos);
            let y = checklist::slide(index, pos, pos + 1 == order.len(), tasks.scroll, fx);
            if !visible && fx.is_none() || !(20.0..125.0).contains(&y) {
                return;
            }
            let (shown, dy) = checklist::arrive(index, row, fx, age);
            let hovered = visible && matches!(s.hovered, Some(Hit::TaskToggle(i) | Hit::TaskRemove(i)) if i == index);
            let y = y + dy;
            let band = rect(l - 6.0, y - 11.0, r + 6.0, y + 11.0);
            if card > 0.0 {
                self.rounded(band, 8.0, p.body, a * card);
                self.rounded(band, 8.0, self.tint(0.08), a * card);
            } else if hovered {
                self.rounded(band, 8.0, self.tint(0.06), a * shown);
            }
            let mark = checklist::mark(index, task.done, row, fx, age);
            self.task_row(l, r, y, &task.text, mark, hovered, a * shown);
            if hovered {
                let over = s.hovered == Some(Hit::TaskRemove(index));
                self.glyph(ICON_CLOSE, &self.fonts.icon_9, r - 8.0, y, if over { RED } else { p.muted }, a);
            }
            // Area klik selalu di tempat akhirnya, walau barisnya masih meluncur.
            if visible {
                let settled = checklist::row_y(row);
                self.hit(rect(l - 6.0, settled - 11.0, r - 20.0, settled + 11.0), Hit::TaskToggle(index));
                if hovered {
                    self.hit(rect(r - 20.0, settled - 11.0, r + 6.0, settled + 11.0), Hit::TaskRemove(index));
                }
            }
        };
        for (pos, &index) in order.iter().enumerate() {
            if lifted.is_none_or(|(i, _)| i != index) {
                draw_row(pos, index, 0.0);
            }
        }
        for (text, done, y, dx, shown) in checklist::gone(fx) {
            if shown > 0.0 {
                self.task_row(l + dx, r + dx, y, text, checklist::Mark::flat(done), false, a * shown);
            }
        }
        if let Some((index, card)) = lifted
            && let Some(pos) = order.iter().position(|&i| i == index)
        {
            draw_row(pos, index, card);
        }
        if fx.is_some() {
            self.pop_clip();
        }
        // Konfeti dari tempat centangnya diklik, di atas daftar.
        if let Some((TaskFx { change: TaskChange::Check { index, .. }, rows, scroll, .. }, _)) = fx
            && let Some(Some(row)) = rows.get(*index)
        {
            self.confetti(l + 9.0, checklist::row_y(*row as f32 - *scroll as f32), fx, a);
        }
        // Penanda masih ada tugas di atas/bawah; digulir dengan roda mouse.
        if tasks.scroll > 0 {
            self.glyph(ICON_CHEVRON_UP, &self.fonts.icon_9, (l + r) / 2.0, 34.5, p.muted, a);
        }
        if order.len() > tasks.scroll + TASKS_VISIBLE {
            self.glyph(ICON_CHEVRON_DOWN, &self.fonts.icon_9, (l + r) / 2.0, 108.0, p.muted, a);
        }

        // Kotak tambah tugas, dan tombol hapus yang selesai.
        let has_done = tasks.has_done();
        let input = rect(l, ROW_Y - 11.0, if has_done { r - 112.0 } else { r }, ROW_Y + 11.0);
        let over = s.hovered == Some(Hit::TaskInput);
        match &tasks.draft {
            Some(draft) => self.typing_box(input, draft, "Ketik tugas, Enter untuk simpan", a),
            None => {
                self.rounded(input, 11.0, self.tint(if over { 0.10 } else { 0.07 }), a);
                let area = rect(input.left + 12.0, input.top, input.right - 12.0, input.bottom);
                self.text("+  Tambah tugas", &self.fonts.body, area, if over { p.soft } else { p.muted }, a);
            }
        }
        self.hit(input, Hit::TaskInput);
        if has_done {
            let clear = rect(r - 104.0, ROW_Y - 11.0, r, ROW_Y + 11.0);
            let over = s.hovered == Some(Hit::TasksClearDone);
            self.chip(clear, "Hapus selesai", over.then(|| self.tint(0.10)), if over { p.text } else { p.muted }, a);
            self.hit(clear, Hit::TasksClearDone);
        }
    }

    /// Monitor sistem: empat cincin beban, kecepatan jaringan, dan status privasi.
    fn system_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        // Data baru tiap detik: titik di kepala berdetak, cincin menghitung naik saat data pertama
        // tiba, dan memindai selagi menunggunya.
        let age = entering(s, Page::System);
        let uptime = format!("Menyala {}", s.uptime);
        self.header(s, Some(ICON_SYSTEM), "Sistem", &uptime, None, x0, x1, a);
        if s.system.is_some() {
            self.beat(r, HEADER_Y, &uptime, s.trend.beat, a);
        }

        let percent = |used: u64, total: u64| if total > 0 { used as f32 / total as f32 * 100.0 } else { 0.0 };
        // Suhu GPU di bawah namanya; oranye mulai 78 °C, merah mulai 87 °C.
        let gpu_detail = |st: &Stats| match (st.gpu, st.gpu_temp) {
            (None, _) => ("tidak tersedia".to_string(), p.muted),
            (Some(_), Some(t)) => (format!("{t:.0}°C"), if t >= 87.0 { RED } else if t >= 78.0 { ORANGE } else { p.muted }),
            (Some(_), None) => (String::new(), p.muted),
        };
        let meters: [(&str, Option<f32>, (String, D2D1_COLOR_F)); 4] = match s.system {
            Some(st) => [
                ("CPU", Some(st.cpu), (String::new(), p.muted)),
                ("GPU", st.gpu, gpu_detail(st)),
                ("RAM", Some(percent(st.ram_used, st.ram_total)), (format!("{} / {}", gigabytes(st.ram_used), gigabytes(st.ram_total)), p.muted)),
                ("Disk C:", Some(percent(st.disk_total.saturating_sub(st.disk_free), st.disk_total)), (format!("{} bebas", gigabytes(st.disk_free)), p.muted)),
            ],
            None => ["CPU", "GPU", "RAM", "Disk C:"].map(|name| (name, None, ("mengukur…".to_string(), p.muted))),
        };
        let spacing = (r - l) / 4.0;
        for (i, (name, value, (detail, detail_color))) in meters.iter().enumerate() {
            let cx = l + spacing * (i as f32 + 0.5);
            let shown = value.map(|v| v * gauge::count_up(i, age));
            let color = shown.map_or(p.dim, |v| load_color(v, p.accent));
            self.ring(cx, 64.0, 20.0, 4.0, shown.unwrap_or(0.0) / 100.0, color, a);
            if value.is_none()
                && let Some(t) = s.trend.scan
            {
                self.scan(i, cx, 64.0, 20.0, 4.0, t, a);
            }
            // Riwayat CPU di tempat keterangan yang kosong.
            if i == 0 {
                self.sparkline(cx, &s.trend.cpu, color, a);
            }
            let label = shown.map_or("–".to_string(), |v| format!("{}%", v.round()));
            self.text(&label, &self.fonts.label_center, rect(cx - 20.0, 54.0, cx + 20.0, 74.0), p.text, a);
            self.text(name, &self.fonts.caption_center, rect(cx - spacing / 2.0, 90.0, cx + spacing / 2.0, 104.0), p.soft, a);
            self.text(detail, &self.fonts.tiny, rect(cx - spacing / 2.0, 104.0, cx + spacing / 2.0, 116.0), *detail_color, a);
        }

        // Baris bawah: jaringan di kiri, privasi di kanan.
        let network = s.system.map_or(String::new(), |st| format!("↓ {}   ↑ {}", rate(st.down), rate(st.up)));
        self.text(&network, &self.fonts.caption, rect(l, ROW_Y - 8.0, l + 170.0, ROW_Y + 8.0), p.muted, a);
        let privacy = s.privacy;
        let (icon, color, text) = match (privacy.camera.first(), privacy.microphone.first()) {
            (Some(app), _) => (ICON_CAMERA, GREEN, format!("Kamera: {app}")),
            (None, Some(app)) => (ICON_MICROPHONE, ORANGE, format!("Mikrofon: {app}")),
            (None, None) => (ICON_MICROPHONE, p.muted, "Kamera & mikrofon tidak dipakai".to_string()),
        };
        self.text(&text, &self.fonts.caption_end, rect(l + 176.0, ROW_Y - 8.0, r, ROW_Y + 8.0), color, a);
        let width = self.text_width(&text, &self.fonts.caption_end).min(r - l - 190.0);
        self.glyph(icon, &self.fonts.icon_11, r - width - 10.0, ROW_Y, color, a);
    }
}

/// Satu baris slider di halaman Kontrol.
struct SliderRow<'a> {
    y: f32,
    icon: char,
    /// Ikon bisa diklik (bisu speaker).
    icon_hit: Option<Hit>,
    /// Skala dan putaran ikon.
    icon_look: (f32, f32),
    hit: Hit,
    /// Isi bilah yang tampil (0..1); `None` = tidak tersedia.
    fill: Option<f32>,
    color: D2D1_COLOR_F,
    label: &'a str,
    roll: Option<&'a Roll>,
    /// Kejelasan angkanya (muncul pelan saat halaman dibuka).
    label_alpha: f32,
}

/// Ikon speaker sesuai tingkat volume.
fn volume_icon(level: f32) -> char {
    if level < 0.005 { ICON_VOLUME[0] } else { ICON_VOLUME[((level * 3.0).ceil() as usize).clamp(1, 3)] }
}

/// Ikon baterai, warnanya, dan tulisan untuk kabar daya.
fn power_look(plugged: bool, percent: Option<u32>, low: bool, normal: D2D1_COLOR_F) -> (char, D2D1_COLOR_F, &'static str) {
    let level = percent.map_or(10, |p| ((p + 5) / 10).min(10));
    let base = if plugged { ICON_BATTERY_CHARGING } else { ICON_BATTERY };
    let icon = char::from_u32(base + level).unwrap_or(ICON_MUTE);
    let color = if low {
        RED
    } else if plugged {
        GREEN
    } else {
        normal
    };
    let label = match (plugged, low, percent) {
        (_, true, _) => "Baterai lemah",
        (true, _, Some(100)) => "Baterai penuh",
        (true, _, _) => "Mengisi daya",
        (false, _, _) => "Memakai baterai",
    };
    (icon, color, label)
}
