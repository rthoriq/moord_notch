//! Thread latar untuk layanan Windows yang berbasis WinRT/COM: media, volume, Bluetooth,
//! privasi, pesan, jaringan, dan pengukuran sistem.
//!
//! Callback dari Windows datang di thread acak. Semuanya diteruskan sebagai data biasa ke
//! satu thread ini supaya diproses berurutan, lalu hasilnya dikirim ke jendela notch lewat
//! channel plus `PostMessage` untuk membangunkannya.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_APP};
use windows::core::Result;

use crate::{audio, bluetooth, brightness, folders, lyrics, media, messages, network, privacy, sysinfo, thumbnail, weather};

/// Dikirim ke jendela notch saat ada kabar baru di channel.
pub const WM_SERVICE: u32 = WM_APP + 3;

/// Aplikasi sering mengirim beberapa perubahan media beruntun; kumpulkan dulu sebentar.
const MEDIA_SETTLE: Duration = Duration::from_millis(60);
/// Jarak pengukuran sistem selama halaman Sistem terbuka.
const SYSTEM_EVERY: Duration = Duration::from_secs(1);
/// Satu aplikasi yang mulai merekam biasanya memicu beberapa kabar sesi beruntun.
const PRIVACY_SETTLE: Duration = Duration::from_millis(150);
/// Selama slider kecerahan digeser, kecerahan diubah paling sering sekali per jeda ini.
const BRIGHTNESS_EVERY: Duration = Duration::from_millis(60);
/// Wi-Fi yang tersambung memicu beberapa kabar beruntun selama beberapa detik.
const NETWORK_SETTLE: Duration = Duration::from_secs(2);

/// Kabar dari layanan untuk jendela notch.
pub enum Event {
    Media(Option<media::Snapshot>),
    /// `initial` = keadaan awal perangkat (untuk slider), bukan perubahan yang perlu dikabarkan.
    Volume { level: f32, muted: bool, initial: bool },
    /// Mikrofon default dibisukan (`Some(true)`), atau `None` kalau tidak ada mikrofon.
    Mic(Option<bool>),
    Bluetooth(bluetooth::Notice),
    BluetoothBattery { id: String, battery: u8 },
    Thumbnail(thumbnail::Thumbnail),
    Privacy(privacy::Usage),
    Lyrics(lyrics::Lyrics),
    System(sysinfo::Stats),
    /// Hasil permintaan izin membaca notifikasi (false juga berarti tidak terpasang sebagai paket).
    MessagesAccess(bool),
    Message(messages::Message),
    Network(network::State),
    /// Unduhan yang sedang berjalan: jumlah file dan total byte; `None` = tidak ada lagi.
    Downloading(Option<(usize, u64)>),
    /// File baru yang sudah selesai ditulis di folder Screenshots atau Downloads.
    Files { kind: folders::Kind, paths: Vec<std::path::PathBuf> },
    /// Hasil pencarian kota cuaca; `None` kalau tidak ditemukan atau gagal.
    City(Option<weather::City>),
    Weather(Option<weather::Weather>),
}

/// Pekerjaan untuk thread layanan.
pub enum Task {
    /// Gabungan bit [`media::PROPS`], [`media::PLAYBACK`], [`media::TIMELINE`].
    MediaChanged(u8),
    MediaSessionChanged,
    MediaCommand(media::Command),
    AudioDeviceChanged,
    Bluetooth(bluetooth::Change),
    /// Mulai/berhenti mengukur sistem (selama halaman Sistem terbuka).
    WatchSystem(bool),
    /// Ada sesi rekaman atau pemakaian kamera yang berubah.
    PrivacyChanged,
    /// Mulai/berhenti mendengarkan notifikasi pesan.
    WatchMessages(bool),
    /// Mulai/berhenti memantau Wi-Fi; keadaan awalnya langsung dikirim.
    WatchNetwork(bool),
    NetworkChanged,
    SetVolume(f32),
    ToggleMute,
    ToggleMic,
    SetBrightness(u8),
    Quit,
}

/// Mengirim kabar ke jendela notch dari thread mana pun.
#[derive(Clone)]
pub struct Notifier {
    hwnd: usize,
    tx: Sender<Event>,
}

impl Notifier {
    pub fn send(&self, event: Event) {
        if self.tx.send(event).is_ok() {
            let hwnd = HWND(self.hwnd as *mut _);
            let _ = unsafe { PostMessageW(Some(hwnd), WM_SERVICE, WPARAM(0), LPARAM(0)) };
        }
    }
}

pub struct Services {
    tasks: Sender<Task>,
    events: Receiver<Event>,
    notify: Notifier,
}

impl Services {
    pub fn start(hwnd: HWND) -> Self {
        let (tasks, task_rx) = mpsc::channel();
        let (event_tx, events) = mpsc::channel();
        let notify = Notifier { hwnd: hwnd.0 as usize, tx: event_tx };
        let (worker_tasks, worker_notify) = (tasks.clone(), notify.clone());
        let spawned = thread::Builder::new()
            .name("layanan".into())
            .spawn(move || run(worker_tasks, task_rx, worker_notify));
        if let Err(e) = spawned {
            error_log!("thread layanan gagal dibuat: {e}");
        }
        Self { tasks, events, notify }
    }

    pub fn send(&self, task: Task) {
        let _ = self.tasks.send(task);
    }

    /// Buat thumbnail file shelf di thread terpisah; hasilnya datang sebagai [`Event::Thumbnail`].
    pub fn thumbnails(&self, items: Vec<(u64, std::path::PathBuf)>) {
        thumbnail::spawn(items, self.notify.clone());
    }

    /// Ambil lirik di thread terpisah; hasilnya datang sebagai [`Event::Lyrics`].
    pub fn lyrics(&self, request: lyrics::Request) {
        lyrics::spawn(request, self.notify.clone());
    }

    /// Untuk pekerjaan yang harus dimulai dari thread jendela tapi menjawab lewat channel.
    pub fn notifier(&self) -> Notifier {
        self.notify.clone()
    }

    pub fn next_event(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}

impl Drop for Services {
    fn drop(&mut self) {
        self.send(Task::Quit);
    }
}

fn run(tasks: Sender<Task>, rx: Receiver<Task>, notify: Notifier) {
    // Semua objek COM/WinRT di thread ini hidup di apartemen multithread, jadi callback
    // dari Windows boleh datang di thread mana saja.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let mut media = started("media", media::Monitor::new(tasks.clone(), notify.clone()));
    let mut audio = started("volume", audio::Monitor::new(tasks.clone(), notify.clone()));
    let mut bluetooth = started("bluetooth", bluetooth::Monitor::new(tasks.clone(), notify.clone()));
    let mut privacy = started("privasi", privacy::Monitor::new(tasks.clone(), notify.clone()));
    let mut system: Option<sysinfo::Monitor> = None;
    let mut chat: Option<messages::Monitor> = None;
    let mut net: Option<network::Monitor> = None;
    // Dibuat saat pertama dipakai; `Err` kalau layar tidak mendukung.
    let mut screen: Option<Result<brightness::Brightness>> = None;

    let mut media_due = Some((Instant::now(), media::ALL));
    let mut system_due: Option<Instant> = None;
    let mut privacy_due: Option<Instant> = None;
    let mut network_due: Option<Instant> = None;
    let mut brightness_due: Option<(Instant, u8)> = None;
    // Hitungan pekerjaan per jenis (media, volume, bluetooth) untuk log debug tiap menit.
    let (mut counts, mut since) = ([0u32; 3], Instant::now());
    loop {
        if since.elapsed() >= Duration::from_secs(60) {
            debug_log!("layanan/menit: media {}, perangkat audio {}, bluetooth {}", counts[0], counts[1], counts[2]);
            (counts, since) = ([0; 3], Instant::now());
        }
        let due = [media_due.map(|(due, _)| due), system_due, privacy_due, network_due, brightness_due.map(|(due, _)| due)]
            .into_iter()
            .flatten()
            .min();
        let task = match due {
            Some(due) => match rx.recv_timeout(due.saturating_duration_since(Instant::now())) {
                Ok(task) => Some(task),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            },
            None => match rx.recv() {
                Ok(task) => Some(task),
                Err(_) => break,
            },
        };
        match task {
            None => {
                let now = Instant::now();
                if media_due.is_some_and(|(due, _)| due <= now)
                    && let (Some(monitor), Some((_, flags))) = (&mut media, media_due.take())
                {
                    monitor.refresh(flags);
                }
                if system_due.is_some_and(|due| due <= now) {
                    system_due = system.as_mut().map(|monitor| {
                        notify.send(Event::System(monitor.sample()));
                        now + SYSTEM_EVERY
                    });
                }
                if let Some((due, percent)) = brightness_due
                    && due <= now
                {
                    brightness_due = None;
                    let screen = screen.get_or_insert_with(brightness::Brightness::new);
                    if let Err(e) = screen.as_ref().map(|s| s.set(percent)) {
                        error_log!("kecerahan tidak bisa diubah: {e}");
                    }
                }
                if network_due.is_some_and(|due| due <= now) {
                    network_due = None;
                    if net.is_some() {
                        notify.send(Event::Network(network::state()));
                    }
                }
                if privacy_due.is_some_and(|due| due <= now) {
                    privacy_due = None;
                    if let Some(monitor) = &mut privacy {
                        monitor.rescan();
                    }
                }
            }
            Some(Task::MediaChanged(flags)) => {
                counts[0] += 1;
                let (due, pending) = media_due.unwrap_or((Instant::now() + MEDIA_SETTLE, 0));
                media_due = Some((due, pending | flags));
            }
            Some(Task::MediaSessionChanged) => {
                if let Some(monitor) = &mut media {
                    monitor.follow_session();
                }
                media_due = Some((Instant::now() + MEDIA_SETTLE, media::ALL));
            }
            Some(Task::MediaCommand(command)) => {
                if let Some(monitor) = &media {
                    monitor.command(command);
                }
            }
            Some(Task::AudioDeviceChanged) => {
                counts[1] += 1;
                if let Some(monitor) = &mut audio {
                    monitor.attach_default();
                }
                // Mikrofon baru dicolok/dicabut: pantau ulang semua perangkat input.
                privacy_due.get_or_insert(Instant::now() + PRIVACY_SETTLE);
            }
            Some(Task::PrivacyChanged) => {
                privacy_due.get_or_insert(Instant::now() + PRIVACY_SETTLE);
            }
            Some(Task::WatchMessages(on)) => {
                if on && chat.is_none() {
                    chat = started("pesan", messages::Monitor::start(notify.clone()));
                } else if !on {
                    chat = None;
                }
            }
            Some(Task::WatchNetwork(on)) => {
                if on && net.is_none() {
                    net = started("jaringan", network::Monitor::new(tasks.clone()));
                    network_due = Some(Instant::now());
                } else if !on {
                    net = None;
                    network_due = None;
                }
            }
            Some(Task::SetVolume(level)) => {
                if let Some(monitor) = &audio {
                    monitor.set_volume(level);
                }
            }
            Some(Task::ToggleMute) => {
                if let Some(monitor) = &audio {
                    monitor.toggle_mute();
                }
            }
            Some(Task::ToggleMic) => {
                if let Some(monitor) = &audio {
                    monitor.toggle_mic();
                }
            }
            Some(Task::SetBrightness(percent)) => {
                // Nilai terbaru dipakai; jadwalnya tidak dimundurkan supaya slider tetap terasa hidup.
                let due = brightness_due.map_or(Instant::now() + BRIGHTNESS_EVERY, |(due, _)| due);
                brightness_due = Some((due, percent));
            }
            Some(Task::NetworkChanged) => {
                // Tunggu sampai kabar beruntunnya reda (setiap kabar baru memundurkan jadwal).
                network_due = Some(Instant::now() + NETWORK_SETTLE);
            }
            Some(Task::Bluetooth(change)) => {
                counts[2] += 1;
                if let Some(monitor) = &mut bluetooth {
                    monitor.apply(change);
                }
            }
            Some(Task::WatchSystem(on)) => {
                if on && system.is_none() {
                    system = sysinfo::Monitor::new();
                    // Angka persen baru berarti setelah jeda dari pengambilan pertama.
                    system_due = system.as_ref().map(|_| Instant::now() + SYSTEM_EVERY);
                } else if !on {
                    system = None;
                    system_due = None;
                }
            }
            Some(Task::Quit) => break,
        }
    }
}

/// Layanan yang gagal dimulai (misal tidak ada perangkat audio) cukup dilewati.
fn started<T>(name: &str, result: Result<T>) -> Option<T> {
    match result {
        Ok(monitor) => Some(monitor),
        Err(e) => {
            error_log!("layanan {name} tidak aktif: {e}");
            None
        }
    }
}
