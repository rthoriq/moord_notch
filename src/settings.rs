//! Pengaturan yang diubah lewat menu tray, disimpan di `settings.txt` sebagai `kunci=nilai`.

use crate::store;

const FILE: &str = "settings.txt";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    #[default]
    Black,
    Graphite,
    Midnight,
    Grape,
    /// Badan semi-transparan dengan tepi terang.
    Glass,
    Light,
}

impl Theme {
    pub const ALL: [Theme; 6] = [Theme::Black, Theme::Graphite, Theme::Midnight, Theme::Grape, Theme::Glass, Theme::Light];

    /// Nama di menu.
    pub fn label(self) -> &'static str {
        match self {
            Theme::Black => "Hitam",
            Theme::Graphite => "Grafit",
            Theme::Midnight => "Biru malam",
            Theme::Grape => "Anggur",
            Theme::Glass => "Kaca",
            Theme::Light => "Terang",
        }
    }

    /// Nama di file pengaturan.
    fn key(self) -> &'static str {
        match self {
            Theme::Black => "hitam",
            Theme::Graphite => "grafit",
            Theme::Midnight => "biru-malam",
            Theme::Grape => "anggur",
            Theme::Glass => "kaca",
            Theme::Light => "terang",
        }
    }
}

/// Notifikasi pesan dari aplikasi chat yang ditampilkan di notch.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Messages {
    Off,
    #[default]
    WhatsApp,
    /// WhatsApp, Telegram, Discord, dan aplikasi chat populer lain.
    AllChat,
}

impl Messages {
    pub const ALL: [Messages; 3] = [Messages::Off, Messages::WhatsApp, Messages::AllChat];

    pub fn label(self) -> &'static str {
        match self {
            Messages::Off => "Mati",
            Messages::WhatsApp => "Hanya WhatsApp",
            Messages::AllChat => "Semua aplikasi chat",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Messages::Off => "mati",
            Messages::WhatsApp => "whatsapp",
            Messages::AllChat => "semua",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    /// Bunyi untuk kabar penting (pengingat, pomodoro, Bluetooth, baterai, privasi, pesan).
    pub sound: bool,
    /// Ambil lirik dari internet (LRCLIB). Mati secara default.
    pub lyrics: bool,
    /// Kabar saat Caps Lock / Num Lock dinyalakan atau dimatikan.
    pub lock_keys: bool,
    /// Kabar saat kecerahan layar berubah.
    pub brightness: bool,
    pub messages: Messages,
    /// Tampilkan "Pesan baru dari …" saja, tanpa isi pesannya.
    pub hide_content: bool,
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            sound: true,
            lyrics: false,
            lock_keys: true,
            brightness: true,
            messages: Messages::default(),
            hide_content: false,
            theme: Theme::default(),
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        Self::parse(&store::load(FILE))
    }

    pub fn save(&self) {
        store::save(FILE, self.lines());
    }

    fn parse(lines: &[String]) -> Self {
        let mut settings = Self::default();
        for line in lines {
            let Some((key, value)) = line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) else { continue };
            let yes = value == "ya";
            match key {
                "suara" => settings.sound = yes,
                "lirik" => settings.lyrics = yes,
                "tombol-kunci" => settings.lock_keys = yes,
                "kecerahan" => settings.brightness = yes,
                "sembunyikan-isi" => settings.hide_content = yes,
                "pesan" => {
                    if let Some(mode) = Messages::ALL.into_iter().find(|m| m.key() == value) {
                        settings.messages = mode;
                    }
                }
                "tema" => {
                    if let Some(theme) = Theme::ALL.into_iter().find(|t| t.key() == value) {
                        settings.theme = theme;
                    }
                }
                _ => {}
            }
        }
        settings
    }

    fn lines(&self) -> Vec<String> {
        let yes = |on: bool| if on { "ya" } else { "tidak" };
        vec![
            format!("suara={}", yes(self.sound)),
            format!("lirik={}", yes(self.lyrics)),
            format!("tombol-kunci={}", yes(self.lock_keys)),
            format!("kecerahan={}", yes(self.brightness)),
            format!("pesan={}", self.messages.key()),
            format!("sembunyikan-isi={}", yes(self.hide_content)),
            format!("tema={}", self.theme.key()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_defaults() {
        let settings = Settings {
            sound: false,
            lyrics: true,
            lock_keys: false,
            brightness: true,
            messages: Messages::AllChat,
            hide_content: true,
            theme: Theme::Glass,
        };
        assert_eq!(Settings::parse(&settings.lines()), settings);
        let messy = vec!["tema = tidak-ada".to_string(), "sampah".to_string()];
        assert_eq!(Settings::parse(&messy), Settings::default());
        // File lama dari tahap 4 tetap terbaca.
        let old = vec!["lirik=ya".to_string(), "tema=hitam".to_string()];
        assert!(Settings::parse(&old).lyrics && Settings::parse(&old).sound);
    }
}
