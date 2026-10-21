//! Musik/video yang sedang diputar di PC, lewat kontrol media bawaan Windows (SMTC).
//! Bekerja untuk Spotify, YouTube di browser, Media Player, dan aplikasi lain yang
//! muncul di kontrol media Windows. Semuanya lokal, tanpa internet.

use std::sync::Arc;
use std::sync::mpsc::Sender;
use std::time::Instant;

use windows::Foundation::TypedEventHandler;
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapBounds, BitmapDecoder, BitmapInterpolationMode, BitmapPixelFormat,
    BitmapTransform, ColorManagementMode, ExifOrientationMode,
};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::IRandomAccessStreamReference;
use windows::Win32::Foundation::E_FAIL;
use windows::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows::core::{Result, RuntimeType};

use crate::services::{Event, Notifier, Task};

/// Judul, artis, atau sampul berubah.
pub const PROPS: u8 = 1;
/// Status putar/jeda atau tombol yang tersedia berubah.
pub const PLAYBACK: u8 = 2;
/// Posisi atau durasi berubah.
pub const TIMELINE: u8 = 4;
pub const ALL: u8 = PROPS | PLAYBACK | TIMELINE;

/// Sisi sampul album dalam piksel; cukup tajam untuk 64 DIP di skala 200%.
pub const ART_PX: u32 = 128;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    TogglePlayPause,
    Next,
    Previous,
    /// Lompat ke posisi lagu, dalam satuan 100 nanodetik pada linimasa sesi.
    Seek(i64),
}

/// Sampul album: piksel BGRA premultiplied berukuran `ART_PX` x `ART_PX`.
pub struct Art {
    pub id: u64,
    pub pixels: Vec<u8>,
    /// Warna dominan sampul (0xRRGGBB), untuk mewarnai gelombang dan progres.
    pub accent: Option<u32>,
}

#[derive(Clone)]
pub struct Snapshot {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    /// Posisi (detik) pada saat `at`, dihitung dari awal linimasa (`start`).
    pub position: f64,
    /// Awal linimasa sesi (detik); posisi untuk lompat dihitung dari sini.
    pub start: f64,
    pub duration: f64,
    pub rate: f64,
    pub at: Instant,
    pub can_previous: bool,
    pub can_next: bool,
    /// Aplikasinya mengizinkan lompat ke posisi lain (bar progres bisa diseret).
    pub can_seek: bool,
    pub art: Option<Arc<Art>>,
}

impl Snapshot {
    /// Posisi sekarang, diperkirakan dari posisi terakhir yang dilaporkan aplikasi.
    pub fn position_now(&self) -> f64 {
        let position = if self.playing {
            self.position + self.at.elapsed().as_secs_f64() * self.rate
        } else {
            self.position
        };
        if self.duration > 0.0 { position.clamp(0.0, self.duration) } else { position.max(0.0) }
    }
}

pub struct Monitor {
    manager: Manager,
    session: Option<(Session, [i64; 3])>,
    tasks: Sender<Task>,
    notify: Notifier,
    last: Option<Snapshot>,
    next_art_id: u64,
}

impl Monitor {
    pub fn new(tasks: Sender<Task>, notify: Notifier) -> Result<Self> {
        let manager = Manager::RequestAsync()?.join()?;
        let on_change = tasks.clone();
        manager.CurrentSessionChanged(&TypedEventHandler::new(move |_, _| {
            let _ = on_change.send(Task::MediaSessionChanged);
            Ok(())
        }))?;
        let mut monitor = Self { manager, session: None, tasks, notify, last: None, next_art_id: 1 };
        monitor.follow_session();
        Ok(monitor)
    }

    /// Pindah mendengarkan sesi yang sekarang dianggap Windows sebagai sesi aktif.
    pub fn follow_session(&mut self) {
        self.detach();
        self.last = None;
        let Ok(session) = self.manager.GetCurrentSession() else { return };
        let tokens = [
            session.MediaPropertiesChanged(&changed(&self.tasks, PROPS)).unwrap_or_default(),
            session.PlaybackInfoChanged(&changed(&self.tasks, PLAYBACK)).unwrap_or_default(),
            session.TimelinePropertiesChanged(&changed(&self.tasks, TIMELINE)).unwrap_or_default(),
        ];
        self.session = Some((session, tokens));
    }

    /// Baca ulang bagian yang berubah lalu kirim ke jendela notch.
    pub fn refresh(&mut self, flags: u8) {
        let snapshot = match &self.session {
            Some((session, _)) => match read(session, flags, self.last.as_ref(), &mut self.next_art_id) {
                Ok(snapshot) => Some(snapshot),
                Err(e) => {
                    debug_log!("gagal membaca media: {e}");
                    None
                }
            },
            None => None,
        };
        // Sesi tanpa judul yang tidak sedang diputar tidak ada gunanya ditampilkan.
        let snapshot = snapshot.filter(|s| s.playing || !s.title.is_empty());
        self.last = snapshot.clone();
        self.notify.send(Event::Media(snapshot));
    }

    pub fn command(&self, command: Command) {
        let Some((session, _)) = &self.session else { return };
        let operation = match command {
            Command::TogglePlayPause => session.TryTogglePlayPauseAsync(),
            Command::Next => session.TrySkipNextAsync(),
            Command::Previous => session.TrySkipPreviousAsync(),
            Command::Seek(ticks) => session.TryChangePlaybackPositionAsync(ticks),
        };
        if let Err(e) = operation.and_then(|op| op.join()) {
            debug_log!("perintah media {command:?} gagal: {e}");
        }
    }

    fn detach(&mut self) {
        if let Some((old, [props, playback, timeline])) = self.session.take() {
            let _ = old.RemoveMediaPropertiesChanged(props);
            let _ = old.RemovePlaybackInfoChanged(playback);
            let _ = old.RemoveTimelinePropertiesChanged(timeline);
        }
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.detach();
    }
}

/// Handler event sesi yang hanya meneruskan "ada yang berubah" ke thread layanan.
fn changed<A: RuntimeType + 'static>(tasks: &Sender<Task>, flags: u8) -> TypedEventHandler<Session, A> {
    let tasks = tasks.clone();
    TypedEventHandler::new(move |_, _| {
        let _ = tasks.send(Task::MediaChanged(flags));
        Ok(())
    })
}

fn read(session: &Session, flags: u8, last: Option<&Snapshot>, next_art_id: &mut u64) -> Result<Snapshot> {
    // Judul dan sampul hanya dibaca ulang kalau memang berubah.
    let (title, artist, album, art) = match last.filter(|_| flags & PROPS == 0) {
        Some(last) => (last.title.clone(), last.artist.clone(), last.album.clone(), last.art.clone()),
        None => {
            let props = session.TryGetMediaPropertiesAsync()?.join()?;
            let art = props.Thumbnail().ok().and_then(|thumbnail| {
                *next_art_id += 1;
                decode_art(&thumbnail, *next_art_id)
                    .inspect_err(|e| debug_log!("sampul tidak terbaca: {e}"))
                    .ok()
            });
            let album = props.AlbumTitle().map(|a| a.to_string()).unwrap_or_default();
            (props.Title()?.to_string(), props.Artist()?.to_string(), album, art.map(Arc::new))
        }
    };

    let info = session.GetPlaybackInfo()?;
    let controls = info.Controls()?;
    let playing = info.PlaybackStatus()? == Status::Playing;
    let rate = info.PlaybackRate().and_then(|r| r.Value()).unwrap_or(1.0);

    let timeline = session.GetTimelineProperties()?;
    let start = timeline.StartTime()?.Duration as f64 / 1e7;
    let duration = (timeline.EndTime()?.Duration as f64 / 1e7 - start).max(0.0);
    let mut position = timeline.Position()?.Duration as f64 / 1e7 - start;
    if playing {
        // Posisi dilaporkan pada suatu waktu di masa lalu; geser ke saat ini.
        let updated = timeline.LastUpdatedTime()?.UniversalTime;
        let now = filetime_now();
        if updated > 0 && now > updated {
            position += (now - updated) as f64 / 1e7 * rate;
        }
    }
    let position = if duration > 0.0 { position.clamp(0.0, duration) } else { 0.0 };

    Ok(Snapshot {
        title,
        artist,
        album,
        playing,
        position,
        start,
        duration,
        rate,
        at: Instant::now(),
        can_previous: controls.IsPreviousEnabled()?,
        can_next: controls.IsNextEnabled()?,
        can_seek: controls.IsPlaybackPositionEnabled().unwrap_or(false),
        art,
    })
}

fn decode_art(reference: &IRandomAccessStreamReference, id: u64) -> Result<Art> {
    let pixels = decode_square(reference, ART_PX)?;
    Ok(Art { id, accent: accent(&pixels), pixels })
}

/// Potong bagian tengah gambar jadi persegi lalu kecilkan ke `size` piksel. Hasilnya BGRA
/// premultiplied, siap dipakai Direct2D. Dipakai untuk sampul album dan logo aplikasi.
pub fn decode_square(reference: &IRandomAccessStreamReference, size: u32) -> Result<Vec<u8>> {
    let stream = reference.OpenReadAsync()?.join()?;
    let decoder = BitmapDecoder::CreateAsync(&stream)?.join()?;
    let (width, height) = (decoder.PixelWidth()?.max(1), decoder.PixelHeight()?.max(1));
    let short = width.min(height);
    let scaled = (
        (width as u64 * size as u64 / short as u64) as u32,
        (height as u64 * size as u64 / short as u64) as u32,
    );
    let transform = BitmapTransform::new()?;
    transform.SetScaledWidth(scaled.0)?;
    transform.SetScaledHeight(scaled.1)?;
    transform.SetInterpolationMode(BitmapInterpolationMode::Fant)?;
    transform.SetBounds(BitmapBounds {
        X: (scaled.0 - size) / 2,
        Y: (scaled.1 - size) / 2,
        Width: size,
        Height: size,
    })?;
    let data = decoder
        .GetPixelDataTransformedAsync(
            BitmapPixelFormat::Bgra8,
            BitmapAlphaMode::Premultiplied,
            &transform,
            ExifOrientationMode::IgnoreExifOrientation,
            ColorManagementMode::DoNotColorManage,
        )?
        .join()?;
    let pixels = data.DetachPixelData()?.to_vec();
    if pixels.len() != (size * size * 4) as usize {
        return Err(E_FAIL.into());
    }
    Ok(pixels)
}

/// Warna dominan yang cukup berwarna; abu-abu/hitam/putih diabaikan.
fn accent(pixels: &[u8]) -> Option<u32> {
    let (mut r, mut g, mut b, mut weight, mut samples) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0u32);
    for px in pixels.chunks_exact(4).step_by(3) {
        samples += 1;
        let (pb, pg, pr) = (px[0] as f32, px[1] as f32, px[2] as f32);
        let max = pr.max(pg).max(pb);
        if max < 40.0 {
            continue;
        }
        let saturation = (max - pr.min(pg).min(pb)) / max;
        let w = saturation * saturation * (max / 255.0);
        r += pr * w;
        g += pg * w;
        b += pb * w;
        weight += w;
    }
    if samples == 0 || weight / (samples as f32) < 0.03 {
        return None;
    }
    let (r, g, b) = (r / weight, g / weight, b / weight);
    // Terangkan supaya tetap terlihat di atas notch hitam.
    let k = 235.0 / r.max(g).max(b).max(1.0);
    let channel = |c: f32| (c * k).min(255.0) as u32;
    Some((channel(r) << 16) | (channel(g) << 8) | channel(b))
}

fn filetime_now() -> i64 {
    let ft = unsafe { GetSystemTimeAsFileTime() };
    ((ft.dwHighDateTime as i64) << 32) | ft.dwLowDateTime as i64
}

/// Nama aplikasi yang enak dibaca dari AppUserModelID atau nama exe, misal "Spotify.exe" → "Spotify".
pub fn app_name(aumid: &str) -> String {
    const KNOWN: [(&str, &str); 11] = [
        ("spotify", "Spotify"),
        ("chrome", "Chrome"),
        ("msedge", "Edge"),
        ("firefox", "Firefox"),
        ("opera", "Opera"),
        ("brave", "Brave"),
        ("zunemusic", "Media Player"),
        ("zunevideo", "Film & TV"),
        ("applemusic", "Apple Music"),
        ("vlc", "VLC"),
        ("foobar", "foobar2000"),
    ];
    let lower = aumid.to_lowercase();
    if let Some((_, name)) = KNOWN.iter().find(|(key, _)| lower.contains(key)) {
        return name.to_string();
    }
    let base = aumid.rsplit(['\\', '/']).next().unwrap_or(aumid);
    let base = base.split('!').next().unwrap_or(base);
    let base = base.split('_').next().unwrap_or(base).trim_end_matches(".exe");
    base.rsplit('.').next().unwrap_or(base).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_app_names() {
        assert_eq!(app_name("Spotify.exe"), "Spotify");
        assert_eq!(app_name("chrome.exe"), "Chrome");
        assert_eq!(app_name("Microsoft.WindowsCamera_8wekyb3d8bbwe"), "WindowsCamera");
        assert_eq!(app_name("Discord.exe"), "Discord");
        assert_eq!(app_name("C:\\Program Files\\Zoom\\bin\\Zoom.exe"), "Zoom");
    }
}