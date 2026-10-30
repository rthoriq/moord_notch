//! Musik latar video promosi, disintesis langsung dari kode: tanpa sampel dan tanpa lagu orang
//! lain, jadi aman dipakai di Instagram. 120 BPM di A minor (Am–F–C–G) dengan melodi pengait
//! berbunyi lonceng, plus efek suara yang jatuh tepat di potongan adegan video: whoosh saat
//! kamera bergerak, pop saat notch membuka, dentuman di setiap "drop".

use std::collections::{HashMap, VecDeque};
use std::f64::consts::{PI, TAU};
use std::path::Path;

use super::{BEAT, LENGTH};

/// Laju sampel (Hz).
pub(super) const RATE: u32 = 48_000;
const SR: f64 = RATE as f64;
const BAR: f64 = BEAT * 4.0;
/// Detik potongan adegan di video (untuk gambar pemeriksa).
pub(super) const CUTS: [f64; 8] = [2.0, 4.5, 8.5, 11.5, 15.0, 18.5, 21.5, 24.0];
/// Kenyaringan sasaran, sama dengan normalisasi Instagram/YouTube, dan puncak maksimum (-1 dBFS).
const TARGET_LUFS: f64 = -14.0;
const CEILING: f64 = 0.891;
/// Posisi tengah, dan tanpa kiriman ke reverb/delay.
const CENTER: (f64, f64) = (0.0, 0.0);
const DRY: (f64, f64) = (0.0, 0.0);

// ---------------------------------------------------------------- lagu

/// Akor tiap birama (2 detik): 0 Am, 1 F, 2 C, 3 G. Penutupnya F lalu C supaya selesai lega.
const CHORDS: [usize; 14] = [0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 1, 2];
/// Am7, Fmaj7, Cadd9, G6 (nada MIDI).
const PADS: [[f64; 4]; 4] = [[57.0, 64.0, 67.0, 72.0], [53.0, 64.0, 69.0, 72.0], [55.0, 64.0, 67.0, 74.0], [55.0, 62.0, 67.0, 71.0]];
/// Nada dasar bass (oktaf 1: A1, F1, C2, G1).
const ROOTS: [f64; 4] = [33.0, 29.0, 36.0, 31.0];
const ARPS: [[f64; 4]; 4] = [[69.0, 72.0, 76.0, 79.0], [65.0, 69.0, 72.0, 76.0], [64.0, 67.0, 72.0, 74.0], [67.0, 71.0, 74.0, 79.0]];
const PATTERN: [usize; 8] = [0, 1, 2, 3, 2, 1, 2, 3];
/// Melodi pengait per akor: (ketukan dalam birama, nada). Ritme 3-3-2 ala lagu pop.
const HOOK: [[(f64, f64); 5]; 4] = [
    [(0.0, 76.0), (0.75, 72.0), (1.5, 69.0), (2.5, 71.0), (3.0, 72.0)],
    [(0.0, 72.0), (0.75, 69.0), (1.5, 65.0), (2.5, 67.0), (3.0, 69.0)],
    [(0.0, 67.0), (0.75, 76.0), (1.5, 74.0), (2.5, 72.0), (3.0, 76.0)],
    [(0.0, 74.0), (0.75, 71.0), (1.5, 67.0), (2.5, 69.0), (3.0, 71.0)],
];

fn chord_at(t: f64) -> usize {
    CHORDS[((t / BAR) as usize).min(CHORDS.len() - 1)]
}

fn within(t: f64, ranges: &[(f64, f64)]) -> bool {
    ranges.iter().any(|&(a, b)| (a..b).contains(&t))
}

/// Bagian ramai: kick tiap ketukan, clap, hi-hat penuh, bass offbeat.
fn full(t: f64) -> bool {
    within(t, &[(4.5, 11.5), (13.0, 14.0), (15.0, 18.5), (21.5, 24.0)])
}

/// Bagian tenang yang tetap berdenyut: hi-hat per setengah ketukan.
fn light(t: f64) -> bool {
    within(t, &[(2.0, 4.5), (11.5, 12.9), (18.5, 21.0), (24.0, 26.0)])
}

/// Bukaan filter pad: terbuka pelan di pembuka, meredup di jeda, terbuka lagi menjelang drop.
fn pad_cutoff(t: f64) -> f64 {
    let keys = [
        (0.0, 350.0),
        (2.0, 3000.0),
        (11.5, 3000.0),
        (11.8, 1400.0),
        (12.9, 1400.0),
        (13.0, 3000.0),
        (18.5, 3000.0),
        (18.7, 1000.0),
        (21.0, 1000.0),
        (21.5, 3400.0),
    ];
    let next = keys.iter().position(|&(at, _)| at > t).unwrap_or(keys.len());
    if next == 0 || next == keys.len() {
        return keys[next.saturating_sub(1)].1;
    }
    let ((a, from), (b, to)) = (keys[next - 1], keys[next]);
    from * (to / from).powf((t - a) / (b - a))
}

fn arrange(mix: &mut Mix) {
    let beats = (LENGTH / BEAT) as usize;

    // Drum.
    let (kick, clap, snare) = (kick(), clap(), snare());
    let (closed, open) = (hat(false), hat(true));
    for b in 0..beats {
        let t = b as f64 * BEAT;
        let half = within(t, &[(2.0, 4.5), (24.0, 26.0)]);
        if full(t) || (half && b % 2 == 0) {
            mix.put(Lane::Drums, t, &kick, 0.9, CENTER, DRY);
            mix.kicks.push(t);
        }
        if (full(t) && b % 2 == 1) || (half && b % 4 == 2) {
            mix.put(Lane::Drums, t, &clap, 0.5, (0.05, 0.05), (0.22, 0.0));
        }
        for step in 0..4 {
            let at = t + step as f64 * BEAT / 4.0;
            if full(at) {
                if step == 2 {
                    mix.put(Lane::Drums, at, &open, 0.14, (0.3, 0.3), DRY);
                } else {
                    let accent = if step == 0 { 0.12 } else { 0.09 };
                    mix.put(Lane::Drums, at, &closed, accent, (-0.25, -0.25), DRY);
                }
            } else if light(at) && step % 2 == 0 {
                mix.put(Lane::Drums, at, &closed, if step == 0 { 0.08 } else { 0.1 }, (-0.25, -0.25), (0.05, 0.0));
            }
        }
    }
    // Gulungan snare menjelang setiap drop, makin rapat dan makin keras.
    for (start, end) in [(4.0, 4.5), (14.0, 15.0), (21.0, 21.5), (23.0, 24.0)] {
        let mut at: f64 = start;
        while at < end - 1e-9 {
            let p = (at - start) / (end - start);
            mix.put(Lane::Drums, at, &snare, 0.2 + 0.35 * p, CENTER, (0.2, 0.0));
            at += if end - at > 0.5 + 1e-9 { BEAT / 2.0 } else { BEAT / 4.0 };
        }
    }

    // Bass: offbeat ala house di bagian ramai, nada panjang di bagian tenang.
    let short: Vec<Vec<f64>> = ROOTS.iter().map(|&root| bass(root, 0.2)).collect();
    for b in 0..beats {
        let t = b as f64 * BEAT + BEAT / 2.0;
        if full(t) {
            mix.put(Lane::Music, t, &short[chord_at(t)], 0.3, CENTER, DRY);
        }
    }
    for (at, len, gain) in [(2.0, 2.0, 0.25), (4.0, 0.5, 0.25), (18.5, 1.5, 0.15), (20.0, 1.5, 0.15), (24.0, 2.0, 0.25), (26.0, 1.8, 0.2)] {
        mix.put(Lane::Music, at, &bass(ROOTS[chord_at(at)], len), gain, CENTER, DRY);
    }

    // Pad, kiri dan kanan disuarakan terpisah supaya lebar.
    for (bar, &chord) in CHORDS.iter().enumerate() {
        let at = bar as f64 * BAR;
        let len = if bar == CHORDS.len() - 1 { 1.6 } else { BAR };
        for (side, seed) in [(-0.6, 1u64), (0.6, 2)] {
            let sound = pad(&PADS[chord], at, len, seed + bar as u64 * 10, &pad_cutoff);
            mix.put(Lane::Music, at, &sound, 0.17, (side, side), (0.3, 0.0));
        }
    }

    // Arpeggio pluck: per 1/16 di bagian ramai, per 1/8 di bagian tenang.
    let mut plucks: HashMap<i64, Vec<f64>> = HashMap::new();
    for s in 0..(LENGTH / (BEAT / 4.0)) as usize {
        let t = s as f64 * BEAT / 4.0;
        let every = if full(t) || within(t, &[(11.5, 15.0)]) {
            1
        } else if within(t, &[(2.0, 4.5), (18.5, 21.5), (24.0, 26.0)]) {
            2
        } else {
            continue;
        };
        if s % every != 0 {
            continue;
        }
        let midi = ARPS[chord_at(t)][PATTERN[s % 8]];
        let sound = plucks.entry(midi as i64).or_insert_with(|| pluck(midi));
        let accent = if s % 4 == 0 { 1.0 } else { 0.75 };
        let side = if (s / every) % 2 == 0 { -0.3 } else { 0.3 };
        mix.put(Lane::Music, t, sound, 0.16 * accent, (side, side), (0.2, 0.25));
    }

    // Melodi pengait (lonceng) di bagian puncak dan penutup.
    let mut bells: HashMap<i64, Vec<f64>> = HashMap::new();
    for (bar, &chord) in CHORDS.iter().enumerate() {
        for &(beat, midi) in &HOOK[chord] {
            let at = bar as f64 * BAR + beat * BEAT;
            if within(at, &[(15.0, 18.5), (21.5, 26.0)]) {
                let sound = bells.entry(midi as i64).or_insert_with(|| bell(midi, 2.0, 0.35));
                mix.put(Lane::Music, at, sound, 0.34, (0.1, 0.1), (0.3, 0.3));
            }
        }
    }

    effects(mix);
}

/// Efek suara, mengikuti waktu di adegan video.
fn effects(mix: &mut Mix) {
    let (boom, crash, thud, click) = (boom(), crash(), thud(), key_click());
    let impact = |mix: &mut Mix, at: f64, size: f64| {
        mix.put(Lane::Sfx, at, &boom, 0.9 * size, CENTER, (0.1, 0.0));
        mix.put(Lane::Sfx, at, &crash, 0.4 * size, CENTER, (0.35, 0.0));
    };
    let chime = |mix: &mut Mix, at: f64, notes: &[f64], gain: f64| {
        for (i, &midi) in notes.iter().enumerate() {
            let side = -0.4 + 0.8 * i as f64 / (notes.len() - 1).max(1) as f64;
            mix.put(Lane::Sfx, at + i as f64 * 0.06, &bell(midi, 3.5, 0.3), gain, (side, side), (0.4, 0.25));
        }
    };

    // 1. Pembuka: garis cahaya, pil muncul, "Notch.", lalu naik ke dentuman pertama.
    mix.put(Lane::Sfx, 0.05, &whoosh(0.85, 700.0, 3500.0, 1), 0.22, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 0.62, &pop(69.0), 0.35, CENTER, (0.15, 0.0));
    mix.put(Lane::Sfx, 0.62, &thud, 0.3, CENTER, DRY);
    mix.put(Lane::Sfx, 1.0, &boom, 0.4, CENTER, DRY);
    for midi in [69.0, 72.0, 76.0] {
        mix.put(Lane::Sfx, 1.0, &bell(midi, 2.0, 0.5), 0.2, CENTER, (0.45, 0.2));
    }
    mix.put(Lane::Sfx, 1.15, &tick(2600.0), 0.08, CENTER, DRY);
    mix.put(Lane::Sfx, 1.5, &bell(76.0, 2.0, 0.4), 0.22, (0.2, 0.2), (0.3, 0.35));
    mix.put(Lane::Sfx, 1.0, &riser(1.0, 2), 0.38, CENTER, (0.15, 0.0));

    // 2. Kamera mundur ke laptop, lalu mendorong masuk lagi.
    impact(mix, 2.0, 1.0);
    mix.put(Lane::Sfx, 2.0, &whoosh(0.9, 4000.0, 250.0, 3), 0.32, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 4.1, &whoosh(0.4, 300.0, 5000.0, 4), 0.32, CENTER, (0.2, 0.0));

    // 3. Musik & lirik.
    mix.put(Lane::Sfx, 4.5, &crash, 0.28, CENTER, (0.3, 0.0));
    mix.put(Lane::Sfx, 4.75, &glide(0.2, 400.0, 1100.0), 0.2, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 4.75, &pop(76.0), 0.22, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 5.75, &whoosh(0.5, 800.0, 2500.0, 5), 0.16, (-0.3, 0.5), (0.2, 0.0));
    for at in [6.75, 7.75] {
        mix.put(Lane::Sfx, at, &tick(3200.0), 0.08, (0.4, 0.4), (0.2, 0.0));
    }

    // 4. Kabar ala Dynamic Island.
    mix.put(Lane::Sfx, 8.35, &whoosh(0.25, 500.0, 4000.0, 6), 0.22, CENTER, DRY);
    mix.put(Lane::Sfx, 8.5, &crash, 0.25, CENTER, (0.3, 0.0));
    mix.put(Lane::Sfx, 8.6, &glide(0.18, 450.0, 1300.0), 0.2, CENTER, (0.2, 0.0));
    for at in [9.0, 10.0] {
        mix.put(Lane::Sfx, at, &tick(2900.0), 0.12, (0.3, 0.3), DRY);
        mix.put(Lane::Sfx, at + 0.05, &tick(3400.0), 0.12, (0.3, 0.3), DRY);
    }
    mix.put(Lane::Sfx, 9.5, &pop(76.0), 0.26, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 10.5, &pop(81.0), 0.26, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 10.5, &click, 0.3, CENTER, DRY);
    mix.put(Lane::Sfx, 11.15, &glide(0.2, 1300.0, 450.0), 0.18, CENTER, (0.2, 0.0));

    // 5. File diseret dan dilepas, lalu naik ke drop.
    mix.put(Lane::Sfx, 11.45, &whoosh(0.3, 3000.0, 600.0, 7), 0.16, CENTER, (0.2, 0.0));
    mix.put(Lane::Sfx, 11.9, &whoosh(0.95, 300.0, 1800.0, 8), 0.24, (0.7, -0.3), (0.15, 0.0));
    mix.put(Lane::Sfx, 12.9, &thud, 0.55, (-0.2, -0.2), (0.15, 0.0));
    chime(mix, 12.9, &[84.0, 88.0, 91.0], 0.12);
    mix.put(Lane::Sfx, 13.45, &whoosh(0.5, 300.0, 2000.0, 9), 0.24, (-0.7, 0.6), (0.15, 0.0));
    mix.put(Lane::Sfx, 14.0, &thud, 0.55, (0.3, 0.3), (0.15, 0.0));
    mix.put(Lane::Sfx, 14.0, &pop(79.0), 0.2, (0.3, 0.3), (0.2, 0.0));
    mix.put(Lane::Sfx, 13.9, &riser(1.1, 10), 0.42, CENTER, (0.15, 0.0));

    // 6. Drop: halaman-halaman lain masuk dari kiri dan kanan.
    impact(mix, 15.0, 1.0);
    for i in 0..4 {
        let side = if i % 2 == 0 { -0.8 } else { 0.8 };
        let at = 15.15 + i as f64 * 0.25;
        mix.put(Lane::Sfx, at, &whoosh(0.4, 2500.0, 600.0, 11 + i), 0.15, (side, side * 0.1), DRY);
    }
    mix.put(Lane::Sfx, 18.1, &whoosh(0.4, 600.0, 3000.0, 15), 0.22, CENTER, (0.2, 0.0));

    // 7. Mata: muncul, mengikuti kursor, berkedip, tersenyum.
    mix.put(Lane::Sfx, 18.5, &pop(69.0), 0.3, CENTER, (0.25, 0.0));
    for (at, len, from, to, seed) in [(18.7, 0.6, 0.0, -0.6, 16), (19.75, 0.7, -0.6, 0.6, 17), (20.55, 0.3, 0.6, 0.0, 18)] {
        mix.put(Lane::Sfx, at, &whoosh(len, 900.0, 1600.0, seed), 0.07, (from, to), DRY);
    }
    mix.put(Lane::Sfx, 19.5, &tick(2600.0), 0.1, CENTER, (0.2, 0.0));
    chime(mix, 20.85, &[84.0, 88.0, 91.0, 96.0], 0.18);
    mix.put(Lane::Sfx, 21.0, &riser(0.5, 19), 0.28, CENTER, (0.15, 0.0));

    // 8. Enam tema bergeser dari kanan ke kiri, makin tinggi tiap geseran.
    mix.put(Lane::Sfx, 21.5, &crash, 0.35, CENTER, (0.3, 0.0));
    for i in 0..6 {
        let lift = 1.15f64.powi(i);
        let at = 21.5 + i as f64 * 0.4;
        mix.put(Lane::Sfx, at, &whoosh(0.3, 450.0 * lift, 2600.0 * lift, 20 + i as u64), 0.17, (0.7, -0.7), DRY);
    }
    mix.put(Lane::Sfx, 23.0, &riser(1.0, 26), 0.42, CENTER, (0.15, 0.0));

    // 9. Penutup.
    impact(mix, 24.0, 1.15);
    mix.put(Lane::Sfx, 24.0, &whoosh(1.2, 6000.0, 800.0, 27), 0.16, (-0.5, 0.5), (0.3, 0.0));
    mix.put(Lane::Sfx, 24.1, &pop(72.0), 0.3, CENTER, (0.25, 0.0));
    mix.put(Lane::Sfx, 25.5, &pop(84.0), 0.22, CENTER, (0.25, 0.0));
    mix.put(Lane::Sfx, 26.0, &boom, 0.45, CENTER, (0.1, 0.0));
    mix.put(Lane::Sfx, 26.0, &crash, 0.25, CENTER, (0.35, 0.0));
    for (i, midi) in [72.0, 76.0, 79.0, 84.0].into_iter().enumerate() {
        mix.put(Lane::Sfx, 26.0 + i as f64 * 0.02, &bell(midi, 2.0, 0.8), 0.2, (-0.3 + 0.2 * i as f64, -0.3 + 0.2 * i as f64), (0.45, 0.3));
    }
    mix.put(Lane::Sfx, 26.4, &tick(2600.0), 0.08, CENTER, (0.2, 0.0));
    chime(mix, 26.9, &[84.0, 88.0, 91.0, 96.0], 0.16);
}

// ---------------------------------------------------------------- partitur

/// Seberapa ramai musik di satu adegan.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Energy {
    /// Hanya pad.
    Quiet,
    /// Pad, arpeggio per 1/8, hi-hat tipis, bass panjang; tanpa kick.
    Break,
    /// Setengah tempo: kick di ketukan 1 dan 3, clap di 3.
    Light,
    /// Kick tiap ketukan, clap, hi-hat penuh, bass offbeat, melodi pengait.
    Full,
}

/// Efek suara yang dipanggil sebuah adegan.
#[derive(Clone, Debug)]
pub(super) enum Cue {
    Whoosh { len: f64, from: f64, to: f64, pan: (f64, f64), gain: f64 },
    Pop { midi: f64, gain: f64 },
    Tick { hz: f64, gain: f64 },
    Click { gain: f64 },
    Thud { gain: f64 },
    Boom { gain: f64 },
    Riser { len: f64, gain: f64 },
    Chime { notes: Vec<f64>, gain: f64 },
    Glide { len: f64, from: f64, to: f64, gain: f64 },
    Bell { midi: f64, gain: f64 },
    /// Ketikan keyboard selama `len` detik.
    Typing { len: f64, gain: f64 },
    /// Rana kamera (screenshot).
    Shutter { gain: f64 },
    /// Nada notifikasi dua ketuk.
    Ding { gain: f64 },
    /// Alarm timer: bip-bip berulang selama `len` detik.
    Alarm { len: f64, gain: f64 },
    /// Desis listrik (charger dicolok).
    Zap { gain: f64 },
}

/// Lagu panjang: bagian-bagian berenergi (dengan "rasa" per bab) plus efek suara di waktunya.
pub(super) struct Score {
    pub(super) length: f64,
    sections: Vec<(f64, f64, Energy, usize)>,
    cues: Vec<(f64, (f64, f64), Cue)>,
    /// Detik awal adegan yang sedang mengisi efeknya; waktu efek ditulis relatif terhadap ini.
    pub(super) offset: f64,
    /// Mulai detik ini semua nada naik dua semitone (bagian akhir terasa makin tinggi).
    pub(super) lift: f64,
    /// Geseran nada (semitone) efek suara bernada, supaya sekunci dengan lagu yang dipakai.
    pub(super) key_shift: f64,
}

impl Score {
    pub(super) fn new(length: f64) -> Self {
        Self { length, sections: Vec::new(), cues: Vec::new(), offset: 0.0, lift: f64::INFINITY, key_shift: 0.0 }
    }

    pub(super) fn section(&mut self, start: f64, end: f64, energy: Energy, flavor: usize) {
        self.sections.push((start, end, energy, flavor));
    }

    /// Efek di tengah, `at` detik setelah awal adegan.
    pub(super) fn cue(&mut self, at: f64, cue: Cue) {
        self.cue_pan(at, (0.0, 0.0), cue);
    }

    /// Efek yang bergeser dari kiri-kanan `pan.0` ke `pan.1`.
    pub(super) fn cue_pan(&mut self, at: f64, pan: (f64, f64), cue: Cue) {
        self.cues.push((self.offset + at, pan, cue));
    }

    fn at(&self, t: f64) -> Option<(f64, f64, Energy, usize)> {
        self.sections.iter().copied().find(|&(a, b, _, _)| (a..b).contains(&t))
    }

    fn energy(&self, t: f64) -> Energy {
        self.at(t).map_or(Energy::Quiet, |s| s.2)
    }

    fn flavor(&self, t: f64) -> usize {
        self.at(t).map_or(0, |s| s.3)
    }

    /// Akor di detik `t`: Am–F–C–G berulang, dua birama terakhir F lalu C.
    fn chord(&self, t: f64) -> usize {
        let bars = (self.length / BAR).ceil() as usize;
        let bar = (t / BAR) as usize;
        if bar + 2 == bars {
            1
        } else if bar + 1 >= bars {
            2
        } else {
            bar % 4
        }
    }

    fn transpose(&self, t: f64) -> f64 {
        if t >= self.lift { 2.0 } else { 0.0 }
    }

    /// Bukaan filter pad: makin ramai makin terang, berpindah halus dalam 0,3 detik.
    fn cutoff(&self, t: f64) -> f64 {
        let level = |energy: Energy| -> f64 {
            match energy {
                Energy::Quiet => 700.0,
                Energy::Break => 1200.0,
                Energy::Light => 2300.0,
                Energy::Full => 3200.0,
            }
        };
        let now = level(self.energy(t));
        let before = level(self.energy(t - 0.3));
        let start = self.at(t).map_or(0.0, |s| s.0);
        let p = ((t - start) / 0.3).clamp(0.0, 1.0);
        before * (now / before).powf(p)
    }
}

/// Merender seluruh partitur jadi audio stereo berselang-seling (L, R, ...).
pub(super) fn perform(score: &Score) -> Vec<f32> {
    let mut mix = Mix::new(score.length);
    let beats = (score.length / BEAT).round() as usize;
    let full = |t: f64| score.energy(t) == Energy::Full;

    // Drum.
    let (kick, clap, snare) = (kick(), clap(), snare());
    let (closed, open) = (hat(false), hat(true));
    for b in 0..beats {
        let t = b as f64 * BEAT;
        let energy = score.energy(t);
        let light = energy == Energy::Light;
        if energy == Energy::Full || (light && b % 2 == 0) {
            mix.put(Lane::Drums, t, &kick, 0.9, CENTER, DRY);
            mix.kicks.push(t);
        }
        if (energy == Energy::Full && b % 2 == 1) || (light && b % 4 == 2) {
            mix.put(Lane::Drums, t, &clap, 0.5, (0.05, 0.05), (0.22, 0.0));
        }
        for step in 0..4 {
            let at = t + step as f64 * BEAT / 4.0;
            match score.energy(at) {
                Energy::Full if step == 2 => mix.put(Lane::Drums, at, &open, 0.14, (0.3, 0.3), DRY),
                Energy::Full => mix.put(Lane::Drums, at, &closed, if step == 0 { 0.12 } else { 0.09 }, (-0.25, -0.25), DRY),
                Energy::Light | Energy::Break if step % 2 == 0 => {
                    mix.put(Lane::Drums, at, &closed, if step == 0 { 0.07 } else { 0.09 }, (-0.25, -0.25), (0.05, 0.0))
                }
                _ => {}
            }
        }
    }
    // Setiap kali musik masuk ke bagian penuh: gulungan snare dan riser sedetik sebelumnya, lalu
    // dentuman tepat di awalnya.
    let (boom, crash) = (boom(), crash());
    for &(start, _, energy, _) in &score.sections {
        if energy != Energy::Full || start < 1.0 || full(start - 0.01) {
            continue;
        }
        let mut at = start - 1.0;
        while at < start - 1e-9 {
            let p = (at - (start - 1.0)) / 1.0;
            mix.put(Lane::Drums, at, &snare, 0.2 + 0.35 * p, CENTER, (0.2, 0.0));
            at += if start - at > 0.5 + 1e-9 { BEAT / 2.0 } else { BEAT / 4.0 };
        }
        mix.put(Lane::Sfx, start - 1.0, &riser(1.0, start as u64), 0.36, CENTER, (0.15, 0.0));
        mix.put(Lane::Sfx, start, &boom, 0.8, CENTER, (0.1, 0.0));
        mix.put(Lane::Sfx, start, &crash, 0.36, CENTER, (0.35, 0.0));
    }

    // Bass: offbeat di bagian penuh, nada panjang per birama di bagian tenang.
    let mut basses: HashMap<i64, Vec<f64>> = HashMap::new();
    for b in 0..beats {
        let t = b as f64 * BEAT + BEAT / 2.0;
        if full(t) {
            let root = ROOTS[score.chord(t)] + score.transpose(t);
            let sound = basses.entry(root as i64).or_insert_with(|| bass(root, 0.2));
            mix.put(Lane::Music, t, sound, 0.3, CENTER, DRY);
        }
    }
    let mut t = 0.0;
    while t < score.length - 1e-9 {
        let energy = score.energy(t);
        // Sampai akhir birama atau akhir bagian, mana yang lebih dulu.
        let section_end = score.at(t).map_or(score.length, |s| s.1);
        let end = ((t / BAR).floor() * BAR + BAR).min(section_end).min(score.length);
        if matches!(energy, Energy::Light | Energy::Break) && end - t > 0.2 {
            let gain = if energy == Energy::Light { 0.25 } else { 0.15 };
            mix.put(Lane::Music, t, &bass(ROOTS[score.chord(t)] + score.transpose(t), end - t), gain, CENTER, DRY);
        }
        t = end;
    }

    // Pad tiap birama, kiri dan kanan disuarakan terpisah supaya lebar.
    let bars = (score.length / BAR).ceil() as usize;
    let cutoff = |t: f64| score.cutoff(t);
    for bar in 0..bars {
        let at = bar as f64 * BAR;
        let notes = PADS[score.chord(at)].map(|m| m + score.transpose(at));
        let len = if bar + 1 == bars { score.length - at } else { BAR };
        for (side, seed) in [(-0.6, 1u64), (0.6, 2)] {
            let sound = pad(&notes, at, len, seed + bar as u64 * 10, &cutoff);
            mix.put(Lane::Music, at, &sound, 0.17, (side, side), (0.3, 0.0));
        }
    }

    // Arpeggio: pola berganti per bab.
    let patterns: [[usize; 8]; 3] = [PATTERN, [0, 2, 1, 3, 0, 2, 3, 1], [3, 2, 1, 0, 1, 2, 3, 2]];
    let mut plucks: HashMap<i64, Vec<f64>> = HashMap::new();
    for s in 0..(score.length / (BEAT / 4.0)) as usize {
        let t = s as f64 * BEAT / 4.0;
        let every = match score.energy(t) {
            Energy::Full => 1,
            Energy::Light | Energy::Break => 2,
            Energy::Quiet => continue,
        };
        if s % every != 0 {
            continue;
        }
        let pattern = patterns[score.flavor(t) % 3];
        let midi = ARPS[score.chord(t)][pattern[s % 8]] + score.transpose(t);
        let sound = plucks.entry(midi as i64).or_insert_with(|| pluck(midi));
        let accent = if s % 4 == 0 { 1.0 } else { 0.75 };
        let side = if (s / every) % 2 == 0 { -0.3 } else { 0.3 };
        mix.put(Lane::Music, t, sound, 0.16 * accent, (side, side), (0.2, 0.25));
    }

    // Melodi pengait di bagian penuh; warna loncengnya berganti per bab.
    let mut bells: HashMap<(i64, bool), Vec<f64>> = HashMap::new();
    for bar in 0..bars {
        let chord = score.chord(bar as f64 * BAR);
        for &(beat, midi) in &HOOK[chord] {
            let at = bar as f64 * BAR + beat * BEAT;
            if at >= score.length || !full(at) {
                continue;
            }
            let flavor = score.flavor(at);
            let glass = flavor % 2 == 1;
            let midi = midi + score.transpose(at) + if flavor % 3 == 2 { 12.0 } else { 0.0 };
            let sound = bells.entry((midi as i64, glass)).or_insert_with(|| bell(midi, if glass { 3.5 } else { 2.0 }, 0.35));
            mix.put(Lane::Music, at, sound, if glass { 0.26 } else { 0.32 }, (0.1, 0.1), (0.3, 0.3));
        }
    }

    for (at, pan, cue) in &score.cues {
        play(&mut mix, *at, *pan, cue, &boom, 0.0);
    }
    master(mix)
}

fn play(mix: &mut Mix, at: f64, pan: (f64, f64), cue: &Cue, boom: &[f64], key: f64) {
    let seed = (at * 1000.0) as u64;
    match cue {
        Cue::Whoosh { len, from, to, pan, gain } => mix.put(Lane::Sfx, at, &whoosh(*len, *from, *to, seed), *gain, *pan, (0.2, 0.0)),
        Cue::Pop { midi, gain } => mix.put(Lane::Sfx, at, &pop(*midi + key), *gain, pan, (0.2, 0.0)),
        Cue::Tick { hz, gain } => mix.put(Lane::Sfx, at, &tick(*hz), *gain, pan, (0.15, 0.0)),
        Cue::Click { gain } => mix.put(Lane::Sfx, at, &key_click(), *gain, pan, DRY),
        Cue::Thud { gain } => mix.put(Lane::Sfx, at, &thud(), *gain, pan, (0.15, 0.0)),
        Cue::Boom { gain } => mix.put(Lane::Sfx, at, boom, *gain, pan, (0.1, 0.0)),
        Cue::Riser { len, gain } => mix.put(Lane::Sfx, at, &riser(*len, seed), *gain, pan, (0.15, 0.0)),
        Cue::Chime { notes, gain } => {
            for (i, &midi) in notes.iter().enumerate() {
                let side = -0.4 + 0.8 * i as f64 / (notes.len() - 1).max(1) as f64;
                mix.put(Lane::Sfx, at + i as f64 * 0.06, &bell(midi + key, 3.5, 0.3), *gain, (side, side), (0.4, 0.25));
            }
        }
        Cue::Glide { len, from, to, gain } => mix.put(Lane::Sfx, at, &glide(*len, *from, *to), *gain, pan, (0.2, 0.0)),
        Cue::Bell { midi, gain } => mix.put(Lane::Sfx, at, &bell(*midi + key, 2.0, 0.45), *gain, pan, (0.35, 0.3)),
        Cue::Typing { len, gain } => {
            let mut noise = Noise::new(seed);
            let mut t = 0.0;
            while t < *len {
                let strength = 0.6 + 0.4 * (noise.next() + 1.0) / 2.0;
                let side = 0.2 * noise.next();
                mix.put(Lane::Sfx, at + t, &key_click(), gain * strength, (side, side), DRY);
                t += 0.07 + 0.06 * (noise.next() + 1.0) / 2.0;
            }
        }
        Cue::Shutter { gain } => {
            mix.put(Lane::Sfx, at, &shutter(seed), *gain, pan, (0.15, 0.0));
            mix.put(Lane::Sfx, at + 0.07, &shutter(seed + 1), gain * 0.8, pan, (0.15, 0.0));
        }
        Cue::Ding { gain } => {
            mix.put(Lane::Sfx, at, &bell(88.0 + key, 2.0, 0.25), *gain, pan, (0.3, 0.2));
            mix.put(Lane::Sfx, at + 0.11, &bell(83.0 + key, 2.0, 0.35), gain * 0.9, pan, (0.3, 0.2));
        }
        Cue::Alarm { len, gain } => {
            let beep = synth(0.09, |t| ((TAU * 1760.0 * t).sin() + 0.4 * (TAU * 3520.0 * t).sin()) * envelope(t, 0.003, 0.07, 0.01));
            let mut t = 0.0;
            while t < *len {
                for k in 0..2 {
                    mix.put(Lane::Sfx, at + t + k as f64 * 0.13, &beep, *gain, pan, (0.1, 0.0));
                }
                t += 0.5;
            }
        }
        Cue::Zap { gain } => mix.put(Lane::Sfx, at, &zap(seed), *gain, pan, (0.2, 0.0)),
    }
}

// ---------------------------------------------------------------- lagu dari luar

/// Hasil membaca sebuah lagu: panjang satu ketukan, detik awal birama pertama, dan kuncinya.
#[derive(Clone, Copy, Debug)]
pub(super) struct Analysis {
    pub(super) beat: f64,
    pub(super) downbeat: f64,
    /// Nada dasar (0 = C, 1 = C#, ...) dan apakah minor.
    pub(super) root: usize,
    pub(super) minor: bool,
}

const NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

impl Analysis {
    pub(super) fn bpm(&self) -> f64 {
        60.0 / self.beat
    }

    pub(super) fn key(&self) -> String {
        format!("{} {}", NOTES[self.root], if self.minor { "minor" } else { "mayor" })
    }

    /// Geseran (semitone) dari C mayor / A minor, tempat efek suara bernada ditulis, ke kunci lagu:
    /// yang terdekat, -6..5.
    pub(super) fn shift(&self) -> f64 {
        let major_root = if self.minor { (self.root + 3) % 12 } else { self.root } as i32;
        (if major_root > 5 { major_root - 12 } else { major_root }) as f64
    }
}

/// Membaca tempo, letak ketukan, awal birama, dan kunci lagu (stereo berselang-seling, 48 kHz).
///
/// Ketukan dari "spectral flux" (seberapa banyak energi baru muncul tiap 10 ms): periode kasarnya
/// dari autokorelasi, lalu dihaluskan dengan melipat seluruh lagu pada periode itu dan memilih
/// lipatan yang paling tajam. Awal birama dari ketukan yang paling sering berisi kick (energi di
/// bawah 150 Hz) dan pergantian akor. Kunci dari kroma (FFT panjang, 160–2000 Hz) yang
/// dicocokkan ke profil Krumhansl.
pub(super) fn analyze(song: &[f32]) -> Analysis {
    let mono: Vec<f64> = song.chunks_exact(2).map(|s| (s[0] + s[1]) as f64 * 0.5).collect();
    let (hop, size) = (480usize, 2048usize);
    let window: Vec<f64> = (0..size).map(|i| 0.5 - 0.5 * (TAU * i as f64 / size as f64).cos()).collect();
    let frames = mono.len().saturating_sub(size) / hop;
    let bin_hz = SR / size as f64;
    let low_bins = (150.0 / bin_hz).ceil() as usize;
    let mut prev = vec![0.0; size / 2];
    let (mut flux, mut low) = (vec![0.0; frames], vec![0.0; frames]);
    let mut frame_chroma = vec![[0.0f64; 12]; frames];
    let pitch: Vec<Option<usize>> = (0..size / 2)
        .map(|b| {
            let f = b as f64 * bin_hz;
            (65.0..1000.0).contains(&f).then(|| ((12.0 * (f / 440.0).log2() + 69.0).round() as i64).rem_euclid(12) as usize)
        })
        .collect();
    for f in 0..frames {
        let mut re: Vec<f64> = (0..size).map(|i| mono[f * hop + i] * window[i]).collect();
        let mut im = vec![0.0; size];
        fft(&mut re, &mut im);
        let (mut total, mut bass) = (0.0, 0.0);
        for b in 1..size / 2 {
            let magnitude = (re[b] * re[b] + im[b] * im[b]).sqrt();
            let level = (1.0 + 100.0 * magnitude).ln();
            let rise = (level - prev[b]).max(0.0);
            total += rise;
            if b <= low_bins {
                bass += rise;
            }
            prev[b] = level;
            if let Some(class) = pitch[b] {
                frame_chroma[f][class] += magnitude * magnitude;
            }
        }
        flux[f] = total;
        low[f] = bass;
        let energy: f64 = frame_chroma[f].iter().sum();
        if energy > 1e-9 {
            frame_chroma[f].iter_mut().for_each(|value| *value /= energy);
        }
    }
    // Kroma untuk kunci: FFT 8192 titik (5,9 Hz per kotak) tiap 100 ms, cukup halus untuk
    // membedakan semitone. Setiap potongan berbobot sama, supaya bagian yang keras tidak
    // menentukan kuncinya sendiri.
    let long = 8192usize;
    let long_window: Vec<f64> = (0..long).map(|i| 0.5 - 0.5 * (TAU * i as f64 / long as f64).cos()).collect();
    let long_hz = SR / long as f64;
    let mut chroma = [0.0f64; 12];
    let mut at = 0;
    while at + long <= mono.len() {
        let mut re: Vec<f64> = (0..long).map(|i| mono[at + i] * long_window[i]).collect();
        let mut im = vec![0.0; long];
        fft(&mut re, &mut im);
        let mut piece = [0.0f64; 12];
        for b in (160.0 / long_hz) as usize..(2000.0 / long_hz) as usize {
            let f = b as f64 * long_hz;
            let class = ((12.0 * (f / 440.0).log2() + 69.0).round() as i64).rem_euclid(12) as usize;
            piece[class] += re[b] * re[b] + im[b] * im[b];
        }
        let energy: f64 = piece.iter().sum();
        if energy > 1e-9 {
            for c in 0..12 {
                chroma[c] += piece[c] / energy;
            }
        }
        at += 4800;
    }
    // Hanya lonjakan di atas rata-rata sekitarnya (±0,5 detik) yang dihitung.
    let onsets = |raw: &[f64]| -> Vec<f64> {
        (0..raw.len())
            .map(|i| {
                let (a, b) = (i.saturating_sub(50), (i + 50).min(raw.len()));
                let mean = raw[a..b].iter().sum::<f64>() / (b - a) as f64;
                (raw[i] - mean).max(0.0)
            })
            .collect()
    };
    let (env, kick) = (onsets(&flux), onsets(&low));
    let fps = SR / hop as f64;

    // Periode kasar: autokorelasi 70–180 BPM, condong ke sekitar 120 BPM.
    let mut coarse = (0usize, f64::MIN);
    for lag in (60.0 * fps / 180.0) as usize..=(60.0 * fps / 70.0) as usize {
        let ac: f64 = (0..env.len() - lag).map(|i| env[i] * env[i + lag]).sum();
        let bpm = 60.0 * fps / lag as f64;
        let score = ac * (-0.5 * ((bpm / 120.0).log2() / 0.6).powi(2)).exp();
        if score > coarse.1 {
            coarse = (lag, score);
        }
    }
    // Lipat seluruh lagu pada periode `p` (bingkai) ke `bins` kotak fase.
    let fold = |signal: &[f64], p: f64, bins: usize| -> Vec<f64> {
        let mut hist = vec![0.0; bins];
        for (i, v) in signal.iter().enumerate() {
            let phase = (i as f64 / p).fract();
            hist[((phase * bins as f64) as usize).min(bins - 1)] += v;
        }
        hist
    };
    let sharpness = |hist: &[f64]| -> (usize, f64) {
        let n = hist.len();
        (0..n)
            .map(|i| (i, hist[(i + n - 1) % n] + 2.0 * hist[i] + hist[(i + 1) % n]))
            .fold((0, f64::MIN), |best, x| if x.1 > best.1 { x } else { best })
    };
    let mut period = (coarse.0 as f64, f64::MIN);
    let mut p = coarse.0 as f64 * 0.97;
    while p <= coarse.0 as f64 * 1.03 {
        let (_, score) = sharpness(&fold(&env, p, 64));
        if score > period.1 {
            period = (p, score);
        }
        p += 0.002;
    }
    let p = period.0;
    let bins = 256;
    let (bin, _) = sharpness(&fold(&env, p, bins));
    // Bingkai `f` mewakili jendela yang berpusat di f·hop + size/2.
    let center = size as f64 / 2.0 / hop as f64;
    let beat_frame = (bin as f64 + 0.5) / bins as f64 * p;
    // Awal birama: dari empat ketukan dalam satu birama, yang paling sering berisi kick dan yang
    // paling sering menjadi tempat akor berganti.
    let bar = fold(&kick, 4.0 * p, 4 * bins);
    let kicks: Vec<f64> = (0..4)
        .map(|j| {
            let at = ((beat_frame / (4.0 * p) + j as f64 / 4.0).fract() * (4 * bins) as f64) as usize;
            (0..5).map(|d| bar[(at + 4 * bins - 2 + d) % (4 * bins)]).sum::<f64>()
        })
        .collect();
    let mut changes = [0.0f64; 4];
    let mut before: Option<[f64; 12]> = None;
    let mut k = 0usize;
    loop {
        let (a, b) = ((beat_frame + k as f64 * p) as usize, (beat_frame + (k + 1) as f64 * p) as usize);
        if b > frames {
            break;
        }
        let mut now = [0.0f64; 12];
        for frame in &frame_chroma[a..b] {
            for c in 0..12 {
                now[c] += frame[c];
            }
        }
        if let Some(last) = before {
            let dot: f64 = (0..12).map(|c| last[c] * now[c]).sum();
            let norm = (last.iter().map(|v| v * v).sum::<f64>() * now.iter().map(|v| v * v).sum::<f64>()).sqrt();
            if norm > 1e-12 {
                changes[k % 4] += 1.0 - dot / norm;
            }
        }
        before = Some(now);
        k += 1;
    }
    let peak = |v: &[f64]| v.iter().cloned().fold(1e-12, f64::max);
    let (kick_peak, change_peak) = (peak(&kicks), peak(&changes));
    let which = (0..4)
        .map(|j| (j, kicks[j] / kick_peak + changes[j] / change_peak))
        .fold((0, f64::MIN), |best, x| if x.1 > best.1 { x } else { best })
        .0;
    let downbeat = ((beat_frame + which as f64 * p) % (4.0 * p) + center) / fps;

    // Kunci: korelasi kroma dengan profil mayor dan minor di 12 nada dasar.
    let major = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
    let minor = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];
    let correlate = |profile: &[f64; 12], root: usize| -> f64 {
        let (mx, my) = (chroma.iter().sum::<f64>() / 12.0, profile.iter().sum::<f64>() / 12.0);
        let (mut num, mut dx, mut dy) = (0.0, 0.0, 0.0);
        for k in 0..12 {
            let (x, y) = (chroma[(root + k) % 12] - mx, profile[k] - my);
            num += x * y;
            dx += x * x;
            dy += y * y;
        }
        num / (dx * dy).sqrt().max(1e-12)
    };
    let mut key = (0usize, false, f64::MIN);
    for root in 0..12 {
        for (profile, is_minor) in [(&major, false), (&minor, true)] {
            let r = correlate(profile, root);
            if r > key.2 {
                key = (root, is_minor, r);
            }
        }
    }
    Analysis { beat: p / fps, downbeat, root: key.0, minor: key.1 }
}

/// Lagu dari luar mulai detik `start`-nya, ditimpa efek suara partitur (sedikit lebih pelan dari
/// versi musik sintesis), lalu dinormalkan seperti biasa. Efek bernada digeser `score.key_shift`.
pub(super) fn with_song(score: &Score, song: &[f32], start: f64) -> Vec<f32> {
    let mut mix = Mix::new(score.length);
    let first = (start * SR).round() as usize;
    let count = mix.music.left.len();
    // Lagu disamakan dulu kenyaringannya (-14 LUFS) supaya imbangannya dengan efek suara tetap.
    let take = |channel: usize| -> Vec<f64> {
        (0..count).map(|n| song.get((first + n) * 2 + channel).copied().unwrap_or(0.0) as f64).collect()
    };
    let (left, right) = (take(0), take(1));
    let gain = 10f64.powf((TARGET_LUFS - loudness(&left, &right)) / 20.0);
    for n in 0..count {
        mix.music.left[n] = left[n] * gain;
        mix.music.right[n] = right[n] * gain;
    }
    let boom = boom();
    for (at, pan, cue) in &score.cues {
        play(&mut mix, *at, *pan, cue, &boom, score.key_shift);
    }
    for bus in [&mut mix.sfx, &mut mix.verb, &mut mix.echo] {
        for x in bus.left.iter_mut().chain(bus.right.iter_mut()) {
            *x *= 0.7;
        }
    }
    master(mix)
}

// ---------------------------------------------------------------- bahan dasar

fn note(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

/// Derau putih yang bisa diulang (xorshift).
struct Noise(u64);

impl Noise {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Low,
    High,
    Band,
    /// Rak tinggi dengan penguatan (dB), untuk pembobotan K pengukur kenyaringan.
    Shelf(f64),
}

/// Filter biquad (resep RBJ).
#[derive(Default)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn new(kind: Kind, freq: f64, q: f64) -> Self {
        let mut filter = Self::default();
        filter.set(kind, freq, q);
        filter
    }

    /// Mengganti koefisien tanpa mengosongkan isi filter, jadi frekuensinya bisa digeser.
    fn set(&mut self, kind: Kind, freq: f64, q: f64) {
        let w = TAU * freq.clamp(10.0, SR * 0.45) / SR;
        let (sin, cos) = w.sin_cos();
        let alpha = sin / (2.0 * q);
        let (b0, b1, b2, a0, a1, a2) = match kind {
            Kind::Low => ((1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            Kind::High => ((1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            Kind::Band => (alpha, 0.0, -alpha, 1.0 + alpha, -2.0 * cos, 1.0 - alpha),
            Kind::Shelf(gain) => {
                let a = 10f64.powf(gain / 40.0);
                let root = 2.0 * a.sqrt() * alpha;
                (
                    a * ((a + 1.0) + (a - 1.0) * cos + root),
                    -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                    a * ((a + 1.0) + (a - 1.0) * cos - root),
                    (a + 1.0) - (a - 1.0) * cos + root,
                    2.0 * ((a - 1.0) - (a + 1.0) * cos),
                    (a + 1.0) - (a - 1.0) * cos - root,
                )
            }
        };
        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    fn run(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Gelombang gergaji tanpa aliasing (PolyBLEP); isinya fase 0..1.
struct Saw(f64);

impl Saw {
    fn next(&mut self, freq: f64) -> f64 {
        let dt = freq / SR;
        let t = self.0;
        let mut y = 2.0 * t - 1.0;
        if t < dt {
            let x = t / dt;
            y -= x + x - x * x - 1.0;
        } else if t > 1.0 - dt {
            let x = (t - 1.0) / dt;
            y -= x * x + x + x + 1.0;
        }
        self.0 = (t + dt).fract();
        y
    }
}

/// Merender satu suara mono sepanjang `len` detik. Puncaknya dinormalkan ke 1, jadi takaran
/// di aransemen langsung berarti "seberapa keras", dan ujungnya dipudarkan supaya tidak klik.
fn synth(len: f64, mut f: impl FnMut(f64) -> f64) -> Vec<f64> {
    let count = (len * SR) as usize;
    let mut out: Vec<f64> = (0..count).map(|i| f(i as f64 / SR)).collect();
    let fade = ((0.008 * SR) as usize).min(count);
    for i in 0..fade {
        out[count - 1 - i] *= i as f64 / fade as f64;
    }
    let peak = out.iter().fold(0.0f64, |m, x| m.max(x.abs()));
    if peak > 0.0 {
        out.iter_mut().for_each(|x| *x /= peak);
    }
    out
}

/// Naik dalam `attack` detik, tahan sampai detik `hold`, lalu meluruh eksponensial.
fn envelope(t: f64, attack: f64, hold: f64, release: f64) -> f64 {
    let rise = (t / attack).min(1.0);
    if t < hold { rise } else { rise * (-(t - hold) / release).exp() }
}

// ---------------------------------------------------------------- instrumen

fn kick() -> Vec<f64> {
    let mut noise = Noise::new(1);
    let mut click = Biquad::new(Kind::High, 2500.0, 0.7);
    let mut phase = 0.0;
    synth(0.45, |t| {
        phase += TAU * (46.0 + 150.0 * (-t / 0.035).exp()) / SR;
        let body = (phase.sin() * 2.2).tanh() * envelope(t, 0.001, 0.0, 0.16);
        body + click.run(noise.next()) * 0.5 * (-t / 0.003).exp()
    })
}

fn clap() -> Vec<f64> {
    let mut noise = Noise::new(2);
    let mut band = Biquad::new(Kind::Band, 1300.0, 0.8);
    let mut high = Biquad::new(Kind::High, 600.0, 0.7);
    synth(0.4, |t| {
        let bursts: f64 = (0..3)
            .map(|k| t - k as f64 * 0.012)
            .filter(|s| *s >= 0.0)
            .map(|s| (-s / 0.005).exp())
            .sum();
        let tail = if t >= 0.024 { 0.7 * (-(t - 0.024) / 0.11).exp() } else { 0.0 };
        high.run(band.run(noise.next())) * (bursts + tail)
    })
}

/// Hi-hat: derau ditambah enam gelombang kotak sumbang ala drum machine klasik.
fn hat(open: bool) -> Vec<f64> {
    let mut noise = Noise::new(if open { 7 } else { 5 });
    let mut high = Biquad::new(Kind::High, 7000.0, 0.7);
    let decay = if open { 0.13 } else { 0.02 };
    let tones = [205.3, 304.4, 369.6, 522.7, 540.0, 800.0];
    synth(decay * 6.0, |t| {
        let metal = tones.iter().map(|f| (TAU * f * 1.7 * t).sin().signum()).sum::<f64>() / 6.0;
        high.run(0.6 * noise.next() + 0.5 * metal) * envelope(t, 0.0005, 0.0, decay)
    })
}

fn snare() -> Vec<f64> {
    let mut noise = Noise::new(3);
    let mut band = Biquad::new(Kind::Band, 3500.0, 0.7);
    let mut high = Biquad::new(Kind::High, 1200.0, 0.7);
    let mut phase = 0.0;
    synth(0.3, |t| {
        phase += TAU * (180.0 + 60.0 * (-t / 0.02).exp()) / SR;
        let body = phase.sin() * envelope(t, 0.001, 0.0, 0.05);
        let rattle = high.run(band.run(noise.next())) * envelope(t, 0.001, 0.0, 0.1);
        0.6 * body + 2.0 * rattle
    })
}

fn crash() -> Vec<f64> {
    let mut noise = Noise::new(4);
    let mut high = Biquad::new(Kind::High, 5000.0, 0.7);
    let mut band = Biquad::new(Kind::Band, 9000.0, 0.6);
    synth(2.2, |t| {
        let n = noise.next();
        (high.run(n) + 0.6 * band.run(n)) * envelope(t, 0.002, 0.0, 0.7)
    })
}

/// Dentuman rendah untuk drop, dengan lapisan "dum" di tengah supaya tetap terasa di speaker HP.
fn boom() -> Vec<f64> {
    let mut noise = Noise::new(6);
    let mut low = Biquad::new(Kind::Low, 900.0, 0.7);
    let mut phase = 0.0;
    synth(1.8, |t| {
        phase += TAU * (36.0 + 70.0 * (-t / 0.08).exp()) / SR;
        let sub = (phase.sin() * 1.6).tanh() * envelope(t, 0.002, 0.0, 0.45);
        sub + 1.5 * low.run(noise.next()) * envelope(t, 0.001, 0.0, 0.05)
    })
}

/// Pad hangat: tiap nada tiga gergaji yang sedikit sumbang, disaring lowpass yang mengikuti
/// `cutoff` pada detik video yang sebenarnya.
fn pad(notes: &[f64; 4], at: f64, len: f64, seed: u64, cutoff: &dyn Fn(f64) -> f64) -> Vec<f64> {
    let mut noise = Noise::new(seed);
    let mut voices = Vec::new();
    for &midi in notes {
        for cents in [-9.0, 0.0, 9.0] {
            voices.push((Saw((noise.next() + 1.0) / 2.0), note(midi + cents / 100.0)));
        }
    }
    let mut first = Biquad::new(Kind::Low, 1000.0, 0.6);
    let mut second = Biquad::new(Kind::Low, 1000.0, 0.6);
    let mut i = 0usize;
    synth(len + 0.9, |t| {
        if i % 32 == 0 {
            let cutoff = cutoff(at + t);
            first.set(Kind::Low, cutoff, 0.6);
            second.set(Kind::Low, cutoff, 0.6);
        }
        i += 1;
        let x: f64 = voices.iter_mut().map(|(saw, f)| saw.next(*f)).sum();
        second.run(first.run(x)) * envelope(t, 0.25, len, 0.22)
    })
}

/// Bass: sinus untuk dentum rendahnya, ditambah gergaji satu oktaf di atas supaya tetap
/// terdengar di speaker HP yang tidak sanggup memutar nada serendah itu.
fn bass(midi: f64, len: f64) -> Vec<f64> {
    let f = note(midi);
    let mut saw = Saw(0.0);
    let mut low = Biquad::new(Kind::Low, 600.0, 0.8);
    synth(len + 0.06, |t| {
        let sub = (TAU * f * t).sin();
        let body = low.run(saw.next(f * 2.0));
        ((sub + 0.5 * body) * 1.4).tanh() * envelope(t, 0.004, len, 0.015)
    })
}

/// Pluck ala musik elektronik: filter yang menutup cepat setelah dipetik.
fn pluck(midi: f64) -> Vec<f64> {
    let f = note(midi);
    let (mut a, mut b) = (Saw(0.0), Saw(0.5));
    let mut low = Biquad::new(Kind::Low, 4000.0, 1.2);
    synth(0.7, |t| {
        low.set(Kind::Low, 350.0 + 4200.0 * (-t / 0.06).exp(), 1.2);
        let x = a.next(f) + 0.6 * b.next(f * 1.004);
        low.run(x) * envelope(t, 0.002, 0.0, 0.2)
    })
}

/// Lonceng FM: `ratio` 2 terdengar seperti marimba elektrik, 3.5 seperti lonceng kaca.
fn bell(midi: f64, ratio: f64, decay: f64) -> Vec<f64> {
    let f = note(midi);
    synth(decay * 5.0, |t| {
        let index = 2.2 * (-t / 0.1).exp() + 0.5;
        let modulator = (TAU * f * ratio * t).sin();
        (TAU * f * t + index * modulator).sin() * envelope(t, 0.002, 0.0, decay)
    })
}

// ---------------------------------------------------------------- efek suara

/// Desir angin: derau band-pass yang frekuensi tengahnya bergeser dari `from` ke `to` Hz.
fn whoosh(len: f64, from: f64, to: f64, seed: u64) -> Vec<f64> {
    let mut noise = Noise::new(100 + seed);
    let mut band = Biquad::new(Kind::Band, from, 1.3);
    let mut body = Biquad::new(Kind::Low, 700.0, 0.7);
    synth(len, |t| {
        let p = t / len;
        band.set(Kind::Band, from * (to / from).powf(p), 1.3);
        let n = noise.next();
        (band.run(n) + 0.3 * body.run(n)) * (PI * p.powf(1.3)).sin().powi(2)
    })
}

/// Naik menuju drop: desir yang makin tinggi dan makin keras ditambah nada yang naik dua
/// oktaf, lalu terpotong tepat di ketukan drop.
fn riser(len: f64, seed: u64) -> Vec<f64> {
    let mut noise = Noise::new(200 + seed);
    let mut band = Biquad::new(Kind::Band, 300.0, 2.0);
    let mut saw = Saw(0.0);
    let mut low = Biquad::new(Kind::Low, 2000.0, 0.7);
    synth(len, |t| {
        let p = t / len;
        band.set(Kind::Band, 300.0 * 25f64.powf(p), 2.0);
        let tone = low.run(saw.next(220.0 * 4f64.powf(p)));
        (2.0 * band.run(noise.next()) + 0.25 * tone) * p.powf(2.2)
    })
}

/// Gelembung UI: sinus yang nadanya melonjak naik sekejap.
fn pop(midi: f64) -> Vec<f64> {
    let f = note(midi);
    let mut phase = 0.0;
    synth(0.14, |t| {
        phase += TAU * f * (1.0 + 0.9 * (1.0 - (-t / 0.02).exp())) / SR;
        phase.sin() * envelope(t, 0.001, 0.0, 0.04)
    })
}

/// Nada meluncur naik atau turun (strip HUD membuka dan menutup).
fn glide(len: f64, from: f64, to: f64) -> Vec<f64> {
    let mut phase = 0.0;
    synth(len, |t| {
        let p = t / len;
        phase += TAU * from * (to / from).powf(p) / SR;
        (phase.sin() + 0.3 * (2.0 * phase).sin()) * (PI * p).sin().powi(2)
    })
}

fn tick(hz: f64) -> Vec<f64> {
    synth(0.035, |t| (TAU * hz * t).sin() * envelope(t, 0.0005, 0.0, 0.006))
}

/// Klik tombol keyboard (Caps Lock).
fn key_click() -> Vec<f64> {
    let mut noise = Noise::new(11);
    let mut band = Biquad::new(Kind::Band, 4500.0, 1.0);
    synth(0.08, |t| {
        3.0 * band.run(noise.next()) * envelope(t, 0.0003, 0.0, 0.004) + 0.5 * (TAU * 230.0 * t).sin() * envelope(t, 0.001, 0.0, 0.018)
    })
}

/// Satu ketukan rana kamera.
fn shutter(seed: u64) -> Vec<f64> {
    let mut noise = Noise::new(300 + seed);
    let mut band = Biquad::new(Kind::Band, 2200.0, 1.2);
    synth(0.06, |t| 2.0 * band.run(noise.next()) * envelope(t, 0.0005, 0.0, 0.008) + 0.3 * (TAU * 420.0 * t).sin() * envelope(t, 0.001, 0.0, 0.012))
}

/// Desis listrik pendek: dengung kotak 120 Hz bercampur derau yang disaring naik.
fn zap(seed: u64) -> Vec<f64> {
    let mut noise = Noise::new(400 + seed);
    let mut band = Biquad::new(Kind::Band, 1500.0, 1.5);
    synth(0.35, |t| {
        let p = t / 0.35;
        band.set(Kind::Band, 1500.0 * 4f64.powf(p), 1.5);
        let buzz = (TAU * 120.0 * t).sin().signum() * 0.35;
        (band.run(noise.next()) * 2.0 + buzz) * envelope(t, 0.002, 0.0, 0.09)
    })
}

/// File mendarat di shelf: dentum lembut.
fn thud() -> Vec<f64> {
    let mut noise = Noise::new(13);
    let mut low = Biquad::new(Kind::Low, 700.0, 0.7);
    let mut phase = 0.0;
    synth(0.4, |t| {
        phase += TAU * (65.0 + 110.0 * (-t / 0.025).exp()) / SR;
        phase.sin() * envelope(t, 0.001, 0.0, 0.085) + 0.5 * low.run(noise.next()) * envelope(t, 0.001, 0.0, 0.02)
    })
}

// ---------------------------------------------------------------- campuran

struct Bus {
    left: Vec<f64>,
    right: Vec<f64>,
}

impl Bus {
    fn new(samples: usize) -> Self {
        Self { left: vec![0.0; samples], right: vec![0.0; samples] }
    }

    /// Menaruh suara mono mulai detik `at`; posisi stereonya bergeser dari `pan.0` ke `pan.1`
    /// (-1 kiri, 1 kanan) sepanjang suara.
    fn place(&mut self, at: f64, sound: &[f64], gain: f64, pan: (f64, f64)) {
        let start = (at * SR).round() as usize;
        let len = sound.len() as f64;
        for (i, x) in sound.iter().enumerate() {
            let n = start + i;
            if n >= self.left.len() {
                break;
            }
            let p = pan.0 + (pan.1 - pan.0) * i as f64 / len;
            let angle = (p.clamp(-1.0, 1.0) + 1.0) * PI / 4.0;
            self.left[n] += x * gain * angle.cos();
            self.right[n] += x * gain * angle.sin();
        }
    }

    fn rms(&self) -> f64 {
        let sum: f64 = self.left.iter().chain(&self.right).map(|x| x * x).sum();
        (sum / (2 * self.left.len()) as f64).sqrt()
    }
}

#[derive(Clone, Copy)]
enum Lane {
    Drums,
    /// Ikut "memompa": meredup sesaat di setiap kick.
    Music,
    Sfx,
}

struct Mix {
    drums: Bus,
    music: Bus,
    sfx: Bus,
    verb: Bus,
    echo: Bus,
    kicks: Vec<f64>,
}

impl Mix {
    fn new(seconds: f64) -> Self {
        let n = (seconds * SR).round() as usize;
        Self { drums: Bus::new(n), music: Bus::new(n), sfx: Bus::new(n), verb: Bus::new(n), echo: Bus::new(n), kicks: Vec::new() }
    }

    /// Satu suara ke jalurnya, plus kiriman ke reverb dan delay (`sends`).
    fn put(&mut self, lane: Lane, at: f64, sound: &[f64], gain: f64, pan: (f64, f64), sends: (f64, f64)) {
        let bus = match lane {
            Lane::Drums => &mut self.drums,
            Lane::Music => &mut self.music,
            Lane::Sfx => &mut self.sfx,
        };
        bus.place(at, sound, gain, pan);
        if sends.0 > 0.0 {
            self.verb.place(at, sound, gain * sends.0, pan);
        }
        if sends.1 > 0.0 {
            self.echo.place(at, sound, gain * sends.1, pan);
        }
    }
}

struct Comb {
    buffer: Vec<f64>,
    index: usize,
    store: f64,
}

impl Comb {
    fn run(&mut self, x: f64, feedback: f64, damp: f64) -> f64 {
        let y = self.buffer[self.index];
        self.store = y * (1.0 - damp) + self.store * damp;
        self.buffer[self.index] = x + self.store * feedback;
        self.index = (self.index + 1) % self.buffer.len();
        y
    }
}

struct Allpass {
    buffer: Vec<f64>,
    index: usize,
}

impl Allpass {
    fn run(&mut self, x: f64) -> f64 {
        let b = self.buffer[self.index];
        self.buffer[self.index] = x + b * 0.5;
        self.index = (self.index + 1) % self.buffer.len();
        b - x
    }
}

/// Reverb ruang besar (Freeverb), dengan jeda awal 20 ms dan tanpa frekuensi rendah supaya
/// tidak membuat bass keruh.
fn reverb(send: &Bus) -> Bus {
    let scale = SR / 44_100.0;
    let pre = (0.02 * SR) as usize;
    let mut high = Biquad::new(Kind::High, 250.0, 0.7);
    let input: Vec<f64> = (0..send.left.len())
        .map(|n| if n >= pre { high.run((send.left[n - pre] + send.right[n - pre]) * 0.5) } else { 0.0 })
        .collect();
    let mut out = Bus::new(send.left.len());
    for (channel, spread) in [(0, 0usize), (1, 23)] {
        let size = |n: usize| ((n + spread) as f64 * scale) as usize;
        let mut combs: Vec<Comb> = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617]
            .iter()
            .map(|&n| Comb { buffer: vec![0.0; size(n)], index: 0, store: 0.0 })
            .collect();
        let mut passes: Vec<Allpass> =
            [556, 441, 341, 225].iter().map(|&n| Allpass { buffer: vec![0.0; size(n)], index: 0 }).collect();
        let target = if channel == 0 { &mut out.left } else { &mut out.right };
        for (n, &x) in input.iter().enumerate() {
            let mut y: f64 = combs.iter_mut().map(|comb| comb.run(x * 0.015, 0.86, 0.2)).sum();
            for pass in &mut passes {
                y = pass.run(y);
            }
            target[n] = y;
        }
    }
    out
}

/// Gema ping-pong (kiri, kanan, kiri, ...), tiap pantulan sedikit lebih redup dan gelap.
fn echo(send: &Bus, time: f64, feedback: f64) -> Bus {
    let d = (time * SR) as usize;
    let mut out = Bus::new(send.left.len());
    let mut dark_l = Biquad::new(Kind::Low, 3500.0, 0.7);
    let mut dark_r = Biquad::new(Kind::Low, 3500.0, 0.7);
    for n in d..out.left.len() {
        let input = (send.left[n - d] + send.right[n - d]) * 0.5;
        out.left[n] = input + feedback * dark_l.run(out.right[n - d]);
        out.right[n] = feedback * dark_r.run(out.left[n - d]);
    }
    out
}

/// Kenyaringan terpadu (LUFS) menurut ITU-R BS.1770: pembobotan K, blok 400 ms, gerbang
/// absolut -70 LUFS dan relatif -10 LU.
pub(super) fn loudness(left: &[f64], right: &[f64]) -> f64 {
    let weigh = |x: &[f64]| {
        let mut shelf = Biquad::new(Kind::Shelf(4.0), 1681.97, 0.7072);
        let mut high = Biquad::new(Kind::High, 38.13, 0.5003);
        x.iter().map(|&v| high.run(shelf.run(v))).collect::<Vec<f64>>()
    };
    let (l, r) = (weigh(left), weigh(right));
    let block = (0.4 * SR) as usize;
    let mut powers = Vec::new();
    let mut start = 0;
    while start + block <= l.len() {
        let sum: f64 = (start..start + block).map(|i| l[i] * l[i] + r[i] * r[i]).sum();
        powers.push(sum / block as f64);
        start += block / 4;
    }
    let lufs = |z: f64| -0.691 + 10.0 * z.max(1e-12).log10();
    let mean = |zs: &[f64]| zs.iter().sum::<f64>() / zs.len().max(1) as f64;
    let gated: Vec<f64> = powers.into_iter().filter(|&z| lufs(z) > -70.0).collect();
    let threshold = lufs(mean(&gated)) - 10.0;
    let kept: Vec<f64> = gated.into_iter().filter(|&z| lufs(z) > threshold).collect();
    lufs(mean(&kept))
}

/// Limiter dengan lihat-depan 5 ms: penguatannya sudah turun sebelum puncak tiba lalu pulih
/// pelan, jadi tidak ada sampel yang melewati `CEILING`. Mengembalikan peredaman terbesar (dB).
fn limit(left: &mut [f64], right: &mut [f64]) -> f64 {
    let n = left.len();
    let ahead = (0.005 * SR) as usize;
    let need: Vec<f64> = (0..n).map(|i| (CEILING / left[i].abs().max(right[i].abs()).max(1e-9)).min(1.0)).collect();
    // Nilai terkecil dari `need` di jendela [i, i + ahead].
    let mut target = vec![1.0; n];
    let mut window: VecDeque<usize> = VecDeque::new();
    for j in 0..n + ahead {
        if j < n {
            while window.back().is_some_and(|&k| need[k] >= need[j]) {
                window.pop_back();
            }
            window.push_back(j);
        }
        if j >= ahead {
            let i = j - ahead;
            while window.front().is_some_and(|&k| k < i) {
                window.pop_front();
            }
            target[i] = need[window[0]];
        }
    }
    let release = 1.0 - (-1.0 / (0.08 * SR)).exp();
    let mut gain = 1.0f64;
    let mut held = vec![1.0; n];
    for i in 0..n {
        gain = target[i].min(gain + (1.0 - gain) * release);
        held[i] = gain;
    }
    // Rata-rata berjalan sepanjang jendela lihat-depan supaya penguatan turun mulus.
    let mut sum = 0.0;
    let mut deepest = 1.0f64;
    for i in 0..n {
        sum += held[i];
        if i >= ahead {
            sum -= held[i - ahead];
        }
        let smooth = sum / ahead.min(i + 1) as f64;
        deepest = deepest.min(smooth);
        left[i] = (left[i] * smooth).clamp(-CEILING, CEILING);
        right[i] = (right[i] * smooth).clamp(-CEILING, CEILING);
    }
    20.0 * deepest.log10()
}

fn master(mut mix: Mix) -> Vec<f32> {
    if mix.music.left.is_empty() {
        return Vec::new();
    }
    // Pompa sidechain: bus musik meredup sesaat di setiap kick lalu pulih.
    let mut last = f64::NEG_INFINITY;
    let mut next = 0;
    for n in 0..mix.music.left.len() {
        let t = n as f64 / SR;
        while next < mix.kicks.len() && mix.kicks[next] <= t {
            last = mix.kicks[next];
            next += 1;
        }
        let since = t - last;
        let duck = 1.0 - 0.5 * (since / 0.006).min(1.0) * (-since / 0.11).exp();
        mix.music.left[n] *= duck;
        mix.music.right[n] *= duck;
    }
    let echoes = echo(&mix.echo, BEAT * 0.75, 0.38);
    for n in 0..mix.verb.left.len() {
        mix.verb.left[n] += 0.4 * echoes.left[n];
        mix.verb.right[n] += 0.4 * echoes.right[n];
    }
    let verb = reverb(&mix.verb);
    eprintln!(
        "bus rms: drum {:.3}, musik {:.3}, efek {:.3}, gema {:.3}, reverb {:.3}",
        mix.drums.rms(),
        mix.music.rms(),
        mix.sfx.rms(),
        echoes.rms(),
        verb.rms()
    );
    let mut sum = Bus::new(mix.music.left.len());
    let mut highs = [Biquad::new(Kind::High, 25.0, 0.7), Biquad::new(Kind::High, 25.0, 0.7)];
    for n in 0..sum.left.len() {
        let l = mix.drums.left[n] + mix.music.left[n] + mix.sfx.left[n] + 0.9 * echoes.left[n] + 3.5 * verb.left[n];
        let r = mix.drums.right[n] + mix.music.right[n] + mix.sfx.right[n] + 0.9 * echoes.right[n] + 3.5 * verb.right[n];
        sum.left[n] = highs[0].run(l);
        sum.right[n] = highs[1].run(r);
    }

    // Samakan kenyaringan ke sasaran; limiter menjaga puncaknya. Diulang karena limiter sedikit
    // menurunkan kenyaringan.
    let mut gain = 1.0;
    let (mut left, mut right) = (Vec::new(), Vec::new());
    for round in 0..4 {
        left = sum.left.iter().map(|x| x * gain).collect();
        right = sum.right.iter().map(|x| x * gain).collect();
        let reduction = limit(&mut left, &mut right);
        let lufs = loudness(&left, &right);
        eprintln!("putaran {round}: penguatan {:+.1} dB, limiter {reduction:.1} dB, {lufs:.1} LUFS", 20.0 * f64::log10(gain));
        if (lufs - TARGET_LUFS).abs() < 0.1 {
            break;
        }
        gain *= 10f64.powf((TARGET_LUFS - lufs) / 20.0);
    }

    // Masuk halus di awal, memudar bersama gambarnya di detik terakhir.
    let count = left.len();
    let end = count as f64 / SR;
    let mut out = Vec::with_capacity(count * 2);
    for n in 0..count {
        let t = n as f64 / SR;
        let fade = (t / 0.01).min(1.0) * (0.5 + 0.5 * (PI * (t - (end - 1.0)).clamp(0.0, 1.0)).cos());
        out.push((left[n] * fade) as f32);
        out.push((right[n] * fade) as f32);
    }
    out
}

/// Seluruh musik dan efek suara, stereo berselang-seling (L, R, L, R, ...), sepanjang video.
pub(super) fn soundtrack() -> Vec<f32> {
    let mut mix = Mix::new(LENGTH);
    arrange(&mut mix);
    master(mix)
}

pub(super) fn pcm16(samples: &[f32]) -> Vec<i16> {
    samples.iter().map(|&x| (x.clamp(-1.0, 1.0) * 32767.0).round() as i16).collect()
}

pub(super) fn save_wav(path: &Path, pcm: &[i16]) {
    let data = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data.to_le_bytes());
    for sample in pcm {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    std::fs::write(path, out).unwrap();
}

pub(super) fn load_wav(path: &Path) -> Vec<i16> {
    let bytes = std::fs::read(path).unwrap();
    bytes[44..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
}

// ---------------------------------------------------------------- pemeriksaan

/// Kenyaringan per adegan (LUFS) dan puncak (dBFS), untuk melihat naik-turunnya lagu.
pub(super) fn report(samples: &[f32], cuts: &[f64]) {
    let left: Vec<f64> = samples.iter().step_by(2).map(|&x| x as f64).collect();
    let right: Vec<f64> = samples.iter().skip(1).step_by(2).map(|&x| x as f64).collect();
    let peak = samples.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    eprintln!("total {:.1} LUFS, puncak {:.1} dBFS", loudness(&left, &right), 20.0 * (peak as f64).log10());
    let mut edges = vec![0.0];
    edges.extend(cuts);
    edges.push(samples.len() as f64 / 2.0 / SR);
    for pair in edges.windows(2) {
        let (a, b) = ((pair[0] * SR) as usize, (pair[1] * SR) as usize);
        eprintln!("  {:5.1}–{:4.1} s: {:.1} LUFS", pair[0], pair[1], loudness(&left[a..b], &right[a..b]));
    }
}

/// Selisih waktu (sampel) `decoded` terhadap `source` di sekitar detik `at`; positif berarti
/// `decoded` terlambat.
pub(super) fn lag(source: &[i16], decoded: &[i16], at: f64) -> i64 {
    let mono = |pcm: &[i16]| pcm.chunks_exact(2).map(|s| (s[0] as f64 + s[1] as f64) / 65536.0).collect::<Vec<f64>>();
    let (a, b) = (mono(source), mono(decoded));
    let start = (at * SR) as i64;
    let span = (0.5 * SR) as i64;
    let mut best = (0i64, f64::MIN);
    for shift in -4800i64..=4800 {
        let mut score = 0.0;
        for i in start..start + span {
            let j = i + shift;
            if j >= 0 && (j as usize) < b.len() && (i as usize) < a.len() {
                score += a[i as usize] * b[j as usize];
            }
        }
        if score > best.1 {
            best = (shift, score);
        }
    }
    best.0
}

/// Gambar pemeriksa: bentuk gelombang di atas, spektrogram (30 Hz–20 kHz, skala log) di bawah,
/// garis putih di setiap potongan adegan dan garis tipis tiap detik.
pub(super) fn picture(samples: &[f32], cuts: &[f64]) -> (Vec<u32>, i32, i32) {
    const WIDTH: usize = 1400;
    const WAVE: usize = 160;
    const SPECTRUM: usize = 320;
    let height = WAVE + 4 + SPECTRUM;
    let mono: Vec<f64> = samples.chunks_exact(2).map(|s| (s[0] + s[1]) as f64 * 0.5).collect();
    let hop = mono.len() / WIDTH;
    let mut pixels = vec![0xFF10_1418u32; WIDTH * height];
    let size = 2048;
    let window: Vec<f64> = (0..size).map(|i| 0.5 - 0.5 * (TAU * i as f64 / size as f64).cos()).collect();
    let stops = [0x000004u32, 0x420A68, 0x932667, 0xDD513A, 0xFCA50A, 0xFCFFA4];
    let color = |v: f64| {
        let x = v.clamp(0.0, 1.0) * (stops.len() - 1) as f64;
        let i = (x as usize).min(stops.len() - 2);
        let f = x - i as f64;
        let channel = |shift: u32| {
            let (a, b) = ((stops[i] >> shift) & 0xFF, (stops[i + 1] >> shift) & 0xFF);
            ((a as f64 + (b as f64 - a as f64) * f) as u32) << shift
        };
        0xFF00_0000 | channel(16) | channel(8) | channel(0)
    };
    for x in 0..WIDTH {
        let chunk = &mono[x * hop..((x + 1) * hop).min(mono.len())];
        let (low, high) = chunk.iter().fold((0.0f64, 0.0f64), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        let rms = (chunk.iter().map(|v| v * v).sum::<f64>() / chunk.len() as f64).sqrt();
        let y = |v: f64| ((WAVE as f64 / 2.0) * (1.0 - v)).clamp(0.0, WAVE as f64 - 1.0) as usize;
        for row in y(high)..=y(low) {
            pixels[row * WIDTH + x] = 0xFF3A_6EA5;
        }
        for row in y(rms)..=y(-rms) {
            pixels[row * WIDTH + x] = 0xFF8F_C8FF;
        }
        let middle = (x * hop + hop / 2) as i64;
        let mut re: Vec<f64> = (0..size)
            .map(|i| {
                let n = middle - size as i64 / 2 + i as i64;
                if n >= 0 && (n as usize) < mono.len() { mono[n as usize] * window[i] } else { 0.0 }
            })
            .collect();
        let mut im = vec![0.0; size];
        fft(&mut re, &mut im);
        for row in 0..SPECTRUM {
            let f = 20_000.0 * (30.0f64 / 20_000.0).powf(row as f64 / (SPECTRUM - 1) as f64);
            let bin = ((f / (SR / size as f64)).round() as usize).min(size / 2);
            let magnitude = (re[bin] * re[bin] + im[bin] * im[bin]).sqrt() / (size as f64 / 4.0);
            let db = 20.0 * magnitude.max(1e-9).log10();
            pixels[(WAVE + 4 + row) * WIDTH + x] = color((db + 90.0) / 80.0);
        }
    }
    let seconds = mono.len() as f64 / SR;
    for second in 1..seconds as usize {
        let x = (second as f64 / seconds * WIDTH as f64) as usize;
        for row in WAVE - 12..WAVE {
            pixels[row * WIDTH + x] = 0xFF70_7880;
        }
    }
    for &cut in cuts {
        let x = (cut / seconds * WIDTH as f64) as usize;
        for row in 0..height {
            pixels[row * WIDTH + x] = 0xFFFF_FFFF;
        }
    }
    (pixels, WIDTH as i32, height as i32)
}

/// FFT radix-2 di tempat.
fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let (ws, wc) = (-TAU / len as f64).sin_cos();
        for start in (0..n).step_by(len) {
            let (mut wr, mut wi) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (a, b) = (start + k, start + k + len / 2);
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = wr * wc - wi * ws;
                wi = wr * ws + wi * wc;
                wr = next;
            }
        }
        len <<= 1;
    }
}
