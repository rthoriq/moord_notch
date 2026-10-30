//! Video promosi Story Instagram (1080×1920, 30 fps, tanpa suara) yang dirender langsung dari
//! kode: latar gelap dengan pita cahaya yang berpendar, notch asli dari renderer ini, dan teks
//! yang bergerak mengikuti tempo 120 BPM. Musik dan efek suaranya juga disintesis (`musik`).
//!
//! `cargo test video_cuplikan -- --ignored` menyimpan satu frame per adegan untuk dicek;
//! `cargo test video_musik -- --ignored` merender musiknya saja (WAV dan gambar spektrogram);
//! `cargo test video_promosi -- --ignored` merender videonya ke `aset-video/NotchWindows-story.mp4`.

mod musik;
mod reels;

use std::collections::HashMap;
use std::f32::consts::PI;
use std::path::Path;

use windows::Win32::Graphics::Direct2D::{
    D2D1_ANTIALIAS_MODE_PER_PRIMITIVE, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES, ID2D1Bitmap,
};
use windows::Win32::Media::MediaFoundation::{
    IMF2DBuffer, IMFSample, IMFSinkWriter, MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, MF_MT_AUDIO_AVG_BYTES_PER_SECOND,
    MF_MT_AUDIO_BITS_PER_SAMPLE, MF_MT_AUDIO_BLOCK_ALIGNMENT, MF_MT_AUDIO_NUM_CHANNELS, MF_MT_AUDIO_SAMPLES_PER_SECOND,
    MF_MT_AVG_BITRATE, MF_SOURCE_READER_ALL_STREAMS, MF_SOURCE_READER_FIRST_AUDIO_STREAM, MFAudioFormat_AAC,
    MFAudioFormat_PCM, MFMediaType_Audio, MF_MT_DEFAULT_STRIDE, MF_MT_FRAME_RATE, MF_MT_FRAME_SIZE,
    MF_MT_INTERLACE_MODE, MF_MT_MAJOR_TYPE, MF_MT_PIXEL_ASPECT_RATIO, MF_MT_SUBTYPE,
    MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, MF_SINK_WRITER_DISABLE_THROTTLING,
    MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, MF_SOURCE_READER_FIRST_VIDEO_STREAM,
    MF_SOURCE_READERF_ENDOFSTREAM, MF_VERSION, MFCreateAttributes, MFCreateMediaType,
    MFCreateMemoryBuffer, MFCreateSample, MFCreateSinkWriterFromURL, MFCreateSourceReaderFromURL,
    MFMediaType_Video, MFSTARTUP_FULL, MFShutdown, MFStartup, MFVideoFormat_H264,
    MFVideoFormat_RGB32, MFVideoInterlace_Progressive,
};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::core::{HSTRING, Interface};

use super::aset::{Motion, home, idle_focus, lyrics_at, save_png, song};
use super::*;

const W: i32 = 1080;
const H: i32 = 1920;
const FPS: f64 = 30.0;
/// 120 BPM: satu ketukan setengah detik.
const BEAT: f64 = 0.5;
const LENGTH: f64 = 28.0;
/// Notch dirender 4× supaya tetap tajam saat diperbesar.
const NOTCH_DPI: u32 = 384;

const CYAN: u32 = 0x4FC3F7;
const ELECTRIC: u32 = 0x0A84FF;
const MAGENTA: u32 = 0xC86BFA;
const SERIF: &str = "Sitka Banner";
const SANS: &str = "Segoe UI Variable Display";
/// Font ikon bawaan Windows 11, sama dengan yang dipakai notch.
const ICONS: &str = "Segoe Fluent Icons";

// ---------------------------------------------------------------- waktu dan kurva

/// Kemajuan 0..1 dari `start` selama `len` detik.
fn span(t: f64, start: f64, len: f64) -> f32 {
    (((t - start) / len) as f32).clamp(0.0, 1.0)
}

fn ease_out(x: f32) -> f32 {
    1.0 - (1.0 - x).powi(3)
}

fn ease_in_out(x: f32) -> f32 {
    if x < 0.5 { 4.0 * x * x * x } else { 1.0 - (-2.0 * x + 2.0).powi(3) / 2.0 }
}

/// Melewati sedikit lalu kembali, seperti pegas.
fn back_out(x: f32) -> f32 {
    let c = 1.9;
    1.0 + (c + 1.0) * (x - 1.0).powi(3) + c * (x - 1.0).powi(2)
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Denyut di setiap ketukan: 1 tepat di ketukan, meluruh cepat.
fn pulse(t: f64) -> f32 {
    let phase = (t % BEAT) / BEAT;
    (-(phase as f32) * 7.0).exp()
}

/// Kedipan mata: tertutup di tengah, terbuka di awal dan akhir (seperti di app).
fn blink(t: f64, at: f64) -> f32 {
    let x = span(t, at, 0.16);
    if x <= 0.0 || x >= 1.0 { 1.0 } else { 1.0 - (x * PI).sin() }
}

// ---------------------------------------------------------------- studio gambar

struct Studio {
    factory: ID2D1Factory,
    dwrite: IDWriteFactory,
    target: ID2D1DCRenderTarget,
    surface: Surface,
    brush: ID2D1SolidColorBrush,
    round: ID2D1StrokeStyle,
    formats: HashMap<(&'static str, u32, bool, i32), IDWriteTextFormat>,
    renderers: Vec<Renderer>,
}

impl Studio {
    fn new() -> Self {
        unsafe {
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).unwrap();
            let target = dc_target(&factory, 96.0).unwrap();
            let surface = Surface::new(W, H).unwrap();
            let brush = target.CreateSolidColorBrush(&WHITE, None).unwrap();
            let props = D2D1_STROKE_STYLE_PROPERTIES {
                startCap: D2D1_CAP_STYLE_ROUND,
                endCap: D2D1_CAP_STYLE_ROUND,
                dashCap: D2D1_CAP_STYLE_ROUND,
                lineJoin: D2D1_LINE_JOIN_ROUND,
                miterLimit: 10.0,
                dashStyle: D2D1_DASH_STYLE_SOLID,
                dashOffset: 0.0,
            };
            let round = factory.CreateStrokeStyle(&props, None).unwrap();
            let dwrite: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).unwrap();
            let renderers = Theme::ALL
                .iter()
                .map(|theme| {
                    let mut renderer = Renderer::new(NOTCH_DPI, *theme).unwrap();
                    renderer.set_art(Some(&gradient_art()));
                    renderer
                })
                .collect();
            Self { factory, dwrite, target, surface, brush, round, formats: HashMap::new(), renderers }
        }
    }

    fn begin(&mut self) {
        unsafe {
            let bounds = RECT { left: 0, top: 0, right: W, bottom: H };
            self.target.BindDC(self.surface.dc, &bounds).unwrap();
            self.target.BeginDraw();
            self.target.SetTransform(&translation(0.0, 0.0));
        }
    }

    fn end(&mut self) -> Vec<u32> {
        unsafe {
            self.target.EndDraw(None, None).unwrap();
            let _ = GdiFlush();
        }
        self.surface.pixels().to_vec()
    }

    fn world(&self, m: Matrix3x2) {
        unsafe { self.target.SetTransform(&m) };
    }

    fn reset(&self) {
        self.world(translation(0.0, 0.0));
    }

    fn color(&self, hex: u32, alpha: f32) {
        unsafe { self.brush.SetColor(&rgb(hex, alpha.clamp(0.0, 1.0))) };
    }

    fn rect(&self, area: D2D_RECT_F, hex: u32, alpha: f32) {
        self.color(hex, alpha);
        unsafe { self.target.FillRectangle(&area, &self.brush) };
    }

    fn rounded(&self, area: D2D_RECT_F, radius: f32, hex: u32, alpha: f32) {
        self.color(hex, alpha);
        unsafe {
            self.target.FillRoundedRectangle(&D2D1_ROUNDED_RECT { rect: area, radiusX: radius, radiusY: radius }, &self.brush)
        };
    }

    fn outline(&self, area: D2D_RECT_F, radius: f32, width: f32, hex: u32, alpha: f32) {
        self.color(hex, alpha);
        unsafe {
            let shape = D2D1_ROUNDED_RECT { rect: area, radiusX: radius, radiusY: radius };
            self.target.DrawRoundedRectangle(&shape, &self.brush, width, &self.round)
        };
    }

    /// Lingkaran cahaya lembut yang memudar ke tepinya.
    fn glow(&self, cx: f32, cy: f32, rx: f32, ry: f32, hex: u32, alpha: f32) {
        if alpha <= 0.002 {
            return;
        }
        unsafe {
            let stops = [
                D2D1_GRADIENT_STOP { position: 0.0, color: rgb(hex, alpha) },
                D2D1_GRADIENT_STOP { position: 0.45, color: rgb(hex, alpha * 0.35) },
                D2D1_GRADIENT_STOP { position: 1.0, color: rgb(hex, 0.0) },
            ];
            let collection = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP).unwrap();
            let props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                center: pt(cx, cy),
                gradientOriginOffset: pt(0.0, 0.0),
                radiusX: rx,
                radiusY: ry,
            };
            let brush = self.target.CreateRadialGradientBrush(&props, None, &collection).unwrap();
            let ellipse = D2D1_ELLIPSE { point: pt(cx, cy), radiusX: rx, radiusY: ry };
            self.target.FillEllipse(&ellipse, &brush);
        }
    }

    /// Latar: gradasi biru malam, pita cahaya yang meliuk pelan, dan vinyet.
    fn background(&self, t: f64, intensity: f32, converge: f32, tint: u32) {
        unsafe {
            let stops = [
                D2D1_GRADIENT_STOP { position: 0.0, color: rgb(0x03050C, 1.0) },
                D2D1_GRADIENT_STOP { position: 0.55, color: rgb(0x060B1C, 1.0) },
                D2D1_GRADIENT_STOP { position: 1.0, color: rgb(0x0A1430, 1.0) },
            ];
            let collection = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP).unwrap();
            let props = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES { startPoint: pt(0.0, 0.0), endPoint: pt(0.0, H as f32) };
            let brush = self.target.CreateLinearGradientBrush(&props, None, &collection).unwrap();
            self.target.FillRectangle(&rect(0.0, 0.0, W as f32, H as f32), &brush);
        }
        self.glow(540.0, 1500.0, 900.0, 700.0, tint, 0.16 * intensity);
        self.glow(540.0, 300.0, 700.0, 500.0, ELECTRIC, 0.08 * intensity);
        let ribbons = [
            (CYAN, 0.0f32, 0.0f32, 1.0f32),
            (ELECTRIC, 1.7, 0.35, 0.8),
            (tint, 3.4, -0.3, 0.7),
        ];
        for (i, (hex, phase, tilt, strength)) in ribbons.into_iter().enumerate() {
            let time = t as f32 * 0.55 + phase;
            let base = 960.0 + 520.0 * (i as f32 - 1.0) + 90.0 * (time * 0.7).sin();
            let wave = 360.0 * (time * 0.9).cos();
            // Saat menutup video, semua pita menyatu ke tengah.
            let y = |v: f32| mix(v, 760.0, converge);
            let start = pt(-220.0, y(base + 300.0 * tilt));
            let c1 = pt(260.0, y(base - wave));
            let c2 = pt(820.0, y(base + wave * 0.8));
            let end = pt(1300.0, y(base - 300.0 * tilt));
            self.ribbon(start, c1, c2, end, hex, intensity * strength);
        }
        self.vignette();
    }

    /// Pita cahaya: beberapa lapis garis dari lebar-redup sampai tipis-terang, meniru pendar.
    fn ribbon(&self, start: Vector2, c1: Vector2, c2: Vector2, end: Vector2, hex: u32, strength: f32) {
        if strength <= 0.01 {
            return;
        }
        unsafe {
            let geometry = self.factory.CreatePathGeometry().unwrap();
            let sink = geometry.Open().unwrap();
            sink.BeginFigure(start, D2D1_FIGURE_BEGIN_HOLLOW);
            sink.AddBezier(&D2D1_BEZIER_SEGMENT { point1: c1, point2: c2, point3: end });
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            sink.Close().unwrap();
            for (width, alpha) in [(150.0, 0.025), (80.0, 0.045), (36.0, 0.08), (14.0, 0.2), (5.0, 0.55), (1.8, 0.95)] {
                self.color(hex, alpha * strength);
                self.target.DrawGeometry(&geometry, &self.brush, width, &self.round);
            }
        }
    }

    fn vignette(&self) {
        unsafe {
            let stops = [
                D2D1_GRADIENT_STOP { position: 0.55, color: rgb(0x000000, 0.0) },
                D2D1_GRADIENT_STOP { position: 1.0, color: rgb(0x000000, 0.7) },
            ];
            let collection = self.target.CreateGradientStopCollection(&stops, D2D1_GAMMA_2_2, D2D1_EXTEND_MODE_CLAMP).unwrap();
            let props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
                center: pt(540.0, 960.0),
                gradientOriginOffset: pt(0.0, 0.0),
                radiusX: 1000.0,
                radiusY: 1350.0,
            };
            let brush = self.target.CreateRadialGradientBrush(&props, None, &collection).unwrap();
            self.target.FillRectangle(&rect(0.0, 0.0, W as f32, H as f32), &brush);
        }
    }

    /// Garis cahaya mendatar yang terang di tengah.
    fn streak(&self, cx: f32, cy: f32, half: f32, hex: u32, alpha: f32) {
        if half < 1.0 || alpha <= 0.01 {
            return;
        }
        for (thick, a) in [(60.0, 0.05), (22.0, 0.15), (7.0, 0.5), (2.5, 1.0)] {
            self.rounded(rect(cx - half, cy - thick / 2.0, cx + half, cy + thick / 2.0), thick / 2.0, hex, a * alpha);
        }
        self.glow(cx, cy, half * 0.8, 120.0, hex, 0.35 * alpha);
    }

    fn format(&mut self, family: &'static str, size: f32, bold: bool) -> IDWriteTextFormat {
        self.format_aligned(family, size, bold, 0)
    }

    /// Format teks dengan perataan `align`: -1 kiri, 0 tengah, 1 kanan.
    fn format_aligned(&mut self, family: &'static str, size: f32, bold: bool, align: i32) -> IDWriteTextFormat {
        let key = (family, (size * 10.0) as u32, bold, align);
        if let Some(format) = self.formats.get(&key) {
            return format.clone();
        }
        let weight = if bold { DWRITE_FONT_WEIGHT_SEMI_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL };
        let format = unsafe {
            let format = self
                .dwrite
                .CreateTextFormat(&HSTRING::from(family), None, weight, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, size, w!("id-ID"))
                .unwrap();
            let alignment = match align {
                -1 => DWRITE_TEXT_ALIGNMENT_LEADING,
                1 => DWRITE_TEXT_ALIGNMENT_TRAILING,
                _ => DWRITE_TEXT_ALIGNMENT_CENTER,
            };
            format.SetTextAlignment(alignment).unwrap();
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER).unwrap();
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP).unwrap();
            format
        };
        self.formats.insert(key, format.clone());
        format
    }

    /// Teks rata tengah di garis `y`. `blur` 0..1: dari kabur (masih masuk) sampai tajam.
    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, s: &str, family: &'static str, size: f32, bold: bool, y: f32, hex: u32, alpha: f32, sharp: f32) {
        if alpha <= 0.01 {
            return;
        }
        let format = self.format(family, size, bold);
        let wide: Vec<u16> = s.encode_utf16().collect();
        let draw = |dx: f32, dy: f32, a: f32| unsafe {
            self.color(hex, a);
            let area = rect(-200.0 + dx, y - size + dy, W as f32 + 200.0 + dx, y + size + dy);
            self.target.DrawText(&wide, &format, &area, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        };
        // Kabur tiruan: salinan bergeser melingkar yang makin rapat saat menajam.
        let radius = (1.0 - sharp) * size * 0.18;
        if radius > 0.5 {
            for i in 0..10 {
                let angle = i as f32 / 10.0 * 2.0 * PI;
                draw(radius * angle.cos(), radius * angle.sin(), alpha * 0.11);
            }
        }
        draw(0.0, 0.0, alpha * sharp.max(0.15));
    }

    /// Teks yang masuk dari kabur dan sedikit naik, mulai di `at` selama `len`.
    #[allow(clippy::too_many_arguments)]
    fn reveal(&mut self, t: f64, at: f64, len: f64, s: &str, family: &'static str, size: f32, bold: bool, y: f32, hex: u32, fade_out: Option<f64>) {
        let p = ease_out(span(t, at, len));
        if p <= 0.0 {
            return;
        }
        let out = fade_out.map_or(1.0, |end| 1.0 - span(t, end - 0.25, 0.25));
        self.text(s, family, size, bold, y + (1.0 - p) * 40.0, hex, p * out, p);
    }

    /// Notch dari `scene` dengan titik tengah atasnya di (`ax`, `ay`), `k` piksel per DIP.
    fn notch(&mut self, scene: &Scene, theme: Theme, ax: f32, ay: f32, k: f32, opacity: f32) {
        if opacity <= 0.01 {
            return;
        }
        let index = Theme::ALL.iter().position(|t| *t == theme).unwrap_or(0);
        let renderer = &mut self.renderers[index];
        renderer.draw(scene).unwrap();
        let _ = unsafe { GdiFlush() };
        let s = NOTCH_DPI as f32 / 96.0;
        let half = scene.width / 2.0 + FLARE_MAX + 8.0;
        let x0 = ((CANVAS.0 / 2.0 - half) * s).floor().max(0.0) as i32;
        let x1 = ((CANVAS.0 / 2.0 + half) * s).ceil().min(renderer.surface.width as f32) as i32;
        let y1 = ((scene.height + SHADOW + 6.0) * s).ceil().min(renderer.surface.height as f32) as i32;
        let width = renderer.surface.width;
        let pixels = renderer.surface.pixels();
        let props = D2D1_BITMAP_PROPERTIES {
            pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED },
            dpiX: 96.0,
            dpiY: 96.0,
        };
        unsafe {
            let bitmap: ID2D1Bitmap = self
                .target
                .CreateBitmap(
                    D2D_SIZE_U { width: (x1 - x0) as u32, height: y1 as u32 },
                    Some(pixels.as_ptr().add(x0 as usize).cast()),
                    width as u32 * 4,
                    &props,
                )
                .unwrap();
            let dest = rect(
                ax + (x0 as f32 / s - CANVAS.0 / 2.0) * k,
                ay,
                ax + (x1 as f32 / s - CANVAS.0 / 2.0) * k,
                ay + y1 as f32 / s * k,
            );
            self.target.DrawBitmap(&bitmap, Some(&dest), opacity, D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, None);
        }
    }

    /// Sepasang mata maskot sebesar apa pun, dengan bentuk yang sama seperti di app.
    #[allow(clippy::too_many_arguments)]
    fn eyes(&self, cx: f32, cy: f32, eye_w: f32, eye_h: f32, gap: f32, eyes: Eyes, alpha: f32) {
        let (dx, dy) = (eyes.look.0 * eye_w * 0.8, eyes.look.1 * eye_h * 0.16);
        let gap = match eyes.mood {
            Mood::Surprised => gap * 1.15,
            Mood::Happy => gap * 1.3,
            _ => gap,
        };
        for x in [cx - gap / 2.0 + dx, cx + gap / 2.0 + dx] {
            let y = cy + dy;
            if eyes.mood == Mood::Happy {
                let r = eye_h * 0.32;
                let base = y + r * 0.45;
                unsafe {
                    let geometry = self.factory.CreatePathGeometry().unwrap();
                    let sink = geometry.Open().unwrap();
                    sink.BeginFigure(pt(x - r, base), D2D1_FIGURE_BEGIN_HOLLOW);
                    sink.AddArc(&arc(x + r, base, r, D2D1_SWEEP_DIRECTION_CLOCKWISE, D2D1_ARC_SIZE_SMALL));
                    sink.EndFigure(D2D1_FIGURE_END_OPEN);
                    sink.Close().unwrap();
                    self.color(0xF5F5F7, alpha);
                    self.target.DrawGeometry(&geometry, &self.brush, eye_w * 0.62, &self.round);
                }
                continue;
            }
            let (w, h) = match eyes.mood {
                Mood::Surprised => (eye_h * 0.62, (eye_h * 0.78 * eyes.open).max(eye_w * 0.45)),
                Mood::Sleepy => (eye_w * 1.3, (eye_h * 0.24 * eyes.open).max(eye_w * 0.4)),
                _ => (eye_w, (eye_h * eyes.open).max(eye_w * 0.45)),
            };
            self.rounded(rect(x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0), w.min(h) / 2.0, 0xF5F5F7, alpha);
        }
    }

    /// Kursor panah sederhana.
    fn cursor(&self, x: f32, y: f32, scale: f32, alpha: f32) {
        let points = [(0.0, 0.0), (0.0, 34.0), (9.0, 26.0), (15.0, 40.0), (21.0, 37.0), (15.0, 24.0), (26.0, 24.0)];
        for (fill, grow, offset) in [(0x000000u32, 3.0f32, 2.0f32), (0xFFFFFF, 0.0, 0.0)] {
            unsafe {
                let geometry = self.factory.CreatePathGeometry().unwrap();
                let sink = geometry.Open().unwrap();
                let p = |(px, py): (f32, f32)| pt(x + offset + px * scale * (1.0 + grow * 0.02), y + offset + py * scale * (1.0 + grow * 0.02));
                sink.BeginFigure(p(points[0]), D2D1_FIGURE_BEGIN_FILLED);
                for point in &points[1..] {
                    sink.AddLine(p(*point));
                }
                sink.EndFigure(D2D1_FIGURE_END_CLOSED);
                sink.Close().unwrap();
                self.color(fill, if fill == 0 { alpha * 0.45 } else { alpha });
                self.target.FillGeometry(&geometry, &self.brush, None::<&ID2D1Brush>);
            }
        }
    }

    /// Kartu file yang diseret (dokumen PDF).
    fn file_card(&mut self, cx: f32, cy: f32, scale: f32, alpha: f32) {
        let (w, h) = (130.0 * scale, 164.0 * scale);
        let area = rect(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
        self.glow(cx, cy, w, h * 0.8, CYAN, 0.25 * alpha);
        self.rounded(rect(area.left + 6.0, area.top + 10.0, area.right + 6.0, area.bottom + 10.0), 18.0 * scale, 0x000000, 0.35 * alpha);
        self.rounded(area, 18.0 * scale, 0xF2F4F8, alpha);
        self.rounded(rect(area.left + 18.0 * scale, area.bottom - 58.0 * scale, area.left + 84.0 * scale, area.bottom - 26.0 * scale), 8.0 * scale, 0xFF453A, alpha);
        for i in 0..3 {
            let y = area.top + (34.0 + i as f32 * 22.0) * scale;
            self.rounded(rect(area.left + 18.0 * scale, y, area.right - (18.0 + i as f32 * 14.0) * scale, y + 8.0 * scale), 4.0 * scale, 0xC7CCD6, alpha);
        }
        let label: Vec<u16> = "PDF".encode_utf16().collect();
        let format = self.format(SANS, 22.0 * scale, true);
        unsafe {
            self.color(0xFFFFFF, alpha);
            let label_area = rect(area.left + 18.0 * scale, area.bottom - 58.0 * scale, area.left + 84.0 * scale, area.bottom - 26.0 * scale);
            self.target.DrawText(&label, &format, &label_area, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        }
    }

    /// Teks satu baris di (`x`, `y`): `align` -1 mulai dari x, 0 berpusat di x, 1 berakhir di x.
    #[allow(clippy::too_many_arguments)]
    fn label(&mut self, s: &str, family: &'static str, size: f32, bold: bool, x: f32, y: f32, hex: u32, alpha: f32, align: i32) {
        if alpha <= 0.01 || s.is_empty() || size < 1.0 {
            return;
        }
        let format = self.format_aligned(family, size, bold, align);
        let wide: Vec<u16> = s.encode_utf16().collect();
        let area = match align {
            -1 => rect(x, y - size, x + 4000.0, y + size),
            1 => rect(x - 4000.0, y - size, x, y + size),
            _ => rect(x - 2000.0, y - size, x + 2000.0, y + size),
        };
        self.color(hex, alpha);
        unsafe {
            self.target.DrawText(&wide, &format, &area, &self.brush, D2D1_DRAW_TEXT_OPTIONS_NONE, DWRITE_MEASURING_MODE_NATURAL);
        }
    }

    /// Ikon dari font ikon Windows, berpusat di (`x`, `y`).
    fn icon(&mut self, glyph: char, size: f32, x: f32, y: f32, hex: u32, alpha: f32) {
        self.label(&glyph.to_string(), ICONS, size, false, x, y, hex, alpha, 0);
    }

    /// Lebar teks satu baris.
    fn measure(&mut self, s: &str, family: &'static str, size: f32, bold: bool) -> f32 {
        let format = self.format_aligned(family, size, bold, -1);
        let wide: Vec<u16> = s.encode_utf16().collect();
        unsafe {
            let layout = self.dwrite.CreateTextLayout(&wide, &format, 10_000.0, size * 2.0).unwrap();
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics).unwrap();
            metrics.widthIncludingTrailingWhitespace
        }
    }

    fn line(&self, a: (f32, f32), b: (f32, f32), width: f32, hex: u32, alpha: f32) {
        if alpha <= 0.005 {
            return;
        }
        self.color(hex, alpha);
        unsafe { self.target.DrawLine(pt(a.0, a.1), pt(b.0, b.1), &self.brush, width, &self.round) };
    }

    fn circle(&self, cx: f32, cy: f32, r: f32, hex: u32, alpha: f32) {
        if alpha <= 0.005 || r <= 0.0 {
            return;
        }
        self.color(hex, alpha);
        unsafe { self.target.FillEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r, radiusY: r }, &self.brush) };
    }

    fn ring(&self, cx: f32, cy: f32, r: f32, width: f32, hex: u32, alpha: f32) {
        if alpha <= 0.005 || r <= 0.0 {
            return;
        }
        self.color(hex, alpha);
        unsafe {
            self.target.DrawEllipse(&D2D1_ELLIPSE { point: pt(cx, cy), radiusX: r, radiusY: r }, &self.brush, width, &self.round)
        };
    }

    /// Busur searah jarum jam mulai dari atas, sepanjang `sweep` (0..1 = satu putaran penuh).
    #[allow(clippy::too_many_arguments)]
    fn arc(&self, cx: f32, cy: f32, r: f32, width: f32, start: f32, sweep: f32, hex: u32, alpha: f32) {
        if sweep <= 0.001 {
            return;
        }
        let steps = (sweep * 96.0).ceil().max(2.0) as usize;
        let points: Vec<(f32, f32)> = (0..=steps)
            .map(|i| {
                let a = (start + sweep * i as f32 / steps as f32) * 2.0 * PI - PI / 2.0;
                (cx + r * a.cos(), cy + r * a.sin())
            })
            .collect();
        self.polyline(&points, width, hex, alpha);
    }

    fn polyline(&self, points: &[(f32, f32)], width: f32, hex: u32, alpha: f32) {
        if points.len() < 2 || alpha <= 0.005 {
            return;
        }
        unsafe {
            let geometry = self.factory.CreatePathGeometry().unwrap();
            let sink = geometry.Open().unwrap();
            sink.BeginFigure(pt(points[0].0, points[0].1), D2D1_FIGURE_BEGIN_HOLLOW);
            for &(x, y) in &points[1..] {
                sink.AddLine(pt(x, y));
            }
            sink.EndFigure(D2D1_FIGURE_END_OPEN);
            sink.Close().unwrap();
            self.color(hex, alpha);
            self.target.DrawGeometry(&geometry, &self.brush, width, &self.round);
        }
    }

    fn polygon(&self, points: &[(f32, f32)], hex: u32, alpha: f32) {
        if points.len() < 3 || alpha <= 0.005 {
            return;
        }
        unsafe {
            let geometry = self.factory.CreatePathGeometry().unwrap();
            let sink = geometry.Open().unwrap();
            sink.BeginFigure(pt(points[0].0, points[0].1), D2D1_FIGURE_BEGIN_FILLED);
            for &(x, y) in &points[1..] {
                sink.AddLine(pt(x, y));
            }
            sink.EndFigure(D2D1_FIGURE_END_CLOSED);
            sink.Close().unwrap();
            self.color(hex, alpha);
            self.target.FillGeometry(&geometry, &self.brush, None::<&ID2D1Brush>);
        }
    }

    /// Gambar berikutnya hanya muncul di dalam `area` sampai `unclip`.
    fn clip(&self, area: D2D_RECT_F) {
        unsafe { self.target.PushAxisAlignedClip(&area, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE) };
    }

    fn unclip(&self) {
        unsafe { self.target.PopAxisAlignedClip() };
    }

    /// Sampul lagu yang dipakai semua tema.
    fn set_art(&mut self, art: &Art) {
        for renderer in &mut self.renderers {
            renderer.set_art(Some(art));
        }
    }

    /// Thumbnail item shelf yang dipakai semua tema.
    fn set_thumbnail(&mut self, thumb: &Thumbnail) {
        for renderer in &mut self.renderers {
            renderer.set_thumbnail(thumb);
        }
    }

    /// Ukuran pil lirik untuk satu baris, sama seperti di app.
    fn lyric_size(&self, line: &str) -> (f32, f32) {
        self.renderers[0].lyric_size(line)
    }

    fn flash(&self, alpha: f32) {
        if alpha > 0.005 {
            self.rect(rect(0.0, 0.0, W as f32, H as f32), 0xFFFFFF, alpha);
        }
    }

    fn fade(&self, alpha: f32) {
        if alpha > 0.005 {
            self.rect(rect(0.0, 0.0, W as f32, H as f32), 0x000000, alpha);
        }
    }
}

/// Zoom `scale` di sekitar titik (`cx`, `cy`), lalu geser (`dx`, `dy`).
fn camera(scale: f32, cx: f32, cy: f32, dx: f32, dy: f32) -> Matrix3x2 {
    Matrix3x2 { M11: scale, M12: 0.0, M21: 0.0, M22: scale, M31: cx * (1.0 - scale) + dx, M32: cy * (1.0 - scale) + dy }
}

// ---------------------------------------------------------------- isi adegan

/// Data contoh untuk semua adegan (lagu dan lirik karangan sendiri).
struct Cast {
    data: Sample,
    idle: FocusView,
    none: Usage,
    calendar: CalendarView,
    timer: TimerView,
}

impl Cast {
    fn new() -> Self {
        let mut data = sample();
        data.media = song();
        Self {
            data,
            idle: idle_focus(),
            none: Usage::default(),
            calendar: sample_calendar(),
            timer: TimerView {
                running: true,
                active: true,
                clock: "03:42".into(),
                short: "4m".into(),
                progress: 0.26,
                total_minutes: 5,
            },
        }
    }

    fn scene(&self, view: View, page: Page, media: bool) -> Scene<'_> {
        let mut scene = home(&self.data, &self.idle, &self.none, view, page, media);
        scene.lyrics_button = Some(false);
        scene
    }
}

/// Ukuran dan transparansi notch yang sedang berpindah bentuk, dihitung ulang dari awal adegan
/// supaya setiap frame (juga sub-frame blur gerak) selalu konsisten.
fn simulate(keys: &[(f64, View, (f32, f32), f32)], t: f64) -> Motion {
    let (start, view, size, shadow) = keys[0];
    let mut motion = Motion::new(view, size);
    motion.shadow_goal = shadow;
    let mut time = start;
    let mut next = 1;
    while time + 1.0 / 60.0 <= t {
        while next < keys.len() && keys[next].0 <= time {
            let (_, view, size, shadow) = keys[next];
            motion.go(view, size);
            motion.shadow_goal = shadow;
            next += 1;
        }
        motion.step();
        time += 1.0 / 60.0;
    }
    motion
}

/// Satu frame video pada detik `t`.
fn frame(studio: &mut Studio, cast: &Cast, t: f64) -> Vec<u32> {
    studio.begin();
    match t {
        t if t < 2.0 => scene_hook(studio, cast, t),
        t if t < 4.5 => scene_laptop(studio, cast, t),
        t if t < 8.5 => scene_music(studio, cast, t),
        t if t < 11.5 => scene_hud(studio, cast, t),
        t if t < 15.0 => scene_drop(studio, cast, t),
        t if t < 18.5 => scene_widgets(studio, cast, t),
        t if t < 21.5 => scene_eyes(studio, t),
        t if t < 24.0 => scene_themes(studio, cast, t),
        _ => scene_outro(studio, cast, t),
    }
    studio.end()
}

/// 1. "Notch." – garis cahaya menyusut jadi pil, mata berkedip, judul muncul dari kabur.
fn scene_hook(studio: &mut Studio, cast: &Cast, t: f64) {
    studio.background(t, ease_out(span(t, 0.0, 0.6)) * 0.7, 0.0, MAGENTA);
    let grow = ease_out(span(t, 0.1, 0.4));
    let shrink = ease_in_out(span(t, 0.5, 0.35));
    studio.streak(540.0, 820.0, mix(0.0, 520.0, grow) * (1.0 - shrink), CYAN, 1.0 - span(t, 0.75, 0.15));
    let pop = span(t, 0.62, 0.45);
    if pop > 0.0 {
        let k = 3.6 * mix(0.35, 1.0, back_out(pop));
        let mut scene = cast.scene(View::Pill, Page::Home, false);
        scene.eyes.open = blink(t, 1.15);
        studio.glow(540.0, 820.0, 520.0, 220.0, CYAN, 0.35 * (1.0 - span(t, 0.7, 0.9)) + 0.12);
        studio.notch(&scene, Theme::Black, 540.0, 820.0 - 15.0 * k, k, ease_out(pop));
    }
    studio.reveal(t, 1.0, 0.35, "Notch.", SERIF, 170.0, false, 1110.0, 0xFFFFFF, None);
    studio.reveal(t, 1.5, 0.3, "…di Windows?", SERIF, 92.0, false, 1260.0, 0xBFD8FF, None);
    studio.flash(0.6 * span(t, 1.85, 0.15));
}

/// 2. Kamera mundur dari pil sampai terlihat di atas layar laptop.
fn scene_laptop(studio: &mut Studio, cast: &Cast, t: f64) {
    studio.background(t, 0.55, 0.0, MAGENTA);
    studio.flash(0.6 * (1.0 - span(t, 2.0, 0.2)));
    let zoom = mix(2.6, 1.0, ease_in_out(span(t, 2.0, 0.9)));
    let push = ease_in_out(span(t, 4.1, 0.4));
    let zoom = zoom * mix(1.0, 3.2, push);
    let (sx, sy, sw, sh) = (540.0f32, 860.0f32, 940.0f32, 588.0f32);
    let top = sy - sh / 2.0;
    studio.world(camera(zoom, 540.0, top + 30.0, 0.0, 0.0));
    // Layar laptop: bingkai tipis berpendar, jendela kaca samar, dan taskbar ala Windows 11.
    let screen = rect(sx - sw / 2.0, top, sx + sw / 2.0, sy + sh / 2.0);
    studio.glow(sx, sy, sw * 0.7, sh * 0.8, ELECTRIC, 0.18);
    studio.rounded(screen, 26.0, 0x0B1226, 0.92);
    studio.outline(screen, 26.0, 3.0, CYAN, 0.55);
    studio.rounded(rect(sx - 330.0, top + 120.0, sx + 210.0, top + 430.0), 18.0, 0x1A2647, 0.55);
    studio.rounded(rect(sx - 120.0, top + 180.0, sx + 380.0, top + 470.0), 18.0, 0x22305A, 0.45);
    let bar = rect(screen.left + 10.0, screen.bottom - 58.0, screen.right - 10.0, screen.bottom - 10.0);
    studio.rounded(bar, 16.0, 0x141C33, 0.95);
    for i in 0..6 {
        let x = sx - 125.0 + i as f32 * 50.0;
        let color = [ELECTRIC, 0xF2F4F8, 0xFFB340, CYAN, MAGENTA, 0x30D158][i];
        studio.rounded(rect(x - 15.0, bar.top + 9.0, x + 15.0, bar.top + 39.0), 8.0, color, 0.85);
    }
    // Notch menempel di tepi atas layar.
    let mut scene = cast.scene(View::Pill, Page::Home, false);
    scene.eyes.open = blink(t, 3.6);
    studio.notch(&scene, Theme::Black, sx, top, 1.7, 1.0);
    studio.reset();
    let out = Some(4.1);
    studio.reveal(t, 2.85, 0.35, "Langsung di layar", SANS, 68.0, true, 1330.0, 0xFFFFFF, out);
    studio.reveal(t, 3.1, 0.35, "laptop Windows-mu.", SERIF, 84.0, false, 1430.0, 0xBFD8FF, out);
}

/// 3. Pil musik membuka jadi pemutar, lalu melebar dan lirik bergeser masuk lalu bergulir.
fn scene_music(studio: &mut Studio, cast: &Cast, t: f64) {
    studio.background(t, 0.75 + 0.25 * pulse(t), 0.0, MAGENTA);
    // Teks raksasa di belakang yang bergeser pelan, seperti referensi.
    let drift = span(t, 4.5, 4.0);
    studio.world(camera(1.0, 540.0, 960.0, mix(260.0, -260.0, drift), 0.0));
    studio.text("MUSIK • LIRIK • MUSIK • LIRIK", SERIF, 150.0, false, 360.0, 0xFFFFFF, 0.05, 1.0);
    studio.world(camera(1.0, 540.0, 960.0, mix(-320.0, 320.0, drift), 0.0));
    studio.text("IKUT BERNYANYI • IKUT BERNYANYI", SERIF, 190.0, false, 1600.0, 0xFFFFFF, 0.05, 1.0);
    studio.reset();
    let keys = [
        (4.5, View::Pill, View::Pill.size(), 0.0f32),
        (4.75, View::Panel, View::Panel.size(), 1.0),
        (5.75, View::PanelWide, View::PanelWide.size(), 1.0),
    ];
    let motion = simulate(&keys, t);
    let wide = ease_in_out(span(t, 5.75, 0.55));
    let k = mix(2.3, 1.52, wide);
    let lyric_mix = ease_out(span(t, 5.85, 0.5));
    // Baris lirik naik dua kali, tepat di ketukan.
    let (current, scroll) = if t < 6.75 {
        (2, 1.0)
    } else if t < 7.75 {
        (3, span(t, 6.75, 0.38))
    } else {
        (4, span(t, 7.75, 0.38))
    };
    let lyrics = lyrics_at(current, scroll);
    let mut scene = cast.scene(View::Panel, Page::Home, true);
    motion.apply(&mut scene);
    scene.lyrics_button = Some(t >= 5.6);
    if t >= 5.75 {
        scene.lyrics = Some(&lyrics);
        scene.lyric_mix = lyric_mix;
    }
    studio.glow(540.0, 760.0, 620.0, 360.0, CYAN, 0.28 + 0.12 * pulse(t));
    studio.glow(540.0, 820.0, 420.0, 260.0, MAGENTA, 0.16);
    let hero = ease_out(span(t, 4.5, 0.3));
    studio.notch(&scene, Theme::Black, 540.0, 600.0, k * mix(0.9, 1.0, hero), hero);
    studio.reveal(t, 5.0, 0.35, "Musik & lirik", SANS, 76.0, true, 1250.0, 0xFFFFFF, Some(8.4));
    studio.reveal(t, 5.9, 0.35, "yang ikut bernyanyi.", SERIF, 84.0, false, 1355.0, 0xBFD8FF, Some(8.4));
}

/// 4. Kabar ala Dynamic Island: pil melebar sekali, lalu isinya berganti di ketukan (volume,
/// kecerahan, Caps Lock) dengan pantulan kecil, dan kembali jadi pil di akhir.
fn scene_hud(studio: &mut Studio, cast: &Cast, t: f64) {
    studio.background(t, 0.6 + 0.35 * pulse(t), 0.0, ELECTRIC);
    let segment = ((t - 8.5) / 1.0).floor().clamp(0.0, 2.0) as usize;
    let start = 8.5 + segment as f64;
    let local = t - start;
    let huds = [
        Hud::Volume { level: if local < 0.5 { 0.61 } else { 0.62 }, muted: false },
        Hud::Brightness { level: if local < 0.5 { 0.55 } else { 0.7 } },
        Hud::LockKey { caps: true, on: true },
    ];
    let words = ["Volume.", "Kecerahan.", "Caps Lock."];
    let keys = [
        (8.5, View::Pill, View::Pill.size(), 0.0f32),
        (8.6, View::Strip, View::Strip.size(), 0.0),
        (11.15, View::Pill, View::Pill.size(), 0.0),
    ];
    let motion = simulate(&keys, t);
    let roll = Roll { from: ["61", "55", ""][segment].into(), t: span(t, start + 0.5, 0.22), up: true };
    let mut scene = cast.scene(View::Strip, Page::Home, false);
    motion.apply(&mut scene);
    scene.hud = Some(&huds[segment]);
    scene.hud_compact = true;
    scene.hud_fill = match &huds[segment] {
        Hud::Volume { level, .. } | Hud::Brightness { level } => *level,
        _ => 0.0,
    };
    scene.hud_roll = (segment < 2 && local >= 0.5 && roll.t < 1.0).then_some(&roll);
    let pop = if segment > 0 { back_out(span(t, start, 0.3)) } else { 1.0 };
    studio.glow(540.0, 760.0, 560.0, 200.0, CYAN, 0.25 + 0.25 * pulse(t));
    studio.notch(&scene, Theme::Black, 540.0, 700.0, 3.1 * mix(0.9, 1.0, pop), 1.0);
    studio.text("Kabar singkat ala Dynamic Island", SANS, 46.0, false, 520.0, 0xBFD8FF, ease_out(span(t, 8.5, 0.3)) * 0.85, 1.0);
    for (i, word) in words.iter().enumerate() {
        let at = 8.5 + i as f64 + 0.05;
        let leave = if i < 2 { Some(at + 0.95) } else { Some(11.45) };
        studio.reveal(t, at, 0.3, word, SERIF, 150.0, false, 1150.0, 0xFFFFFF, leave);
    }
    studio.flash(0.18 * pulse(t) * span(t, 8.5, 0.1));
}

/// 5. File diseret ke notch, dilepas di shelf; file kedua dilepas di zona Bagikan.
fn scene_drop(studio: &mut Studio, cast: &Cast, t: f64) {
    studio.background(t, 0.6, 0.0, CYAN);
    let mut data = sample();
    data.media = song();
    let first_drop = 12.9;
    let second_drop = 14.0;
    data.shelf.drop_hover = !(first_drop..13.45).contains(&t) && t < 14.55;
    let first = ease_in_out(span(t, 12.0, 0.85));
    let second = ease_in_out(span(t, 13.45, 0.5));
    let fx = ShelfFx {
        targeted: (12.55..first_drop).contains(&t),
        share_targeted: (13.75..second_drop + 0.5).contains(&t),
        dash: (t * 5.0 % 5.0) as f32,
        glow: (first_drop..first_drop + 0.7).contains(&t).then(|| span(t, first_drop, 0.7)),
        tiles: HashMap::new(),
    };
    let mut scene = home(&data, &cast.idle, &cast.none, View::Panel, Page::Shelf, false);
    scene.shelf_fx = &fx;
    scene.seconds = t;
    let appear = ease_out(span(t, 11.5, 0.3));
    let (ax, ay, k) = (540.0, 560.0, 2.3);
    studio.glow(540.0, 740.0, 600.0, 320.0, CYAN, 0.22 + 0.2 * span(t, 12.55, 0.3) * (1.0 - span(t, first_drop + 0.1, 0.4)));
    studio.notch(&scene, Theme::Black, ax, ay, k * mix(0.92, 1.0, appear), appear);
    // Kartu file terbang di sepanjang lengkung menuju zona lepas.
    let flight = |p: f32, from: (f32, f32), to: (f32, f32)| {
        let control = (from.0 + 80.0, to.1 + 420.0);
        let a = mix(from.0, control.0, p);
        let b = mix(control.0, to.0, p);
        let c = mix(from.1, control.1, p);
        let d = mix(control.1, to.1, p);
        (mix(a, b, p), mix(c, d, p))
    };
    if t < first_drop {
        let (x, y) = flight(first, (900.0, 1650.0), (420.0, 770.0));
        studio.file_card(x, y, mix(1.15, 0.8, first), span(t, 11.9, 0.15));
        studio.cursor(x + 30.0, y + 40.0, 1.6, span(t, 11.9, 0.15));
    }
    if (13.45..second_drop).contains(&t) {
        let (x, y) = flight(second, (180.0, 1650.0), (850.0, 770.0));
        studio.file_card(x, y, mix(1.15, 0.8, second), 1.0);
        studio.cursor(x + 30.0, y + 40.0, 1.6, 1.0);
    }
    studio.flash(0.35 * (1.0 - span(t, first_drop, 0.18)) * span(t, first_drop - 0.01, 0.01));
    studio.flash(0.35 * (1.0 - span(t, second_drop, 0.18)) * span(t, second_drop - 0.01, 0.01));
    for (i, (word, at)) in [("Seret.", 12.0), ("Lepas.", first_drop), ("Bagikan.", second_drop)].into_iter().enumerate() {
        studio.reveal(t, at, 0.3, word, SERIF, 120.0, false, 1230.0 + i as f32 * 130.0, 0xFFFFFF, Some(14.95));
    }
}

/// 6. Halaman lain masuk bergantian dari kiri dan kanan.
fn scene_widgets(studio: &mut Studio, cast: &Cast, t: f64) {
    studio.background(t, 0.55, 0.0, 0xFF9F0A);
    studio.reveal(t, 15.0, 0.35, "Dan masih banyak lagi", SANS, 60.0, true, 300.0, 0xFFFFFF, Some(18.45));
    let rows: [(View, Page, f32, f32); 4] = [
        (View::PanelWide, Page::Home, 1.5, 420.0),
        (View::Panel, Page::Focus, 2.0, 700.0),
        (View::Panel, Page::Timer, 2.0, 1040.0),
        (View::Panel, Page::Water, 2.0, 1380.0),
    ];
    for (i, (view, page, k, y)) in rows.into_iter().enumerate() {
        let at = 15.15 + i as f64 * 0.25;
        let p = ease_out(span(t, at, 0.45));
        if p <= 0.0 {
            continue;
        }
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let leave = ease_in_out(span(t, 18.15, 0.3));
        let x = 540.0 + side * (1.0 - p) * 900.0 - side * leave * 900.0;
        let mut scene = home(&cast.data, &cast.idle, &cast.none, view, page, false);
        if page == Page::Home {
            scene.calendar = Some(&cast.calendar);
            scene.weather = Some("29° Cerah");
        }
        if page == Page::Focus {
            scene.focus = &cast.data.focus;
        }
        if page == Page::Timer {
            scene.timer = &cast.timer;
        }
        let float = (t as f32 * 1.6 + i as f32).sin() * 6.0;
        studio.notch(&scene, Theme::Black, x, y + float, k, p);
    }
}

/// 7. Mata maskot dari dekat mengikuti kursor, berkedip, lalu tersenyum.
fn scene_eyes(studio: &mut Studio, t: f64) {
    studio.background(t, 0.5, 0.0, MAGENTA);
    studio.glow(540.0, 820.0, 520.0, 420.0, ELECTRIC, 0.22);
    // Kursor melintas kiri-kanan; mata meliriknya.
    let path = ease_in_out(span(t, 18.7, 0.6)) - ease_in_out(span(t, 19.75, 0.7)) * 2.0 + ease_in_out(span(t, 20.55, 0.3));
    let cursor_x = 540.0 - path * 380.0;
    let look_x = (cursor_x - 540.0) / 380.0;
    let happy = t >= 20.85;
    let eyes = Eyes {
        open: blink(t, 19.5),
        look: (look_x.clamp(-1.0, 1.0), 0.25),
        mood: if happy { Mood::Happy } else { Mood::Normal },
    };
    let appear = ease_out(span(t, 18.5, 0.3));
    studio.eyes(540.0, 820.0, 66.0 * mix(0.6, 1.0, back_out(appear)), 150.0 * mix(0.6, 1.0, back_out(appear)), 125.0, eyes, appear);
    if !happy {
        studio.cursor(cursor_x, 1080.0, 2.2, span(t, 18.6, 0.2));
    }
    studio.reveal(t, 19.0, 0.35, "Dia juga ngelirik kamu.", SERIF, 92.0, false, 1340.0, 0xFFFFFF, Some(21.45));
}

/// 8. Enam tema berganti cepat dengan geseran.
fn scene_themes(studio: &mut Studio, cast: &Cast, t: f64) {
    let index = (((t - 21.5) / 0.4).floor() as usize).min(Theme::ALL.len() - 1);
    let tint = [0x8E8E93, 0xB0B0B8, 0x64D2FF, 0xBF5AF2, CYAN, 0xF5F5F7][index];
    studio.background(t, 0.7, 0.0, tint);
    // Geser seperti carousel: kartu lama keluar ke kiri saat kartu baru masuk dari kanan,
    // nama tema ikut kartunya.
    // Kurva sinus: mulai dan berhenti lembut, kecepatan puncaknya rendah sehingga blurnya rata.
    let enter = (1.0 - (span(t, 21.5 + index as f64 * 0.4, 0.26) * PI).cos()) / 2.0;
    let mut slides = vec![(index, 1100.0 * (1.0 - enter))];
    if index > 0 && enter < 1.0 {
        slides.push((index - 1, -1100.0 * enter));
    }
    let scene = cast.scene(View::Panel, Page::Home, true);
    for (i, dx) in slides {
        let theme = Theme::ALL[i];
        // Tema kaca dipamerkan di atas warna, supaya tembus pandangnya terlihat.
        if theme == Theme::Glass {
            studio.glow(540.0 + dx, 760.0, 600.0, 300.0, MAGENTA, 0.55);
            studio.glow(380.0 + dx, 700.0, 300.0, 200.0, CYAN, 0.5);
        }
        studio.notch(&scene, theme, 540.0 + dx, 600.0, 2.3, 1.0);
        studio.world(camera(1.0, 540.0, 960.0, dx, 0.0));
        studio.text(theme.label(), SANS, 54.0, true, 1340.0, 0xBFD8FF, 0.9, 1.0);
        studio.reset();
    }
    studio.text("6 tema", SERIF, 150.0, false, 1220.0, 0xFFFFFF, ease_out(span(t, 21.5, 0.25)), ease_out(span(t, 21.5, 0.25)));
}

/// 9. Penutup: pita cahaya menyatu ke pil, nama, dan ajakan mengunduh.
fn scene_outro(studio: &mut Studio, cast: &Cast, t: f64) {
    let converge = ease_in_out(span(t, 24.0, 1.2));
    studio.background(t, 0.85, converge * 0.85, MAGENTA);
    studio.glow(540.0, 760.0, 560.0, 260.0, CYAN, 0.3 + 0.25 * converge);
    let appear = span(t, 24.1, 0.5);
    let mut scene = cast.scene(View::Pill, Page::Home, false);
    scene.eyes.open = blink(t, 26.4);
    if t >= 26.9 {
        scene.eyes.mood = Mood::Happy;
    }
    let k = 3.4 * mix(0.4, 1.0, back_out(appear));
    studio.notch(&scene, Theme::Black, 540.0, 720.0 - 15.0 * k, k, ease_out(appear));
    studio.reveal(t, 24.5, 0.4, "NotchWindows", SERIF, 150.0, false, 1000.0, 0xFFFFFF, None);
    studio.reveal(t, 25.0, 0.35, "Gratis · Open source · Tanpa akun", SANS, 46.0, false, 1120.0, 0xBFD8FF, None);
    let button = ease_out(span(t, 25.5, 0.35));
    if button > 0.0 {
        let area = rect(290.0, 1380.0 - 46.0, 790.0, 1380.0 + 46.0);
        studio.glow(540.0, 1380.0, 320.0, 90.0, CYAN, 0.25 * button);
        studio.rounded(area, 46.0, 0x0A84FF, 0.9 * button);
        studio.text("Unduh di GitHub", SANS, 44.0, true, 1380.0, 0xFFFFFF, button, 1.0);
    }
    studio.fade(span(t, 27.55, 0.45));
}

// ---------------------------------------------------------------- blur gerak

/// Saat ada gerakan cepat (zoom, geseran), beberapa sub-frame dirata-rata supaya terlihat
/// seperti blur gerak kamera sungguhan.
fn subframes(t: f64) -> usize {
    // Geseran tema paling cepat (ratusan piksel per sub-frame): butuh lebih banyak sampel
    // supaya blurnya halus, bukan bayangan bertumpuk.
    if (21.5..24.0).contains(&t) {
        return 24;
    }
    let fast = [(2.0, 2.9), (4.1, 4.6), (8.4, 8.6), (11.4, 11.6), (15.1, 16.5), (18.1, 18.5)];
    if fast.iter().any(|(a, b)| (*a..*b).contains(&t)) { 4 } else { 1 }
}

fn render(studio: &mut Studio, cast: &Cast, t: f64) -> Vec<u32> {
    let count = subframes(t);
    if count == 1 {
        return frame(studio, cast, t);
    }
    let shutter = 0.6 / FPS;
    let mut sum = vec![[0u32; 4]; (W * H) as usize];
    for i in 0..count {
        let sub = t + shutter * i as f64 / (count - 1) as f64;
        let pixels = frame(studio, cast, sub);
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

// ---------------------------------------------------------------- MP4

/// Pengode H.264 dan AAC bawaan Windows (Media Foundation).
struct Encoder {
    writer: IMFSinkWriter,
    stream: u32,
    sound: u32,
    /// Seluruh audio (PCM 16-bit stereo); potongan sepanjang satu frame ditulis bersama frame-nya.
    audio: Vec<i16>,
    frame: u64,
}

fn pack(a: u32, b: u32) -> u64 {
    ((a as u64) << 32) | b as u64
}

impl Encoder {
    fn new(path: &Path, audio: Vec<i16>) -> Self {
        unsafe {
            MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
            let mut attributes = None;
            MFCreateAttributes(&mut attributes, 2).unwrap();
            let attributes = attributes.unwrap();
            attributes.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1).unwrap();
            attributes.SetUINT32(&MF_SINK_WRITER_DISABLE_THROTTLING, 1).unwrap();
            let _ = std::fs::remove_file(path);
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from(path.as_os_str()), None, &attributes).unwrap();
            let output = MFCreateMediaType().unwrap();
            output.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            output.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264).unwrap();
            output.SetUINT32(&MF_MT_AVG_BITRATE, 14_000_000).unwrap();
            output.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).unwrap();
            output.SetUINT64(&MF_MT_FRAME_SIZE, pack(W as u32, H as u32)).unwrap();
            output.SetUINT64(&MF_MT_FRAME_RATE, pack(FPS as u32, 1)).unwrap();
            output.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1)).unwrap();
            let stream = writer.AddStream(&output).unwrap();
            let input = MFCreateMediaType().unwrap();
            input.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
            input.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).unwrap();
            input.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32).unwrap();
            input.SetUINT64(&MF_MT_FRAME_SIZE, pack(W as u32, H as u32)).unwrap();
            input.SetUINT64(&MF_MT_FRAME_RATE, pack(FPS as u32, 1)).unwrap();
            input.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1)).unwrap();
            // Baris pertama di atas (positif), sama seperti kanvas Direct2D.
            input.SetUINT32(&MF_MT_DEFAULT_STRIDE, (W * 4) as u32).unwrap();
            writer.SetInputMediaType(stream, &input, None).unwrap();
            let aac = MFCreateMediaType().unwrap();
            aac.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).unwrap();
            aac.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_AAC).unwrap();
            aac.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).unwrap();
            aac.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, musik::RATE).unwrap();
            aac.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).unwrap();
            // 192 kbps, salah satu laju yang diterima pengode AAC Windows; profil AAC-LC.
            aac.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, 24_000).unwrap();
            aac.SetUINT32(&MF_MT_AAC_AUDIO_PROFILE_LEVEL_INDICATION, 0x29).unwrap();
            let sound = writer.AddStream(&aac).unwrap();
            let pcm = MFCreateMediaType().unwrap();
            pcm.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).unwrap();
            pcm.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).unwrap();
            pcm.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).unwrap();
            pcm.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, musik::RATE).unwrap();
            pcm.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).unwrap();
            pcm.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4).unwrap();
            pcm.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, musik::RATE * 4).unwrap();
            writer.SetInputMediaType(sound, &pcm, None).unwrap();
            writer.BeginWriting().unwrap();
            Self { writer, stream, sound, audio, frame: 0 }
        }
    }

    fn write(&mut self, pixels: &[u32]) {
        let duration = (10_000_000.0 / FPS) as i64;
        let video = media_sample(as_bytes(pixels), self.frame as i64 * duration, duration);
        // Audio sepanjang frame ini (1600 sampel per kanal di 48 kHz).
        let per_frame = (musik::RATE as f64 / FPS) as usize;
        let first = (self.frame as usize * per_frame * 2).min(self.audio.len());
        let last = (first + per_frame * 2).min(self.audio.len());
        let clock = |samples: usize| (samples as i64 / 2) * 10_000_000 / musik::RATE as i64;
        unsafe {
            self.writer.WriteSample(self.stream, &video).unwrap();
            if last > first {
                let chunk = &self.audio[first..last];
                let bytes = std::slice::from_raw_parts(chunk.as_ptr().cast::<u8>(), chunk.len() * 2);
                let audio = media_sample(bytes, clock(first), clock(last) - clock(first));
                self.writer.WriteSample(self.sound, &audio).unwrap();
            }
        }
        self.frame += 1;
    }

    fn finish(self) {
        assert_eq!(self.frame as usize * (musik::RATE as f64 / FPS) as usize * 2, self.audio.len(), "audio sepanjang video");
        unsafe {
            self.writer.Finalize().unwrap();
            let _ = MFShutdown();
        }
    }
}

fn as_bytes(pixels: &[u32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(pixels.as_ptr().cast::<u8>(), pixels.len() * 4) }
}

/// Satu sampel Media Foundation berisi `bytes`, mulai `time` selama `duration` (satuan 100 ns).
fn media_sample(bytes: &[u8], time: i64, duration: i64) -> IMFSample {
    unsafe {
        let buffer = MFCreateMemoryBuffer(bytes.len() as u32).unwrap();
        let mut data = std::ptr::null_mut();
        buffer.Lock(&mut data, None, None).unwrap();
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len());
        buffer.Unlock().unwrap();
        buffer.SetCurrentLength(bytes.len() as u32).unwrap();
        let sample = MFCreateSample().unwrap();
        sample.AddBuffer(&buffer).unwrap();
        sample.SetSampleTime(time).unwrap();
        sample.SetSampleDuration(duration).unwrap();
        sample
    }
}

/// Membaca seluruh trek audio sebuah file (MP4, MP3, M4A, WAV) sebagai PCM 16-bit stereo 48 kHz;
/// Media Foundation yang mengubah laju sampel dan jumlah kanalnya kalau berbeda.
fn decode_audio(path: &Path) -> Vec<i16> {
    unsafe {
        MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
        let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), None).unwrap();
        let stream = MF_SOURCE_READER_FIRST_AUDIO_STREAM.0 as u32;
        reader.SetStreamSelection(MF_SOURCE_READER_ALL_STREAMS.0 as u32, false).unwrap();
        reader.SetStreamSelection(stream, true).unwrap();
        let wanted = MFCreateMediaType().unwrap();
        wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Audio).unwrap();
        wanted.SetGUID(&MF_MT_SUBTYPE, &MFAudioFormat_PCM).unwrap();
        wanted.SetUINT32(&MF_MT_AUDIO_BITS_PER_SAMPLE, 16).unwrap();
        wanted.SetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND, musik::RATE).unwrap();
        wanted.SetUINT32(&MF_MT_AUDIO_NUM_CHANNELS, 2).unwrap();
        wanted.SetUINT32(&MF_MT_AUDIO_BLOCK_ALIGNMENT, 4).unwrap();
        wanted.SetUINT32(&MF_MT_AUDIO_AVG_BYTES_PER_SECOND, musik::RATE * 4).unwrap();
        reader.SetCurrentMediaType(stream, None, &wanted).unwrap();
        let current = reader.GetCurrentMediaType(stream).unwrap();
        assert_eq!(current.GetUINT32(&MF_MT_AUDIO_SAMPLES_PER_SECOND).unwrap(), musik::RATE);
        assert_eq!(current.GetUINT32(&MF_MT_AUDIO_NUM_CHANNELS).unwrap(), 2);
        let mut pcm = Vec::new();
        loop {
            let mut flags = 0u32;
            let mut sample = None;
            reader.ReadSample(stream, 0, None, Some(&mut flags), None, Some(&mut sample)).unwrap();
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
            let Some(sample) = sample else { continue };
            let buffer = sample.ConvertToContiguousBuffer().unwrap();
            let mut data = std::ptr::null_mut();
            let mut length = 0u32;
            buffer.Lock(&mut data, None, Some(&mut length)).unwrap();
            pcm.extend_from_slice(std::slice::from_raw_parts(data.cast::<i16>(), length as usize / 2));
            buffer.Unlock().unwrap();
        }
        let _ = MFShutdown();
        pcm
    }
}

/// Baca ulang video jadi untuk dicek: frame ke-`indices` disimpan sebagai PNG.
fn decode_frames(path: &Path, indices: &[usize], out: &Path) -> usize {
    unsafe {
        MFStartup(MF_VERSION, MFSTARTUP_FULL).unwrap();
        let mut attributes = None;
        MFCreateAttributes(&mut attributes, 1).unwrap();
        let attributes = attributes.unwrap();
        attributes.SetUINT32(&MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING, 1).unwrap();
        let reader = MFCreateSourceReaderFromURL(&HSTRING::from(path.as_os_str()), &attributes).unwrap();
        let wanted = MFCreateMediaType().unwrap();
        wanted.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).unwrap();
        wanted.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).unwrap();
        let stream = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
        reader.SetCurrentMediaType(stream, None, &wanted).unwrap();
        let current = reader.GetCurrentMediaType(stream).unwrap();
        let size = current.GetUINT64(&MF_MT_FRAME_SIZE).unwrap();
        let (width, height) = ((size >> 32) as i32, (size & 0xFFFF_FFFF) as i32);
        let mut index = 0usize;
        loop {
            let mut flags = 0u32;
            let mut sample = None;
            reader.ReadSample(stream, 0, None, Some(&mut flags), None, Some(&mut sample)).unwrap();
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                break;
            }
            let Some(sample) = sample else { continue };
            if indices.contains(&index) {
                // Decoder H.264 bisa menyimpan baris dengan pitch-nya sendiri (lebar 1080
                // dibulatkan ke kelipatan 16), jadi pitch diambil dari buffernya, bukan dari lebar.
                let buffer = sample.ConvertToContiguousBuffer().unwrap();
                let flat = buffer.cast::<IMF2DBuffer>().ok();
                let mut first = std::ptr::null_mut();
                let mut pitch = 0i32;
                let mut length = 0u32;
                match &flat {
                    Some(flat) => flat.Lock2D(&mut first, &mut pitch).unwrap(),
                    None => {
                        buffer.Lock(&mut first, None, Some(&mut length)).unwrap();
                        pitch = (length / height as u32) as i32;
                    }
                }
                if index == indices[0] {
                    eprintln!("decode: {width}x{height}, pitch {pitch}, 2D {}, length {length}", flat.is_some());
                }
                let mut pixels = vec![0u32; (width * height) as usize];
                for y in 0..height as usize {
                    // Pitch negatif berarti baris berikutnya ada di alamat yang lebih kecil.
                    let row = first.offset(y as isize * pitch as isize).cast::<u32>();
                    let row = std::slice::from_raw_parts(row, width as usize);
                    for (x, px) in row.iter().enumerate() {
                        pixels[y * width as usize + x] = px | 0xFF00_0000;
                    }
                }
                match &flat {
                    Some(flat) => flat.Unlock2D().unwrap(),
                    None => buffer.Unlock().unwrap(),
                }
                save_png(&out.join(format!("frame-{index:04}.png")), &pixels, width, (0, 0, width, height));
            }
            index += 1;
        }
        let _ = MFShutdown();
        index
    }
}

// ---------------------------------------------------------------- tes

fn output_dir() -> std::path::PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("aset-video");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Satu frame per adegan, untuk dicek sebelum merender seluruh video.
#[test]
#[ignore]
fn video_cuplikan() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let dir = output_dir().join("cuplikan");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut studio = Studio::new();
    let cast = Cast::new();
    let times: [f64; 14] = [0.8, 1.4, 3.6, 5.2, 6.4, 9.2, 10.9, 12.6, 16.6, 19.3, 21.6, 22.43, 23.6, 26.0];
    for t in times {
        let pixels = render(&mut studio, &cast, t);
        save_png(&dir.join(format!("t{:05.2}.png", t)), &pixels, W, (0, 0, W, H));
    }
}

/// Video promosi lengkap: `aset-video/NotchWindows-story.mp4`.
#[test]
#[ignore]
fn video_promosi() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let dir = output_dir();
    let path = dir.join("NotchWindows-story.mp4");
    let audio = musik::soundtrack();
    musik::report(&audio, &musik::CUTS);
    let pcm = musik::pcm16(&audio);
    musik::save_wav(&dir.join("NotchWindows-story-musik.wav"), &pcm);
    let mut studio = Studio::new();
    let cast = Cast::new();
    let mut encoder = Encoder::new(&path, pcm);
    let frames = (LENGTH * FPS).round() as usize;
    for i in 0..frames {
        let pixels = render(&mut studio, &cast, i as f64 / FPS);
        encoder.write(&pixels);
    }
    encoder.finish();
    check_mp4();
}

/// Musiknya saja: WAV, gambar gelombang + spektrogram dengan garis potongan adegan, dan angka
/// kenyaringannya per adegan. Jauh lebih cepat daripada merender videonya.
#[test]
#[ignore]
fn video_musik() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let dir = output_dir();
    let audio = musik::soundtrack();
    musik::report(&audio, &musik::CUTS);
    musik::save_wav(&dir.join("NotchWindows-story-musik.wav"), &musik::pcm16(&audio));
    let (pixels, width, height) = musik::picture(&audio, &musik::CUTS);
    save_png(&dir.join("musik.png"), &pixels, width, (0, 0, width, height));
}

/// Membaca ulang beberapa frame dari MP4 yang sudah jadi ke `aset-video/cek-mp4/` dan
/// memastikan jumlah frame-nya lengkap.
#[test]
#[ignore]
fn video_cek_mp4() {
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    check_mp4();
}

fn check_mp4() {
    let dir = output_dir();
    let check = dir.join("cek-mp4");
    let _ = std::fs::remove_dir_all(&check);
    std::fs::create_dir_all(&check).unwrap();
    let video = dir.join("NotchWindows-story.mp4");
    let total = decode_frames(&video, &[15, 75, 195, 285, 390, 500, 610, 690, 790], &check);
    assert_eq!(total, (LENGTH * FPS).round() as usize, "jumlah frame di MP4");
    // Audionya lengkap dan sinkron dengan WAV aslinya (dicek di dentuman detik 2 dan 24).
    let source = musik::load_wav(&dir.join("NotchWindows-story-musik.wav"));
    let decoded = decode_audio(&video);
    let seconds = decoded.len() as f64 / 2.0 / musik::RATE as f64;
    let lags = [2.0, 15.0, 24.0].map(|at| musik::lag(&source, &decoded, at - 0.1));
    eprintln!("audio MP4: {seconds:.3} detik, geser {lags:?} sampel");
    assert!((seconds - LENGTH).abs() < 0.05, "panjang audio {seconds}");
    assert!(lags.iter().all(|lag| lag.abs() <= 48), "audio tidak sinkron: {lags:?}");
}
