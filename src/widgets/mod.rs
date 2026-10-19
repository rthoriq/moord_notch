//! Widget di panel notch. Modul-modul ini hanya berisi data dan aturannya (tanpa gambar
//! dan tanpa API Windows), supaya mudah diuji.

pub mod focus;
pub mod shelf;
pub mod tasks;
pub mod water;

/// Halaman panel, urut sesuai titik penanda di bawah panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    /// Jam, atau pemutar musik kalau ada media.
    Home,
    Shelf,
    Focus,
    Water,
    Tasks,
    /// Monitor sistem dan status privasi.
    System,
}

impl Page {
    pub const ALL: [Page; 6] = [Page::Home, Page::Shelf, Page::Focus, Page::Water, Page::Tasks, Page::System];

    pub fn index(self) -> usize {
        Page::ALL.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// Halaman berikutnya (`step` = 1) atau sebelumnya (`step` = -1), berputar di ujung.
    pub fn step(self, step: i32) -> Page {
        let n = Page::ALL.len() as i32;
        Page::ALL[(self.index() as i32 + step).rem_euclid(n) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_wrap_around() {
        assert_eq!(Page::Home.step(-1), Page::System);
        assert_eq!(Page::System.step(1), Page::Home);
        assert_eq!(Page::Shelf.step(1), Page::Focus);
    }
}
