//! Status Wi-Fi untuk kabar tersambung/terputus. Windows mengabari setiap perubahan jaringan;
//! keadaannya baru dibaca setelah kabarnya reda (lihat `services`). Nama jaringan diambil dari
//! nama profilnya, jadi tidak perlu izin lokasi.

use std::sync::mpsc::Sender;

use windows::Networking::Connectivity::{
    ConnectionProfile, NetworkConnectivityLevel, NetworkInformation, NetworkStatusChangedEventHandler,
};
use windows::core::Result;

use crate::services::Task;

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct State {
    /// Nama Wi-Fi yang sedang tersambung; `None` kalau tidak memakai Wi-Fi.
    pub wifi: Option<String>,
    pub internet: bool,
}

pub struct Monitor {
    token: i64,
}

impl Monitor {
    pub fn new(tasks: Sender<Task>) -> Result<Self> {
        let handler = NetworkStatusChangedEventHandler::new(move |_| {
            let _ = tasks.send(Task::NetworkChanged);
            Ok(())
        });
        Ok(Self { token: NetworkInformation::NetworkStatusChanged(&handler)? })
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = NetworkInformation::RemoveNetworkStatusChanged(self.token);
    }
}

pub fn state() -> State {
    // Profil yang dipakai ke internet; kalau tidak ada, Wi-Fi yang tersambung tanpa internet.
    let profile = NetworkInformation::GetInternetConnectionProfile().ok().or_else(connected_wifi);
    let Some(profile) = profile else { return State::default() };
    let level = profile.GetNetworkConnectivityLevel().unwrap_or(NetworkConnectivityLevel::None);
    let wifi = profile
        .IsWlanConnectionProfile()
        .unwrap_or(false)
        .then(|| profile.ProfileName().map(|name| name.to_string()).unwrap_or_default());
    State { wifi, internet: level == NetworkConnectivityLevel::InternetAccess }
}

fn connected_wifi() -> Option<ConnectionProfile> {
    let profiles = NetworkInformation::GetConnectionProfiles().ok()?;
    profiles.into_iter().find(|p| {
        p.IsWlanConnectionProfile().unwrap_or(false)
            && p.GetNetworkConnectivityLevel().is_ok_and(|level| level != NetworkConnectivityLevel::None)
    })
}
