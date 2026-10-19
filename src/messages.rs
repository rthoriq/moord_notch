//! Pesan masuk dari aplikasi chat (WhatsApp, Telegram, dan lainnya), dibaca dari notifikasi
//! Windows lewat `UserNotificationListener`.
//!
//! Notifikasi dari browser (misal WhatsApp Web di Brave) juga dibaca, tapi Windows hanya
//! memberikan judul dan isinya, tanpa alamat situs pengirimnya. Jadi notch tidak bisa tahu situs
//! mana yang mengirim: semuanya ditandai sebagai pesan browser, dan baru ditampilkan kalau
//! pengguna menyalakan "Notifikasi browser".
//!
//! Windows hanya mengizinkan ini untuk aplikasi yang punya identitas paket. Jadi fitur ini
//! baru aktif setelah notch dipasang lewat `packaging/install.ps1` dan pengguna mengizinkan
//! akses notifikasi satu kali. Tanpa itu, semua fungsi di sini gagal dengan tenang.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use windows::ApplicationModel::Package;
use windows::Foundation::{Size, TypedEventHandler};
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus};
use windows::UI::Notifications::{
    KnownNotificationBindings, NotificationKinds, UserNotification, UserNotificationChangedEventArgs,
    UserNotificationChangedKind,
};
use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow,
};
use windows::core::{BOOL, Ref, Result};
use windows_future::AsyncOperationCompletedHandler;

use crate::dragdrop;
use crate::media::decode_square;
use crate::services::{Event, Notifier};

/// Sisi logo aplikasi dalam piksel; tajam untuk 26 DIP sampai skala 150%.
pub const LOGO_PX: u32 = 48;

/// Aplikasi chat yang dikenali (dicocokkan dengan nama tampilannya).
const CHAT_APPS: [&str; 13] = [
    "whatsapp", "telegram", "discord", "signal", "messenger", "instagram", "line", "slack", "teams",
    "zalo", "wechat", "skype", "viber",
];

/// Browser yang dikenali, untuk pesan dari versi web (misal WhatsApp Web).
const BROWSERS: [&str; 10] = ["brave", "chrome", "edge", "firefox", "opera", "vivaldi", "chromium", "arc", "zen", "yandex"];

pub struct Message {
    pub app: String,
    /// AppUserModelID, untuk membuka aplikasinya saat kabar diklik.
    pub aumid: String,
    pub sender: String,
    pub text: String,
    /// Logo aplikasi (BGRA premultiplied, `LOGO_PX` persegi).
    pub logo: Option<Arc<Vec<u8>>>,
    /// Dari browser: bisa dari situs mana saja (lihat keterangan modul).
    pub from_browser: bool,
}

impl Message {
    pub fn is_whatsapp(&self) -> bool {
        self.app.to_lowercase().contains("whatsapp")
    }
}

/// Nama aplikasi memuat salah satu `names` sebagai kata utuh ("line" tidak cocok dengan "Outline").
fn has_word(app: &str, names: &[&str]) -> bool {
    let app = app.to_lowercase();
    app.split(|c: char| !c.is_alphanumeric()).any(|word| names.contains(&word))
}

fn is_chat(app: &str) -> bool {
    has_word(app, &CHAT_APPS)
}

fn is_browser(app: &str) -> bool {
    has_word(app, &BROWSERS)
}

/// Isi notifikasi yang sudah dipilah.
#[derive(Debug, PartialEq)]
struct Parsed {
    app: String,
    from_browser: bool,
    sender: String,
    text: String,
}

/// Pilah notifikasi dari `app` dengan baris-baris teksnya: baris pertama = pengirim, sisanya =
/// isi. Hanya aplikasi chat dan browser yang diterima.
fn classify(app: &str, lines: Vec<String>) -> Option<Parsed> {
    let from_browser = !is_chat(app) && is_browser(app);
    if !is_chat(app) && !from_browser {
        return None;
    }
    let mut lines = lines.into_iter();
    let sender = lines.next().filter(|s| !s.trim().is_empty()).unwrap_or_else(|| app.to_string());
    let text = lines.filter(|s| !s.trim().is_empty()).collect::<Vec<_>>().join(" ");
    Some(Parsed { app: app.to_string(), from_browser, sender, text })
}

/// Buka aplikasi chat saat kabar pesannya diklik. Pesan dari browser membawa ke jendela
/// WhatsApp kalau ada (misal tab WhatsApp Web yang sedang aktif), lalu ke jendela browsernya.
pub fn open(hwnd: HWND, aumid: &str, app: &str, from_browser: bool) {
    if from_browser && (focus_window_titled("whatsapp") || focus_window_titled(app)) {
        return;
    }
    dragdrop::open_app(hwnd, aumid);
}

/// Bawa ke depan jendela yang judulnya memuat `name` (misal tab "WhatsApp" di browser).
fn focus_window_titled(name: &str) -> bool {
    struct Search {
        needle: String,
        found: Option<HWND>,
    }
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        let mut title = [0u16; 256];
        let len = unsafe { GetWindowTextW(hwnd, &mut title) };
        if len > 0 && unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let title = String::from_utf16_lossy(&title[..len as usize]).to_lowercase();
            if title.contains(&search.needle) {
                search.found = Some(hwnd);
                return false.into();
            }
        }
        true.into()
    }
    if name.trim().is_empty() {
        return false;
    }
    let mut search = Search { needle: name.to_lowercase(), found: None };
    let _ = unsafe { EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize)) };
    let Some(window) = search.found else { return false };
    unsafe {
        if IsIconic(window).as_bool() {
            let _ = ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window).as_bool()
    }
}

/// Notch dijalankan dengan identitas paket (lihat `packaging/`); tanpa itu Windows tidak
/// mengizinkan membaca notifikasi.
pub fn packaged() -> bool {
    Package::Current().is_ok()
}

/// Minta izin membaca notifikasi kalau belum pernah. Hasilnya datang sebagai
/// [`Event::MessagesAccess`]. Harus dipanggil dari thread jendela karena bisa memunculkan dialog.
pub fn request_access(notify: Notifier) {
    let Ok(listener) = UserNotificationListener::Current() else {
        notify.send(Event::MessagesAccess(false));
        return;
    };
    match listener.GetAccessStatus() {
        Ok(UserNotificationListenerAccessStatus::Allowed) => notify.send(Event::MessagesAccess(true)),
        Ok(UserNotificationListenerAccessStatus::Unspecified) => {
            let asked = listener.RequestAccessAsync().and_then(|op| {
                let reply = notify.clone();
                op.SetCompleted(&AsyncOperationCompletedHandler::new(move |op, _| {
                    let allowed = op
                        .as_ref()
                        .and_then(|op| op.GetResults().ok())
                        .is_some_and(|status| status == UserNotificationListenerAccessStatus::Allowed);
                    reply.send(Event::MessagesAccess(allowed));
                    Ok(())
                }))
            });
            if let Err(e) = asked {
                debug_log!("izin notifikasi tidak bisa diminta: {e}");
                notify.send(Event::MessagesAccess(false));
            }
        }
        // Ditolak, atau aplikasi tidak punya identitas paket.
        _ => notify.send(Event::MessagesAccess(false)),
    }
}

/// Mendengarkan notifikasi baru selama hidup.
pub struct Monitor {
    listener: UserNotificationListener,
    token: i64,
}

impl Monitor {
    pub fn start(notify: Notifier) -> Result<Self> {
        let listener = UserNotificationListener::Current()?;
        let reader = listener.clone();
        let logos: Arc<Mutex<HashMap<String, Arc<Vec<u8>>>>> = Arc::default();
        let token = listener.NotificationChanged(&TypedEventHandler::new(
            move |_, args: Ref<UserNotificationChangedEventArgs>| {
                if let Some(args) = args.as_ref()
                    && args.ChangeKind()? == UserNotificationChangedKind::Added
                    && let Some(message) = read(&reader, args.UserNotificationId()?, &logos)
                {
                    notify.send(Event::Message(message));
                }
                Ok(())
            },
        ))?;
        Ok(Self { listener, token })
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = self.listener.RemoveNotificationChanged(self.token);
    }
}

/// Baris-baris teks notifikasi.
fn lines_of(notification: &UserNotification) -> Option<Vec<String>> {
    let binding = notification
        .Notification()
        .ok()?
        .Visual()
        .ok()?
        .GetBinding(&KnownNotificationBindings::ToastGeneric().ok()?)
        .ok()?;
    let texts = binding.GetTextElements().ok()?;
    Some((0..texts.Size().ok()?).filter_map(|i| texts.GetAt(i).ok()?.Text().ok()).map(|s| s.to_string()).collect())
}

/// Baca satu notifikasi; hanya dari aplikasi chat atau browser.
fn read(listener: &UserNotificationListener, id: u32, logos: &Mutex<HashMap<String, Arc<Vec<u8>>>>) -> Option<Message> {
    let notification = listener.GetNotification(id).ok()?;
    let info = notification.AppInfo().ok()?;
    let display = info.DisplayInfo().ok()?;
    let app = display.DisplayName().ok()?.to_string();
    let Parsed { app, from_browser, sender, text } = classify(&app, lines_of(&notification)?)?;
    let aumid = info.AppUserModelId().map(|s| s.to_string()).unwrap_or_default();

    let mut cache = logos.lock().unwrap_or_else(|e| e.into_inner());
    let logo = match cache.get(&aumid) {
        Some(logo) => Some(logo.clone()),
        None => {
            let size = Size { Width: LOGO_PX as f32, Height: LOGO_PX as f32 };
            let logo = display.GetLogo(size).ok().and_then(|r| decode_square(&r.into(), LOGO_PX).ok()).map(Arc::new);
            if let Some(logo) = &logo {
                cache.insert(aumid.clone(), logo.clone());
            }
            logo
        }
    };
    Some(Message { app, aumid, sender, text, logo, from_browser })
}

/// Pemeriksaan (`notch.exe --cek-notifikasi <file>`): tulis status izin dan notifikasi yang ada
/// di pusat notifikasi beserta cara notch memilahnya. Isi pesan dan nama pengirim tidak ditulis;
/// hanya nama aplikasi dan jumlah barisnya.
///
/// Dengan `listen`, notch juga mendengarkan notifikasi baru selama itu dan mencatat masing-masing
/// (untuk memastikan kabar notifikasi baru memang sampai).
pub fn diagnose(out: &Path, listen: Duration) -> std::io::Result<()> {
    let mut report = vec![format!("identitas paket: {}", if packaged() { "ada" } else { "tidak ada" })];
    let listener = UserNotificationListener::Current().ok();
    let status = listener.as_ref().and_then(|l| l.GetAccessStatus().ok());
    report.push(format!(
        "izin membaca notifikasi: {}",
        match status {
            Some(UserNotificationListenerAccessStatus::Allowed) => "diizinkan",
            Some(UserNotificationListenerAccessStatus::Denied) => "ditolak",
            Some(_) => "belum pernah ditanyakan",
            None => "tidak tersedia",
        }
    ));
    if let (Some(listener), Some(UserNotificationListenerAccessStatus::Allowed)) = (&listener, status) {
        let list = listener.GetNotificationsAsync(NotificationKinds::Toast).and_then(|op| op.join());
        match list {
            Ok(list) => {
                report.push(format!("notifikasi di pusat notifikasi: {}", list.Size().unwrap_or(0)));
                for notification in list {
                    report.push(format!("- {}", describe(&notification)));
                }
            }
            Err(e) => report.push(format!("daftar notifikasi gagal dibaca: {e}")),
        }
        if !listen.is_zero() {
            report.push(format!("mendengarkan notifikasi baru selama {} detik:", listen.as_secs()));
            std::fs::write(out, report.join("\r\n"))?;
            listen_for(listener, listen, out, &mut report)?;
            report.push("selesai".to_string());
        }
    }
    std::fs::write(out, report.join("\r\n"))
}

/// Catat notifikasi yang ditambahkan atau dihapus selama `listen`; file ditulis ulang tiap ada kabar.
fn listen_for(listener: &UserNotificationListener, listen: Duration, out: &Path, report: &mut Vec<String>) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel::<(bool, u32)>();
    let handler = TypedEventHandler::new(move |_, args: Ref<UserNotificationChangedEventArgs>| {
        if let Some(args) = args.as_ref() {
            let added = args.ChangeKind()? == UserNotificationChangedKind::Added;
            let _ = tx.send((added, args.UserNotificationId()?));
        }
        Ok(())
    });
    let token = match listener.NotificationChanged(&handler) {
        Ok(token) => token,
        Err(e) => {
            report.push(format!("kabar notifikasi baru tidak bisa didengarkan: {e}"));
            return Ok(());
        }
    };
    let deadline = Instant::now() + listen;
    while let Ok((added, id)) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        let line = match (added, listener.GetNotification(id)) {
            (true, Ok(notification)) => format!("+ baru: {}", describe(&notification)),
            (true, Err(_)) => "+ baru, tapi sudah hilang sebelum sempat dibaca".to_string(),
            (false, _) => "- sebuah notifikasi dihapus".to_string(),
        };
        report.push(line);
        std::fs::write(out, report.join("\r\n"))?;
    }
    let _ = listener.RemoveNotificationChanged(token);
    Ok(())
}

/// Ringkasan satu notifikasi tanpa isi pesan atau nama pengirim.
fn describe(notification: &UserNotification) -> String {
    let info = notification.AppInfo().ok();
    let app = info
        .as_ref()
        .and_then(|i| i.DisplayInfo().ok())
        .and_then(|d| d.DisplayName().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();
    let aumid = info.and_then(|i| i.AppUserModelId().ok()).map(|s| s.to_string()).unwrap_or_default();
    let lines = lines_of(notification).unwrap_or_default();
    let verdict = match classify(&app, lines.clone()) {
        Some(parsed) if parsed.from_browser => "pesan browser (tampil kalau Notifikasi browser menyala)".to_string(),
        Some(parsed) => format!("pesan {}", parsed.app),
        None => "diabaikan".to_string(),
    };
    format!("{app} [{aumid}]: {} baris, {verdict}", lines.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_chat_apps() {
        assert!(is_chat("WhatsApp"));
        assert!(is_chat("Telegram Desktop"));
        assert!(is_chat("Microsoft Teams"));
        assert!(!is_chat("Windows Security"));
        assert!(!is_chat("Outline"), "kata 'line' di dalam kata lain tidak dihitung");
    }

    fn lines(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn browser_notifications_are_marked() {
        let web = classify("Brave", lines(&["Ibu", "Nak, pulang jam berapa?"])).unwrap();
        assert!(web.from_browser);
        assert_eq!((web.app.as_str(), web.sender.as_str(), web.text.as_str()), ("Brave", "Ibu", "Nak, pulang jam berapa?"));
        assert!(classify("Google Chrome", lines(&["Grup Kantor", "Rapat jam 3"])).unwrap().from_browser);
        assert!(classify("Microsoft Edge", lines(&["Ibu"])).is_some_and(|p| p.from_browser && p.text.is_empty()));
        // Aplikasi WhatsApp sendiri bukan pesan browser; aplikasi lain diabaikan.
        assert!(!classify("WhatsApp", lines(&["Ibu", "Halo"])).unwrap().from_browser);
        assert!(classify("NVIDIA App", lines(&["Driver baru", "Pasang sekarang"])).is_none());
        assert!(classify("Snipping Tool", lines(&["Tangkapan layar disalin"])).is_none());
    }
}
