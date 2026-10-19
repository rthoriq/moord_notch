//! Status daya laptop: charger dicolok/dicabut, persentase baterai, dan kecerahan layar.
//! Windows mengirim kabarnya sebagai pesan ke jendela notch, tanpa polling.

use windows::Win32::Foundation::{HANDLE, HWND, LPARAM};
use windows::Win32::System::Power::{
    GetSystemPowerStatus, HPOWERNOTIFY, POWERBROADCAST_SETTING, RegisterPowerSettingNotification,
    SYSTEM_POWER_STATUS, UnregisterPowerSettingNotification,
};
use windows::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING, GUID_VIDEO_CURRENT_MONITOR_BRIGHTNESS,
};
use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_WINDOW_HANDLE;

pub enum Change {
    /// `true` = memakai charger.
    Plugged(bool),
    Percent(u32),
    /// Kecerahan layar bawaan, 0..100.
    Brightness(u32),
}

pub struct Power {
    handles: Vec<HPOWERNOTIFY>,
}

impl Power {
    pub fn register(hwnd: HWND) -> Self {
        let settings = [GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING, GUID_VIDEO_CURRENT_MONITOR_BRIGHTNESS];
        let handles = settings
            .iter()
            .filter_map(|guid| unsafe {
                RegisterPowerSettingNotification(HANDLE(hwnd.0), guid, DEVICE_NOTIFY_WINDOW_HANDLE).ok()
            })
            .collect();
        Self { handles }
    }
}

impl Drop for Power {
    fn drop(&mut self) {
        for handle in self.handles.drain(..) {
            let _ = unsafe { UnregisterPowerSettingNotification(handle) };
        }
    }
}

/// Baca isi `WM_POWERBROADCAST` dengan `PBT_POWERSETTINGCHANGE`.
pub fn parse(lparam: LPARAM) -> Option<Change> {
    let setting = unsafe { (lparam.0 as *const POWERBROADCAST_SETTING).as_ref()? };
    if setting.DataLength < 4 {
        return None;
    }
    let value = unsafe { std::ptr::read_unaligned(setting.Data.as_ptr().cast::<u32>()) };
    match setting.PowerSetting {
        GUID_ACDC_POWER_SOURCE => Some(Change::Plugged(value == 0)), // 0 = listrik AC
        GUID_BATTERY_PERCENTAGE_REMAINING => Some(Change::Percent(value)),
        GUID_VIDEO_CURRENT_MONITOR_BRIGHTNESS => Some(Change::Brightness(value.min(100))),
        _ => None,
    }
}

pub fn battery_percent() -> Option<u32> {
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    (status.BatteryLifePercent <= 100).then_some(status.BatteryLifePercent as u32)
}
