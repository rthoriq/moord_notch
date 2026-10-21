//! Pantau folder Screenshots dan Downloads: kabar saat ada file baru yang sudah selesai
//! ditulis. Windows mengabari perubahan folder, jadi tidak ada pemindaian berkala; thread
//! pemantau tidur sampai ada file yang berubah.

use std::collections::{HashMap, HashSet};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ACTION_ADDED, FILE_ACTION_MODIFIED, FILE_ACTION_RENAMED_NEW_NAME,
    FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OVERLAPPED, FILE_LIST_DIRECTORY, FILE_NOTIFY_CHANGE_FILE_NAME,
    FILE_NOTIFY_CHANGE_LAST_WRITE, FILE_NOTIFY_CHANGE_SIZE, FILE_NOTIFY_INFORMATION, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, ReadDirectoryChangesW,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{CreateEventW, INFINITE, SetEvent, WaitForMultipleObjects};
use windows::Win32::UI::Shell::{FOLDERID_Downloads, FOLDERID_Screenshots, KF_FLAG_CREATE, SHGetKnownFolderPath};
use windows::core::HSTRING;

use crate::services::{Event, Notifier};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Pictures\Screenshots: tempat Win+Shift+S, PrtScn, dan Win+PrtScn menyimpan hasilnya.
    Screenshot,
    Downloads,
}

impl Kind {
    /// Jeda sejak perubahan terakhir sebelum file dianggap selesai. Unduhan lebih lama karena
    /// browser sering menulis beberapa kali sesudah mengganti nama (misal tanda "dari internet").
    fn settle(self) -> Duration {
        match self {
            Kind::Screenshot => Duration::from_millis(600),
            Kind::Downloads => Duration::from_millis(1_500),
        }
    }
}

/// Batas menunggu file yang masih ditulis (misal placeholder unduhan Firefox).
const GIVE_UP: Duration = Duration::from_secs(120);
/// File yang sudah dikabarkan tidak dikabarkan lagi selama ini.
const REMEMBER: Duration = Duration::from_secs(60);
const ERROR_SHARING_VIOLATION: i32 = 32;

/// Pemantau satu folder; berhenti saat dilepas.
pub struct Watch {
    stop: HANDLE,
    thread: Option<JoinHandle<()>>,
}

impl Watch {
    pub fn start(kind: Kind, notify: Notifier) -> Option<Self> {
        let dir = folder(kind)?;
        let stop = unsafe { CreateEventW(None, true, false, None) }.ok()?;
        // HANDLE tidak boleh langsung dibawa ke thread lain; nilainya saja yang dibawa.
        let stop_raw = stop.0 as usize;
        let spawned = thread::Builder::new()
            .name(format!("pantau {kind:?}"))
            .spawn(move || run(&dir, kind, HANDLE(stop_raw as *mut _), &notify));
        match spawned {
            Ok(thread) => Some(Self { stop, thread: Some(thread) }),
            Err(e) => {
                debug_log!("pemantau folder gagal dibuat: {e}");
                let _ = unsafe { CloseHandle(stop) };
                None
            }
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let _ = unsafe { SetEvent(self.stop) };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        let _ = unsafe { CloseHandle(self.stop) };
    }
}

fn folder(kind: Kind) -> Option<PathBuf> {
    let id = match kind {
        Kind::Screenshot => &FOLDERID_Screenshots,
        Kind::Downloads => &FOLDERID_Downloads,
    };
    unsafe {
        // Folder Screenshots baru dibuat Windows saat screenshot pertama; buat sekarang supaya bisa dipantau.
        let path = SHGetKnownFolderPath(id, KF_FLAG_CREATE, None).ok()?;
        let dir = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0 as *const _));
        dir
    }
}

/// File yang sedang ditunggu selesai: kapan pertama dan terakhir berubah.
struct Pending {
    first: Instant,
    last: Instant,
}

fn run(dir: &Path, kind: Kind, stop: HANDLE, notify: &Notifier) {
    let share = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
    let flags = FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OVERLAPPED;
    let opened = unsafe {
        CreateFileW(&HSTRING::from(dir.as_os_str()), FILE_LIST_DIRECTORY.0, share, None, OPEN_EXISTING, flags, None)
    };
    let (handle, event) = match (opened, unsafe { CreateEventW(None, true, false, None) }) {
        (Ok(handle), Ok(event)) => (handle, event),
        (opened, _) => {
            debug_log!("folder {} tidak bisa dipantau", dir.display());
            if let Ok(handle) = opened {
                let _ = unsafe { CloseHandle(handle) };
            }
            return;
        }
    };
    // Buffer harus rata 4 byte; 64 KB cukup untuk ratusan perubahan sekaligus.
    let mut buffer = vec![0u32; 16 * 1024];
    let mut pending: HashMap<String, Pending> = HashMap::new();
    let mut reported: Vec<(String, Instant)> = Vec::new();
    let mut downloading = Downloading::default();
    'watch: loop {
        let mut overlapped = OVERLAPPED { hEvent: event, ..Default::default() };
        let filter = FILE_NOTIFY_CHANGE_FILE_NAME | FILE_NOTIFY_CHANGE_SIZE | FILE_NOTIFY_CHANGE_LAST_WRITE;
        let size = (buffer.len() * 4) as u32;
        let started =
            unsafe { ReadDirectoryChangesW(handle, buffer.as_mut_ptr().cast(), size, false, filter, None, Some(&mut overlapped), None) };
        if started.is_err() {
            break;
        }
        loop {
            let timeout = if !pending.is_empty() {
                300
            } else if !downloading.names.is_empty() {
                1_000
            } else {
                INFINITE
            };
            let woke = unsafe { WaitForMultipleObjects(&[event, stop], false, timeout) };
            if woke == WAIT_OBJECT_0 {
                let mut bytes = 0;
                if unsafe { GetOverlappedResult(handle, &overlapped, &mut bytes, false) }.is_ok() {
                    // 0 byte berarti terlalu banyak perubahan sekaligus; yang terlewat dibiarkan.
                    let now = Instant::now();
                    for name in changed_names(&buffer, bytes as usize) {
                        if kind == Kind::Downloads && partial(&name) {
                            downloading.names.insert(name);
                        } else if wanted(&name, kind) {
                            pending.entry(name).and_modify(|p| p.last = now).or_insert(Pending { first: now, last: now });
                        }
                    }
                }
                check(dir, kind, &mut pending, &mut reported, notify);
                downloading.report(dir, notify);
                continue 'watch;
            } else if woke == WAIT_TIMEOUT {
                check(dir, kind, &mut pending, &mut reported, notify);
                downloading.report(dir, notify);
            } else {
                // Diminta berhenti, atau galat.
                unsafe {
                    let _ = CancelIoEx(handle, Some(&overlapped));
                    let mut bytes = 0;
                    let _ = GetOverlappedResult(handle, &overlapped, &mut bytes, true);
                }
                break 'watch;
            }
        }
    }
    unsafe {
        let _ = CloseHandle(event);
        let _ = CloseHandle(handle);
    }
}

/// Nama file yang ditambahkan, diganti namanya, atau diubah, dari hasil `ReadDirectoryChangesW`.
fn changed_names(buffer: &[u32], bytes: usize) -> Vec<String> {
    let base = buffer.as_ptr().cast::<u8>();
    let mut names = Vec::new();
    let mut offset = 0usize;
    while offset + size_of::<FILE_NOTIFY_INFORMATION>() <= bytes {
        let info = unsafe { &*base.add(offset).cast::<FILE_NOTIFY_INFORMATION>() };
        let len = info.FileNameLength as usize / 2;
        let name = unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), len) };
        if matches!(info.Action, FILE_ACTION_ADDED | FILE_ACTION_RENAMED_NEW_NAME | FILE_ACTION_MODIFIED) {
            names.push(String::from_utf16_lossy(name));
        }
        if info.NextEntryOffset == 0 {
            break;
        }
        offset += info.NextEntryOffset as usize;
    }
    names
}

/// File sementara browser dan file tersembunyi tidak pernah dikabarkan.
fn wanted(name: &str, kind: Kind) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with('~') || lower.starts_with('.') || lower.contains(':') || lower == "desktop.ini" {
        return false;
    }
    let ext = lower.rsplit_once('.').map_or("", |(_, ext)| ext);
    match kind {
        Kind::Screenshot => matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp"),
        Kind::Downloads => !TEMP_EXTENSIONS.contains(&ext),
    }
}

/// File sementara browser untuk unduhan yang sedang berjalan.
fn partial(name: &str) -> bool {
    let lower = name.to_lowercase();
    let ext = lower.rsplit_once('.').map_or("", |(_, ext)| ext);
    matches!(ext, "crdownload" | "part" | "partial" | "download" | "opdownload" | "!ut")
}

/// Unduhan yang sedang berjalan: file sementara browser di Downloads. Selama masih ada, jumlah
/// dan total ukurannya dikabarkan paling sering sekali sedetik.
#[derive(Default)]
struct Downloading {
    names: HashSet<String>,
    /// Kabar terakhir (jumlah, byte); `None` = belum ada unduhan yang dikabarkan.
    reported: Option<(usize, u64)>,
    checked: Option<Instant>,
}

impl Downloading {
    fn report(&mut self, dir: &Path, notify: &Notifier) {
        let now = Instant::now();
        if self.names.is_empty() && self.reported.is_none()
            || self.checked.is_some_and(|t| now.duration_since(t) < Duration::from_millis(900))
        {
            return;
        }
        self.checked = Some(now);
        // File sementara yang hilang berarti unduhannya selesai (diganti nama) atau dibatalkan.
        self.names.retain(|name| dir.join(name).is_file());
        let bytes = self.names.iter().filter_map(|name| std::fs::metadata(dir.join(name)).ok()).map(|m| m.len()).sum();
        let state = (self.names.len() > 0).then_some((self.names.len(), bytes));
        if state != self.reported {
            self.reported = state;
            notify.send(Event::Downloading(state));
        }
    }
}

/// Akhiran file unduhan yang belum selesai (Chrome/Edge, Firefox, Opera, uTorrent, dll.).
const TEMP_EXTENSIONS: [&str; 8] = ["crdownload", "part", "partial", "tmp", "download", "opdownload", "!ut", "temp"];

enum Ready {
    Yes,
    /// Masih ditulis; dicek lagi nanti.
    NotYet,
    /// Sudah hilang, berupa folder, atau sudah dikabarkan.
    Never,
}

fn readiness(path: &Path, kind: Kind) -> Ready {
    let Ok(meta) = std::fs::metadata(path) else { return Ready::Never };
    if !meta.is_file() {
        return Ready::Never;
    }
    if meta.len() == 0 {
        return Ready::NotYet;
    }
    // Firefox membuat file kosong bernama akhir sejak awal, lalu menulis ke "nama.part".
    if kind == Kind::Downloads {
        let busy = TEMP_EXTENSIONS.iter().any(|ext| {
            let mut sibling = path.as_os_str().to_owned();
            sibling.push(format!(".{ext}"));
            Path::new(&sibling).exists()
        });
        if busy {
            return Ready::NotYet;
        }
    }
    // Masih dibuka untuk ditulis oleh aplikasi lain → belum selesai.
    match std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ.0).open(path) {
        Ok(_) => Ready::Yes,
        Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Ready::NotYet,
        Err(_) => Ready::Never,
    }
}

/// Kabarkan file yang sudah diam cukup lama dan selesai ditulis. Beberapa file yang selesai
/// bersamaan (misal hasil ekstrak zip) dikirim dalam satu kabar.
fn check(dir: &Path, kind: Kind, pending: &mut HashMap<String, Pending>, reported: &mut Vec<(String, Instant)>, notify: &Notifier) {
    let now = Instant::now();
    reported.retain(|(_, at)| now.duration_since(*at) < REMEMBER);
    let mut ready = Vec::new();
    pending.retain(|name, p| {
        if now.duration_since(p.last) < kind.settle() {
            return true;
        }
        if reported.iter().any(|(seen, _)| seen.eq_ignore_ascii_case(name)) {
            return false;
        }
        let path = dir.join(name);
        match readiness(&path, kind) {
            Ready::Yes => {
                ready.push(path);
                reported.push((name.clone(), now));
                false
            }
            Ready::NotYet => now.duration_since(p.first) < GIVE_UP,
            Ready::Never => false,
        }
    });
    if !ready.is_empty() {
        ready.sort();
        notify.send(Event::Files { kind, paths: ready });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_downloads_in_progress() {
        assert!(partial("Unconfirmed 1234.crdownload"));
        assert!(partial("film.mp4.part"));
        assert!(!partial("laporan.pdf"));
        assert!(!partial("~$dokumen.tmp"), "file sementara biasa bukan unduhan");
    }

    #[test]
    fn skips_temporary_and_hidden_files() {
        assert!(wanted("Screenshot 2026-10-19 101112.png", Kind::Screenshot));
        assert!(!wanted("catatan.txt", Kind::Screenshot));
        assert!(wanted("laporan.pdf", Kind::Downloads));
        assert!(wanted("installer.EXE", Kind::Downloads));
        for name in ["Unconfirmed 1234.crdownload", "film.mp4.part", "~$dokumen.docx", ".tmp123", "desktop.ini", "a.pdf:Zone.Identifier"] {
            assert!(!wanted(name, Kind::Downloads), "{name}");
        }
    }

    #[test]
    fn waits_for_firefox_placeholder_and_reports_once() {
        let dir = std::env::temp_dir().join(format!("notch-folders-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("lagu.mp3");
        std::fs::write(&file, b"").unwrap();
        assert!(matches!(readiness(&file, Kind::Downloads), Ready::NotYet), "file kosong belum selesai");
        std::fs::write(&file, b"isi").unwrap();
        std::fs::write(dir.join("lagu.mp3.part"), b"sebagian").unwrap();
        assert!(matches!(readiness(&file, Kind::Downloads), Ready::NotYet), "masih ada .part");
        std::fs::remove_file(dir.join("lagu.mp3.part")).unwrap();
        assert!(matches!(readiness(&file, Kind::Downloads), Ready::Yes));
        {
            let _writer = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
            assert!(matches!(readiness(&file, Kind::Downloads), Ready::NotYet), "masih ditulis");
        }
        assert!(matches!(readiness(&dir.join("hilang.pdf"), Kind::Downloads), Ready::Never));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
