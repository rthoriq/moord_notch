//! Pasang, perbarui, dan cabut notch tanpa hak Administrator. Notch disalin ke folder program
//! milik pengguna (`%LOCALAPPDATA%\Programs\NotchWindows`), diberi pintasan di menu Start,
//! didaftarkan di Pengaturan > Aplikasi (bisa dicabut dari sana), dan berjalan bersama Windows.
//!
//! notch.exe yang dijalankan dari tempat lain (misal folder Unduhan) menawarkan untuk memasang
//! dirinya atau memperbarui versi yang terpasang. Tidak ada yang diunduh.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE, HWND, LPARAM, WAIT_TIMEOUT, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, IPersistFile,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW,
};
use windows::Win32::System::Threading::{
    GetExitCodeProcess, INFINITE, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess, WaitForSingleObject,
};
use windows::Win32::UI::Shell::{
    FOLDERID_Programs, FOLDERID_UserProgramFiles, IShellLinkW, KNOWN_FOLDER_FLAG, SEE_MASK_NOASYNC,
    SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, SHGetKnownFolderPath, ShellExecuteExW, ShellLink,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowThreadProcessId, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION,
    MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO, MB_YESNOCANCEL, MESSAGEBOX_RESULT, MESSAGEBOX_STYLE, MessageBoxW,
    PostMessageW, RegisterWindowMessageW, SW_HIDE,
};
use windows::core::{GUID, HSTRING, Interface, PCWSTR, PWSTR, Result, w};

use crate::{autostart, identity};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = "NotchWindows";
/// Pesan untuk meminta notch yang sedang berjalan keluar dengan rapi (ikon tray ikut hilang).
pub const QUIT_MESSAGE: PCWSTR = w!("NotchWindows.Keluar");
/// Kelas jendela notch (lihat `app`), untuk menemukan notch yang sedang berjalan.
const WINDOW_CLASS: PCWSTR = w!("NotchWindow");
const TITLE: PCWSTR = w!("NotchWindows");

/// Build debug bisa menguji pemasangan di folder lain (`NOTCH_INSTALL_ROOT`). Di mode uji itu,
/// nama di registry dan menu Start juga lain, dan hanya notch dari folder uji yang dihentikan,
/// supaya pemasangan sungguhan tidak tersentuh sama sekali.
fn test_root() -> Option<PathBuf> {
    if cfg!(debug_assertions) { std::env::var_os("NOTCH_INSTALL_ROOT").map(PathBuf::from) } else { None }
}

/// Nama notch di registry (Mulai bersama Windows, Aplikasi terpasang) dan di menu Start.
pub fn name() -> &'static str {
    if test_root().is_some() { "NotchWindowsUji" } else { NAME }
}

fn uninstall_key() -> HSTRING {
    HSTRING::from(format!("Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{}", name()))
}

fn shortcut_path() -> Option<PathBuf> {
    known_folder(&FOLDERID_Programs).map(|programs| programs.join(format!("{}.lnk", name())))
}

/// Folder pemasangan: `%LOCALAPPDATA%\Programs\NotchWindows`.
pub fn dir() -> Option<PathBuf> {
    match test_root() {
        Some(root) => Some(root.join(NAME)),
        None => known_folder(&FOLDERID_UserProgramFiles).map(|programs| programs.join(NAME)),
    }
}

pub fn exe() -> Option<PathBuf> {
    dir().map(|dir| dir.join("notch.exe"))
}

/// Notch ini berjalan dari folder pemasangan.
pub fn is_installed_copy() -> bool {
    matches!((std::env::current_exe(), exe()), (Ok(current), Some(installed)) if same_path(&current, &installed))
}

/// Dijalankan paling awal di `main`. `true` berarti proses ini selesai di sini: notch sudah
/// dipasang lalu dijalankan dari folder pemasangan, atau pengguna membatalkan.
pub fn offer() -> bool {
    let (Ok(current), Some(installed)) = (std::env::current_exe(), exe()) else { return false };
    if same_path(&current, &installed) {
        refresh_entry(&installed);
        return false;
    }
    // Build dari folder proyek (`cargo build`) dijalankan apa adanya, tanpa ditawari pasang.
    if current.components().any(|part| part.as_os_str().eq_ignore_ascii_case("target")) {
        return false;
    }
    let old = installed_version().filter(|_| installed.exists());
    let text = match &old {
        // Versi yang sama sudah terpasang: cukup jalankan yang terpasang.
        Some(version) if version == VERSION => {
            launch(&installed, None);
            return true;
        }
        Some(version) => format!(
            "Perbarui NotchWindows {version} ke versi {VERSION}?\n\n\
             Pengaturan dan datamu tetap.\n\n\
             Ya: perbarui.   Tidak: jalankan versi ini sekali tanpa memasang."
        ),
        None => format!(
            "Pasang NotchWindows {VERSION}?\n\n\
             • Disalin ke folder program milikmu, tanpa hak Administrator.\n\
             • Muncul di menu Start dan berjalan bersama Windows.\n\
             • Bisa dicabut kapan saja lewat Pengaturan > Aplikasi.\n\n\
             Ya: pasang.   Tidak: jalankan sekali tanpa memasang."
        ),
    };
    match ask(&text, MB_YESNOCANCEL | MB_ICONQUESTION) {
        IDYES => {
            match install(&current, &installed, old.is_none()) {
                Ok(()) => launch(&installed, None),
                Err(e) => {
                    error_log!("gagal memasang: {e}");
                    ask(&format!("NotchWindows gagal dipasang:\n{e}"), MB_ICONERROR);
                }
            }
            true
        }
        windows::Win32::UI::WindowsAndMessaging::IDNO => false,
        _ => true,
    }
}

/// Pasang tanpa bertanya (`notch.exe --pasang`), lalu jalankan notch yang terpasang.
pub fn install_quietly() -> bool {
    let (Ok(current), Some(installed)) = (std::env::current_exe(), exe()) else { return false };
    if same_path(&current, &installed) {
        return true;
    }
    let fresh = installed_version().is_none() || !installed.exists();
    match install(&current, &installed, fresh) {
        Ok(()) => {
            launch(&installed, None);
            true
        }
        Err(e) => {
            error_log!("gagal memasang: {e}");
            false
        }
    }
}

fn install(source: &Path, target: &Path, fresh: bool) -> std::io::Result<()> {
    let dir = target.parent().ok_or(std::io::ErrorKind::NotFound)?;
    stop_running();
    std::fs::create_dir_all(dir)?;
    // Disalin isinya saja (bukan dengan CopyFile), jadi tanda "diunduh dari internet" tidak ikut:
    // peringatan SmartScreen cukup sekali, saat file unduhan pertama dibuka.
    let bytes = std::fs::read(source)?;
    let temp = dir.join("notch.exe.baru");
    std::fs::write(&temp, bytes)?;
    retry(|| std::fs::rename(&temp, target))?;
    identity::write_assets(dir)?;
    let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if let Err(e) = shortcut(target) {
        error_log!("pintasan menu Start gagal dibuat: {e}");
    }
    write_entry(target).map_err(std::io::Error::other)?;
    // Pemasangan baru langsung berjalan bersama Windows; pembaruan mengikuti pilihan sebelumnya.
    if fresh || autostart::is_enabled() {
        autostart::set_command(target).map_err(std::io::Error::other)?;
    }
    Ok(())
}

/// Dijalankan dari Pengaturan > Aplikasi (`notch.exe --cabut`). Tanpa bertanya
/// (`--cabut --diam`): datamu tetap disimpan dan sertifikat tidak dibuang (butuh Administrator).
pub fn uninstall(quiet: bool) {
    let question = "Cabut NotchWindows dari komputer ini?\n\n\
                    Notch, pintasan menu Start, dan \"Mulai bersama Windows\" akan dihapus.";
    if !quiet && ask(question, MB_YESNO | MB_ICONQUESTION) != IDYES {
        return;
    }
    let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
    stop_running();
    if let Some(dir) = dir()
        && let Err(e) = identity::remove_package(&dir)
    {
        error_log!("paket identitas gagal dicabut: {e}");
    }
    // Sertifikat notch dibuang dari daftar terpercaya (butuh Administrator), kalau tidak ada lagi
    // paket notch yang memakainya. Kalau ditolak pun tidak berbahaya: kuncinya sudah lama dihapus.
    if !quiet && identity::certificate_trusted() && !identity::any_package() {
        let _ = run_elevated("--hapus-sertifikat");
    }
    let _ = autostart::set_enabled(false);
    unsafe {
        let _ = RegDeleteTreeW(HKEY_CURRENT_USER, &uninstall_key());
    }
    if let Some(link) = shortcut_path() {
        let _ = std::fs::remove_file(link);
    }
    let data = "Hapus juga datamu?\n\n\
                Pengaturan, tugas, agenda, catatan minum, dan daftar shelf. File aslinya yang ada \
                di shelf tidak ikut terhapus.";
    if let Some(folder) = crate::store::dir()
        && !quiet
        && ask(data, MB_YESNO | MB_ICONQUESTION) == IDYES
    {
        let _ = std::fs::remove_dir_all(folder);
    }
    if let Some(dir) = dir() {
        remove_later(&dir);
    }
    if !quiet {
        ask("NotchWindows sudah dicabut. Terima kasih sudah mencoba!", MB_ICONINFORMATION);
    }
}

/// Hapus folder pemasangan sesaat setelah proses ini selesai (notch.exe yang sedang berjalan
/// tidak bisa menghapus dirinya sendiri).
fn remove_later(dir: &Path) {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let command = format!("/c ping -n 3 127.0.0.1 >nul & rmdir /s /q \"{}\"", dir.display());
    let spawned = std::process::Command::new("cmd.exe").raw_arg(command).creation_flags(CREATE_NO_WINDOW).spawn();
    if let Err(e) = spawned {
        error_log!("folder pemasangan gagal dihapus: {e}");
    }
}

/// Minta notch yang sedang berjalan keluar. Versi lama yang belum mengenal pesan itu dihentikan.
pub fn stop_running() {
    unsafe {
        let Ok(hwnd) = FindWindowW(WINDOW_CLASS, PCWSTR::null()) else { return };
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 || pid == std::process::id() {
            return;
        }
        let access = PROCESS_SYNCHRONIZE | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION;
        let Ok(process) = OpenProcess(access, false, pid) else { return };
        // Mode uji: notch di luar folder uji (misal yang sedang dipakai sungguhan) dibiarkan.
        if let Some(root) = test_root()
            && !process_path(process).is_some_and(|path| path.starts_with(&root))
        {
            let _ = CloseHandle(process);
            return;
        }
        let _ = PostMessageW(Some(hwnd), RegisterWindowMessageW(QUIT_MESSAGE), WPARAM(0), LPARAM(0));
        if WaitForSingleObject(process, 3_000) == WAIT_TIMEOUT {
            let _ = TerminateProcess(process, 1);
            WaitForSingleObject(process, 3_000);
        }
        let _ = CloseHandle(process);
    }
}

fn process_path(process: HANDLE) -> Option<PathBuf> {
    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;
    unsafe { QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut len).ok()? };
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..len as usize])))
}

/// Jalankan notch (atau salinan lain) dengan argumen `arg`.
pub fn launch(exe: &Path, arg: Option<&str>) {
    let mut command = std::process::Command::new(exe);
    command.args(arg);
    if let Err(e) = command.spawn() {
        error_log!("notch gagal dijalankan: {e}");
    }
}

/// Jalankan notch.exe ini sebagai Administrator dengan argumen `arg` dan tunggu selesai.
/// `Ok(true)` = berhasil, `Ok(false)` = gagal, `Err` = pengguna menolak izin Administrator.
pub fn run_elevated(arg: &str) -> Result<bool> {
    let exe = HSTRING::from(std::env::current_exe().unwrap_or_default().as_os_str());
    let arg = HSTRING::from(arg);
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(exe.as_ptr()),
        lpParameters: PCWSTR(arg.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };
    unsafe {
        ShellExecuteExW(&mut info)?;
        if info.hProcess.is_invalid() {
            return Ok(false);
        }
        WaitForSingleObject(info.hProcess, INFINITE);
        let mut code = 1u32;
        let _ = GetExitCodeProcess(info.hProcess, &mut code);
        let _ = CloseHandle(info.hProcess);
        Ok(code == 0)
    }
}

/// Penolakan izin Administrator di jendela UAC.
pub fn cancelled(e: &windows::core::Error) -> bool {
    e.code() == ERROR_CANCELLED.to_hresult()
}

/// Notch terpasang yang berjalan memperbarui catatannya di Pengaturan > Aplikasi (misal setelah
/// diperbarui), dan membuat pintasan menu Start kalau belum ada.
fn refresh_entry(installed: &Path) {
    if installed_version().as_deref() != Some(VERSION)
        && let Err(e) = write_entry(installed)
    {
        error_log!("catatan Aplikasi terpasang gagal diperbarui: {e}");
    }
    let missing = shortcut_path().is_some_and(|link| !link.exists());
    // Pintasan dibuat lewat COM; di thread utama COM baru disiapkan nanti oleh seret-lepas file
    // (apartemen yang sama, jadi aman disiapkan di sini lebih dulu).
    if missing {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    }
    if missing && let Err(e) = shortcut(installed) {
        error_log!("pintasan menu Start gagal dibuat: {e}");
    }
}

/// Catatan di Pengaturan > Aplikasi (dan Panel Kontrol > Program), lengkap dengan perintah cabut.
fn write_entry(exe: &Path) -> Result<()> {
    let dir = exe.parent().unwrap_or(exe);
    let size_kb = std::fs::metadata(exe).map_or(0, |m| m.len() / 1024) as u32;
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(HKEY_CURRENT_USER, &uninstall_key(), None, None, REG_OPTION_NON_VOLATILE, KEY_WRITE, None, &mut key, None)
            .ok()?;
        let text = |name: PCWSTR, value: &str| {
            let wide: Vec<u16> = value.encode_utf16().chain([0]).collect();
            RegSetValueExW(key, name, None, REG_SZ, Some(std::slice::from_raw_parts(wide.as_ptr().cast(), wide.len() * 2)))
        };
        let number = |name: PCWSTR, value: u32| RegSetValueExW(key, name, None, REG_DWORD, Some(&value.to_le_bytes()));
        let results = [
            text(w!("DisplayName"), "NotchWindows"),
            text(w!("DisplayVersion"), VERSION),
            text(w!("Publisher"), "NotchWindows (open source)"),
            text(w!("DisplayIcon"), &format!("{},0", exe.display())),
            text(w!("InstallLocation"), &dir.display().to_string()),
            text(w!("UninstallString"), &format!("\"{}\" --cabut", exe.display())),
            text(w!("QuietUninstallString"), &format!("\"{}\" --cabut --diam", exe.display())),
            number(w!("NoModify"), 1),
            number(w!("NoRepair"), 1),
            number(w!("EstimatedSize"), size_kb),
        ];
        let _ = RegCloseKey(key);
        results.into_iter().try_for_each(|status| status.ok())
    }
}

fn installed_version() -> Option<String> {
    let mut buffer = [0u16; 64];
    let mut size = (buffer.len() * 2) as u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, &uninstall_key(), w!("DisplayVersion"), RRF_RT_REG_SZ, None, Some(buffer.as_mut_ptr().cast()), Some(&mut size))
            .ok()
            .ok()?;
    }
    let len = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..len]))
}

/// Pintasan "NotchWindows" di menu Start.
fn shortcut(exe: &Path) -> Result<()> {
    let link_path = shortcut_path().ok_or_else(windows::core::Error::empty)?;
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        link.SetPath(&HSTRING::from(exe.as_os_str()))?;
        if let Some(dir) = exe.parent() {
            link.SetWorkingDirectory(&HSTRING::from(dir.as_os_str()))?;
        }
        link.SetDescription(w!("Notch ala MacBook untuk Windows"))?;
        link.SetIconLocation(&HSTRING::from(exe.as_os_str()), 0)?;
        link.cast::<IPersistFile>()?.Save(&HSTRING::from(link_path.as_os_str()), true)
    }
}

fn known_folder(id: &GUID) -> Option<PathBuf> {
    unsafe {
        let path = SHGetKnownFolderPath(id, KNOWN_FOLDER_FLAG(0), None).ok()?;
        let result = path.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(path.0 as *const _));
        result
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    a.as_os_str().to_string_lossy().to_lowercase() == b.as_os_str().to_string_lossy().to_lowercase()
}

/// File yang baru dilepas proses lain kadang masih terkunci sesaat.
fn retry(mut action: impl FnMut() -> std::io::Result<()>) -> std::io::Result<()> {
    let mut last = Ok(());
    for _ in 0..15 {
        last = action();
        if last.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    last
}

pub fn ask(text: &str, style: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    unsafe { MessageBoxW(Some(HWND::default()), &HSTRING::from(text), TITLE, style | MB_TOPMOST | MB_SETFOREGROUND) }
}
