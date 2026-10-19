//! Volume speaker: dengarkan perubahan volume perangkat output default, dan pindah
//! otomatis saat perangkat default berganti (misal headset Bluetooth tersambung).

use std::sync::Mutex;
use std::sync::mpsc::Sender;

use windows::Win32::Foundation::PROPERTYKEY;
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, EDataFlow, ERole, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator, eMultimedia, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::core::{PCWSTR, Result, implement};

use crate::services::{Event, Notifier, Task};

pub struct Monitor {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
    endpoint: Option<(IAudioEndpointVolume, IAudioEndpointVolumeCallback)>,
    notify: Notifier,
}

impl Monitor {
    pub fn new(tasks: Sender<Task>, notify: Notifier) -> Result<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let client: IMMNotificationClient = DeviceClient { tasks }.into();
            enumerator.RegisterEndpointNotificationCallback(&client)?;
            let mut monitor = Self { enumerator, client, endpoint: None, notify };
            monitor.attach_default();
            Ok(monitor)
        }
    }

    /// Dengarkan volume perangkat output default yang sekarang.
    pub fn attach_default(&mut self) {
        self.detach();
        match attach(&self.enumerator, &self.notify) {
            Ok(endpoint) => self.endpoint = Some(endpoint),
            Err(e) => debug_log!("perangkat audio default tidak bisa dipantau: {e}"),
        }
    }

    fn detach(&mut self) {
        if let Some((volume, callback)) = self.endpoint.take() {
            let _ = unsafe { volume.UnregisterControlChangeNotify(&callback) };
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.detach();
        let _ = unsafe { self.enumerator.UnregisterEndpointNotificationCallback(&self.client) };
    }
}

fn attach(
    enumerator: &IMMDeviceEnumerator,
    notify: &Notifier,
) -> Result<(IAudioEndpointVolume, IAudioEndpointVolumeCallback)> {
    unsafe {
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        // Nilai awal disimpan supaya kabar pertama yang sama tidak memunculkan HUD.
        let current = key(volume.GetMasterVolumeLevelScalar()?, volume.GetMute()?.as_bool());
        let callback: IAudioEndpointVolumeCallback =
            VolumeCallback { notify: notify.clone(), last: Mutex::new(Some(current)) }.into();
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
            self.notify.send(Event::Volume { level, muted });
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
        if flow == eRender && role == eMultimedia {
            let _ = self.tasks.send(Task::AudioDeviceChanged);
        }
        Ok(())
    }

    fn OnPropertyValueChanged(&self, _id: &PCWSTR, _key: &PROPERTYKEY) -> Result<()> {
        Ok(())
    }
}
