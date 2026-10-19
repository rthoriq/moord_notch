//! Pesan masuk dari aplikasi chat (WhatsApp, Telegram, dan lainnya), dibaca dari notifikasi
//! Windows lewat `UserNotificationListener`.
//!
//! Windows hanya mengizinkan ini untuk aplikasi yang punya identitas paket. Jadi fitur ini
//! baru aktif setelah notch dipasang lewat `packaging/install.ps1` dan pengguna mengizinkan
//! akses notifikasi satu kali. Tanpa itu, semua fungsi di sini gagal dengan tenang.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use windows::ApplicationModel::Package;
use windows::Foundation::{Size, TypedEventHandler};
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus};
use windows::UI::Notifications::{
    KnownNotificationBindings, UserNotificationChangedEventArgs, UserNotificationChangedKind,
};
use windows::core::{Ref, Result};
use windows_future::AsyncOperationCompletedHandler;

use crate::media::decode_square;
use crate::services::{Event, Notifier};

/// Sisi logo aplikasi dalam piksel; tajam untuk 26 DIP sampai skala 150%.
pub const LOGO_PX: u32 = 48;

/// Aplikasi chat yang dikenali (dicocokkan dengan nama tampilannya).
const CHAT_APPS: [&str; 13] = [
    "whatsapp", "telegram", "discord", "signal", "messenger", "instagram", "line", "slack", "teams",
    "zalo", "wechat", "skype", "viber",
];

pub struct Message {
    pub app: String,
    /// AppUserModelID, untuk membuka aplikasinya saat kabar diklik.
    pub aumid: String,
    pub sender: String,
    pub text: String,
    /// Logo aplikasi (BGRA premultiplied, `LOGO_PX` persegi).
    pub logo: Option<Arc<Vec<u8>>>,
}

impl Message {
    pub fn is_whatsapp(&self) -> bool {
        self.app.to_lowercase().contains("whatsapp")
    }
}

fn is_chat(app: &str) -> bool {
    let app = app.to_lowercase();
    CHAT_APPS.iter().any(|name| app.split(|c: char| !c.is_alphanumeric()).any(|word| word == *name))
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

/// Baca satu notifikasi; hanya dari aplikasi chat. Baris pertama = pengirim, sisanya = isi.
fn read(listener: &UserNotificationListener, id: u32, logos: &Mutex<HashMap<String, Arc<Vec<u8>>>>) -> Option<Message> {
    let notification = listener.GetNotification(id).ok()?;
    let info = notification.AppInfo().ok()?;
    let display = info.DisplayInfo().ok()?;
    let app = display.DisplayName().ok()?.to_string();
    if !is_chat(&app) {
        return None;
    }
    let aumid = info.AppUserModelId().map(|s| s.to_string()).unwrap_or_default();
    let binding = notification
        .Notification()
        .ok()?
        .Visual()
        .ok()?
        .GetBinding(&KnownNotificationBindings::ToastGeneric().ok()?)
        .ok()?;
    let texts = binding.GetTextElements().ok()?;
    let mut lines = (0..texts.Size().ok()?).filter_map(|i| texts.GetAt(i).ok()?.Text().ok()).map(|s| s.to_string());
    let sender = lines.next().filter(|s| !s.is_empty()).unwrap_or_else(|| app.clone());
    let text = lines.collect::<Vec<_>>().join(" ");

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
    Some(Message { app, aumid, sender, text, logo })
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
}
