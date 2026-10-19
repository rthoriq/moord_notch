//! Daftar tugas sederhana. Disimpan di `todo.txt` dengan format "[ ] teks" / "[x] teks",
//! jadi tetap enak dibaca dan diedit dengan Notepad.

use crate::store;

const FILE: &str = "todo.txt";
pub const MAX_LEN: usize = 120;
/// Baris yang muat di panel sekaligus; sisanya digulir dengan roda mouse.
pub const VISIBLE: usize = 3;

pub struct Task {
    pub text: String,
    pub done: bool,
}

#[derive(Default)]
pub struct Tasks {
    pub items: Vec<Task>,
    /// Teks yang sedang diketik, selama kotak tambah tugas aktif.
    pub draft: Option<String>,
    pub scroll: usize,
}

impl Tasks {
    pub fn load() -> Self {
        Self::parse(store::load(FILE))
    }

    pub fn save(&self) {
        store::save(FILE, self.lines());
    }

    fn parse(lines: Vec<String>) -> Self {
        let items = lines
            .iter()
            .filter_map(|line| {
                let (done, text) = if let Some(text) = line.strip_prefix("[x] ") {
                    (true, text)
                } else {
                    (false, line.strip_prefix("[ ] ").unwrap_or(line))
                };
                let text = clean(text);
                (!text.is_empty()).then_some(Task { text, done })
            })
            .collect();
        Self { items, ..Default::default() }
    }

    fn lines(&self) -> Vec<String> {
        self.items.iter().map(|t| format!("[{}] {}", if t.done { "x" } else { " " }, t.text)).collect()
    }

    /// Tambah tugas; teks kosong diabaikan.
    pub fn add(&mut self, text: &str) -> bool {
        let text = clean(text);
        if text.is_empty() {
            return false;
        }
        self.items.push(Task { text, done: false });
        true
    }

    pub fn toggle(&mut self, index: usize) {
        if let Some(task) = self.items.get_mut(index) {
            task.done = !task.done;
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            self.clamp_scroll();
        }
    }

    pub fn clear_done(&mut self) {
        self.items.retain(|t| !t.done);
        self.clamp_scroll();
    }

    pub fn remaining(&self) -> usize {
        self.items.iter().filter(|t| !t.done).count()
    }

    pub fn has_done(&self) -> bool {
        self.items.iter().any(|t| t.done)
    }

    /// Urutan tampil: yang belum selesai dulu, lalu yang selesai. Isinya indeks ke `items`.
    pub fn order(&self) -> Vec<usize> {
        let (mut open, done): (Vec<usize>, Vec<usize>) = (0..self.items.len()).partition(|&i| !self.items[i].done);
        open.extend(done);
        open
    }

    /// Gulir supaya tugas `index` terlihat.
    pub fn reveal(&mut self, index: usize) {
        if let Some(pos) = self.order().iter().position(|&i| i == index) {
            if pos < self.scroll {
                self.scroll = pos;
            } else if pos >= self.scroll + VISIBLE {
                self.scroll = pos + 1 - VISIBLE;
            }
        }
    }

    pub fn scroll_by(&mut self, step: i32) {
        self.scroll = (self.scroll as i32 + step).max(0) as usize;
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.min(self.items.len().saturating_sub(VISIBLE));
    }
}

/// Satu baris, tanpa karakter kontrol, dan tidak terlalu panjang.
pub fn clean(text: &str) -> String {
    let single: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    single.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(MAX_LEN).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_toggle_order_and_round_trip() {
        let mut tasks = Tasks::default();
        assert!(tasks.add("  Beli   kopi \n"));
        assert!(!tasks.add("   "));
        tasks.add("Kerjakan laporan");
        tasks.add("Telepon ibu");
        tasks.toggle(0);
        assert_eq!(tasks.remaining(), 2);
        assert_eq!(tasks.order(), vec![1, 2, 0]);
        assert_eq!(tasks.items[0].text, "Beli kopi");

        let again = Tasks::parse(tasks.lines());
        assert_eq!(again.items.len(), 3);
        assert!(again.items[0].done);
        assert_eq!(again.items[2].text, "Telepon ibu");
    }

    #[test]
    fn clear_done_and_scroll_limits() {
        let mut tasks = Tasks::default();
        for i in 0..6 {
            tasks.add(&format!("Tugas {i}"));
        }
        tasks.scroll_by(10);
        assert_eq!(tasks.scroll, 3);
        for i in 0..4 {
            tasks.toggle(i);
        }
        tasks.clear_done();
        assert_eq!(tasks.items.len(), 2);
        assert_eq!(tasks.scroll, 0);
        tasks.scroll_by(-5);
        assert_eq!(tasks.scroll, 0);
    }

    #[test]
    fn clean_limits_length() {
        assert_eq!(clean(&"a".repeat(500)).len(), MAX_LEN);
        assert_eq!(clean("satu\tdua\r\ntiga"), "satu dua tiga");
    }
}
