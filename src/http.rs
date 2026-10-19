//! Permintaan HTTPS GET sederhana lewat WinHTTP, untuk cuaca. Klien HTTP WinRT menolak rantai
//! sertifikat Let's Encrypt yang baru (penerbit "YR1") walau sertifikatnya sah, sedangkan WinHTTP
//! memakai Schannel seperti curl bawaan Windows dan menerimanya. Sertifikat tetap diperiksa penuh,
//! termasuk apakah sudah dicabut (revocation).

use std::ffi::c_void;

use windows::Win32::Foundation::{E_FAIL, E_INVALIDARG};
use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_ENABLE_SSL_REVOCATION,
    WINHTTP_FLAG_SECURE, WINHTTP_OPTION_ENABLE_FEATURE, WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE,
    WinHttpCloseHandle, WinHttpConnect, WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData,
    WinHttpReceiveResponse, WinHttpSendRequest, WinHttpSetOption, WinHttpSetTimeouts,
};
use windows::core::{Error, HSTRING, PCWSTR, Result, w};

/// Jawaban sebesar ini sudah pasti bukan data cuaca.
const MAX_BODY: usize = 1 << 20;

/// Handle WinHTTP yang ditutup otomatis.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> Result<Self> {
        if raw.is_null() { Err(Error::from_thread()) } else { Ok(Self(raw)) }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

/// Isi `url` (harus https) sebagai teks; gagal kalau status bukan 200.
pub fn get(url: &str, agent: &str) -> Result<String> {
    let rest = url.strip_prefix("https://").ok_or_else(|| Error::from(E_INVALIDARG))?;
    let (host, path) = match rest.split_once('/') {
        Some((host, path)) => (host, format!("/{path}")),
        None => (rest, "/".to_string()),
    };
    unsafe {
        let proxy = WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY;
        let session = Handle::new(WinHttpOpen(&HSTRING::from(agent), proxy, PCWSTR::null(), PCWSTR::null(), 0))?;
        let _ = WinHttpSetTimeouts(session.0, 10_000, 10_000, 10_000, 15_000);
        let connect = Handle::new(WinHttpConnect(session.0, &HSTRING::from(host), INTERNET_DEFAULT_HTTPS_PORT, 0))?;
        let path = HSTRING::from(path);
        let request = Handle::new(WinHttpOpenRequest(
            connect.0,
            w!("GET"),
            &path,
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        let revocation = WINHTTP_ENABLE_SSL_REVOCATION.to_ne_bytes();
        WinHttpSetOption(Some(request.0), WINHTTP_OPTION_ENABLE_FEATURE, Some(&revocation))?;
        WinHttpSendRequest(request.0, None, None, 0, 0, 0)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut())?;

        let (mut status, mut size) = (0u32, size_of::<u32>() as u32);
        let query = WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER;
        let buffer = Some((&mut status as *mut u32).cast());
        WinHttpQueryHeaders(request.0, query, PCWSTR::null(), buffer, &mut size, std::ptr::null_mut())?;
        if status != 200 {
            return Err(Error::new(E_FAIL, format!("HTTP {status}")));
        }

        let mut body = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let mut read = 0u32;
            WinHttpReadData(request.0, chunk.as_mut_ptr().cast(), chunk.len() as u32, &mut read)?;
            if read == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..read as usize]);
            if body.len() > MAX_BODY {
                return Err(Error::new(E_FAIL, "jawaban terlalu besar"));
            }
        }
        String::from_utf8(body).map_err(|_| Error::new(E_FAIL, "jawaban bukan UTF-8"))
    }
}
