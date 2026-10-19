//! Tata letak isi notch: pil, HUD, dan halaman-halaman panel. Semua koordinat dalam DIP
//! relatif terhadap kanvas; `x0`/`x1` adalah tepi kiri/kanan badan notch saat ini.

use super::*;
use crate::sysinfo::{gigabytes, rate};
use crate::widgets::shelf::VISIBLE as SHELF_VISIBLE;
use crate::widgets::tasks::VISIBLE as TASKS_VISIBLE;
use crate::widgets::water::{GLASS_ML, liters};

/// Jarak isi panel dari tepinya; sisanya untuk panah pindah halaman.
const MARGIN: f32 = 32.0;
const HEADER_Y: f32 = 22.0;
const ARROW_Y: f32 = 84.0;
const DOTS_Y: f32 = 146.0;
/// Baris tombol di bagian bawah halaman.
const ROW_Y: f32 = 128.0;
/// Jarak geser isi saat berpindah halaman.
const PAGE_SLIDE: f32 = 28.0;
const BUTTON_RADIUS: f32 = 17.0;

impl Renderer {
    pub(super) fn draw_pill(&self, s: &Scene, x0: f32, x1: f32, h: f32, a: f32) {
        let p = &self.pal;
        let cy = h / 2.0;
        let playing = s.media.filter(|m| m.playing);
        let focus = Some(s.focus).filter(|f| f.running);
        // Kiri: cincin pomodoro, sampul lagu, atau mata maskot.
        if let Some(f) = focus {
            self.ring(x0 + 18.0, cy, 6.5, 2.2, f.progress, phase_color(f.phase), a);
        } else if playing.is_some() {
            self.art(x0 + 9.0, cy - 9.0, 18.0, 5.0, a);
        } else {
            self.eyes(x0 + 18.0, cy, 3.4, 9.0, 7.5, s.eyes_open, a);
        }
        // Kanan: gelombang musik, atau sisa menit pomodoro; paling kanan titik privasi.
        match (playing, focus) {
            (Some(m), _) => self.wave(x1 - 22.0, cy, s.seconds, accent(m.accent, p.text), a),
            (None, Some(f)) => {
                let left = format!("{}m", f.minutes_left);
                self.text(&left, &self.fonts.caption_end, rect(x1 - 52.0, 0.0, x1 - 13.0, h), phase_color(f.phase), a);
            }
            (None, None) => {}
        }
        self.privacy_dot(s.privacy, x1 - 8.0, cy, a);
        self.text(s.time, &self.fonts.pill_time, rect(x0 + 34.0, 0.0, x1 - 34.0, h), p.text, a);
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
        self.privacy_dot(s.privacy, x1 - 8.0, cy, a);
    }

    pub(super) fn draw_hud(&self, hud: &Hud, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let cy = 22.0;
        let row = |left: f32, right: f32| rect(left, cy - 10.0, right, cy + 10.0);
        match hud {
            Hud::Volume { level, muted } => {
                let icon = if *muted {
                    ICON_MUTE
                } else if *level < 0.005 {
                    ICON_VOLUME[0]
                } else {
                    ICON_VOLUME[((level * 3.0).ceil() as usize).clamp(1, 3)]
                };
                self.glyph(icon, &self.fonts.icon_16, x0 + 27.0, cy, p.text, a);
                self.bar(x0 + 50.0, x1 - 62.0, cy, *level, if *muted { p.muted } else { p.text }, a);
                let value = if *muted { "Senyap".to_string() } else { format!("{}", (level * 100.0).round()) };
                self.text(&value, &self.fonts.label_end, row(x1 - 70.0, x1 - 20.0), p.text, a);
            }
            Hud::Power { plugged, percent, low } => {
                let level = percent.map_or(10, |p| ((p + 5) / 10).min(10));
                let base = if *plugged { ICON_BATTERY_CHARGING } else { ICON_BATTERY };
                let icon = char::from_u32(base + level).unwrap_or(ICON_MUTE);
                let color = if *low {
                    RED
                } else if *plugged {
                    GREEN
                } else {
                    p.text
                };
                let label = match (*plugged, *low, *percent) {
                    (_, true, _) => "Baterai lemah",
                    (true, _, Some(100)) => "Baterai penuh",
                    (true, _, _) => "Mengisi daya",
                    (false, _, _) => "Memakai baterai",
                };
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
            Hud::Brightness { level } => {
                self.glyph(ICON_BRIGHTNESS, &self.fonts.icon_16, x0 + 27.0, cy, p.text, a);
                self.bar(x0 + 50.0, x1 - 62.0, cy, *level, p.text, a);
                self.text(&format!("{}", (level * 100.0).round()), &self.fonts.label_end, row(x1 - 70.0, x1 - 20.0), p.text, a);
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
            Hud::Message { app, aumid, sender, text } => {
                self.logo(aumid, x0 + 28.0, cy, 26.0, a);
                self.text(sender, &self.fonts.label, rect(x0 + 50.0, 6.0, x1 - 70.0, 22.0), p.text, a);
                self.text(app, &self.fonts.caption_end, rect(x1 - 90.0, 6.0, x1 - 20.0, 22.0), p.muted, a);
                self.text(text, &self.fonts.caption, rect(x0 + 50.0, 22.0, x1 - 20.0, 37.0), p.soft, a);
            }
        }
    }

    pub(super) fn draw_panel(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        // Isi panel sedikit turun saat muncul.
        let dy = (1.0 - a) * -6.0;
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
    }

    fn page(&self, s: &Scene, page: Page, x0: f32, x1: f32, a: f32, dx: f32, dy: f32) {
        if a < 0.01 {
            return;
        }
        self.set_offset(dx, dy);
        match page {
            Page::Home => match s.media {
                Some(m) => self.media_page(s, m, x0, x1, a),
                None => self.clock_page(s, x0, x1, a),
            },
            Page::Shelf => self.shelf_page(s, x0, x1, a),
            Page::Focus => self.focus_page(s, x0, x1, a),
            Page::Water => self.water_page(s, x0, x1, a),
            Page::Tasks => self.tasks_page(s, x0, x1, a),
            Page::System => self.system_page(s, x0, x1, a),
        }
        self.set_offset(0.0, 0.0);
    }

    /// Panah kiri/kanan dan titik penanda halaman.
    fn navigation(&self, s: &Scene, x0: f32, x1: f32, a: f32, dy: f32) {
        let p = &self.pal;
        for (hit, icon, x) in [(Hit::PagePrev, ICON_CHEVRON_LEFT, x0 + 15.0), (Hit::PageNext, ICON_CHEVRON_RIGHT, x1 - 15.0)] {
            let color = if s.hovered == Some(hit) { p.text } else { p.muted };
            self.glyph(icon, &self.fonts.icon_11, x, ARROW_Y + dy, color, a);
            self.hit(rect(x - 13.0, ARROW_Y - 26.0, x + 13.0, ARROW_Y + 26.0), hit);
        }
        let cx = (x0 + x1) / 2.0;
        let spacing = 12.0;
        let first = cx - (Page::ALL.len() - 1) as f32 * spacing / 2.0;
        for (i, page) in Page::ALL.iter().enumerate() {
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
            self.glyph(icon, &self.fonts.icon_13, l + 6.0, HEADER_Y, p.soft, a);
        }
        self.text(title, &self.fonts.header, rect(l + 18.0, HEADER_Y - 9.0, r - 180.0, HEADER_Y + 9.0), p.soft, a);
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

    fn clock_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        self.text(s.greeting, &self.fonts.greeting, rect(l, 34.0, r - 90.0, 52.0), p.muted, a);
        self.text(s.time, &self.fonts.big_time, rect(l - 2.0, 50.0, r - 90.0, 102.0), p.text, a);
        self.text(s.date, &self.fonts.date, rect(l, 98.0, r - 90.0, 116.0), p.soft, a);
        self.eyes(r - 40.0, 76.0, 13.0, 30.0, 25.0, s.eyes_open, a);
    }

    fn media_page(&self, s: &Scene, m: &MediaView, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let tint = accent(m.accent, p.text);

        // Kepala: aplikasi pemutar di kiri, jam di kanan.
        self.glyph(ICON_MUSIC, &self.fonts.icon_13, l + 6.0, HEADER_Y, accent(m.accent, p.muted), a);
        self.text(&m.source, &self.fonts.header, rect(l + 18.0, HEADER_Y - 9.0, r - 120.0, HEADER_Y + 9.0), p.muted, a);
        self.text(s.time, &self.fonts.caption_end, rect(r - 120.0, HEADER_Y - 8.0, r, HEADER_Y + 8.0), p.soft, a);

        // Sampul, judul, artis, dan baris lirik kalau ada. Tanpa artis (misal video), judul
        // diletakkan di tengah sampul.
        self.art(l, 38.0, 64.0, 12.0, a);
        let (title_y, artist_y) = match (m.artist.is_empty(), s.lyric.is_some()) {
            (true, false) => (70.0, 0.0),
            (true, true) => (56.0, 0.0),
            (false, false) => (59.0, 80.0),
            (false, true) => (50.0, 69.0),
        };
        self.text(&m.title, &self.fonts.title, rect(l + 78.0, title_y - 10.0, r - 118.0, title_y + 10.0), p.text, a);
        if !m.artist.is_empty() {
            self.text(&m.artist, &self.fonts.body, rect(l + 78.0, artist_y - 9.0, r - 118.0, artist_y + 9.0), p.muted, a);
        }
        if let Some(line) = s.lyric {
            // Sampai dua baris; baris yang lebih panjang lagi dipotong "…".
            self.text(line, &self.fonts.lyric_block, rect(l + 78.0, 80.0, r, 112.0), tint, a);
        }

        // Tombol putar.
        for (button, offset) in [(Button::Previous, 100.0), (Button::PlayPause, 60.0), (Button::Next, 20.0)] {
            let (bx, by) = (r - offset, if s.lyric.is_some() { 62.0 } else { 70.0 });
            let enabled = match button {
                Button::Previous => m.can_previous,
                Button::Next => m.can_next,
                Button::PlayPause => true,
            };
            let hit = Hit::Media(button);
            if enabled && s.hovered == Some(hit) {
                self.circle(bx, by, BUTTON_RADIUS, self.tint(0.14), a);
            }
            let (icon, font) = match button {
                Button::Previous => (ICON_PREVIOUS, &self.fonts.icon_16),
                Button::Next => (ICON_NEXT, &self.fonts.icon_16),
                Button::PlayPause => (if m.playing { ICON_PAUSE } else { ICON_PLAY }, &self.fonts.icon_22),
            };
            self.glyph(icon, font, bx, by, if enabled { p.text } else { p.dim }, a);
            if enabled {
                self.hit(rect(bx - BUTTON_RADIUS, by - BUTTON_RADIUS, bx + BUTTON_RADIUS, by + BUTTON_RADIUS), hit);
            }
        }

        // Progres.
        if let Some(progress) = m.progress {
            let cy = 124.0;
            self.text(&m.elapsed, &self.fonts.caption, rect(l, cy - 8.0, l + 40.0, cy + 8.0), p.soft, a);
            self.bar(l + 44.0, r - 44.0, cy, progress, tint, a);
            self.text(&m.remaining, &self.fonts.caption_end, rect(r - 40.0, cy - 8.0, r, cy + 8.0), p.soft, a);
        }
    }

    fn shelf_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let shelf = s.shelf;
        let count = shelf.items.len();
        let title = if count > SHELF_VISIBLE {
            let last = (shelf.scroll + SHELF_VISIBLE).min(count);
            format!("Shelf · {}–{last} dari {count}", shelf.scroll + 1)
        } else if count > 0 {
            format!("Shelf · {count}")
        } else {
            "Shelf".to_string()
        };
        let (right, right_hit) = if count > 0 { ("Bersihkan", Some(Hit::ShelfClear)) } else { ("", None) };
        self.header(s, Some(ICON_FOLDER), &title, right, right_hit, x0, x1, a);

        let strip = rect(l, 38.0, r, 118.0);
        self.hit(strip, Hit::ShelfStrip);
        if count == 0 {
            let (border, text) = if shelf.drop_hover {
                self.rounded(strip, 12.0, with_alpha(p.accent, 0.15), a);
                (p.accent, "Lepaskan untuk menyimpan di shelf")
            } else {
                (p.dim, "Seret file ke notch untuk disimpan sementara")
            };
            self.outline(strip, 12.0, 1.2, true, border, a);
            let cx = (l + r) / 2.0;
            let color = if shelf.drop_hover { p.text } else { p.muted };
            self.glyph(ICON_FOLDER, &self.fonts.icon_20, cx, 66.0, color, a);
            self.text(text, &self.fonts.caption_center, rect(l, 86.0, r, 102.0), color, a);
            return;
        }

        for (slot, item) in shelf.items.iter().skip(shelf.scroll).take(SHELF_VISIBLE).enumerate() {
            let tx = l + slot as f32 * 76.0;
            let tile = rect(tx, 38.0, tx + 68.0, 118.0);
            let hovered = matches!(s.hovered, Some(Hit::ShelfItem(id) | Hit::ShelfRemove(id)) if id == item.id);
            if hovered {
                self.rounded(tile, 10.0, self.tint(0.08), a);
            }
            self.thumb(item.id, item.is_dir, tx + 34.0, 44.0, 48.0, a);
            self.text(&item.name, &self.fonts.caption_center, rect(tx + 3.0, 96.0, tx + 65.0, 112.0), p.soft, a);
            self.hit(tile, Hit::ShelfItem(item.id));
            if hovered {
                let (bx, by) = (tx + 60.0, 45.0);
                let over = s.hovered == Some(Hit::ShelfRemove(item.id));
                self.circle(bx, by, 8.5, if over { RED } else { p.badge }, a);
                self.glyph(ICON_CLOSE, &self.fonts.icon_9, bx, by, if over { WHITE } else { p.text }, a);
                self.hit(rect(bx - 9.5, by - 9.5, bx + 9.5, by + 9.5), Hit::ShelfRemove(item.id));
            }
        }
        if shelf.drop_hover {
            self.rounded(strip, 12.0, with_alpha(p.accent, 0.22), a);
            self.outline(strip, 12.0, 1.2, true, p.accent, a);
            self.text("Lepaskan untuk menambahkan", &self.fonts.label_center, strip, p.text, a);
        } else {
            let hint = "Klik 2x buka · klik kanan lihat di folder · seret keluar";
            self.text(hint, &self.fonts.caption_center, rect(l, ROW_Y - 8.0, r, ROW_Y + 8.0), p.muted, a);
        }
    }

    fn focus_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let f = s.focus;
        let right = if f.sessions > 0 { format!("{} sesi hari ini", f.sessions) } else { String::new() };
        self.header(s, Some(ICON_STOPWATCH), "Pomodoro", &right, None, x0, x1, a);

        // Cincin progres sekaligus tombol mulai/jeda.
        let (cx, cy) = (l + 34.0, 76.0);
        if s.hovered == Some(Hit::FocusToggle) {
            self.circle(cx, cy, 24.0, self.tint(0.14), a);
        }
        self.ring(cx, cy, 30.0, 5.0, if f.active { f.progress } else { 0.0 }, phase_color(f.phase), a);
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

        let done = (w.ml as f32 / goal).min(1.0);
        self.ring(r - 30.0, 72.0, 26.0, 5.0, done, BLUE, a);
        self.text(&format!("{}%", (done * 100.0).round()), &self.fonts.label_center, rect(r - 56.0, 62.0, r - 4.0, 82.0), p.text, a);

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

    fn tasks_page(&self, s: &Scene, x0: f32, x1: f32, a: f32) {
        let p = &self.pal;
        let (l, r) = (x0 + MARGIN, x1 - MARGIN);
        let tasks = s.tasks;
        let remaining = tasks.remaining();
        let right = if tasks.items.is_empty() {
            String::new()
        } else if remaining == 0 {
            "Semua selesai".to_string()
        } else {
            format!("{remaining} tersisa")
        };
        self.header(s, Some(ICON_TASKS), "Tugas", &right, None, x0, x1, a);

        self.hit(rect(l - 6.0, 34.0, r + 6.0, 108.0), Hit::TaskList);
        let order = tasks.order();
        if order.is_empty() {
            self.text("Belum ada tugas. Tambahkan di bawah.", &self.fonts.caption_center, rect(l, 62.0, r, 80.0), p.muted, a);
        }
        for (row, &index) in order.iter().skip(tasks.scroll).take(TASKS_VISIBLE).enumerate() {
            let task = &tasks.items[index];
            let y = 46.0 + row as f32 * 24.0;
            let hovered = matches!(s.hovered, Some(Hit::TaskToggle(i) | Hit::TaskRemove(i)) if i == index);
            if hovered {
                self.rounded(rect(l - 6.0, y - 11.0, r + 6.0, y + 11.0), 8.0, self.tint(0.06), a);
            }
            let cx = l + 9.0;
            if task.done {
                self.circle(cx, y, 8.0, p.accent, a);
                self.glyph(ICON_CHECK, &self.fonts.icon_9, cx, y, WHITE, a);
            } else {
                self.circle_outline(cx, y, 7.0, 1.5, if hovered { p.text } else { p.muted }, a);
            }
            let area = rect(l + 26.0, y - 10.0, r - 26.0, y + 10.0);
            if task.done {
                self.text_struck(&task.text, &self.fonts.body, area, p.muted, a);
            } else {
                self.text(&task.text, &self.fonts.body, area, p.text, a);
            }
            self.hit(rect(l - 6.0, y - 11.0, r - 20.0, y + 11.0), Hit::TaskToggle(index));
            if hovered {
                let over = s.hovered == Some(Hit::TaskRemove(index));
                self.glyph(ICON_CLOSE, &self.fonts.icon_9, r - 8.0, y, if over { RED } else { p.muted }, a);
                self.hit(rect(r - 20.0, y - 11.0, r + 6.0, y + 11.0), Hit::TaskRemove(index));
            }
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
        let fill = if tasks.draft.is_some() {
            0.14
        } else if over {
            0.10
        } else {
            0.07
        };
        self.rounded(input, 11.0, self.tint(fill), a);
        let area = rect(input.left + 12.0, input.top, input.right - 12.0, input.bottom);
        match &tasks.draft {
            Some(draft) => {
                self.outline(input, 11.0, 1.0, false, p.accent, a);
                // Teks panjang digeser supaya ujung yang sedang diketik tetap terlihat.
                let width = self.text_width(draft, &self.fonts.input);
                let start = if width > area.right - area.left { area.right - width } else { area.left };
                self.push_clip(area);
                self.text(draft, &self.fonts.input, rect(start, area.top, start + width + 4.0, area.bottom), p.text, a);
                let caret = start + width + 1.0;
                self.rounded(rect(caret, ROW_Y - 7.0, caret + 1.5, ROW_Y + 7.0), 0.75, p.text, a);
                if draft.is_empty() {
                    self.text("Ketik tugas, Enter untuk simpan", &self.fonts.input, rect(caret + 6.0, area.top, area.right, area.bottom), p.muted, a);
                }
                self.pop_clip();
            }
            None => self.text("+  Tambah tugas", &self.fonts.body, area, if over { p.soft } else { p.muted }, a),
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
        self.header(s, Some(ICON_SYSTEM), "Sistem", &format!("Menyala {}", s.uptime), None, x0, x1, a);

        let percent = |used: u64, total: u64| if total > 0 { used as f32 / total as f32 * 100.0 } else { 0.0 };
        let meters: [(&str, Option<f32>, String); 4] = match s.system {
            Some(st) => [
                ("CPU", Some(st.cpu), String::new()),
                ("GPU", st.gpu, if st.gpu.is_none() { "tidak tersedia".into() } else { String::new() }),
                ("RAM", Some(percent(st.ram_used, st.ram_total)), format!("{} / {}", gigabytes(st.ram_used), gigabytes(st.ram_total))),
                ("Disk C:", Some(percent(st.disk_total.saturating_sub(st.disk_free), st.disk_total)), format!("{} bebas", gigabytes(st.disk_free))),
            ],
            None => ["CPU", "GPU", "RAM", "Disk C:"].map(|name| (name, None, "mengukur…".to_string())),
        };
        let spacing = (r - l) / 4.0;
        for (i, (name, value, detail)) in meters.iter().enumerate() {
            let cx = l + spacing * (i as f32 + 0.5);
            let color = value.map_or(p.dim, |v| load_color(v, p.accent));
            self.ring(cx, 64.0, 20.0, 4.0, value.unwrap_or(0.0) / 100.0, color, a);
            let label = value.map_or("–".to_string(), |v| format!("{}%", v.round()));
            self.text(&label, &self.fonts.label_center, rect(cx - 20.0, 54.0, cx + 20.0, 74.0), p.text, a);
            self.text(name, &self.fonts.caption_center, rect(cx - spacing / 2.0, 90.0, cx + spacing / 2.0, 104.0), p.soft, a);
            self.text(detail, &self.fonts.tiny, rect(cx - spacing / 2.0, 104.0, cx + spacing / 2.0, 116.0), p.muted, a);
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
