//! Bunyi kabar, memakai suara bawaan Windows. Ikut skema suara yang dipilih di pengaturan
//! Windows, jadi tidak perlu file suara sendiri, dan diam kalau Windows sedang tidak boleh
//! diganggu (presentasi, game atau video full-screen, jam tenang).

use windows::Win32::Media::Audio::{PlaySoundW, SND_ALIAS, SND_ASYNC, SND_NODEFAULT};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_QUIET_TIME, QUNS_RUNNING_D3D_FULL_SCREEN,
    SHQueryUserNotificationState,
};
use windows::core::{PCWSTR, w};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sound {
    /// Pengingat minum.
    Reminder,
    /// Fase pomodoro selesai.
    FocusDone,
    /// Perangkat Bluetooth tersambung, charger dicolok.
    Connect,
    /// Perangkat Bluetooth terputus, charger dicabut.
    Disconnect,
    LowBattery,
    /// Aplikasi mulai memakai kamera/mikrofon.
    Privacy,
    /// Unduhan selesai.
    Download,
    /// Timer cepat habis.
    Alarm,
}

impl Sound {
    /// Nama kejadian suara Windows (Panel Kontrol → Suara).
    fn alias(self) -> PCWSTR {
        match self {
            Sound::Reminder => w!("Notification.Reminder"),
            Sound::FocusDone => w!("Notification.Default"),
            Sound::Connect => w!("DeviceConnect"),
            Sound::Disconnect => w!("DeviceDisconnect"),
            Sound::LowBattery => w!("LowBatteryAlarm"),
            Sound::Privacy => w!("SystemNotification"),
            Sound::Alarm => w!("Notification.Looping.Alarm"),
            Sound::Download => w!("Notification.Default"),
        }
    }
}

pub fn play(sound: Sound) {
    if quiet() {
        return;
    }
    unsafe {
        let _ = PlaySoundW(sound.alias(), None, SND_ALIAS | SND_ASYNC | SND_NODEFAULT);
    }
}

fn quiet() -> bool {
    matches!(
        unsafe { SHQueryUserNotificationState() },
        Ok(QUNS_BUSY | QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE | QUNS_QUIET_TIME)
    )
}
