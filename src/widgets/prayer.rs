//! Jadwal sholat, dihitung sendiri dari koordinat kota cuaca tanpa internet. Kriterianya
//! mengikuti Kementerian Agama RI: Subuh saat matahari 20° di bawah ufuk, Isya 18°, Ashar saat
//! bayangan benda sama panjang dengan bendanya ditambah bayangan tengah hari, dan ihtiyat
//! (pengaman) 2 menit. Imsak 10 menit sebelum Subuh. Rumus posisi matahari dari praytimes.org.

use super::agenda::Date;

/// Waktu di halaman Sholat, berurutan dalam sehari.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Time {
    Imsak,
    Subuh,
    Terbit,
    Dzuhur,
    Ashar,
    Maghrib,
    Isya,
}

impl Time {
    pub const ALL: [Time; 7] = [Time::Imsak, Time::Subuh, Time::Terbit, Time::Dzuhur, Time::Ashar, Time::Maghrib, Time::Isya];
    /// Lima waktu sholat: yang punya pengingat dan koreksi menit.
    pub const PRAYERS: [Time; 5] = [Time::Subuh, Time::Dzuhur, Time::Ashar, Time::Maghrib, Time::Isya];

    pub fn label(self) -> &'static str {
        match self {
            Time::Imsak => "Imsak",
            Time::Subuh => "Subuh",
            Time::Terbit => "Terbit",
            Time::Dzuhur => "Dzuhur",
            Time::Ashar => "Ashar",
            Time::Maghrib => "Maghrib",
            Time::Isya => "Isya",
        }
    }
}

/// Koreksi menit untuk Subuh, Dzuhur, Ashar, Maghrib, dan Isya, misal supaya sama dengan
/// jadwal masjid setempat.
pub type Offsets = [i8; 5];

/// Pilihan koreksi di Pengaturan.
pub const OFFSET_RANGE: std::ops::RangeInclusive<i8> = -3..=3;

const FAJR_ANGLE: f64 = 20.0;
const ISHA_ANGLE: f64 = 18.0;
/// Terbit dan terbenam: tepi atas matahari menyentuh ufuk, termasuk pembiasan udara.
const RISE_SET_ANGLE: f64 = 1.0;
const IHTIYAT: f64 = 2.0;
/// Dzuhur dimulai saat seluruh piringan matahari sudah lewat meridian, sekitar 1 menit
/// sesudah tengah hari.
const DHUHR_AFTER_NOON: f64 = 1.0;
const IMSAK_BEFORE: i32 = 10;

/// Jadwal satu hari, dalam menit sejak tengah malam waktu laptop (urutan [`Time::ALL`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Schedule {
    pub minutes: [i32; 7],
}

impl Schedule {
    pub fn at(&self, time: Time) -> i32 {
        self.minutes[Time::ALL.iter().position(|t| *t == time).unwrap_or(0)]
    }

    /// Waktu sholat yang jatuh tepat pada menit ini.
    pub fn due(&self, minute: i32) -> Option<Time> {
        Time::PRAYERS.into_iter().find(|t| self.at(*t) == minute)
    }

    /// Waktu sholat berikutnya sesudah menit ini (hari ini saja).
    pub fn next(&self, minute: i32) -> Option<Time> {
        Time::PRAYERS.into_iter().find(|t| self.at(*t) > minute)
    }

    /// Waktu sholat yang sedang berlangsung: Subuh sampai terbit, lalu Dzuhur sampai Isya
    /// berikutnya. Antara terbit dan Dzuhur tidak ada.
    pub fn current(&self, minute: i32) -> Option<Time> {
        let started = Time::PRAYERS.into_iter().rev().find(|t| self.at(*t) <= minute)?;
        (started != Time::Subuh || minute < self.at(Time::Terbit)).then_some(started)
    }

    /// Suasana langit pada menit ini, untuk warna halaman Sholat.
    pub fn sky(&self, minute: i32) -> Sky {
        let before = |time| minute < self.at(time);
        if before(Time::Subuh) || !before(Time::Isya) {
            Sky::Night
        } else if before(Time::Terbit) {
            Sky::Dawn
        } else if before(Time::Dzuhur) {
            Sky::Morning
        } else if before(Time::Ashar) {
            Sky::Noon
        } else if before(Time::Maghrib) {
            Sky::Afternoon
        } else {
            Sky::Dusk
        }
    }
}

/// Langit di halaman Sholat, mengikuti waktu yang sedang berjalan.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sky {
    /// Subuh sampai terbit.
    Dawn,
    /// Terbit sampai Dzuhur.
    Morning,
    /// Dzuhur sampai Ashar.
    Noon,
    /// Ashar sampai Maghrib.
    Afternoon,
    /// Maghrib sampai Isya.
    Dusk,
    /// Isya sampai Subuh.
    Night,
}

impl Sky {
    /// Matahari sudah terbit dan belum terbenam.
    pub fn daylight(self) -> bool {
        matches!(self, Sky::Morning | Sky::Noon | Sky::Afternoon)
    }
}

/// Jadwal hari ini dan besok untuk kota yang dipilih. Dihitung ulang hanya kalau tanggal,
/// kota, zona waktu laptop, atau koreksinya berubah.
#[derive(Default)]
pub struct Days {
    key: Option<(Date, i64, i64, i32, Offsets)>,
    pub today: Option<Schedule>,
    pub tomorrow: Option<Schedule>,
}

impl Days {
    pub fn update(&mut self, date: Date, lat: f64, lon: f64, utc_offset_minutes: i32, offsets: Offsets) {
        let key = (date, (lat * 1e4).round() as i64, (lon * 1e4).round() as i64, utc_offset_minutes, offsets);
        if self.key == Some(key) {
            return;
        }
        let hours = utc_offset_minutes as f64 / 60.0;
        self.today = schedule(date, lat, lon, hours, offsets);
        self.tomorrow = schedule(date.add_days(1), lat, lon, hours, offsets);
        self.key = Some(key);
    }
}

/// Sisa waktu, misal "1 jam 12 menit" atau "8 menit".
pub fn left(minutes: i32) -> String {
    match (minutes / 60, minutes % 60) {
        (0, m) => format!("{m} menit"),
        (h, 0) => format!("{h} jam"),
        (h, m) => format!("{h} jam {m} menit"),
    }
}

/// "04:31"
pub fn clock(minutes: i32) -> String {
    let m = minutes.rem_euclid(24 * 60);
    format!("{:02}:{:02}", m / 60, m % 60)
}

/// Jadwal di koordinat `lat`/`lon` pada `date`, untuk jam laptop yang `utc_offset` jam dari
/// UTC. `None` di lintang ekstrem tempat matahari tidak cukup turun (misal Subuh di musim
/// panas dekat kutub).
pub fn schedule(date: Date, lat: f64, lon: f64, utc_offset: f64, offsets: Offsets) -> Option<Schedule> {
    let [fajr, sunrise, dhuhr, asr, sunset, isha] = solar_times(date, lat, lon)?.map(|t| (t + utc_offset - lon / 15.0) * 60.0);
    // Ihtiyat ditambahkan lalu dibulatkan ke atas, kecuali terbit (dikurangi, dibulatkan ke bawah),
    // supaya waktu sholat tidak pernah terlalu awal.
    let prayer = |t: f64, i: usize| (t + IHTIYAT + offsets[i] as f64).ceil() as i32;
    let subuh = prayer(fajr, 0);
    Some(Schedule {
        minutes: [
            subuh - IMSAK_BEFORE,
            subuh,
            (sunrise - IHTIYAT).floor() as i32,
            prayer(dhuhr + DHUHR_AFTER_NOON, 1),
            prayer(asr, 2),
            prayer(sunset, 3),
            prayer(isha, 4),
        ],
    })
}

/// Subuh, terbit, Dzuhur, Ashar, terbenam, dan Isya dalam jam waktu matahari setempat (belum
/// digeser ke zona waktu), tanpa ihtiyat.
fn solar_times(date: Date, lat: f64, lon: f64) -> Option<[f64; 6]> {
    let jd = date.days() as f64 + 2_440_587.5 - lon / (15.0 * 24.0);
    let sun = |hours: f64| sun_position(jd + hours / 24.0);
    let mid_day = |hours: f64| fix_hour(12.0 - sun(hours).1);
    // Jam saat matahari `angle` derajat di bawah ufuk, sebelum (`morning`) atau sesudah tengah hari.
    let angle_time = |angle: f64, hours: f64, morning: bool| {
        let decl = sun(hours).0;
        let cos = (-dsin(angle) - dsin(decl) * dsin(lat)) / (dcos(decl) * dcos(lat));
        let half = cos.acos().to_degrees() / 15.0;
        mid_day(hours) + if morning { -half } else { half }
    };
    let asr_time = |hours: f64| {
        let decl = sun(hours).0;
        let altitude = (1.0 / (1.0 + dtan((lat - decl).abs()))).atan().to_degrees();
        angle_time(-altitude, hours, false)
    };
    // Tebakan awal, lalu dihitung ulang dengan posisi matahari pada jam tebakan itu.
    let mut times = [5.0, 6.0, 12.0, 13.0, 18.0, 18.0];
    for _ in 0..2 {
        times = [
            angle_time(FAJR_ANGLE, times[0], true),
            angle_time(RISE_SET_ANGLE, times[1], true),
            mid_day(times[2]),
            asr_time(times[3]),
            angle_time(RISE_SET_ANGLE, times[4], false),
            angle_time(ISHA_ANGLE, times[5], false),
        ];
    }
    times.iter().all(|t| t.is_finite()).then_some(times)
}

/// Deklinasi matahari (derajat) dan persamaan waktu (jam) pada tanggal Julian `jd`.
fn sun_position(jd: f64) -> (f64, f64) {
    let d = jd - 2_451_545.0;
    let g = fix_angle(357.529 + 0.985_600_28 * d);
    let q = fix_angle(280.459 + 0.985_647_36 * d);
    let l = fix_angle(q + 1.915 * dsin(g) + 0.020 * dsin(2.0 * g));
    let e = 23.439 - 0.000_000_36 * d;
    let ra = (dcos(e) * dsin(l)).atan2(dcos(l)).to_degrees() / 15.0;
    let decl = (dsin(e) * dsin(l)).asin().to_degrees();
    (decl, q / 15.0 - fix_hour(ra))
}

fn dsin(deg: f64) -> f64 {
    deg.to_radians().sin()
}

fn dcos(deg: f64) -> f64 {
    deg.to_radians().cos()
}

fn dtan(deg: f64) -> f64 {
    deg.to_radians().tan()
}

fn fix_angle(a: f64) -> f64 {
    a.rem_euclid(360.0)
}

fn fix_hour(h: f64) -> f64 {
    h.rem_euclid(24.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_and_next() {
        let s = Schedule { minutes: [270, 280, 345, 720, 915, 1085, 1160] };
        assert_eq!(s.next(0), Some(Time::Subuh));
        assert_eq!(s.current(0), None);
        assert_eq!(s.current(280), Some(Time::Subuh));
        assert_eq!(s.current(400), None, "sesudah terbit belum ada waktu sholat");
        assert_eq!((s.current(800), s.next(800)), (Some(Time::Dzuhur), Some(Time::Ashar)));
        assert_eq!((s.current(1200), s.next(1200)), (Some(Time::Isya), None));
        assert_eq!(s.due(915), Some(Time::Ashar));
        assert_eq!(s.due(345), None, "terbit bukan waktu sholat");
        let skies: Vec<Sky> = [0, 275, 280, 345, 720, 915, 1085, 1160, 1439].iter().map(|m| s.sky(*m)).collect();
        assert_eq!(skies, [Sky::Night, Sky::Night, Sky::Dawn, Sky::Morning, Sky::Noon, Sky::Afternoon, Sky::Dusk, Sky::Night, Sky::Night]);
        assert_eq!(clock(915), "15:15");
        assert_eq!((left(8), left(60), left(72)), ("8 menit".into(), "1 jam".into(), "1 jam 12 menit".into()));
    }

    #[test]
    fn offsets_shift_only_their_prayer() {
        let date = Date::new(2026, 11, 1);
        let base = schedule(date, -6.2146, 106.8451, 7.0, [0; 5]).unwrap();
        let moved = schedule(date, -6.2146, 106.8451, 7.0, [2, 0, -1, 0, 3]).unwrap();
        let diff: Vec<i32> = (0..7).map(|i| moved.minutes[i] - base.minutes[i]).collect();
        assert_eq!(diff, [2, 2, 0, 0, -1, 0, 3], "imsak ikut subuh, terbit tetap");
    }

    #[test]
    fn polar_summer_has_no_schedule() {
        assert!(schedule(Date::new(2026, 6, 21), 70.0, 25.0, 2.0, [0; 5]).is_none());
    }

    /// Jadwal resmi Kemenag (Bimas Islam, lewat api.myquran.com) untuk Subuh, Terbit, Dzuhur,
    /// Ashar, Maghrib, dan Isya, dengan koordinat kota dari Open-Meteo seperti di notch. Selisih
    /// paling banyak 1 menit (titik koordinat kota Kemenag sedikit berbeda), dan sebagian besar
    /// harus sama persis.
    #[test]
    fn matches_kemenag_schedule() {
        let references = [
            (-6.21462, 106.84513, 7.0, (2026, 1, 15), "04:27 05:45 12:05 15:30 18:18 19:33"),
            (-6.21462, 106.84513, 7.0, (2026, 4, 1), "04:40 05:52 12:00 15:14 18:01 19:10"),
            (-6.21462, 106.84513, 7.0, (2026, 7, 1), "04:42 06:00 12:00 15:21 17:53 19:07"),
            (-6.21462, 106.84513, 7.0, (2026, 11, 1), "04:09 05:23 11:40 14:55 17:49 19:00"),
            (-5.14861, 119.43194, 8.0, (2026, 1, 15), "04:39 05:57 12:15 15:39 18:26 19:40"),
            (-5.14861, 119.43194, 8.0, (2026, 4, 1), "04:50 06:01 12:10 15:23 18:11 19:19"),
            (-5.14861, 119.43194, 8.0, (2026, 7, 1), "04:50 06:08 12:10 15:31 18:04 19:18"),
            (-5.14861, 119.43194, 8.0, (2026, 11, 1), "04:20 05:34 11:49 15:05 17:58 19:09"),
            (-2.9925, 120.19694, 8.0, (2026, 4, 1), "04:46 05:58 12:07 15:17 18:09 19:17"),
            (-2.9925, 120.19694, 8.0, (2026, 7, 1), "04:44 06:01 12:07 15:30 18:05 19:19"),
            (-2.9925, 120.19694, 8.0, (2026, 11, 1), "04:19 05:33 11:46 15:04 17:53 19:03"),
            (3.58333, 98.66667, 7.0, (2026, 4, 1), "05:10 06:22 12:33 15:34 18:37 19:45"),
            (3.58333, 98.66667, 7.0, (2026, 7, 1), "04:57 06:16 12:33 15:58 18:42 19:57"),
            (3.58333, 98.66667, 7.0, (2026, 11, 1), "04:52 06:06 12:12 15:32 18:12 19:22"),
        ];
        let mut exact = 0;
        for (lat, lon, tz, (y, m, d), expected) in references {
            let ours = schedule(Date::new(y, m, d), lat, lon, tz, [0; 5]).unwrap();
            for (ours, expected) in ours.minutes[1..].iter().zip(expected.split(' ')) {
                let (h, mm) = expected.split_once(':').unwrap();
                let expected = h.parse::<i32>().unwrap() * 60 + mm.parse::<i32>().unwrap();
                assert!((ours - expected).abs() <= 1, "{y}-{m}-{d} {lat},{lon}: {} vs {}", clock(*ours), clock(expected));
                exact += (*ours == expected) as usize;
            }
        }
        assert!(exact >= 80, "hanya {exact} dari 84 yang sama persis");
    }
}