//! Lirik lagu tersinkron dari LRCLIB (lrclib.net), layanan lirik gratis tanpa akun.
//! Opsional dan mati secara default. Yang dikirim hanya judul, artis, album, dan durasi lagu
//! yang sedang diputar, lewat HTTPS. Hasilnya hanya disimpan di memori.
//!
//! Lagu dari YouTube dan YouTube Music di browser juga dicarikan: judul video seperti
//! "TULUS - Jatuh Suka (Official Lyric Video)" dirapikan dulu jadi judul dan artis lagunya.

use std::thread;

use windows::Data::Json::{JsonArray, JsonObject, JsonValueType};
use windows::Foundation::Uri;
use windows::Web::Http::HttpClient;
use windows::core::{HSTRING, Result};

use crate::services::{Event, Notifier};

const API: &str = "https://lrclib.net/api";
/// LRCLIB meminta setiap aplikasi memperkenalkan diri.
const USER_AGENT: &str = concat!("NotchWindows/", env!("CARGO_PKG_VERSION"), " (aplikasi notch pribadi)");
/// Hasil yang durasinya sedekat ini dengan lagu yang diputar dianggap versi yang sama; pencarian
/// berhenti di situ.
const CLOSE_SECS: f64 = 3.0;
/// Selisih anggapan untuk hasil tanpa durasi, supaya hasil berdurasi yang dekat didahulukan.
const UNKNOWN_GAP: f64 = 60.0;
/// Hasil sependek ini kemungkinan potongan lagu, bukan lagunya.
const SHORTEST_SONG: f64 = 45.0;
/// Kata tambahan di judul video yang bukan bagian judul lagu, misal "(Official Music Video)".
const NOISE: [&str; 27] = [
    "official", "video", "videoclip", "clip", "mv", "audio", "lyric", "lyrics", "lirik", "visualizer",
    "visualiser", "hd", "hq", "4k", "1080p", "720p", "remaster", "remastered", "explicit", "feat", "ft",
    "prod", "with", "music", "musik", "lagu", "full",
];
/// Kata yang cukup untuk membuang seluruh isi kurung, misal "(Lirik Lagu)" atau "[MV]". Kata
/// umum seperti "music" saja tidak, supaya "(Music from the Movie)" tetap utuh.
const STRONG_NOISE: usize = 23;
/// Baris ditampilkan sedikit lebih awal supaya terasa pas dengan suara.
const LEAD_SECS: f64 = 0.15;

pub struct Line {
    pub at: f64,
    pub text: String,
}

/// Lirik satu lagu; `lines` kosong berarti lirik tersinkron tidak ditemukan.
pub struct Lyrics {
    pub key: String,
    pub lines: Vec<Line>,
}

impl Lyrics {
    /// Indeks baris yang sedang dinyanyikan pada `position` detik.
    pub fn current(&self, position: f64) -> Option<usize> {
        let index = self.lines.partition_point(|line| line.at <= position + LEAD_SECS);
        index.checked_sub(1)
    }

    /// Teks baris; jeda instrumental (baris kosong) ditampilkan sebagai not balok.
    pub fn text(&self, index: usize) -> &str {
        self.lines.get(index).map(|l| l.text.as_str()).filter(|t| !t.is_empty()).unwrap_or("♪")
    }
}

pub struct Request {
    pub key: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
}

/// Kunci lagu untuk mencocokkan lirik dengan lagu yang sedang diputar.
pub fn key(title: &str, artist: &str) -> String {
    format!("{title}\u{1}{artist}")
}

/// Ambil lirik di thread terpisah; hasilnya datang sebagai [`Event::Lyrics`].
pub fn spawn(request: Request, notify: Notifier) {
    let spawned = thread::Builder::new().name("lirik".into()).spawn(move || {
        let lines = match fetch(&request) {
            Ok(Some(lrc)) => parse(&lrc),
            Ok(None) => Vec::new(),
            Err(e) => {
                debug_log!("lirik gagal diambil: {e}");
                Vec::new()
            }
        };
        notify.send(Event::Lyrics(Lyrics { key: request.key, lines }));
    });
    if let Err(e) = spawned {
        debug_log!("thread lirik gagal dibuat: {e}");
    }
}

fn fetch(request: &Request) -> Result<Option<String>> {
    let client = HttpClient::new()?;
    client.DefaultRequestHeaders()?.UserAgent()?.TryParseAdd(&HSTRING::from(USER_AGENT))?;
    let get = |url: String| -> Result<HSTRING> { client.GetStringAsync(&Uri::CreateUri(&HSTRING::from(url))?)?.join() };

    // Cocok persis berdasarkan judul, artis, album, dan durasi (selisih ±2 detik).
    if request.duration > 0.0 {
        let url = format!(
            "{API}/get?track_name={}&artist_name={}&album_name={}&duration={}",
            encode(&request.title),
            encode(&request.artist),
            encode(&request.album),
            request.duration.round() as u64
        );
        if let Ok(body) = get(url)
            && let Some(lrc) = JsonObject::Parse(&body).ok().and_then(|o| synced(&o))
        {
            return Ok(Some(lrc));
        }
    }

    // Kalau tidak cocok persis, cari bertahap: judul dan artis apa adanya, judul yang sudah
    // dirapikan (judul video YouTube), lalu pencarian bebas. Dari semua hasil, dipilih yang
    // durasinya paling dekat; berhenti begitu ada yang hampir sama persis.
    let (title, artist) = tidy(&request.title, &request.artist);
    let mut queries = vec![format!("track_name={}&artist_name={}", encode(&request.title), encode(&request.artist))];
    if (title.as_str(), artist.as_str()) != (request.title.trim(), request.artist.trim()) {
        queries.push(format!("track_name={}&artist_name={}", encode(&title), encode(&artist)));
    }
    queries.push(format!("q={}", encode(&format!("{artist} {title}"))));
    let mut best: Option<(f64, String)> = None;
    let mut reached = false;
    let mut last_error = None;
    for query in &queries {
        // Pencarian bebas bisa mengembalikan lagu lain: judulnya harus cocok.
        let free = query.starts_with("q=");
        let results = match get(format!("{API}/search?{query}")).and_then(|body| JsonArray::Parse(&body)) {
            Ok(results) => results,
            Err(e) => {
                last_error = Some(e);
                continue;
            }
        };
        reached = true;
        for i in 0..results.Size().unwrap_or(0) {
            let Some(object) = results.GetAt(i).ok().and_then(|v| v.GetObject().ok()) else { continue };
            let Some(lrc) = synced(&object) else { continue };
            if free && !same(&title, &string(&object, "trackName")) {
                continue;
            }
            // Tanpa durasi lagu, potongan pendek (misal 4 detik) tidak didahulukan.
            let gap = match object.GetNamedNumber(&HSTRING::from("duration")) {
                Ok(seconds) if seconds > 0.0 && request.duration > 0.0 => (seconds - request.duration).abs(),
                _ if request.duration > 0.0 => UNKNOWN_GAP,
                Ok(seconds) if seconds >= SHORTEST_SONG => 0.0,
                _ => UNKNOWN_GAP,
            };
            if best.as_ref().is_none_or(|(old, _)| gap < *old) {
                best = Some((gap, lrc));
            }
        }
        if best.as_ref().is_some_and(|(gap, _)| *gap <= CLOSE_SECS) {
            break;
        }
    }
    match (best, last_error) {
        (Some((_, lrc)), _) => Ok(Some(lrc)),
        (None, Some(e)) if !reached => Err(e),
        (None, _) => Ok(None),
    }
}

fn string(object: &JsonObject, name: &str) -> String {
    object.GetNamedString(&HSTRING::from(name)).map(|s| s.to_string()).unwrap_or_default()
}

/// Judul dan artis yang lebih mungkin ditemukan untuk video YouTube: tambahan seperti
/// "(Official Music Video)" atau "[MV]" dibuang, "Artis - Judul" dan "Artis 'Judul' MV" dipisah,
/// dan nama channel seperti "Tulus - Topic" atau "TulusVEVO" dirapikan jadi nama penyanyinya.
pub fn tidy(title: &str, artist: &str) -> (String, String) {
    let mut artist = tidy_artist(artist);
    // "Judul | Official Video" atau "Artis - Judul // Lirik": yang dipakai bagian pertamanya.
    let first = title.split(" | ").next().unwrap_or(title).split(" // ").next().unwrap_or(title);
    let mut title = strip_brackets(first);
    if let Some((before, inside)) = quoted(&title) {
        if !before.is_empty() {
            artist = tidy_artist(&before);
        }
        title = inside;
    } else if let Some((left, right)) = split_dash(&title) {
        if is_noise(&right) || (same(&right, &artist) && !same(&left, &artist)) {
            // "Judul - Remastered 2011" atau "Judul - Artis".
            title = left;
        } else {
            artist = tidy_artist(&left);
            title = right;
        }
    }
    let title = cut_featuring(&strip_trailing(&title));
    (if title.is_empty() { squash(first) } else { title }, cut_featuring(&artist))
}

/// Nama channel jadi nama penyanyi: tanpa " - Topic", "VEVO", "Official", dan isi kurung
/// (misal "IU (아이유)" → "IU").
fn tidy_artist(artist: &str) -> String {
    let all = PAIRS.iter().fold(squash(artist), |text, &(open, close)| remove_brackets(&text, open, close, |_| true));
    let mut name = all.as_str();
    for suffix in [" - Topic", "VEVO"] {
        name = name.strip_suffix(suffix).unwrap_or(name);
    }
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1 && words.last().is_some_and(|w| ["official", "channel", "youtube"].contains(&w.to_lowercase().as_str())) {
        words.pop();
    }
    words.join(" ")
}

const PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('【', '】'), ('「', '」')];

/// Buang isi kurung yang berisi tambahan video, misal "(Official Lyric Video)" atau "[MV]";
/// kurung lain seperti "(Acoustic)" atau "(Live)" tetap.
fn strip_brackets(text: &str) -> String {
    let noisy = |inside: &str| {
        inside.to_lowercase().contains("m/v") || words(inside).any(|w| NOISE[..STRONG_NOISE].contains(&w.as_str()))
    };
    PAIRS.iter().fold(squash(text), |text, &(open, close)| remove_brackets(&text, open, close, noisy))
}

fn remove_brackets(text: &str, open: char, close: char, remove: impl Fn(&str) -> bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let inner = start + open.len_utf8();
        let Some(len) = rest[inner..].find(close) else { break };
        let end = inner + len + close.len_utf8();
        out.push_str(&rest[..start]);
        if !remove(&rest[inner..inner + len]) {
            out.push_str(&rest[start..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    squash(&out)
}

/// Judul ala K-pop: "BTS 'Dynamite' Official MV" → ("BTS", "Dynamite"). Tanda kutip pembuka harus
/// di awal atau sesudah spasi (bukan apostrof seperti "Don't"), dan sesudah kutip penutup hanya
/// boleh ada tambahan video.
fn quoted(text: &str) -> Option<(String, String)> {
    const QUOTES: [(char, char); 4] = [('\'', '\''), ('"', '"'), ('‘', '’'), ('“', '”')];
    for (i, c) in text.char_indices() {
        let Some(&(_, close)) = QUOTES.iter().find(|(open, _)| *open == c) else { continue };
        if i > 0 && !text[..i].ends_with(' ') {
            continue;
        }
        let inner = i + c.len_utf8();
        let Some(len) = text[inner..].find(close) else { continue };
        let inside = text[inner..inner + len].trim();
        let after = &text[inner + len + close.len_utf8()..];
        if inside.chars().count() >= 2 && is_noise(after) {
            return Some((squash(&text[..i]), inside.to_string()));
        }
    }
    None
}

fn split_dash(text: &str) -> Option<(String, String)> {
    [" - ", " – ", " — "].iter().find_map(|sep| {
        let (left, right) = text.split_once(sep)?;
        let (left, right) = (squash(left), squash(right));
        (!left.is_empty() && !right.is_empty()).then_some((left, right))
    })
}

/// Buang kata tambahan di ujung judul, misal "Sial Official Music Video" → "Sial" (angka tetap,
/// seperti "Song 2").
fn strip_trailing(text: &str) -> String {
    let mut words: Vec<&str> = text.split_whitespace().collect();
    let noise = |w: &str| NOISE.contains(&w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase().as_str());
    while words.len() > 1 && words.last().is_some_and(|w| noise(w)) {
        words.pop();
    }
    words.join(" ")
}

/// "Judul ft. Penyanyi Lain" → "Judul".
fn cut_featuring(text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let cut = [" ft. ", " ft ", " feat. ", " feat ", " featuring "].iter().filter_map(|m| lower.find(m)).min();
    squash(&text[..cut.unwrap_or(text.len())])
}

/// Teks yang hanya berisi tambahan video atau angka (misal "Official MV" atau "Remastered 2011").
fn is_noise(text: &str) -> bool {
    words(text).all(|w| NOISE.contains(&w.as_str()) || w.chars().all(|c| c.is_ascii_digit()))
}

fn words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase)
}

/// Dua nama dianggap sama kalau huruf dan angkanya sama, atau yang satu memuat yang lain.
fn same(a: &str, b: &str) -> bool {
    let norm = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect::<String>();
    let (a, b) = (norm(a), norm(b));
    !a.is_empty() && !b.is_empty() && (a.contains(&b) || b.contains(&a))
}

/// Spasi berlebih dan pemisah yang tertinggal di ujung dibuang.
fn squash(text: &str) -> String {
    let joined = text.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.trim_matches(|c: char| c.is_whitespace() || matches!(c, '-' | '–' | '—' | '|' | '/' | ',' | ':')).to_string()
}

fn synced(object: &JsonObject) -> Option<String> {
    let value = object.GetNamedValue(&HSTRING::from("syncedLyrics")).ok()?;
    if value.ValueType().ok()? != JsonValueType::String {
        return None;
    }
    Some(value.GetString().ok()?.to_string()).filter(|s| !s.trim().is_empty())
}

/// Format LRC: "[01:23.45]teks". Satu baris bisa punya beberapa waktu; tag lain diabaikan.
fn parse(lrc: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    for raw in lrc.lines() {
        let mut rest = raw.trim();
        let mut times = Vec::new();
        while let Some(tag) = rest.strip_prefix('[').and_then(|r| r.split_once(']')) {
            match timestamp(tag.0) {
                Some(at) => times.push(at),
                None => break,
            }
            rest = tag.1;
        }
        let text = rest.trim().to_string();
        lines.extend(times.into_iter().map(|at| Line { at, text: text.clone() }));
    }
    lines.sort_by(|a, b| a.at.total_cmp(&b.at));
    lines
}

fn timestamp(tag: &str) -> Option<f64> {
    let (minutes, seconds) = tag.split_once(':')?;
    Some(minutes.trim().parse::<f64>().ok()? * 60.0 + seconds.trim().parse::<f64>().ok()?)
}

/// Percent-encoding untuk parameter URL.
pub fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lrc_with_repeated_times_and_tags() {
        let lyrics = Lyrics {
            key: String::new(),
            lines: parse("[ar:Penyanyi]\n[00:12.50]Baris satu\n[00:20.00][01:05.00]Reff\n[00:30.10]\n"),
        };
        let times: Vec<f64> = lyrics.lines.iter().map(|l| l.at).collect();
        assert_eq!(times, vec![12.5, 20.0, 30.1, 65.0]);
        assert_eq!(lyrics.current(5.0), None);
        assert_eq!(lyrics.text(lyrics.current(12.4).unwrap()), "Baris satu");
        assert_eq!(lyrics.text(lyrics.current(31.0).unwrap()), "♪");
        assert_eq!(lyrics.text(lyrics.current(70.0).unwrap()), "Reff");
    }

    /// Uji sungguhan ke LRCLIB dengan lagu umum (butuh internet).
    /// Jalankan dengan `cargo test fetches_from_lrclib -- --ignored`.
    #[test]
    #[ignore]
    fn fetches_from_lrclib() {
        let request = Request {
            key: String::new(),
            title: "Bohemian Rhapsody".into(),
            artist: "Queen".into(),
            album: "A Night at the Opera".into(),
            duration: 354.0,
        };
        let lrc = fetch(&request).expect("LRCLIB bisa dihubungi").expect("lirik tersinkron ada");
        assert!(parse(&lrc).len() > 20);
    }

    #[test]
    fn tidies_youtube_titles() {
        let cases = [
            (("TULUS - Jatuh Suka (Official Lyric Video)", "Tulus"), ("Jatuh Suka", "TULUS")),
            (("Jatuh Suka", "Tulus - Topic"), ("Jatuh Suka", "Tulus")),
            (("Mahalini - Sial (Official Music Video)", "MahaliniVEVO"), ("Sial", "Mahalini")),
            (("Sial | Lirik Lagu", "Mahalini Official"), ("Sial", "Mahalini")),
            (("Hindia - Evaluasi [Official Audio]", "Hindia"), ("Evaluasi", "Hindia")),
            (("Bohemian Rhapsody - Remastered 2011", "Queen"), ("Bohemian Rhapsody", "Queen")),
            (("Kangen Band - Doy (Lirik)", "Lirik Lagu Indonesia"), ("Doy", "Kangen Band")),
            (("BTS (방탄소년단) 'Dynamite' Official MV", "HYBE LABELS"), ("Dynamite", "BTS")),
            (("Song 2 (Official Video)", "Blur"), ("Song 2", "Blur")),
            (("Don't Stop Me Now", "Queen"), ("Don't Stop Me Now", "Queen")),
            (("Perfect (feat. Someone) [Official Video]", "Ed Sheeran"), ("Perfect", "Ed Sheeran")),
            (("Ed Sheeran ft. Someone - Perfect Official Music Video", "Ed Sheeran"), ("Perfect", "Ed Sheeran")),
            (("Laskar Pelangi (Live) - Nidji", "Nidji"), ("Laskar Pelangi (Live)", "Nidji")),
            (("【MV】Yoru ni Kakeru", "YOASOBI"), ("Yoru ni Kakeru", "YOASOBI")),
            (("Video Killed the Radio Star", "The Buggles"), ("Video Killed the Radio Star", "The Buggles")),
        ];
        for ((title, artist), (want_title, want_artist)) in cases {
            assert_eq!(tidy(title, artist), (want_title.to_string(), want_artist.to_string()), "{title} / {artist}");
        }
    }

    /// Judul video YouTube apa adanya tetap ditemukan liriknya (butuh internet).
    /// Jalankan dengan `cargo test finds_youtube_titles -- --ignored`.
    #[test]
    #[ignore]
    fn finds_youtube_titles() {
        for (title, artist, duration) in [
            ("TULUS - Jatuh Suka (Official Lyric Video)", "Tulus", 238.0),
            ("Queen – Bohemian Rhapsody (Official Video Remastered)", "Queen Official", 359.0),
            ("Jatuh Suka", "Tulus - Topic", 0.0),
        ] {
            let request = Request { key: String::new(), title: title.into(), artist: artist.into(), album: String::new(), duration };
            let lrc = fetch(&request).expect("LRCLIB bisa dihubungi");
            assert!(lrc.is_some_and(|lrc| parse(&lrc).len() > 10), "lirik {title} tidak ketemu");
        }
    }

    #[test]
    fn encodes_url_parameters() {
        assert_eq!(encode("Kangen Band"), "Kangen%20Band");
        assert_eq!(encode("Tak & Ku"), "Tak%20%26%20Ku");
        assert_eq!(encode("é"), "%C3%A9");
    }
}
