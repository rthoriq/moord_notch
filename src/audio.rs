//! Volume speaker dan bisu mikrofon: dengarkan perubahannya di perangkat default, pindah
//! otomatis saat perangkat default berganti (misal headset Bluetooth tersambung), dan ubah
//! keduanya dari halaman kontrol cepat. Selama panggilan, mikrofon yang dipakai aplikasi
//! panggilannya (bisa bukan mikrofon default) ikut dipantau untuk tombol bisu panggilan.

use std::sync::Mutex;
use std::sync::mpsc::Sender;

use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, EDataFlow, ERole, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, eCapture, eConsole, eMultimedia,
    eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::{HSTRING, PCWSTR, Result, implement};

use crate::services::{Event, Notifier, Task};

type Endpoint = (IAudioEndpointVolume, IAudioEndpointVolumeCallback);

pub struct Monitor {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
    endpoint: Option<Endpoint>,
    /// Mikrofon default, untuk tombol bisu.
    mic: Option<Endpoint>,
    /// Mikrofon yang dipakai aplikasi panggilan (id perangkat dari `privacy`).
    call: Vec<(String, Endpoint)>,
    /// Bisu mikrofon panggilan yang terakhir dikabarkan.
    call_muted: Option<bool>,
    tasks: Sender<Task>,
    notify: Notifier,
}

impl Monitor {
    pub fn new(tasks: Sender<Task>, notify: Notifier) -> Result<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let client: IMMNotificationClient = DeviceClient { tasks: tasks.clone() }.into();
            enumerator.RegisterEndpointNotificationCallback(&client)?;
            let mut monitor =
                Self { enumerator, client, endpoint: None, mic: None, call: Vec::new(), call_muted: None, tasks, notify };
            monitor.attach_default();
            Ok(monitor)
        }
    }

    /// Dengarkan volume speaker dan bisu mikrofon default yang sekarang.
    pub fn attach_default(&mut self) {
        self.detach();
        match attach(&self.enumerator, &self.notify) {
            Ok(endpoint) => self.endpoint = Some(endpoint),
            Err(e) => error_log!("perangkat audio default tidak bisa dipantau: {e}"),
        }
        self.mic = attach_mic(&self.enumerator, &self.notify).ok();
        if self.mic.is_none() {
            self.notify.send(Event::Mic(None));
        }
    }

    pub fn set_volume(&self, level: f32) {
        if let Some((volume, _)) = &self.endpoint {
            unsafe {
                let _ = volume.SetMasterVolumeLevelScalar(level.clamp(0.0, 1.0), std::ptr::null());
                // Menggeser volume juga membatalkan bisu, seperti slider Windows.
                let _ = volume.SetMute(false, std::ptr::null());
            }
        }
    }

    pub fn toggle_mute(&self) {
        if let Some((volume, _)) = &self.endpoint {
            unsafe {
                if let Ok(muted) = volume.GetMute() {
                    let _ = volume.SetMute(!muted.as_bool(), std::ptr::null());
                }
            }
        }
    }

    pub fn toggle_mic(&self) {
        if let Some((mic, _)) = &self.mic {
            unsafe {
                if let Ok(muted) = mic.GetMute() {
                    let _ = mic.SetMute(!muted.as_bool(), std::ptr::null());
                }
            }
        }
    }

    /// Pantau mikrofon yang sedang dipakai aplikasi panggilan (kosong = tidak ada panggilan).
    pub fn watch_call(&mut self, ids: &[String]) {
        if self.call.iter().map(|(id, _)| id).eq(ids) {
            return;
        }
        self.detach_call();
        for id in ids {
            match attach_call(&self.enumerator, id, &self.tasks) {
                Ok(endpoint) => self.call.push((id.clone(), endpoint)),
                Err(e) => debug_log!("mikrofon panggilan tidak bisa dipantau: {e}"),
            }
        }
        self.report_call();
    }

    /// Kabarkan bisu mikrofon panggilan kalau berubah: bisu kalau semuanya dibisukan, `None`
    /// kalau tidak ada panggilan.
    pub fn report_call(&mut self) {
        let muted = (!self.call.is_empty())
            .then(|| self.call.iter().all(|(_, (volume, _))| unsafe { volume.GetMute() }.is_ok_and(|m| m.as_bool())));
        if muted != self.call_muted {
            self.call_muted = muted;
            self.notify.send(Event::CallMic(muted));
        }
    }

    /// Bisukan mikrofon panggilan, atau nyalakan lagi kalau semuanya sudah bisu. Tanpa panggilan,
    /// mikrofon default yang diubah. Mengembalikan keadaan barunya (true = bisu).
    pub fn toggle_call_mic(&self) -> Option<bool> {
        let volumes: Vec<&IAudioEndpointVolume> = if self.call.is_empty() {
            self.mic.iter().map(|(volume, _)| volume).collect()
        } else {
            self.call.iter().map(|(_, (volume, _))| volume).collect()
        };
        if volumes.is_empty() {
            return None;
        }
        unsafe {
            let muted = !volumes.iter().all(|volume| volume.GetMute().is_ok_and(|m| m.as_bool()));
            for volume in volumes {
                let _ = volume.SetMute(muted, std::ptr::null());
            }
            Some(muted)
        }
    }

    fn detach(&mut self) {
        for (volume, callback) in [self.endpoint.take(), self.mic.take()].into_iter().flatten() {
            let _ = unsafe { volume.UnregisterControlChangeNotify(&callback) };
        }
    }

    fn detach_call(&mut self) {
        for (_, (volume, callback)) in self.call.drain(..) {
            let _ = unsafe { volume.UnregisterControlChangeNotify(&callback) };
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.detach();
        self.detach_call();
        let _ = unsafe { self.enumerator.UnregisterEndpointNotificationCallback(&self.client) };
    }
}

fn attach(enumerator: &IMMDeviceEnumerator, notify: &Notifier) -> Result<Endpoint> {
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        // Nilai awal disimpan supaya kabar pertama yang sama tidak memunculkan HUD, dan dikirim
        // sebagai keadaan awal untuk slider di halaman kontrol.
        let (level, muted) = (volume.GetMasterVolumeLevelScalar()?, volume.GetMute()?.as_bool());
        notify.send(Event::Volume { level, muted, initial: true });
        let callback: IAudioEndpointVolumeCallback =
            VolumeCallback { notify: notify.clone(), last: Mutex::new(Some(key(level, muted))) }.into();
        volume.RegisterControlChangeNotify(&callback)?;
        Ok((volume, callback))
    }
}

fn attach_mic(enumerator: &IMMDeviceEnumerator, notify: &Notifier) -> Result<Endpoint> {
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eCapture, eConsole)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let muted = volume.GetMute()?.as_bool();
        notify.send(Event::Mic(Some(muted)));
        let callback: IAudioEndpointVolumeCallback = MicCallback { notify: notify.clone(), last: Mutex::new(muted) }.into();
        volume.RegisterControlChangeNotify(&callback)?;
        Ok((volume, callback))
    }
}

/// Mikrofon dengan id `id`; perubahan bisunya diminta dibaca ulang lewat [`Task::CallMicChanged`].
fn attach_call(enumerator: &IMMDeviceEnumerator, id: &str, tasks: &Sender<Task>) -> Result<Endpoint> {
    unsafe {
        let device = enumerator.GetDevice(&HSTRING::from(id))?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        let muted = volume.GetMute()?.as_bool();
        let callback: IAudioEndpointVolumeCallback = CallCallback { tasks: tasks.clone(), last: Mutex::new(muted) }.into();
        volume.RegisterControlChangeNotify(&callback)?;
        Ok((volume, callback))
    }
}

/// Volume dibulatkan ke persen supaya kabar yang nyaris sama tidak dihitung perubahan.
fn key(level: f32, muted: bool) -> (u32, bool) {
    ((level * 100.0).round() as u32, muted)
}

#[implement(IAudioEndpointVolumeCallback)]
struct VolumeCallback {
    notify: Notifier,
    last: Mutex<Option<(u32, bool)>>,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        let Some(data) = (unsafe { data.as_ref() }) else { return Ok(()) };
        let (level, muted) = (data.fMasterVolume, data.bMuted.as_bool());
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if *last != Some(key(level, muted)) {
            *last = Some(key(level, muted));
            self.notify.send(Event::Volume { level, muted, initial: false });
        }
        Ok(())
    }
}

/// Hanya bisu/tidaknya mikrofon yang dikabarkan; volume rekamnya tidak dipakai.
#[implement(IAudioEndpointVolumeCallback)]
struct MicCallback {
    notify: Notifier,
    last: Mutex<bool>,
}

impl IAudioEndpointVolumeCallback_Impl for MicCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        let Some(data) = (unsafe { data.as_ref() }) else { return Ok(()) };
        let muted = data.bMuted.as_bool();
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if *last != muted {
            *last = muted;
            self.notify.send(Event::Mic(Some(muted)));
        }
        Ok(())
    }
}

/// Mikrofon panggilan dibisukan atau dinyalakan (misal dari Zoom): bisu semua mikrofon
/// panggilannya dibaca ulang di thread layanan. Volume rekam yang diatur otomatis oleh aplikasi
/// panggilan diabaikan.
#[implement(IAudioEndpointVolumeCallback)]
struct CallCallback {
    tasks: Sender<Task>,
    last: Mutex<bool>,
}

impl IAudioEndpointVolumeCallback_Impl for CallCallback_Impl {
    fn OnNotify(&self, data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> Result<()> {
        let Some(data) = (unsafe { data.as_ref() }) else { return Ok(()) };
        let muted = data.bMuted.as_bool();
        let mut last = self.last.lock().unwrap_or_else(|e| e.into_inner());
        if *last != muted {
            *last = muted;
            let _ = self.tasks.send(Task::CallMicChanged);
        }
        Ok(())
    }
}

#[implement(IMMNotificationClient)]
struct DeviceClient {
    tasks: Sender<Task>,
}

impl IMMNotificationClient_Impl for DeviceClient_Impl {
    fn OnDeviceStateChanged(&self, _id: &PCWSTR, _state: DEVICE_STATE) -> Result<()> {
        // Perangkat dicolok/dicabut; pemantau mikrofon juga perlu memperbarui daftarnya.
        let _ = self.tasks.send(Task::AudioDeviceChanged);
        Ok(())
    }

    fn OnDeviceAdded(&self, _id: &PCWSTR) -> Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, _id: &PCWSTR) -> Result<()> {
        Ok(())
    }

    fn OnDefaultDeviceChanged(&self, flow: EDataFlow, role: ERole, _id: &PCWSTR) -> Result<()> {
        if (flow == eRender && role == eMultimedia) || (flow == eCapture && role == eConsole) {
            let _ = self.tasks.send(Task::AudioDeviceChanged);
        }
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
