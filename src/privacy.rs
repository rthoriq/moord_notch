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
    IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
};
use windows::Win32::Media::MediaFoundation::{
    IMFSensorActivitiesReport, IMFSensorActivitiesReportCallback,
    IMFSensorActivitiesReportCallback_Impl, IMFSensorActivityMonitor, MF_VERSION,
    MFCreateSensorActivityMonitor, MFStartup,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
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
        let microphone = self.attach().unwrap_or_else(|e| {
            debug_log!("sesi mikrofon tidak terbaca: {e}");
            Vec::new()
        });
        let camera = self.camera_apps.lock().map(|apps| apps.clone()).unwrap_or_default();
        let usage = Usage { camera, microphone };
        if self.last.as_ref() != Some(&usage) {
            self.notify.send(Event::Privacy(usage.clone()));
            self.last = Some(usage);
        }
    }

    /// Daftarkan pendengar di semua perangkat input dan sesinya; kembalikan aplikasi yang
    /// sedang merekam.
    fn attach(&mut self) -> Result<Vec<String>> {
        let mut apps = Vec::new();
        unsafe {
            let devices = self.enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)?;
            for i in 0..devices.GetCount()? {
                let manager: IAudioSessionManager2 = devices.Item(i)?.Activate(CLSCTX_ALL, None)?;
                // Pendengar sesi baru harus didaftarkan sebelum sesi-sesinya dibaca.
                manager.RegisterSessionNotification(&self.watcher)?;
                let list = manager.GetSessionEnumerator()?;
                for j in 0..list.GetCount()? {
                    let session = list.GetSession(j)?;
                    let events: IAudioSessionEvents = self.watcher.cast()?;
                    let _ = session.RegisterAudioSessionNotification(&events);
                    if let Some(app) = recording_app(&session) {
                        apps.push(app);
                    }
                    self.sessions.push((session, events));
                }
                self.managers.push((manager, self.watcher.clone()));
            }
        }
        apps.sort();
        apps.dedup();
        Ok(apps)
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

/// Aplikasi pemilik sesi kalau sesinya sedang aktif merekam (bukan suara sistem, bukan notch).
fn recording_app(session: &IAudioSessionControl) -> Option<String> {
    unsafe {
        if session.GetState().ok()? != AudioSessionStateActive {
            return None;
        }
        let session: IAudioSessionControl2 = session.cast().ok()?;
        if session.IsSystemSoundsSession() == S_OK {
            return None;
        }
        process_name(session.GetProcessId().ok()?)
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
    if pid == 0 || pid == std::process::id() {
        return None;
    }
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 512];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(process);
        ok.then(|| app_name(&String::from_utf16_lossy(&buf[..len as usize])))
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
