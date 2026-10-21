//! Shelf: tempat menaruh file sementara. Yang disimpan hanya lokasi filenya (di `shelf.txt`),
//! bukan salinannya, jadi tidak memakan ruang disk.

use std::path::{Path, PathBuf};

use crate::store;

const FILE: &str = "shelf.txt";
const MAX_ITEMS: usize = 30;
/// Ubin yang muat di panel sekaligus; sisanya digulir dengan roda mouse.
pub const VISIBLE: usize = 5;

pub struct Item {
    pub id: u64,
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
}

#[derive(Default)]
pub struct Shelf {
    pub items: Vec<Item>,
    next_id: u64,
    pub scroll: usize,
    /// Ada file yang sedang diseret di atas notch.
    pub drop_hover: bool,
}

impl Shelf {
    /// Muat dari disk; file yang sudah tidak ada dilewati.
    pub fn load() -> Self {
        let mut shelf = Self::default();
        let paths: Vec<PathBuf> = store::load(FILE).into_iter().map(PathBuf::from).filter(|p| p.exists()).collect();
        shelf.add(paths);
        shelf
    }

    pub fn save(&self) {
        store::save(FILE, self.items.iter().map(|item| item.path.to_string_lossy().into_owned()));
    }

    /// Tambahkan file; yang sudah ada di shelf diabaikan. Mengembalikan item baru (id, path)
    /// untuk dibuatkan thumbnail.
    pub fn add(&mut self, paths: impl IntoIterator<Item = PathBuf>) -> Vec<(u64, PathBuf)> {
        let mut added = Vec::new();
        for path in paths {
            if self.items.iter().any(|item| same_path(&item.path, &path)) {
                continue;
            }
            self.next_id += 1;
            let name = path.file_name().map_or_else(|| path.to_string_lossy(), |n| n.to_string_lossy()).into_owned();
            added.push((self.next_id, path.clone()));
            self.items.push(Item { id: self.next_id, is_dir: path.is_dir(), name, path });
        }
        // Yang terlama dibuang kalau shelf penuh.
        let excess = self.items.len().saturating_sub(MAX_ITEMS);
        self.items.drain(..excess);
        // Tampilkan ujung yang baru ditambahkan.
        if !added.is_empty() {
            self.scroll = self.items.len().saturating_sub(VISIBLE);
        }
        added
    }

    pub fn remove(&mut self, id: u64) {
        self.items.retain(|item| item.id != id);
        self.clamp_scroll();
    }

    /// Buang file yang sudah dipindah/dihapus di luar notch. `true` kalau ada yang berubah.
    pub fn prune_missing(&mut self) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.path.exists());
        self.clamp_scroll();
        before != self.items.len()
    }

    pub fn get(&self, id: u64) -> Option<&Item> {
        self.items.iter().find(|item| item.id == id)
    }

    pub fn scroll_by(&mut self, step: i32) {
        self.scroll = (self.scroll as i32 + step).max(0) as usize;
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.min(self.items.len().saturating_sub(VISIBLE));
    }
}

/// Path di Windows tidak membedakan huruf besar/kecil.
fn same_path(a: &Path, b: &Path) -> bool {
    a.as_os_str().eq_ignore_ascii_case(b.as_os_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_skips_duplicates_and_scrolls_to_new_items() {
        let mut shelf = Shelf::default();
        let added = shelf.add((0..7).map(|i| PathBuf::from(format!("C:\\contoh\\file{i}.png"))));
        assert_eq!(added.len(), 7);
        assert_eq!(shelf.scroll, 2);
        assert!(shelf.add([PathBuf::from("c:\\CONTOH\\file3.png")]).is_empty());
        assert_eq!(shelf.items[0].name, "file0.png");

        shelf.remove(added[6].0);
        assert_eq!(shelf.items.len(), 6);
        assert_eq!(shelf.scroll, 1);
        shelf.scroll_by(-5);
        assert_eq!(shelf.scroll, 0);
    }
}
