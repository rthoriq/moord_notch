//! Video Reels lengkap (±3 menit, 1080×1920, 30 fps): semua fitur notch bab demi bab, masing-
//! masing dengan animasinya sendiri. Memakai studio, encoder, dan mesin musik yang sama dengan
//! video teaser; musiknya disusun dari energi tiap adegan, ditambah efek suara tiap adegan.
//!
//! `cargo test --release reels_cuplikan -- --ignored` menyimpan tiga frame per adegan,
//! `reels_bab` merender satu bab jadi MP4 (pilih dengan variabel `BAB`, misal `BAB=1`),
//! `reels_musik` merender musiknya saja, dan `reels_video` merender seluruh video ke
//! `aset-video/NotchWindows-reels.mp4`.

mod awal;
mod berbagi;
mod gaya;
mod kabar;
mod kontrol;
mod lagu;
mod penutup;
mod peraga;
mod pintar;
mod privasi;
mod produktif;
mod ringkas;

use std::path::PathBuf;

use super::musik::{self, Analysis, Cue, Energy, Score};
use super::*;

/// Satu adegan: panjangnya dalam ketukan (0,5 detik), energi musiknya, cara menggambarnya pada
/// detik lokal `t` dari panjang `len`, efek suaranya, dan rentang gerak cepat yang diberi blur
/// gerak (mulai, selesai, jumlah sub-frame).
pub(super) struct Shot {
    pub(super) beats: u32,
    pub(super) energy: Energy,
    pub(super) draw: fn(&mut Studio, &Cast, f64, f64),
    pub(super) sound: fn(&mut Score),
    pub(super) blur: &'static [(f64, f64, usize)],
}

pub(super) struct Chapter {
    pub(super) title: &'static str,
    pub(super) tint: u32,
    pub(super) shots: Vec<Shot>,
}

/// Bab 0 adalah pembuka tanpa kartu; bab lain diawali kartu bernomor.
fn chapters() -> Vec<Chapter> {
    vec![
        awal::chapter(),
        lagu::chapter(),
        ringkas::chapter(),
        berbagi::chapter(),
        kabar::chapter(),
        produktif::chapter(),
        kontrol::chapter(),
        privasi::chapter(),
        pintar::chapter(),
        gaya::chapter(),
        penutup::chapter(),
    ]
}

/// Kartu topik sekitar satu detik.
const CARD_SECONDS: f64 = 1.0;
/// Batas panjang Reels.
const LIMIT: f64 = 179.5;

/// Satu potongan di timeline: kartu bab (`shot` kosong), atau adegan ke-`shot` dari bab
/// ke-`chapter`.
struct Slot {
    start: f64,
    len: f64,
    chapter: usize,
    shot: Option<usize>,
}

struct Film {
    chapters: Vec<Chapter>,
    slots: Vec<Slot>,
    length: f64,
}

impl Film {
    /// Timeline dengan ketukan sepanjang `beat` detik (0,5 untuk musik sintesis, atau ketukan
    /// lagu). Setiap adegan panjangnya bulat dalam ketukan, dan setiap topik mulai di awal birama;
    /// kalau videonya jadi lewat batas Reels, cukup di awal setengah birama, atau di ketukan.
    fn new(beat: f64) -> Self {
        BEAT_LEN.with(|b| b.set(beat));
        let chapters = chapters();
        for unit in [4, 2, 1] {
            let slots = Self::layout(&chapters, beat, unit);
            let length = slots.last().map_or(0.0, |slot| slot.start + slot.len);
            if length <= LIMIT || unit == 1 {
                return Self { chapters, slots, length };
            }
        }
        unreachable!()
    }

    fn layout(chapters: &[Chapter], beat: f64, unit: u32) -> Vec<Slot> {
        // (ketukan awal, panjang dalam ketukan, bab, adegan)
        let mut plan: Vec<(u32, u32, usize, Option<usize>)> = Vec::new();
        let mut at = 0u32;
        let card = ((CARD_SECONDS / beat).round() as u32).max(2);
        for (c, chapter) in chapters.iter().enumerate() {
            let begin = at;
            if c > 0 {
                plan.push((at, card, c, None));
                at += card;
            }
            for (i, shot) in chapter.shots.iter().enumerate() {
                // Panjang rancangannya (dalam ketukan 0,5 detik) dibulatkan ke ketukan lagu, paling
                // banyak seperempat detik lebih pendek.
                let seconds = shot.beats as f64 * BEAT;
                let mut n = ((seconds / beat).round() as u32).max(1);
                if n as f64 * beat < seconds - 0.25 {
                    n += 1;
                }
                plan.push((at, n, c, Some(i)));
                at += n;
            }
            let pad = (unit - (at - begin) % unit) % unit;
            if let Some(last) = plan.last_mut() {
                last.1 += pad;
            }
            at += pad;
        }
        plan.into_iter()
            .map(|(start, n, chapter, shot)| Slot { start: start as f64 * beat, len: n as f64 * beat, chapter, shot })
            .collect()
    }

    fn slot(&self, t: f64) -> &Slot {
        self.slots.iter().rev().find(|slot| slot.start <= t + 1e-9).unwrap_or(&self.slots[0])
    }

    fn shot(&self, slot: &Slot) -> Option<&Shot> {
        slot.shot.map(|i| &self.chapters[slot.chapter].shots[i])
    }

    /// Detik awal dan akhir bab ke-`chapter`.
    fn range(&self, chapter: usize) -> (f64, f64) {
        let mine = self.slots.iter().filter(|slot| slot.chapter == chapter);
        let start = mine.clone().map(|slot| slot.start).fold(f64::INFINITY, f64::min);
        let end = mine.map(|slot| slot.start + slot.len).fold(0.0, f64::max);
        (start, end)
    }

    /// Detik awal setiap bab (untuk garis di gambar pemeriksa musik).
    fn cuts(&self) -> Vec<f64> {
        (1..self.chapters.len()).map(|c| self.range(c).0).collect()
    }

    fn frame(&self, studio: &mut Studio, cast: &Cast, t: f64) -> Vec<u32> {
        let slot = self.slot(t);
        let local = t - slot.start;
        studio.begin();
        match self.shot(slot) {
            Some(shot) => (shot.draw)(studio, cast, local, slot.len),
            None => {
                let chapter = &self.chapters[slot.chapter];
                card(studio, local, slot.len, slot.chapter, chapter.title, chapter.tint);
            }
        }
        studio.end()
    }

    fn subframes(&self, t: f64) -> usize {
        let slot = self.slot(t);
        let local = t - slot.start;
        match self.shot(slot) {
            Some(shot) => shot.blur.iter().find(|(a, b, _)| (*a..*b).contains(&local)).map_or(1, |w| w.2),
            None if local > slot.len - 0.3 => 8,
            None => 1,
        }
    }

    /// Satu frame dengan blur gerak: beberapa sub-frame mulai dari detik frame ini dirata-rata,
    /// jadi potongan tepat di detik frame tidak pernah tercampur.
    fn render(&self, studio: &mut Studio, cast: &Cast, t: f64) -> Vec<u32> {
        let count = self.subframes(t);
        if count == 1 {
            return self.frame(studio, cast, t);
        }
        let shutter = 0.6 / FPS;
        let mut sum = vec![[0u32; 4]; (W * H) as usize];
        for i in 0..count {
            let pixels = self.frame(studio, cast, t + shutter * i as f64 / (count - 1) as f64);
            for (acc, px) in sum.iter_mut().zip(&pixels) {
                for (c, slot) in acc.iter_mut().enumerate() {
                    *slot += (px >> (c * 8)) & 0xFF;
                }
            }
        }
        sum.iter()
            .map(|acc| acc.iter().enumerate().fold(0u32, |out, (c, v)| out | ((v / count as u32) << (c * 8))))
            .collect()
    }

    /// Partitur seluruh video: energi dan efek suara tiap adegan, nada naik mulai bab 8.
    fn score(&self) -> Score {
        let mut score = Score::new(self.length);
        for slot in &self.slots {
            score.offset = slot.start;
            match self.shot(slot) {
                Some(shot) => {
                    score.section(slot.start, slot.start + slot.len, shot.energy, slot.chapter);
                    (shot.sound)(&mut score);
                }
                None => {
                    score.section(slot.start, slot.start + slot.len, Energy::Break, slot.chapter);
                    score.cue_pan(0.0, (-0.6, 0.6), Cue::Whoosh { len: 0.8, from: 4000.0, to: 300.0, pan: (-0.6, 0.6), gain: 0.2 });
                    score.cue(0.05, Cue::Bell { midi: 76.0, gain: 0.14 });
                }
            }
        }
        if self.chapters.len() > 8 {
            score.lift = self.range(8).0;
        }
        score.offset = 0.0;
        score
    }

    fn describe(&self) {
        eprintln!("ketukan {:.4} detik ({:.1} BPM)", BEAT_LEN.with(|b| b.get()), 60.0 / BEAT_LEN.with(|b| b.get()));
        for slot in &self.slots {
            let name = self.chapters[slot.chapter].title;
            let what = slot.shot.map_or("kartu".to_string(), |i| format!("adegan {}", i + 1));
            eprintln!("{:7.2}–{:7.2}  bab {:2} {name}: {what}", slot.start, slot.start + slot.len, slot.chapter);
        }
        eprintln!("total {:.1} detik", self.length);
    }
}

// ---------------------------------------------------------------- tata letak bersama

/// Pergantian topik: hanya nama topiknya, masuk dari kabur di atas garis cahaya, lalu melesat
/// ke arah kamera saat keluar.
fn card(studio: &mut Studio, t: f64, len: f64, seed: usize, title: &str, tint: u32) {
    studio.background(t + seed as f64 * 7.0, 0.85, 0.0, tint);
    let leave = ease_in_out(span(t, len - 0.3, 0.3));
    let enter = ease_out(span(t, 0.0, 0.35));
    studio.glow(540.0, 900.0, 640.0, 360.0, tint, 0.35 * enter * (1.0 - leave));
    studio.streak(540.0, 1010.0, 460.0 * ease_out(span(t, 0.1, 0.5)), tint, 0.8 * (1.0 - leave));
    studio.world(camera(1.0 + 0.6 * leave, 540.0, 900.0, 0.0, 0.0));
    // Judul panjang diperkecil supaya tetap muat dengan jarak dari tepi layar.
    let size = (140.0 * 900.0 / studio.measure(title, SERIF, 140.0, false)).min(140.0);
    studio.reveal(t, 0.05, 0.35, title, SERIF, size, false, 900.0, 0xFFFFFF, None);
    studio.reset();
    studio.fade(0.5 * leave);
}

/// Judul fitur (serif besar) dan satu baris penjelasan di bawahnya, masuk dari kabur dan
/// memudar menjelang akhir adegan.
fn caption(studio: &mut Studio, t: f64, len: f64, title: &str, sub: &str) {
    caption_at(studio, t, len, 1300.0, title, sub);
}

fn caption_at(studio: &mut Studio, t: f64, len: f64, y: f32, title: &str, sub: &str) {
    let out = Some(len + 0.05);
    studio.reveal(t, 0.12, 0.35, title, SERIF, 104.0, false, y, 0xFFFFFF, out);
    studio.reveal(t, 0.3, 0.35, sub, SANS, 44.0, false, y + 100.0, 0xBFD8FF, out);
}

/// Label kecil berbentuk pil, misal "opsional · pakai internet".
fn badge(studio: &mut Studio, t: f64, at: f64, len: f64, text: &str, y: f32, tint: u32) {
    let p = ease_out(span(t, at, 0.3)) * (1.0 - span(t, len - 0.2, 0.2));
    if p <= 0.0 {
        return;
    }
    let w = studio.measure(text, SANS, 34.0, true) + 56.0;
    let area = rect(540.0 - w / 2.0, y - 30.0 + (1.0 - p) * 16.0, 540.0 + w / 2.0, y + 30.0 + (1.0 - p) * 16.0);
    studio.rounded(area, 30.0, tint, 0.18 * p);
    studio.outline(area, 30.0, 2.0, tint, 0.7 * p);
    studio.label(text, SANS, 34.0, true, 540.0, (area.top + area.bottom) / 2.0, 0xFFFFFF, p, 0);
}

/// Latar bab dengan denyut di setiap ketukan lagu.
fn backdrop(studio: &mut Studio, t: f64, tint: u32) {
    studio.background(t, 0.6 + 0.25 * beat_pulse(t), 0.0, tint);
}

thread_local! {
    /// Panjang satu ketukan lagu yang sedang dipakai (detik).
    static BEAT_LEN: std::cell::Cell<f64> = const { std::cell::Cell::new(BEAT) };
}

/// Denyut di setiap ketukan lagu: 1 tepat di ketukan, meluruh cepat. Setiap adegan mulai tepat
/// di sebuah ketukan, jadi detik lokal adegan sudah cukup.
fn beat_pulse(t: f64) -> f32 {
    let beat = BEAT_LEN.with(|b| b.get());
    (-((t % beat) / beat) as f32 * 7.0).exp()
}

/// Kemunculan dengan sedikit pantulan, 0..1 (bisa sedikit lewat 1).
fn pop_in(t: f64, at: f64, len: f64) -> f32 {
    let p = span(t, at, len);
    if p <= 0.0 { 0.0 } else { back_out(p) }
}

// ---------------------------------------------------------------- lagu

/// Lagu pilihan di `aset-video/lagu.mp3` (folder yang tidak ikut git), atau file lain lewat
/// variabel `LAGU`, kalau ada.
struct Song {
    pcm: Vec<f32>,
    analysis: Analysis,
    /// Detik lagu tempat video dimulai: awal birama pertamanya.
    start: f64,
}

fn load_song() -> Option<Song> {
    let path = std::env::var("LAGU").map(PathBuf::from).unwrap_or_else(|_| output_dir().join("lagu.mp3"));
    if !path.exists() {
        return None;
    }
    let pcm: Vec<f32> = decode_audio(&path).iter().map(|&s| s as f32 / 32768.0).collect();
    let analysis = musik::analyze(&pcm);
    eprintln!(
        "lagu: {:.1} detik, {:.2} BPM, birama pertama di {:.3} detik, kunci {}",
        pcm.len() as f64 / 2.0 / musik::RATE as f64,
        analysis.bpm(),
        analysis.downbeat,
        analysis.key()
    );
    Some(Song { start: analysis.downbeat, pcm, analysis })
}

/// Timeline mengikuti ketukan lagu kalau ada, kalau tidak 120 BPM untuk musik sintesis.
fn film(song: Option<&Song>) -> Film {
    Film::new(song.map_or(BEAT, |s| s.analysis.beat))
}

/// Lagu yang lebih pendek dari video diperpanjang dengan mengulang frasa 8 birama di sekitar
/// sepertiga lagu (tengah sebuah bagian, bukan awalnya), disambung tepat di garis birama, supaya
/// akhir lagunya tetap jatuh di penutup video.
fn lengthen(song: &Song, needed: f64) -> Vec<f32> {
    let rate = musik::RATE as f64;
    let have = song.pcm.len() as f64 / 2.0 / rate - song.start;
    if have >= needed {
        return song.pcm.clone();
    }
    let bar = 4.0 * song.analysis.beat;
    let bars = (have / bar).floor() as usize;
    let count = 8 * ((needed - have) / (8.0 * bar)).ceil() as usize;
    let from = (bars as f64 * 0.35 / 4.0).round() as usize * 4;
    let at = |index: usize| ((song.start + index as f64 * bar) * rate).round() as usize * 2;
    let (a, b) = (at(from), at(from + count));
    eprintln!("lagu diperpanjang: birama {from}–{} diulang sekali ({:.1}–{:.1} detik)", from + count, a as f64 / 2.0 / rate, b as f64 / 2.0 / rate);
    let mut out = Vec::with_capacity(song.pcm.len() + b - a);
    out.extend_from_slice(&song.pcm[..b]);
    out.extend_from_slice(&song.pcm[a..b]);
    out.extend_from_slice(&song.pcm[b..]);
    // Sambungan pertama (akhir frasa asli ke awal salinannya) dihaluskan 30 ms; sambungan kedua
    // sudah mulus karena salinannya berakhir tepat di tempat lagu aslinya berlanjut.
    let fade = (0.03 * rate) as usize * 2;
    for i in 0..fade {
        let w = i as f32 / fade as f32;
        out[b + i] = song.pcm[b + i] * (1.0 - w) + song.pcm[a + i] * w;
    }
    out
}

/// Audio seluruh video: lagu pilihan ditimpa efek suara, atau musik sintesis.
fn soundtrack(film: &Film, song: Option<&Song>) -> Vec<f32> {
    let mut score = film.score();
    let audio = match song {
        Some(song) => {
            score.key_shift = song.analysis.shift();
            let pcm = lengthen(song, film.length);
            musik::with_song(&score, &pcm, song.start)
        }
        None => musik::perform(&score),
    };
    musik::report(&audio, &film.cuts());
    audio
}

// ---------------------------------------------------------------- tes

fn reels_dir() -> PathBuf {
    let dir = output_dir().join("reels");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Bab yang dipilih lewat variabel lingkungan `BAB`, kalau ada.
fn selected() -> Option<usize> {
    std::env::var("BAB").ok()?.trim().parse().ok()
}

/// Tiga frame per adegan (30%, 60%, 92% panjangnya) untuk dicek sebelum merender video.
#[test]
#[ignore]
fn reels_cuplikan() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let film = film(load_song().as_ref());
    film.describe();
    let dir = reels_dir().join("cuplikan");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut studio = Studio::new();
    let cast = Cast::new();
    for slot in film.slots.iter().filter(|slot| selected().is_none_or(|c| c == slot.chapter)) {
        for f in [0.3, 0.6, 0.92] {
            let t = ((slot.start + slot.len * f) * FPS).round() / FPS;
            let pixels = film.render(&mut studio, &cast, t);
            save_png(&dir.join(format!("b{:02}-{:06.2}.png", slot.chapter, t)), &pixels, W, (0, 0, W, H));
        }
    }
}

/// Musiknya saja: WAV dan gambar gelombang + spektrogram dengan garis di awal setiap bab.
#[test]
#[ignore]
fn reels_musik() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let song = load_song();
    let film = film(song.as_ref());
    let audio = soundtrack(&film, song.as_ref());
    musik::save_wav(&reels_dir().join("musik.wav"), &musik::pcm16(&audio));
    let (pixels, width, height) = musik::picture(&audio, &film.cuts());
    save_png(&reels_dir().join("musik.png"), &pixels, width, (0, 0, width, height));
}

/// Satu bab (`BAB=n`) jadi MP4 lengkap dengan suaranya, untuk dicek per bab.
#[test]
#[ignore]
fn reels_bab() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let chapter = selected().expect("pilih bab dengan variabel BAB, misal BAB=1");
    let song = load_song();
    let film = film(song.as_ref());
    let (start, end) = film.range(chapter);
    let audio = soundtrack(&film, song.as_ref());
    write_mp4(&film, &audio, &reels_dir().join(format!("bab-{chapter:02}.mp4")), start, end);
}

/// Seluruh video.
#[test]
#[ignore]
fn reels_video() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let song = load_song();
    let film = film(song.as_ref());
    film.describe();
    let path = output_dir().join("NotchWindows-reels.mp4");
    let audio = soundtrack(&film, song.as_ref());
    musik::save_wav(&reels_dir().join("audio.wav"), &musik::pcm16(&audio));
    write_mp4(&film, &audio, &path, 0.0, film.length);
    let decoded = decode_audio(&path);
    let seconds = decoded.len() as f64 / 2.0 / musik::RATE as f64;
    eprintln!("audio MP4: {seconds:.3} detik dari {:.3}", film.length);
    assert!((seconds - film.length).abs() < 0.05, "panjang audio {seconds}");
}

fn write_mp4(film: &Film, audio: &[f32], path: &Path, start: f64, end: f64) {
    let pcm = musik::pcm16(audio);
    let first = (start * FPS).round() as usize;
    let last = (end * FPS).round() as usize;
    let per_frame = (musik::RATE as f64 / FPS) as usize * 2;
    let mut slice = pcm[(first * per_frame).min(pcm.len())..(last * per_frame).min(pcm.len())].to_vec();
    // Ketukan lagu jarang tepat kelipatan 1/30 detik: audio disamakan dengan jumlah frame
    // (paling banyak beberapa milidetik hening di ujung).
    slice.resize((last - first) * per_frame, 0);
    let mut studio = Studio::new();
    let cast = Cast::new();
    let mut encoder = Encoder::new(path, slice);
    for i in first..last {
        encoder.write(&film.render(&mut studio, &cast, i as f64 / FPS));
    }
    encoder.finish();
}

/// Memeriksa MP4 yang sudah jadi: satu frame dari tengah setiap bab ke `reels/cek-mp4/`, jumlah
/// frame lengkap, dan audionya sinkron dengan partitur di awal beberapa bab.
#[test]
#[ignore]
fn reels_cek() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let film = film(load_song().as_ref());
    let path = output_dir().join("NotchWindows-reels.mp4");
    let out = reels_dir().join("cek-mp4");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).unwrap();
    let frames: Vec<usize> = (0..film.chapters.len())
        .map(|c| {
            let (start, end) = film.range(c);
            (((start + end) / 2.0) * FPS).round() as usize
        })
        .collect();
    let total = decode_frames(&path, &frames, &out);
    assert_eq!(total, (film.length * FPS).round() as usize, "jumlah frame di MP4");
    let source = musik::load_wav(&reels_dir().join("audio.wav"));
    let decoded = decode_audio(&path);
    let lags: Vec<i64> = film.cuts().iter().step_by(3).map(|&at| musik::lag(&source, &decoded, at - 0.1)).collect();
    eprintln!("frame {total}, geser audio {lags:?} sampel");
    assert!(lags.iter().all(|lag| lag.abs() <= 48), "audio tidak sinkron: {lags:?}");
}

/// Membaca `aset-video/lagu.mp3`: tempo, awal birama, dan kunci, plus gambar 24 detik pertama
/// (mulai dari awal birama) dengan garis putih di setiap awal birama untuk dicek dengan mata.
#[test]
#[ignore]
fn reels_lagu() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let song = load_song().expect("simpan lagunya sebagai aset-video/lagu.mp3");
    let first = (song.start * musik::RATE as f64) as usize * 2;
    let span = (24.0 * musik::RATE as f64) as usize * 2;
    let part = &song.pcm[first..(first + span).min(song.pcm.len())];
    let bar = 4.0 * song.analysis.beat;
    let bars: Vec<f64> = (1..).map(|i| i as f64 * bar).take_while(|&t| t < 24.0).collect();
    let (pixels, width, height) = musik::picture(part, &bars);
    save_png(&reels_dir().join("lagu.png"), &pixels, width, (0, 0, width, height));
    // Seluruh lagu, garis tiap 4 birama, untuk melihat bagian-bagiannya.
    let phrases: Vec<f64> = (0..).map(|i| song.start + i as f64 * 4.0 * bar).take_while(|&t| t < song.pcm.len() as f64 / 2.0 / musik::RATE as f64).collect();
    let (pixels, width, height) = musik::picture(&song.pcm, &phrases);
    save_png(&reels_dir().join("lagu-utuh.png"), &pixels, width, (0, 0, width, height));
    // 18 detik terakhir dengan garis tiap birama, untuk melihat pukulan penutupnya.
    let end = song.pcm.len() / 2;
    let tail_start = end.saturating_sub((18.0 * musik::RATE as f64) as usize);
    let offset = tail_start as f64 / musik::RATE as f64;
    let marks: Vec<f64> = (0..200).map(|i| song.start + i as f64 * bar - offset).filter(|&t| t > 0.0 && t < 18.0).collect();
    let (pixels, width, height) = musik::picture(&song.pcm[tail_start * 2..], &marks);
    save_png(&reels_dir().join("lagu-akhir.png"), &pixels, width, (0, 0, width, height));
    let film = film(Some(&song));
    film.describe();
}
