//! Monitor sistem: CPU, GPU, RAM, disk C:, dan kecepatan jaringan. CPU dan GPU memakai
//! Performance Counters Windows (angka yang sama dengan Task Manager). Hanya diukur selama
//! halaman Sistem terbuka; di luar itu objeknya dilepas supaya tidak memakan memori.

use std::time::Instant;

use windows::Win32::NetworkManagement::IpHelper::{FreeMibTable, GetIfTable2, MIB_IF_TABLE2};
use windows::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY,
    PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
    PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::core::{PCWSTR, w};

#[derive(Clone, Default, Debug)]
pub struct Stats {
    /// Persen, 0..100.
    pub cpu: f32,
    /// Persen beban 3D; `None` kalau penghitung GPU tidak tersedia.
    pub gpu: Option<f32>,
    pub ram_used: u64,
    pub ram_total: u64,
    pub disk_free: u64,
    pub disk_total: u64,
    /// Byte per detik.
    pub down: f64,
    pub up: f64,
}

pub struct Monitor {
    query: PDH_HQUERY,
    cpu: PDH_HCOUNTER,
    gpu: Option<PDH_HCOUNTER>,
    /// Total byte jaringan terakhir, untuk menghitung kecepatan.
    network: Option<(Instant, u64, u64)>,
}

impl Monitor {
    pub fn new() -> Option<Self> {
        unsafe {
            let mut query = PDH_HQUERY(std::ptr::null_mut());
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let mut cpu = PDH_HCOUNTER(std::ptr::null_mut());
            if PdhAddEnglishCounterW(query, w!("\\Processor Information(_Total)\\% Processor Utility"), 0, &mut cpu) != 0 {
                PdhCloseQuery(query);
                return None;
            }
            let mut gpu = PDH_HCOUNTER(std::ptr::null_mut());
            let gpu = (PdhAddEnglishCounterW(query, w!("\\GPU Engine(*engtype_3D)\\Utilization Percentage"), 0, &mut gpu)
                == 0)
                .then_some(gpu);
            // Persen dihitung dari dua pengambilan; yang pertama hanya titik awal.
            PdhCollectQueryData(query);
            Some(Self { query, cpu, gpu, network: network_totals().map(|(down, up)| (Instant::now(), down, up)) })
        }
    }

    pub fn sample(&mut self) -> Stats {
        let mut stats = Stats::default();
        unsafe {
            PdhCollectQueryData(self.query);
            stats.cpu = value(self.cpu).unwrap_or(0.0).clamp(0.0, 100.0) as f32;
            stats.gpu = self.gpu.and_then(|gpu| sum(gpu)).map(|v| v.clamp(0.0, 100.0) as f32);

            let mut memory = MEMORYSTATUSEX { dwLength: size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
            if GlobalMemoryStatusEx(&mut memory).is_ok() {
                stats.ram_total = memory.ullTotalPhys;
                stats.ram_used = memory.ullTotalPhys.saturating_sub(memory.ullAvailPhys);
            }
            let _ = GetDiskFreeSpaceExW(w!("C:\\"), None, Some(&mut stats.disk_total), Some(&mut stats.disk_free));
        }
        if let Some((down, up)) = network_totals() {
            let now = Instant::now();
            if let Some((then, last_down, last_up)) = self.network {
                let secs = now.duration_since(then).as_secs_f64().max(0.001);
                stats.down = down.saturating_sub(last_down) as f64 / secs;
                stats.up = up.saturating_sub(last_up) as f64 / secs;
            }
            self.network = Some((now, down, up));
        }
        stats
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe { PdhCloseQuery(self.query) };
    }
}

unsafe fn value(counter: PDH_HCOUNTER) -> Option<f64> {
    let mut out = PDH_FMT_COUNTERVALUE::default();
    let status = unsafe { PdhGetFormattedCounterValue(counter, PDH_FMT_DOUBLE, None, &mut out) };
    (status == 0 && out.CStatus <= 1).then(|| unsafe { out.Anonymous.doubleValue })
}

/// Jumlah semua instance penghitung ber-wildcard (misal mesin 3D tiap proses).
unsafe fn sum(counter: PDH_HCOUNTER) -> Option<f64> {
    let (mut bytes, mut count) = (0u32, 0u32);
    unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None) };
    if bytes == 0 {
        return None;
    }
    let item = size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
    let mut items = vec![PDH_FMT_COUNTERVALUE_ITEM_W::default(); bytes as usize / item + 1];
    let status = unsafe { PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, Some(items.as_mut_ptr())) };
    if status != 0 {
        return None;
    }
    let total = items
        .iter()
        .take(count as usize)
        .filter(|i| i.FmtValue.CStatus <= 1)
        .map(|i| unsafe { i.FmtValue.Anonymous.doubleValue })
        .sum();
    Some(total)
}

/// Total byte diterima/dikirim lewat kartu jaringan fisik yang sedang tersambung.
fn network_totals() -> Option<(u64, u64)> {
    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        if GetIfTable2(&mut table).is_err() || table.is_null() {
            return None;
        }
        let rows = std::slice::from_raw_parts((*table).Table.as_ptr(), (*table).NumEntries as usize);
        // Bit 0 = antarmuka perangkat keras, bit 1 = filter (salinan dari antarmuka lain),
        // bit 2 = ada konektor fisik.
        let totals = rows
            .iter()
            .filter(|r| {
                let flags = r.InterfaceAndOperStatusFlags._bitfield;
                flags & 0b001 != 0 && flags & 0b010 == 0 && flags & 0b100 != 0 && r.OperStatus == IfOperStatusUp
            })
            .fold((0u64, 0u64), |(down, up), r| (down + r.InOctets, up + r.OutOctets));
        FreeMibTable(table.cast());
        Some(totals)
    }
}

/// "9,8 GB"
pub fn gigabytes(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_073_741_824.0).replace('.', ",")
}

/// "1,2 MB/s" atau "80 KB/s"
pub fn rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_048_576.0 {
        format!("{:.1} MB/s", bytes_per_sec / 1_048_576.0).replace('.', ",")
    } else {
        format!("{:.0} KB/s", bytes_per_sec / 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes_and_rates() {
        assert_eq!(gigabytes(10_522_669_875), "9,8 GB");
        assert_eq!(rate(1_258_291.0), "1,2 MB/s");
        assert_eq!(rate(81_920.0), "80 KB/s");
    }

    /// Pengukuran sungguhan di mesin ini; dua kali ambil supaya persen CPU terisi.
    #[test]
    fn samples_this_machine() {
        let mut monitor = Monitor::new().expect("penghitung CPU tersedia");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let stats = monitor.sample();
        assert!(stats.ram_total > 0 && stats.ram_used <= stats.ram_total);
        assert!(stats.disk_total > 0 && stats.disk_free <= stats.disk_total);
        assert!((0.0..=100.0).contains(&stats.cpu));
    }
}
