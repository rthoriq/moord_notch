//! Penyimpanan lokal berupa file teks biasa di `%APPDATA%\NotchWindows`. Isinya bisa dibaca
//! dan diedit sendiri dengan Notepad; tidak ada yang dikirim ke mana pun.

use std::path::PathBuf;

/// Folder data notch: `%APPDATA%\NotchWindows`.
pub fn dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("NotchWindows"))
}

/// Baris-baris file `name`; kosong kalau file belum ada.
pub fn load(name: &str) -> Vec<String> {
    dir()
        .and_then(|dir| std::fs::read_to_string(dir.join(name)).ok())
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Tulis ulang file `name`. Ditulis ke file sementara dulu lalu diganti, supaya file lama
/// tidak rusak kalau laptop mati di tengah penulisan.
pub fn save(name: &str, lines: impl IntoIterator<Item = String>) {
    let Some(dir) = dir() else { return };
    let mut text = String::new();
    for line in lines {
        text.push_str(&line);
        text.push('\n');
    }
    let temp = dir.join(format!("{name}.tmp"));
    let result = std::fs::create_dir_all(&dir)
        .and_then(|_| std::fs::write(&temp, text))
        .and_then(|_| std::fs::rename(&temp, dir.join(name)));
    if let Err(e) = result {
        error_log!("gagal menyimpan {name}: {e}");
    }
}
