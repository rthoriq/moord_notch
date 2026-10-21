//! Thumbnail file untuk shelf, diambil dari Windows (sama seperti yang tampil di File Explorer).
//! Dikerjakan di thread sendiri karena membuat thumbnail video/PDF bisa butuh waktu.

use std::path::{Path, PathBuf};
use std::thread;

use windows::Win32::Foundation::{E_FAIL, SIZE};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC,
    DeleteObject, GetDIBits, GetObjectW, HBITMAP,
};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize, IBindCtx};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_RESIZETOFIT};
use windows::core::{HSTRING, Result};

use crate::services::{Event, Notifier};

/// Sisi terpanjang thumbnail dalam piksel; tajam untuk 48 DIP sampai skala 200%.
const THUMB_PX: i32 = 96;

pub struct Thumbnail {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    /// BGRA premultiplied.
    pub pixels: Vec<u8>,
}

pub fn spawn(items: Vec<(u64, PathBuf)>, notify: Notifier) {
    if items.is_empty() {
        return;
    }
    let spawned = thread::Builder::new().name("thumbnail".into()).spawn(move || {
        let _ = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
        for (id, path) in items {
            match extract(&path) {
                Ok((width, height, pixels)) => notify.send(Event::Thumbnail(Thumbnail { id, width, height, pixels })),
                Err(e) => debug_log!("thumbnail {} gagal: {e}", path.display()),
            }
        }
        unsafe { CoUninitialize() };
    });
    if let Err(e) = spawned {
        error_log!("thread thumbnail gagal dibuat: {e}");
    }
}

fn extract(path: &Path) -> Result<(u32, u32, Vec<u8>)> {
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None::<&IBindCtx>)?;
        let bitmap = factory.GetImage(SIZE { cx: THUMB_PX, cy: THUMB_PX }, SIIGBF_RESIZETOFIT)?;
        let result = pixels(bitmap);
        let _ = DeleteObject(bitmap.into());
        result
    }
}

fn pixels(bitmap: HBITMAP) -> Result<(u32, u32, Vec<u8>)> {
    unsafe {
        let mut info = BITMAP::default();
        let size = size_of::<BITMAP>() as i32;
        if GetObjectW(bitmap.into(), size, Some((&mut info as *mut BITMAP).cast())) == 0 {
            return Err(E_FAIL.into());
        }
        let (width, height) = (info.bmWidth, info.bmHeight.abs());
        let mut header = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut px = vec![0u8; (width * height * 4) as usize];
        let dc = CreateCompatibleDC(None);
        let lines = GetDIBits(dc, bitmap, 0, height as u32, Some(px.as_mut_ptr().cast()), &mut header, DIB_RGB_COLORS);
        let _ = DeleteDC(dc);
        if lines == 0 {
            return Err(E_FAIL.into());
        }
        fix_alpha(&mut px);
        Ok((width as u32, height as u32, px))
    }
}

/// Foto biasanya tanpa alfa (semua 0) → jadikan pekat. Ikon kadang beralfa lurus →
/// ubah ke premultiplied seperti yang diminta Direct2D.
fn fix_alpha(px: &mut [u8]) {
    if px.chunks_exact(4).all(|p| p[3] == 0) {
        px.chunks_exact_mut(4).for_each(|p| p[3] = 255);
    } else if px.chunks_exact(4).any(|p| p[0] > p[3] || p[1] > p[3] || p[2] > p[3]) {
        for p in px.chunks_exact_mut(4) {
            let a = p[3] as u32;
            for c in &mut p[..3] {
                *c = (*c as u32 * a / 255) as u8;
            }
        }
    }
}
