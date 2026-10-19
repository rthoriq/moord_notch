//! Widget di panel notch. Modul-modul ini hanya berisi data dan aturannya (tanpa gambar
//! dan tanpa API Windows), supaya mudah diuji.

pub mod focus;
pub mod shelf;
pub mod tasks;
pub mod timer;
pub mod water;

/// Halaman panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    /// Jam, atau pemutar musik kalau ada media.
    Home,
    /// Volume, kecerahan, mikrofon, senyap, dan kota cuaca.
    Controls,
    Shelf,
    Focus,
    Timer,
    Water,
    Tasks,
    /// Monitor sistem dan status privasi.
    System,
}

impl Page {
    pub const ALL: [Page; 8] =
        [Page::Home, Page::Controls, Page::Shelf, Page::Focus, Page::Timer, Page::Water, Page::Tasks, Page::System];

    /// Nama di menu.
    pub fn label(self) -> &'static str {
        match self {
            Page::Home => "Beranda (jam/musik)",
            Page::Controls => "Kontrol cepat",
            Page::Shelf => "Shelf",
            Page::Focus => "Pomodoro",
            Page::Timer => "Timer",
            Page::Water => "Minum air",
            Page::Tasks => "Tugas",
            Page::System => "Sistem",
        }
    }

    /// Nama di file pengaturan.
    fn key(self) -> &'static str {
        match self {
            Page::Home => "beranda",
            Page::Controls => "kontrol",
            Page::Shelf => "shelf",
            Page::Focus => "pomodoro",
            Page::Timer => "timer",
            Page::Water => "minum",
            Page::Tasks => "tugas",
            Page::System => "sistem",
        }
    }
}

/// Urutan halaman dan halaman mana yang disembunyikan. Beranda selalu tampil.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pages {
    order: [Page; 8],
    /// Bit per halaman, menurut urutan di [`Page::ALL`].
    hidden: u8,
}

impl Default for Pages {
    fn default() -> Self {
        Self { order: Page::ALL, hidden: 0 }
    }
}

impl Pages {
    /// Semua halaman sesuai urutan pengguna, termasuk yang disembunyikan.
    pub fn order(&self) -> [Page; 8] {
        self.order
    }

    pub fn shown(&self, page: Page) -> bool {
        self.hidden & bit(page) == 0
    }

    /// Halaman yang tampil, berurutan.
    pub fn visible(&self) -> impl Iterator<Item = Page> + '_ {
        self.order.iter().copied().filter(|p| self.shown(*p))
    }

    /// Posisi halaman di antara yang tampil (untuk titik penanda dan arah geser).
    pub fn index(&self, page: Page) -> usize {
        self.visible().position(|p| p == page).unwrap_or(0)
    }

    /// Halaman tampil berikutnya (`step` = 1) atau sebelumnya (`step` = -1), berputar di ujung.
    pub fn step(&self, from: Page, step: i32) -> Page {
        let visible: Vec<Page> = self.visible().collect();
        let Some(at) = visible.iter().position(|p| *p == from) else { return Page::Home };
        visible[(at as i32 + step).rem_euclid(visible.len() as i32) as usize]
    }

    /// Tampilkan atau sembunyikan; Beranda tidak bisa disembunyikan.
    pub fn set_shown(&mut self, page: Page, shown: bool) {
        if page == Page::Home {
            return;
        }
        if shown {
            self.hidden &= !bit(page);
        } else {
            self.hidden |= bit(page);
        }
    }

    /// Bisa digeser ke kiri (`-1`) atau kanan (`1`) di antara halaman yang tampil?
    pub fn can_move(&self, page: Page, delta: i32) -> bool {
        let i = self.index(page) as i32 + delta;
        self.shown(page) && i >= 0 && i < self.visible().count() as i32
    }

    /// Tukar tempat dengan halaman tampil di sebelahnya.
    pub fn move_by(&mut self, page: Page, delta: i32) {
        if !self.can_move(page, delta) {
            return;
        }
        let neighbour = self.step(page, delta);
        let a = self.order.iter().position(|p| *p == page).unwrap_or(0);
        let b = self.order.iter().position(|p| *p == neighbour).unwrap_or(0);
        self.order.swap(a, b);
    }

    /// "beranda,shelf,-minum,…": urutan, tanda "-" = disembunyikan.
    pub fn to_text(&self) -> String {
        let names: Vec<String> =
            self.order.iter().map(|p| format!("{}{}", if self.shown(*p) { "" } else { "-" }, p.key())).collect();
        names.join(",")
    }

    /// Kebalikan [`Pages::to_text`]. Nama yang tidak dikenal dilewati; halaman yang tidak
    /// disebut ditambahkan di akhir (misal halaman baru dari versi berikutnya).
    pub fn parse(text: &str) -> Self {
        let mut order = Vec::new();
        let mut pages = Self { order: Page::ALL, hidden: 0 };
        for name in text.split(',').map(str::trim) {
            let (hidden, key) = match name.strip_prefix('-') {
                Some(key) => (true, key),
                None => (false, name),
            };
            if let Some(page) = Page::ALL.into_iter().find(|p| p.key() == key)
                && !order.contains(&page)
            {
                order.push(page);
                pages.set_shown(page, !hidden);
            }
        }
        for page in Page::ALL {
            if !order.contains(&page) {
                order.push(page);
            }
        }
        pages.order.copy_from_slice(&order);
        pages
    }
}

fn bit(page: Page) -> u8 {
    1 << Page::ALL.iter().position(|p| *p == page).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_wrap_around() {
        let pages = Pages::default();
        assert_eq!(pages.step(Page::Home, -1), Page::System);
        assert_eq!(pages.step(Page::System, 1), Page::Home);
        assert_eq!(pages.step(Page::Shelf, 1), Page::Focus);
    }

    #[test]
    fn hidden_pages_are_skipped_and_order_round_trips() {
        let mut pages = Pages::default();
        pages.set_shown(Page::Water, false);
        pages.set_shown(Page::Home, false);
        assert!(pages.shown(Page::Home), "beranda selalu tampil");
        assert_eq!(pages.step(Page::Timer, 1), Page::Tasks);
        assert_eq!(pages.visible().count(), 7);

        pages.move_by(Page::System, -1);
        assert_eq!(
            pages.visible().collect::<Vec<_>>(),
            [Page::Home, Page::Controls, Page::Shelf, Page::Focus, Page::Timer, Page::System, Page::Tasks]
        );
        assert!(!pages.can_move(Page::Home, -1));
        assert!(!pages.can_move(Page::Tasks, 1));

        assert_eq!(Pages::parse(&pages.to_text()), pages);
        assert_eq!(pages.to_text(), "beranda,kontrol,shelf,pomodoro,timer,-minum,sistem,tugas");
        // Nama asing dilewati, halaman yang tidak disebut ditambahkan di akhir.
        let partial = Pages::parse("tugas, sampah, -shelf");
        assert_eq!(partial.order()[0], Page::Tasks);
        assert!(!partial.shown(Page::Shelf));
        assert_eq!(partial.visible().count(), 7);
        // Pengaturan lama (sebelum ada halaman baru) tetap terbaca; halaman baru ikut tampil di akhir.
        let old = Pages::parse("beranda,shelf,pomodoro,minum,tugas,sistem");
        assert_eq!(old.order()[6..], [Page::Controls, Page::Timer]);
        assert!(old.shown(Page::Timer));
    }
}
