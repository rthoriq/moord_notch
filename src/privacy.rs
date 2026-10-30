//! Indikator privasi: aplikasi yang sedang memakai kamera atau mikrofon.
//!
//! Windows versi baru tidak lagi mencatat pemakaian ini di registry (dipindah ke database
//! sistem yang terkunci), jadi dipakai API resmi yang tidak butuh admin dan berbasis event:
//! - mikrofon: sesi rekaman yang aktif di perangkat input audio (Core Audio);
//! - kamera: laporan aktivitas sensor dari Media Foundation.

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{CloseHandle, S_OK};
use windows::Win32::Media::Audio::{
    AudioSessionDisconnectReason, AudioSessionState, AudioSessionStateActive, DEVICE_STATE_ACTIVE,
    IAudioSessionControl, IAudioSessionControl2, IAudioSessionEvents, IAudioSessionEvents_Impl,
    IAudioSessionManager2, IAudioSessionNotification, IAudioSessionNotification_Impl,
    IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
};
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
    IMFSensorActivitiesReportCallback_Impl, IMFSensorActivityMonitor, MF_VERSION,
    MFCreateSensorActivityMonitor, MFStartup,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::core::{BOOL, Interface, PCWSTR, PWSTR, Ref, Result, implement};

use crate::media::app_name;
use crate::services::{Event, Notifier, Task};

/// `MFSTARTUP_LITE`: Media Foundation tanpa komponen jaringan.
const MF_LITE: u32 = 1;

/// Nama aplikasi yang sedang memakai kamera dan mikrofon.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Usage {
    pub camera: Vec<String>,
    pub microphone: Vec<String>,
    /// Pemakai mikrofon selain perantara mikrofon (lihat [`is_mediator`]), untuk panggilan aktif.
    pub callers: Vec<String>,
}

impl Usage {
    pub fn active(&self) -> bool {
        !self.camera.is_empty() || !self.microphone.is_empty()
    }
}

pub struct Monitor {
    enumerator: IMMDeviceEnumerator,
    /// Pendengar sesi baru per perangkat input, dan pendengar status per sesi.
    managers: Vec<(IAudioSessionManager2, IAudioSessionNotification)>,
    sessions: Vec<(IAudioSessionControl, IAudioSessionEvents)>,
    watcher: IAudioSessionNotification,
    /// Dipertahankan supaya laporan kamera terus datang.
    camera: Option<IMFSensorActivityMonitor>,
    camera_apps: Arc<Mutex<Vec<String>>>,
    /// Id perangkat mikrofon yang dipakai `callers`, untuk tombol bisu panggilan.
    call_devices: Vec<String>,
    notify: Notifier,
    last: Option<Usage>,
}

impl Monitor {
    pub fn new(tasks: Sender<Task>, notify: Notifier) -> Result<Self> {
        let enumerator: IMMDeviceEnumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let watcher: IAudioSessionNotification = SessionWatcher { tasks: tasks.clone() }.into();
        let camera_apps = Arc::new(Mutex::new(Vec::new()));
        let camera = start_camera(tasks, camera_apps.clone())
            .inspect_err(|e| error_log!("pemantau kamera tidak aktif: {e}"))
            .ok();
        let mut monitor = Self {
            enumerator,
            managers: Vec::new(),
            sessions: Vec::new(),
            watcher,
            camera,
            camera_apps,
            call_devices: Vec::new(),
            notify,
            last: None,
        };
        monitor.rescan();
        Ok(monitor)
    }

    /// Baca ulang semua sesi rekaman, daftarkan ulang pendengarnya, lalu kabari jendela notch
    /// kalau daftar aplikasinya berubah.
    pub fn rescan(&mut self) {
        self.detach();
        let recorders = self.attach().unwrap_or_else(|e| {
            debug_log!("sesi mikrofon tidak terbaca: {e}");
            Vec::new()
        });
        let sorted = |mut items: Vec<String>| {
            items.sort();
            items.dedup();
            items
        };
        let callers = recorders.iter().filter(|r| !r.mediator);
        let microphone = sorted(recorders.iter().map(|r| r.app.clone()).collect());
        self.call_devices = sorted(callers.clone().map(|r| r.device.clone()).collect());
        let callers = sorted(callers.map(|r| r.app.clone()).collect());
        let camera = self.camera_apps.lock().map(|apps| apps.clone()).unwrap_or_default();
        let usage = Usage { camera, microphone, callers };
        if self.last.as_ref() != Some(&usage) {
            self.notify.send(Event::Privacy(usage.clone()));
            self.last = Some(usage);
        }
    }

    /// Mikrofon yang sedang dipakai aplikasi panggilan.
    pub fn call_devices(&self) -> &[String] {
        &self.call_devices
    }

    /// Daftarkan pendengar di semua perangkat input dan sesinya; kembalikan sesi yang sedang
    /// merekam.
    fn attach(&mut self) -> Result<Vec<Recorder>> {
        let mut recorders = Vec::new();
        unsafe {
            let devices = self.enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
            for i in 0..devices.GetCount()? {
                let device = devices.Item(i)?;
                let manager: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
                let id = device_id(&device);
                // Pendengar sesi baru harus didaftarkan sebelum sesi-sesinya dibaca.
                manager.RegisterSessionNotification(&self.watcher)?;
                let list = manager.GetSessionEnumerator()?;
                for j in 0..list.GetCount()? {
                    let session = list.GetSession(j)?;
                    let events: IAudioSessionEvents = self.watcher.cast()?;
                    let _ = session.RegisterAudioSessionNotification(&events);
                    if let Some(path) = recording_app(&session) {
                        recorders.push(Recorder { app: app_name(&path), mediator: is_mediator(&path), device: id.clone() });
                    }
                    self.sessions.push((session, events));
                }
                self.managers.push((manager, self.watcher.clone()));
            }
        }
        Ok(recorders)
    }

    fn detach(&mut self) {
        unsafe {
            for (session, events) in self.sessions.drain(..) {
                let _ = session.UnregisterAudioSessionNotification(&events);
            }
            for (manager, watcher) in self.managers.drain(..) {
                let _ = manager.UnregisterSessionNotification(&watcher);
            }
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.detach();
        if let Some(camera) = self.camera.take() {
            let _ = unsafe { camera.Stop() };
        }
    }
}

/// Sesi rekaman yang aktif: aplikasinya dan mikrofon yang dipakainya.
struct Recorder {
    app: String,
    mediator: bool,
    device: String,
}

/// Perantara mikrofon (peredam bising, mixer) dan fitur dengar Windows (Akses suara, Teks
/// langsung) yang memakai mikrofon terus-menerus. Bukan panggilan; aplikasi yang memakai
/// mikrofon lewat perantara itu tetap terdeteksi di mikrofon virtualnya.
const MEDIATORS: [&str; 7] = ["voicemeeter", "nvidiabroadcast", "rtxvoice", "wavelink", "krisp", "voiceaccess", "livecaptions"];

/// `path` (jalur exe) adalah perantara mikrofon, dicocokkan dari nama file tanpa spasi.
fn is_mediator(path: &str) -> bool {
    let file = path.rsplit(['\\', '/']).next().unwrap_or(path).to_lowercase();
    let name: String = file.trim_end_matches(".exe").chars().filter(|c| c.is_alphanumeric()).collect();
    MEDIATORS.iter().any(|m| name.contains(m))
}

/// Id perangkat (untuk `IMMDeviceEnumerator::GetDevice`); kosong kalau tidak terbaca.
fn device_id(device: &IMMDevice) -> String {
    unsafe {
        let Ok(id) = device.GetId() else { return String::new() };
        let text = id.to_string().unwrap_or_default();
        CoTaskMemFree(Some(id.0 as *const _));
        text
    }
}

/// Jalur exe pemilik sesi kalau sesinya sedang aktif merekam (bukan suara sistem, bukan notch).
fn recording_app(session: &IAudioSessionControl) -> Option<String> {
    unsafe {
        if session.GetState().ok()? != AudioSessionStateActive {
            return None;
        }
        let session: IAudioSessionControl2 = session.cast().ok()?;
        if session.IsSystemSoundsSession() == S_OK {
            return None;
        }
        process_path(session.GetProcessId().ok()?)
    }
}

fn start_camera(tasks: Sender<Task>, apps: Arc<Mutex<Vec<String>>>) -> Result<IMFSensorActivityMonitor> {
    unsafe {
        MFStartup(MF_VERSION, MF_LITE)?;
        let callback: IMFSensorActivitiesReportCallback = CameraWatcher { tasks, apps }.into();
        let monitor = MFCreateSensorActivityMonitor(&callback)?;
        monitor.Start()?;
        Ok(monitor)
    }
}

/// Nama aplikasi yang enak dibaca dari id proses.
fn process_name(pid: u32) -> Option<String> {
    process_path(pid).map(|path| app_name(&path))
}

/// Jalur exe proses (bukan notch sendiri).
fn process_path(pid: u32) -> Option<String> {
    if pid == 0 || pid == std::process::id() {
        return None;
    }
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(process);
        ok.then(|| String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// Pendengar sesi baru dan perubahan status sesi; cukup meminta pemindaian ulang.
#[implement(IAudioSessionNotification, IAudioSessionEvents)]
struct SessionWatcher {
    tasks: Sender<Task>,
}

impl SessionWatcher {
    fn changed(&self) {
        let _ = self.tasks.send(Task::PrivacyChanged);
    }
}

impl IAudioSessionNotification_Impl for SessionWatcher_Impl {
    fn OnSessionCreated(&self, _session: Ref<IAudioSessionControl>) -> Result<()> {
        self.changed();
        Ok(())
    }
}

impl IAudioSessionEvents_Impl for SessionWatcher_Impl {
    fn OnDisplayNameChanged(&self, _name: &PCWSTR, _context: *const windows::core::GUID) -> Result<()> {
        Ok(())
    }

    fn OnIconPathChanged(&self, _path: &PCWSTR, _context: *const windows::core::GUID) -> Result<()> {
        Ok(())
    }

    fn OnSimpleVolumeChanged(&self, _volume: f32, _mute: BOOL, _context: *const windows::core::GUID) -> Result<()> {
        Ok(())
    }

    fn OnChannelVolumeChanged(
        &self,
        _count: u32,
        _volumes: *const f32,
        _changed: u32,
        _context: *const windows::core::GUID,
    ) -> Result<()> {
        Ok(())
    }

    fn OnGroupingParamChanged(&self, _param: *const windows::core::GUID, _context: *const windows::core::GUID) -> Result<()> {
        Ok(())
    }

    fn OnStateChanged(&self, _state: AudioSessionState) -> Result<()> {
        self.changed();
        Ok(())
    }

    fn OnSessionDisconnected(&self, _reason: AudioSessionDisconnectReason) -> Result<()> {
        self.changed();
        Ok(())
    }
}

/// Penerima laporan aktivitas kamera: catat aplikasi yang sedang mengalirkan gambar.
#[implement(IMFSensorActivitiesReportCallback)]
struct CameraWatcher {
    tasks: Sender<Task>,
    apps: Arc<Mutex<Vec<String>>>,
}

impl IMFSensorActivitiesReportCallback_Impl for CameraWatcher_Impl {
    fn OnActivitiesReport(&self, report: Ref<IMFSensorActivitiesReport>) -> Result<()> {
        let Some(report) = report.as_ref() else { return Ok(()) };
        let mut apps = Vec::new();
        unsafe {
            for i in 0..report.GetCount()? {
                let sensor = report.GetActivityReport(i)?;
                for j in 0..sensor.GetProcessCount()? {
                    let process = sensor.GetProcessActivity(j)?;
                    if process.GetStreamingState()?.as_bool()
                        && let Some(app) = process_name(process.GetProcessId()?)
                    {
                        apps.push(app);
                    }
                }
            }
        }
        apps.sort();
        apps.dedup();
        let mut current = self.apps.lock().unwrap_or_else(|e| e.into_inner());
        if *current != apps {
            *current = apps;
            let _ = self.tasks.send(Task::PrivacyChanged);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mediators() {
        for path in [
            r"C:\Program Files (x86)\VB\Voicemeeter\voicemeeter8x64.exe",
            r"C:\Program Files\NVIDIA Corporation\NVIDIA Broadcast\NVIDIA Broadcast.exe",
            r"C:\Program Files\Elgato\WaveLink\WaveLink.exe",
            r"C:\Windows\System32\VoiceAccess.exe",
        ] {
            assert!(is_mediator(path), "{path}");
        }
        for path in [r"C:\Users\a\AppData\Local\Discord\app-1.0\Discord.exe", r"C:\Program Files\Zoom\bin\Zoom.exe", "ms-teams.exe"] {
            assert!(!is_mediator(path), "{path}");
        }
    }
}
