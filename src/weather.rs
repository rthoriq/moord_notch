//! Cuaca (opsional, mati secara default) dari Open-Meteo (open-meteo.com), tanpa akun dan
//! tanpa API key. Yang dikirim hanya nama kota yang diketik pengguna (sekali, untuk mencari
//! koordinatnya) lalu koordinat pusat kota itu. Lokasi laptop tidak pernah dibaca.

use std::thread;

use windows::Data::Json::JsonObject;
use windows::core::{HSTRING, Result};

use crate::http;
use crate::lyrics::encode;
use crate::services::{Event, Notifier};
use crate::store;

const FILE: &str = "kota.txt";
const USER_AGENT: &str = "NotchWindows/0.7 (aplikasi notch pribadi)";

#[derive(Clone, Debug, PartialEq)]
pub struct City {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl City {
    /// Kota yang tersimpan, kalau pernah diatur.
    pub fn load() -> Option<City> {
        store::load(FILE).first().and_then(|line| City::parse(line))
    }

    pub fn save(&self) {
        store::save(FILE, [format!("{};{:.4};{:.4}", self.name.replace(';', ","), self.lat, self.lon)]);
    }

    fn parse(line: &str) -> Option<City> {
        let mut parts = line.split(';');
        let name = parts.next()?.trim().to_string();
        let lat = parts.next()?.trim().parse().ok()?;
        let lon = parts.next()?.trim().parse().ok()?;
        (!name.is_empty()).then_some(City { name, lat, lon })
    }
}

#[derive(Clone, Debug)]
pub struct Weather {
    /// °C
    pub temp: f64,
    /// Kode cuaca WMO.
    pub code: u32,
}

/// Keterangan singkat untuk kode cuaca WMO.
pub fn describe(code: u32) -> &'static str {
    match code {
        0 => "Cerah",
        1 => "Cerah berawan",
        2 => "Berawan",
        3 => "Mendung",
        45 | 48 => "Berkabut",
        51..=57 => "Gerimis",
        61..=67 | 80..=82 => "Hujan",
        71..=77 | 85 | 86 => "Salju",
        95..=99 => "Badai petir",
        _ => "",
    }
}

/// Cari koordinat kota di thread terpisah; hasilnya datang sebagai [`Event::City`].
pub fn find_city(name: String, notify: Notifier) {
    spawn("cari kota", notify, move || {
        let url = format!(
            "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=id&format=json",
            encode(&name)
        );
        let body = get(&url)?;
        Ok(Event::City(parse_city(&body)))
    });
}

/// Ambil cuaca saat ini di thread terpisah; hasilnya datang sebagai [`Event::Weather`].
pub fn fetch(city: &City, notify: Notifier) {
    let (lat, lon) = (city.lat, city.lon);
    spawn("cuaca", notify, move || {
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={lat:.2}&longitude={lon:.2}&current=temperature_2m,weather_code&timezone=auto"
        );
        Ok(Event::Weather(parse_weather(&get(&url)?)))
    });
}

fn spawn(name: &str, notify: Notifier, work: impl FnOnce() -> Result<Event> + Send + 'static) {
    let failed = match name {
        "cari kota" => Event::City(None),
        _ => Event::Weather(None),
    };
    let reply = notify.clone();
    let spawned = thread::Builder::new().name(name.into()).spawn(move || match work() {
        Ok(event) => reply.send(event),
        Err(e) => {
            error_log!("cuaca gagal diambil: {e}");
            reply.send(failed);
        }
    });
    if let Err(e) = spawned {
        error_log!("thread cuaca gagal dibuat: {e}");
    }
}

fn get(url: &str) -> Result<String> {
    http::get(url, USER_AGENT)
}

fn parse_city(body: &str) -> Option<City> {
    let root = JsonObject::Parse(&HSTRING::from(body)).ok()?;
    let first = root.GetNamedArray(&HSTRING::from("results")).ok()?.GetObjectAt(0).ok()?;
    let text = |key: &str| first.GetNamedString(&HSTRING::from(key)).map(|s| s.to_string()).unwrap_or_default();
    let number = |key: &str| first.GetNamedNumber(&HSTRING::from(key)).ok();
    Some(City { name: text("name"), lat: number("latitude")?, lon: number("longitude")? }).filter(|c| !c.name.is_empty())
}

fn parse_weather(body: &str) -> Option<Weather> {
    let root = JsonObject::Parse(&HSTRING::from(body)).ok()?;
    let current = root.GetNamedObject(&HSTRING::from("current")).ok()?;
    let temp = current.GetNamedNumber(&HSTRING::from("temperature_2m")).ok()?;
    let code = current.GetNamedNumber(&HSTRING::from("weather_code")).ok()? as u32;
    Some(Weather { temp, code })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_open_meteo_answers() {
        let city = parse_city(r#"{"results":[{"id":1642911,"name":"Jakarta","latitude":-6.21462,"longitude":106.84513,"country":"Indonesia"}]}"#).unwrap();
        assert_eq!(city.name, "Jakarta");
        assert!((city.lat + 6.21462).abs() < 1e-9);
        assert!(parse_city(r#"{"generationtime_ms":0.5}"#).is_none(), "kota tidak ditemukan");

        let weather = parse_weather(r#"{"current":{"time":"2026-10-19T19:00","interval":900,"temperature_2m":28.4,"weather_code":3}}"#).unwrap();
        assert_eq!((weather.temp, weather.code, describe(weather.code)), (28.4, 3, "Mendung"));
    }

    /// Permintaan sungguhan ke Open-Meteo. Jalankan dengan `cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn asks_open_meteo() {
        let body = get("https://geocoding-api.open-meteo.com/v1/search?name=Jakarta&count=1&language=id&format=json").unwrap();
        let city = parse_city(&body).expect("Jakarta ditemukan");
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={:.2}&longitude={:.2}&current=temperature_2m,weather_code&timezone=auto",
            city.lat, city.lon
        );
        let weather = parse_weather(&get(&url).unwrap()).expect("cuaca terbaca");
        println!("{} {:.4},{:.4}: {:.1}° {}", city.name, city.lat, city.lon, weather.temp, describe(weather.code));
        assert!((-60.0..60.0).contains(&weather.temp));
    }

    #[test]
    fn city_round_trip() {
        let city = City { name: "Yogyakarta".into(), lat: -7.80139, lon: 110.36472 };
        let line = format!("{};{:.4};{:.4}", city.name, city.lat, city.lon);
        assert_eq!(City::parse(&line).unwrap().name, "Yogyakarta");
        assert!(City::parse("tanpa koordinat").is_none());
    }
}
