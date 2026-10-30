//! Panggilan aktif: selama ada aplikasi yang memakai mikrofon (Zoom, Teams, Discord, WhatsApp,
//! Meet di browser, ...), pil menampilkan lama panggilannya.

use std::time::{Duration, Instant};

use crate::store;

/// Pemakaian mikrofon yang lebih singkat dari ini (misal pencarian suara) tidak dianggap panggilan.
pub const SHOW_AFTER: Duration = Duration::from_secs(3);
/// Mikrofon yang dilepas sebentar (misal ganti perangkat di tengah panggilan) tidak mengakhiri
/// panggilan; baru dianggap selesai setelah jeda ini.
pub const GRACE: Duration = Duration::from_secs(5);

pub struct Call {
    /// Aplikasi panggilan, misal "Zoom".
    pub app: String,
    pub since: Instant,
    /// Kapan mikrofon dilepas, selama masih dalam jeda [`GRACE`].
    ended: Option<Instant>,
}

/// Panggilan yang sudah selesai, untuk kabar "Panggilan selesai".
#[derive(Debug, PartialEq, Eq)]
pub struct Ended {
    pub app: String,
    pub length: Duration,
}

#[derive(Default)]
pub struct Calls {
    current: Option<Call>,
}

impl Calls {
    /// Aplikasi yang sedang memakai mikrofon berubah.
    pub fn update(&mut self, apps: &[String], now: Instant) {
        let Some(first) = apps.first() else {
            match &mut self.current {
                // Belum sempat tampil: lupakan saja, tanpa kabar selesai.
                Some(call) if now.saturating_duration_since(call.since) < SHOW_AFTER => self.current = None,
                Some(call) => {
                    call.ended.get_or_insert(now);
                }
                None => {}
            }
            return;
        };
        match &mut self.current {
            Some(call) => {
                call.ended = None;
                if !apps.contains(&call.app) {
                    call.app = first.clone();
                }
            }
            None => self.current = Some(Call { app: first.clone(), since: now, ended: None }),
        }
    }

    /// Panggilan yang ditampilkan: sudah berjalan [`SHOW_AFTER`], termasuk selama jeda sesudah
    /// mikrofon dilepas.
    pub fn shown(&self, now: Instant) -> Option<&Call> {
        self.current.as_ref().filter(|call| now.saturating_duration_since(call.since) >= SHOW_AFTER)
    }

    /// Akhiri panggilan yang jedanya sudah habis.
    pub fn finish_if_due(&mut self, now: Instant) -> Option<Ended> {
        let ended = self.current.as_ref()?.ended?;
        if now < ended + GRACE {
            return None;
        }
        let call = self.current.take()?;
        Some(Ended { app: call.app, length: ended.saturating_duration_since(call.since) })
    }

    /// Kapan perlu dicek lagi: saat panggilan baru mulai tampil, atau saat jedanya habis.
    pub fn next_check(&self, now: Instant) -> Option<Duration> {
        let call = self.current.as_ref()?;
        let due = match call.ended {
            Some(ended) => ended + GRACE,
            None => call.since + SHOW_AFTER,
        };
        (due > now || call.ended.is_some()).then(|| due.saturating_duration_since(now))
    }

    pub fn clear(&mut self) {
        self.current = None;
    }

    /// `app` berhenti dihitung sebagai panggilan: panggilannya hilang tanpa kabar selesai.
    pub fn forget(&mut self, app: &str) {
        if self.current.as_ref().is_some_and(|call| call.app == app) {
            self.current = None;
        }
    }
}

/// Aplikasi yang pernah terdeteksi memakai mikrofon, dan apakah masing-masing dihitung sebagai
/// panggilan (misal Discord yang dibiarkan di voice channel bisa diabaikan). Disimpan di
/// `panggilan.txt`, satu aplikasi per baris: "ya:Zoom" atau "tidak:Discord".
#[derive(Default, Debug, PartialEq)]
pub struct CallApps {
    apps: Vec<(String, bool)>,
}

const FILE: &str = "panggilan.txt";
/// Daftar terpanjang. Kalau penuh, aplikasi terlama yang masih dihitung dibuang; yang diabaikan
/// tetap disimpan supaya tidak muncul lagi sebagai panggilan.
const MAX_APPS: usize = 12;

impl CallApps {
    pub fn load() -> Self {
        Self::parse(&store::load(FILE))
    }

    pub fn save(&self) {
        store::save(FILE, self.lines());
    }

    fn lines(&self) -> Vec<String> {
        self.apps.iter().map(|(app, counted)| format!("{}:{app}", if *counted { "ya" } else { "tidak" })).collect()
    }

    fn parse(lines: &[String]) -> Self {
        let mut list = Self::default();
        for line in lines {
            if let Some((flag, app)) = line.split_once(':')
                && !app.trim().is_empty()
                && !list.apps.iter().any(|(known, _)| known == app.trim())
            {
                list.apps.push((app.trim().to_string(), flag.trim() != "tidak"));
            }
        }
        list
    }

    /// Catat aplikasi yang baru terlihat memakai mikrofon (dihitung sebagai panggilan); `true`
    /// kalau daftarnya berubah.
    pub fn note(&mut self, apps: &[String]) -> bool {
        let mut changed = false;
        for app in apps {
            if self.apps.iter().any(|(known, _)| known == app) {
                continue;
            }
            if self.apps.len() >= MAX_APPS
                && let Some(oldest) = self.apps.iter().position(|(_, counted)| *counted)
            {
                self.apps.remove(oldest);
            }
            if self.apps.len() < MAX_APPS {
                self.apps.push((app.clone(), true));
                changed = true;
            }
        }
        changed
    }

    /// Hanya aplikasi yang dihitung sebagai panggilan.
    pub fn counted(&self, apps: &[String]) -> Vec<String> {
        apps.iter().filter(|app| !self.apps.iter().any(|(known, counted)| known == *app && !counted)).cloned().collect()
    }

    /// Balik pilihan aplikasi ke-`index`; mengembalikan aplikasinya dan pilihan barunya.
    pub fn toggle(&mut self, index: usize) -> Option<(String, bool)> {
        let (app, counted) = self.apps.get_mut(index)?;
        *counted = !*counted;
        Some((app.clone(), *counted))
    }

    pub fn list(&self) -> &[(String, bool)] {
        &self.apps
    }
}

/// Lama panggilan di pil dan Beranda: "4:05", atau "1:02:03" sesudah sejam.
pub fn clock(length: Duration) -> String {
    let secs = length.as_secs();
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

/// Lama panggilan di kabar selesai: "42 detik", "42 menit", atau "1 j 5 m".
pub fn spoken(length: Duration) -> String {
    let secs = length.as_secs();
    match secs {
        0..60 => format!("{secs} detik"),
        60..3600 => format!("{} menit", secs / 60),
        _ => format!("{} j {} m", secs / 3600, secs / 60 % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn shown_after_a_few_seconds_and_ends_after_grace() {
        let t0 = Instant::now();
        let s = Duration::from_secs;
        let mut calls = Calls::default();
        calls.update(&apps(&["Zoom"]), t0);
        assert!(calls.shown(t0 + s(2)).is_none());
        assert_eq!(calls.next_check(t0), Some(SHOW_AFTER));
        assert_eq!(calls.shown(t0 + s(3)).map(|c| c.app.as_str()), Some("Zoom"));

        // Mikrofon dilepas: masih tampil selama jeda, lalu selesai.
        calls.update(&[], t0 + s(60));
        assert!(calls.shown(t0 + s(62)).is_some());
        assert_eq!(calls.finish_if_due(t0 + s(64)), None);
        assert_eq!(calls.next_check(t0 + s(64)), Some(s(1)));
        assert_eq!(calls.finish_if_due(t0 + s(65)), Some(Ended { app: "Zoom".into(), length: s(60) }));
        assert!(calls.shown(t0 + s(65)).is_none());
        assert_eq!(calls.next_check(t0 + s(65)), None);
    }

    #[test]
    fn short_use_and_quick_return() {
        let t0 = Instant::now();
        let s = Duration::from_secs;
        let mut calls = Calls::default();
        // Pencarian suara sekilas: tidak pernah tampil, tanpa kabar selesai.
        calls.update(&apps(&["Chrome"]), t0);
        calls.update(&[], t0 + s(2));
        assert!(calls.shown(t0 + s(10)).is_none());
        assert_eq!(calls.finish_if_due(t0 + s(10)), None);

        // Ganti perangkat di tengah panggilan: lama panggilannya tetap dihitung dari awal.
        calls.update(&apps(&["Teams"]), t0 + s(20));
        calls.update(&[], t0 + s(80));
        calls.update(&apps(&["Teams", "Chrome"]), t0 + s(82));
        assert_eq!(calls.finish_if_due(t0 + s(90)), None);
        assert_eq!(calls.shown(t0 + s(90)).map(|c| c.since), Some(t0 + s(20)));

        // Aplikasi pertama berhenti, yang lain masih memakai mikrofon.
        calls.update(&apps(&["Chrome"]), t0 + s(95));
        assert_eq!(calls.shown(t0 + s(95)).map(|c| c.app.as_str()), Some("Chrome"));
    }

    #[test]
    fn ignored_app_is_not_a_call() {
        let t0 = Instant::now();
        let mut list = CallApps::default();
        assert!(list.note(&apps(&["Discord", "Zoom"])));
        assert!(!list.note(&apps(&["Zoom"])));
        assert_eq!(list.counted(&apps(&["Discord", "Zoom"])), apps(&["Discord", "Zoom"]));

        // Discord diabaikan di tengah panggilan: pilnya hilang tanpa kabar selesai.
        let mut calls = Calls::default();
        calls.update(&list.counted(&apps(&["Discord"])), t0);
        assert_eq!(list.toggle(0), Some(("Discord".into(), false)));
        calls.forget("Discord");
        calls.update(&list.counted(&apps(&["Discord"])), t0 + Duration::from_secs(60));
        assert!(calls.shown(t0 + Duration::from_secs(60)).is_none());
        assert_eq!(calls.finish_if_due(t0 + Duration::from_secs(90)), None);
        assert_eq!(list.counted(&apps(&["Discord", "Zoom"])), apps(&["Zoom"]));
        assert_eq!(list.toggle(5), None);

        // Tersimpan dan terbaca lagi apa adanya.
        let lines = list.lines();
        assert_eq!(lines, apps(&["tidak:Discord", "ya:Zoom"]));
        assert_eq!(CallApps::parse(&lines), list);
        assert_eq!(CallApps::parse(&apps(&["ya:Zoom: Workplace", "rusak", "ya:"])).list(), [("Zoom: Workplace".to_string(), true)]);
    }

    #[test]
    fn full_list_keeps_ignored_apps() {
        let mut list = CallApps::default();
        list.note(&apps(&["Discord"]));
        list.toggle(0);
        let many: Vec<String> = (0..MAX_APPS).map(|i| format!("App {i}")).collect();
        list.note(&many);
        assert_eq!(list.list().len(), MAX_APPS);
        assert_eq!(list.list()[0], ("Discord".to_string(), false));
        assert_eq!(list.list()[MAX_APPS - 1].0, format!("App {}", MAX_APPS - 1));
        assert!(!list.list().iter().any(|(app, _)| app == "App 0"));
    }

    #[test]
    fn durations() {
        let s = Duration::from_secs;
        assert_eq!(clock(s(245)), "4:05");
        assert_eq!(clock(s(3723)), "1:02:03");
        assert_eq!(spoken(s(42)), "42 detik");
        assert_eq!(spoken(s(42 * 60 + 10)), "42 menit");
        assert_eq!(spoken(s(3900)), "1 j 5 m");
    }
}
