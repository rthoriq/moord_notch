//! Lirik lagu tersinkron dari LRCLIB (lrclib.net), layanan lirik gratis tanpa akun.
//! Opsional dan mati secara default. Yang dikirim hanya judul, artis, album, dan durasi lagu
//! yang sedang diputar, lewat HTTPS. Hasilnya hanya disimpan di memori.

use std::thread;

use windows::Data::Json::{JsonArray, JsonObject, JsonValueType};
use windows::Foundation::Uri;
use windows::Web::Http::HttpClient;
use windows::core::{HSTRING, Result};

use crate::services::{Event, Notifier};

const API: &str = "https://lrclib.net/api";
/// LRCLIB meminta setiap aplikasi memperkenalkan diri.
const USER_AGENT: &str = "NotchWindows/0.4 (aplikasi notch pribadi)";
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

    // Kalau tidak cocok persis, cari dengan judul dan artis saja.
    let url = format!("{API}/search?track_name={}&artist_name={}", encode(&request.title), encode(&request.artist));
    let results = JsonArray::Parse(&get(url)?)?;
    for i in 0..results.Size()? {
        if let Some(lrc) = results.GetAt(i)?.GetObject().ok().and_then(|o| synced(&o)) {
            return Ok(Some(lrc));
        }
    }
    Ok(None)
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
    fn encodes_url_parameters() {
        assert_eq!(encode("Kangen Band"), "Kangen%20Band");
        assert_eq!(encode("Tak & Ku"), "Tak%20%26%20Ku");
        assert_eq!(encode("é"), "%C3%A9");
    }
}
