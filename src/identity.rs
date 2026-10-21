//! Identitas paket untuk membaca notifikasi pesan (opsional). Windows hanya memberi akses
//! notifikasi ke aplikasi yang punya identitas paket. Notch mendapatkannya lewat paket "sparse":
//! paket kecil berisi manifest dan logo saja (tanpa program) yang menunjuk ke notch.exe di folder
//! pemasangan.
//!
//! Paket itu harus ditandatangani sertifikat yang dipercaya Windows. Tanpa sertifikat berbayar,
//! notch membuatnya sendiri di komputer ini, dalam satu langkah yang butuh Administrator:
//! 1. kunci RSA baru dibuat di penyimpanan kunci Windows dengan nama acak,
//! 2. sertifikat "CN=NotchWindows Pribadi" dibuat dengan kunci itu, lalu dipakai menandatangani
//!    paket yang juga dibuat di tempat dari manifest dan logo di dalam notch.exe,
//! 3. kuncinya langsung dihapus (juga kalau ada langkah yang gagal), jadi sertifikat itu tidak
//!    bisa menandatangani apa pun lagi,
//! 4. sertifikatnya (tanpa kunci) dipercaya di LocalMachine\TrustedPeople.
//!
//! Langkah Administrator itu tidak menerima masukan apa pun dari luar (tidak ada file atau
//! sertifikat yang dibaca), supaya tidak bisa dipakai program lain untuk memercayakan sesuatu.
//! Paketnya lalu didaftarkan untuk pengguna saat ini, tanpa Administrator.

use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};

use windows::Foundation::Uri;
use windows::Management::Deployment::{AddPackageOptions, PackageManager};
use windows::Win32::Foundation::{HANDLE, HWND, SYSTEMTIME};
use windows::Win32::Security::Cryptography::{
    BCRYPT_RSA_ALGORITHM, CALG_SHA_256, CERT_BASIC_CONSTRAINTS2_INFO, CERT_CONTEXT,
    CERT_CREATE_SELFSIGN_FLAGS, CERT_EXTENSION, CERT_EXTENSIONS, CERT_FIND_SUBJECT_STR,
    CERT_KEY_SPEC, CERT_OPEN_STORE_FLAGS, CRYPT_KEY_PROV_INFO,
    CERT_QUERY_ENCODING_TYPE, CERT_STORE_ADD_REPLACE_EXISTING, CERT_STORE_PROV_SYSTEM_W,
    CERT_STORE_READONLY_FLAG, CERT_SYSTEM_STORE_LOCAL_MACHINE, CERT_X500_NAME_STR,
    CRYPT_ALGORITHM_IDENTIFIER, CRYPT_ENCODE_OBJECT_FLAGS, CRYPT_INTEGER_BLOB, CTL_USAGE,
    CertAddEncodedCertificateToStore, CertCloseStore, CertCreateSelfSignCertificate,
    CertDeleteCertificateFromStore, CertFindCertificateInStore,
    CertFreeCertificateContext, CertOpenStore, CertStrToNameW,
    CryptEncodeObjectEx, HCERTSTORE, HCRYPTPROV_OR_NCRYPT_KEY_HANDLE, MS_KEY_STORAGE_PROVIDER,
    NCRYPT_FLAGS, NCRYPT_HANDLE, NCRYPT_KEY_HANDLE, NCRYPT_LENGTH_PROPERTY, NCRYPT_PROV_HANDLE,
    NCryptCreatePersistedKey, NCryptDeleteKey, NCryptFinalizeKey, NCryptFreeObject, NCryptOpenStorageProvider,
    NCryptSetProperty, SIGNER_CERT, SIGNER_CERT_0, SIGNER_CERT_POLICY_CHAIN_NO_ROOT,
    SIGNER_CERT_STORE, SIGNER_CERT_STORE_INFO, SIGNER_CONTEXT, SIGNER_FILE_INFO, SIGNER_NO_ATTR,
    SIGNER_SIGN_FLAGS, SIGNER_SIGNATURE_INFO, SIGNER_SUBJECT_FILE, SIGNER_SUBJECT_INFO,
    SIGNER_SUBJECT_INFO_0, SignerFreeSignerContext, SignerSignEx2, X509_ASN_ENCODING,
    X509_BASIC_CONSTRAINTS2, X509_ENHANCED_KEY_USAGE, szOID_BASIC_CONSTRAINTS2,
    szOID_ENHANCED_KEY_USAGE, szOID_PKIX_KP_CODE_SIGNING, szOID_RSA_SHA256RSA,
};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
use windows::Win32::Storage::Packaging::Appx::{
    APPX_COMPRESSION_OPTION_NORMAL, APPX_PACKAGE_SETTINGS, AppxFactory, IAppxFactory,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, CoCreateInstance, CreateUri, STGM_SHARE_EXCLUSIVE, STGM_WRITE, Uri_CREATE_CANONICALIZE,
};
use windows::Win32::System::SystemInformation::GetSystemTime;
use windows::Win32::UI::Shell::{SHCreateMemStream, SHCreateStreamOnFileEx};
use windows::core::{Error, HSTRING, IUnknown, Interface, PCSTR, PCWSTR, PSTR, PWSTR, Result, w};

/// Penerbit paket; sama dengan `publisher` di packaging/notch.exe.manifest dan di manifest paket.
pub const SUBJECT: &str = "CN=NotchWindows Pribadi";
const PACKAGE_NAME: &str = "NotchWindows";
const MANIFEST: &str = include_str!("../packaging/package/AppxManifest.xml");
/// Paket yang sudah ditandatangani, di folder pemasangan.
pub const PACKAGE_FILE: &str = "identitas.msix";
/// Salinan sementara notch yang mendaftarkan paket. Notch yang sedang berjalan dari folder
/// pemasangan bisa saja memegang identitas paket lama, jadi tidak boleh mengganti paket itu sendiri.
pub const REGISTER_COPY: &str = "notch-daftar-identitas.exe";
/// Logo yang dirujuk manifest; ikut ke dalam paket dan ke folder pemasangan.
pub const ASSETS: [(&str, &[u8]); 3] = [
    ("Square44x44Logo.png", include_bytes!("../packaging/package/assets/Square44x44Logo.png")),
    ("Square150x150Logo.png", include_bytes!("../packaging/package/assets/Square150x150Logo.png")),
    ("StoreLogo.png", include_bytes!("../packaging/package/assets/StoreLogo.png")),
];

/// Langkah Administrator: buat paket di folder notch.exe ini, tandatangani dengan kunci sekali
/// pakai, lalu percayai sertifikatnya. Tidak membaca masukan apa pun dari luar.
pub fn prepare() -> Result<()> {
    let dir = exe_dir()?;
    write_assets(&dir).map_err(io_error)?;
    // Paket dibuat dan ditandatangani di folder Temp milik Windows dengan nama acak. File yang
    // dibuat proses Administrator di sana tidak bisa diubah atau dihapus program biasa, jadi
    // paketnya tidak bisa ditukar sebelum ditandatangani. Baru sesudahnya disalin ke folder
    // pemasangan (paket bertanda tangan yang diubah akan ditolak Windows).
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let work = windows_temp()?.join(format!("notch-identitas-{}-{nanos:x}.msix", std::process::id()));
    let result = (|| {
        build_package(&work, &manifest(crate::install::VERSION))?;
        let key = Key::new()?;
        let cert = Certificate::new(&key)?;
        sign(&work, cert.0)?;
        let encoded = cert.encoded();
        // Kunci dihapus sebelum sertifikatnya dipercaya.
        drop(cert);
        key.delete()?;
        std::fs::copy(&work, dir.join(PACKAGE_FILE)).map_err(io_error)?;
        trust(&encoded)
    })();
    let _ = std::fs::remove_file(&work);
    result
}

/// `C:\Windows\Temp`, dari API Windows (bukan variabel lingkungan yang bisa diubah pengguna).
fn windows_temp() -> Result<PathBuf> {
    let mut buffer = [0u16; 260];
    let len = unsafe { windows::Win32::System::SystemInformation::GetSystemWindowsDirectoryW(Some(&mut buffer)) } as usize;
    if len == 0 || len >= buffer.len() {
        return Err(Error::from_thread());
    }
    Ok(PathBuf::from(String::from_utf16_lossy(&buffer[..len])).join("Temp"))
}

fn io_error(e: std::io::Error) -> Error {
    Error::new(windows::Win32::Foundation::E_FAIL, e.to_string())
}

/// Daftarkan paket di `dir` untuk pengguna saat ini, menunjuk ke notch.exe di folder yang sama.
/// Paket lama dengan nama yang sama diganti.
pub fn register(dir: &Path) -> Result<()> {
    let manager = PackageManager::new()?;
    let options = AddPackageOptions::new()?;
    options.SetExternalLocationUri(&Uri::CreateUri(&HSTRING::from(file_uri(dir)))?)?;
    options.SetForceUpdateFromAnyVersion(true)?;
    let package = Uri::CreateUri(&HSTRING::from(file_uri(&dir.join(PACKAGE_FILE))))?;
    let result = manager.AddPackageByUriAsync(&package, &options)?.join()?;
    let code = result.ExtendedErrorCode()?;
    if code.is_err() {
        return Err(Error::new(code, result.ErrorText()?.to_string()));
    }
    Ok(())
}

/// Cabut paket identitas milik pengguna saat ini, tapi hanya yang menunjuk ke `dir` (folder
/// pemasangan yang sedang dicabut). Paket untuk notch di folder lain tidak disentuh.
pub fn remove_package(dir: &Path) -> Result<()> {
    let manager = PackageManager::new()?;
    let packages = manager.FindPackagesByUserSecurityIdNamePublisher(&HSTRING::new(), &HSTRING::from(PACKAGE_NAME), &HSTRING::from(SUBJECT))?;
    for package in packages {
        // Windows lama tanpa keterangan letak luar: lebih aman dibiarkan.
        let Ok(location) = package.EffectiveExternalPath() else { continue };
        let ours = location.to_string_lossy().trim_end_matches('\\').eq_ignore_ascii_case(dir.to_string_lossy().trim_end_matches('\\'));
        if !ours {
            continue;
        }
        let name = package.Id()?.FullName()?;
        let result = manager.RemovePackageAsync(&name)?.join()?;
        result.ExtendedErrorCode()?.ok()?;
    }
    Ok(())
}

/// Masih ada paket identitas notch untuk pengguna ini (misal notch di folder lain).
pub fn any_package() -> bool {
    let found = PackageManager::new().and_then(|manager| {
        manager.FindPackagesByUserSecurityIdNamePublisher(&HSTRING::new(), &HSTRING::from(PACKAGE_NAME), &HSTRING::from(SUBJECT))
    });
    found.is_ok_and(|packages| packages.First().is_ok_and(|first| first.HasCurrent().unwrap_or(false)))
}

/// Sertifikat notch sudah dipercaya di komputer ini.
pub fn certificate_trusted() -> bool {
    let Ok(store) = Store::open(true) else { return false };
    let found = store.find(None);
    if !found.is_null() {
        let _ = unsafe { CertFreeCertificateContext(Some(found)) };
    }
    !found.is_null()
}

/// Langkah Administrator saat dicabut: buang semua sertifikat notch dari daftar terpercaya.
pub fn remove_certificates() -> Result<()> {
    let store = Store::open(false)?;
    loop {
        let found = store.find(None);
        if found.is_null() {
            return Ok(());
        }
        // CertDeleteCertificateFromStore selalu melepas konteks yang diberikan.
        unsafe { CertDeleteCertificateFromStore(found)? };
    }
}

/// Logo yang dirujuk manifest ditaruh di folder pemasangan (dibaca Windows dari sana).
pub fn write_assets(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("assets"))?;
    for (name, bytes) in ASSETS {
        std::fs::write(dir.join("assets").join(name), bytes)?;
    }
    Ok(())
}

fn exe_dir() -> Result<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .ok_or_else(|| Error::new(windows::Win32::Foundation::E_FAIL, "folder notch.exe tidak diketahui"))
}

/// Manifest paket dengan versi notch ini ("0.10.0" → "0.10.0.0"), supaya paket baru selalu
/// menggantikan yang lama.
fn manifest(version: &str) -> String {
    let mut parts: Vec<&str> = version.split('.').collect();
    parts.resize(4, "0");
    let version = parts.join(".");
    let Some(start) = MANIFEST.find("<Identity") else { return MANIFEST.to_string() };
    let identity_end = MANIFEST[start..].find("/>").map_or(MANIFEST.len(), |i| start + i);
    let Some(attr) = MANIFEST[start..identity_end].find("Version=\"").map(|i| start + i + "Version=\"".len()) else {
        return MANIFEST.to_string();
    };
    let Some(close) = MANIFEST[attr..].find('"').map(|i| attr + i) else { return MANIFEST.to_string() };
    format!("{}{version}{}", &MANIFEST[..attr], &MANIFEST[close..])
}

/// "file:///C:/Users/…/NotchWindows" untuk API paket Windows.
fn file_uri(path: &Path) -> String {
    let text = path.to_string_lossy().replace('%', "%25").replace(' ', "%20").replace('#', "%23").replace('\\', "/");
    format!("file:///{}", text.trim_start_matches('/'))
}

/// Paket berisi manifest dan logo, dibuat lewat API pengemasan bawaan Windows.
fn build_package(path: &Path, manifest: &str) -> Result<()> {
    unsafe {
        let factory: IAppxFactory = CoCreateInstance(&AppxFactory, None, CLSCTX_INPROC_SERVER)?;
        // File baru saja (gagal kalau sudah ada), tanpa berbagi akses selama ditulis.
        let output = SHCreateStreamOnFileEx(
            &HSTRING::from(path.as_os_str()),
            (STGM_WRITE | STGM_SHARE_EXCLUSIVE).0,
            FILE_ATTRIBUTE_NORMAL.0,
            true,
            None::<&windows::Win32::System::Com::IStream>,
        )?;
        let hash = CreateUri(w!("http://www.w3.org/2001/04/xmlenc#sha256"), Uri_CREATE_CANONICALIZE, None)?;
        let mut settings = APPX_PACKAGE_SETTINGS { forceZip32: false.into(), hashMethod: ManuallyDrop::new(Some(hash)) };
        let written = (|| {
            let writer = factory.CreatePackageWriter(&output, &settings)?;
            for (name, bytes) in ASSETS {
                let stream = SHCreateMemStream(Some(bytes)).ok_or_else(Error::empty)?;
                let name = HSTRING::from(format!("assets\\{name}"));
                writer.AddPayloadFile(&name, w!("image/png"), APPX_COMPRESSION_OPTION_NORMAL, &stream)?;
            }
            let manifest = SHCreateMemStream(Some(manifest.as_bytes())).ok_or_else(Error::empty)?;
            writer.Close(&manifest)
        })();
        ManuallyDrop::drop(&mut settings.hashMethod);
        written
    }
}

/// Kunci RSA sekali pakai di penyimpanan kunci Windows (penanda tangan paket Windows hanya mau
/// kunci yang punya nama). Kunci ini selalu dihapus lagi: lewat `delete`, atau saat dilepas kalau
/// ada langkah yang gagal di tengah jalan.
struct Key {
    handle: NCRYPT_KEY_HANDLE,
    name: HSTRING,
}

impl Key {
    fn new() -> Result<Self> {
        // Nama acak supaya tidak pernah bertabrakan dengan kunci lain.
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let name = HSTRING::from(format!("NotchWindows-{}-{nanos:x}", std::process::id()));
        unsafe {
            let mut provider = NCRYPT_PROV_HANDLE::default();
            NCryptOpenStorageProvider(&mut provider, MS_KEY_STORAGE_PROVIDER, 0)?;
            let mut handle = NCRYPT_KEY_HANDLE::default();
            let created = NCryptCreatePersistedKey(provider, &mut handle, BCRYPT_RSA_ALGORITHM, &name, CERT_KEY_SPEC(0), NCRYPT_FLAGS(0));
            let _ = NCryptFreeObject(NCRYPT_HANDLE(provider.0));
            created?;
            let key = Self { handle, name };
            NCryptSetProperty(NCRYPT_HANDLE(handle.0), NCRYPT_LENGTH_PROPERTY, &3072u32.to_le_bytes(), NCRYPT_FLAGS(0))?;
            NCryptFinalizeKey(handle, NCRYPT_FLAGS(0))?;
            Ok(key)
        }
    }

    /// Hapus kunci dari penyimpanan kunci Windows.
    fn delete(self) -> Result<()> {
        let handle = self.handle;
        std::mem::forget(self);
        // NCryptDeleteKey sekaligus melepas handle-nya.
        unsafe { NCryptDeleteKey(handle, 0) }
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        let _ = unsafe { NCryptDeleteKey(self.handle, 0) };
    }
}

/// Sertifikat tanda tangan kode "CN=NotchWindows Pribadi" dengan kunci sekali pakai. Harus
/// dilepas sebelum kuncinya dihapus.
struct Certificate(*const CERT_CONTEXT);

impl Certificate {
    fn new(key: &Key) -> Result<Self> {
        unsafe {
            let subject = HSTRING::from(SUBJECT);
            let mut size = 0u32;
            CertStrToNameW(X509_ASN_ENCODING, &subject, CERT_X500_NAME_STR, None, None, &mut size, None)?;
            let mut name = vec![0u8; size as usize];
            CertStrToNameW(X509_ASN_ENCODING, &subject, CERT_X500_NAME_STR, None, Some(name.as_mut_ptr()), &mut size, None)?;
            let name_blob = CRYPT_INTEGER_BLOB { cbData: size, pbData: name.as_mut_ptr() };

            // Hanya untuk menandatangani kode, dan bukan sertifikat induk (CA).
            let mut purposes = [PSTR(szOID_PKIX_KP_CODE_SIGNING.0.cast_mut())];
            let usage = CTL_USAGE { cUsageIdentifier: 1, rgpszUsageIdentifier: purposes.as_mut_ptr() };
            let mut usage_der = encode(X509_ENHANCED_KEY_USAGE, (&raw const usage).cast())?;
            let basic = CERT_BASIC_CONSTRAINTS2_INFO { fCA: false.into(), fPathLenConstraint: false.into(), dwPathLenConstraint: 0 };
            let mut basic_der = encode(X509_BASIC_CONSTRAINTS2, (&raw const basic).cast())?;
            let mut extensions = [
                CERT_EXTENSION { pszObjId: PSTR(szOID_ENHANCED_KEY_USAGE.0.cast_mut()), fCritical: false.into(), Value: blob(&mut usage_der) },
                CERT_EXTENSION { pszObjId: PSTR(szOID_BASIC_CONSTRAINTS2.0.cast_mut()), fCritical: true.into(), Value: blob(&mut basic_der) },
            ];
            let list = CERT_EXTENSIONS { cExtension: extensions.len() as u32, rgExtension: extensions.as_mut_ptr() };
            let algorithm = CRYPT_ALGORITHM_IDENTIFIER { pszObjId: PSTR(szOID_RSA_SHA256RSA.0.cast_mut()), Parameters: CRYPT_INTEGER_BLOB::default() };

            // Berlaku sejak setahun lalu (jam yang meleset tidak membuatnya "belum berlaku")
            // sampai 20 tahun lagi.
            let now = GetSystemTime();
            let at = |years: i32| SYSTEMTIME { wYear: (now.wYear as i32 + years) as u16, wDay: now.wDay.min(28), ..now };
            let (start, end) = (at(-1), at(20));
            // Letak kunci privat yang dicatat di sertifikat, supaya penanda tangan bisa memakainya.
            let provider = HSTRING::from(MS_KEY_STORAGE_PROVIDER.to_string().unwrap_or_default());
            let key_info = CRYPT_KEY_PROV_INFO {
                pwszContainerName: PWSTR(key.name.as_ptr().cast_mut()),
                pwszProvName: PWSTR(provider.as_ptr().cast_mut()),
                ..Default::default()
            };
            let context = CertCreateSelfSignCertificate(
                Some(HCRYPTPROV_OR_NCRYPT_KEY_HANDLE(key.handle.0)),
                &name_blob,
                CERT_CREATE_SELFSIGN_FLAGS(0),
                Some(&key_info),
                Some(&algorithm),
                Some(&start),
                Some(&end),
                Some(&list),
            );
            if context.is_null() {
                return Err(Error::from_thread());
            }
            Ok(Self(context))
        }
    }

    /// Isi sertifikat (bagian publik saja).
    fn encoded(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts((*self.0).pbCertEncoded, (*self.0).cbCertEncoded as usize).to_vec() }
    }
}

impl Drop for Certificate {
    fn drop(&mut self) {
        let _ = unsafe { CertFreeCertificateContext(Some(self.0)) };
    }
}

fn encode(kind: PCSTR, value: *const c_void) -> Result<Vec<u8>> {
    unsafe {
        let mut size = 0u32;
        CryptEncodeObjectEx(X509_ASN_ENCODING, kind, value, CRYPT_ENCODE_OBJECT_FLAGS(0), None, None, &mut size)?;
        let mut bytes = vec![0u8; size as usize];
        CryptEncodeObjectEx(X509_ASN_ENCODING, kind, value, CRYPT_ENCODE_OBJECT_FLAGS(0), None, Some(bytes.as_mut_ptr().cast()), &mut size)?;
        bytes.truncate(size as usize);
        Ok(bytes)
    }
}

fn blob(bytes: &mut [u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB { cbData: bytes.len() as u32, pbData: bytes.as_mut_ptr() }
}

/// SIGNER_SIGN_EX2_PARAMS dan APPX_SIP_CLIENT_DATA dari mssign32.h / appxsip.h (belum ada di
/// crate `windows`): data tambahan yang diminta Windows untuk menandatangani paket MSIX.
#[repr(C)]
struct SignerSignEx2Params {
    flags: u32,
    subject: *const SIGNER_SUBJECT_INFO,
    cert: *const SIGNER_CERT,
    signature: *const SIGNER_SIGNATURE_INFO,
    provider: *const c_void,
    timestamp_flags: u32,
    algorithm_oid: PCSTR,
    timestamp_url: PCWSTR,
    attributes: *const c_void,
    sip_data: *const c_void,
    context: *mut *mut SIGNER_CONTEXT,
    crypto_policy: *const c_void,
    reserved: *const c_void,
}

#[repr(C)]
struct AppxSipClientData {
    params: *mut SignerSignEx2Params,
    state: *mut c_void,
}

/// Tandatangani paket dengan SHA-256, sama seperti `signtool sign /fd SHA256`.
fn sign(path: &Path, cert: *const CERT_CONTEXT) -> Result<()> {
    unsafe {
        let file = HSTRING::from(path.as_os_str());
        let mut index = 0u32;
        let mut file_info = SIGNER_FILE_INFO {
            cbSize: size_of::<SIGNER_FILE_INFO>() as u32,
            pwszFileName: PCWSTR(file.as_ptr()),
            hFile: HANDLE::default(),
        };
        let subject = SIGNER_SUBJECT_INFO {
            cbSize: size_of::<SIGNER_SUBJECT_INFO>() as u32,
            pdwIndex: &mut index,
            dwSubjectChoice: SIGNER_SUBJECT_FILE,
            Anonymous: SIGNER_SUBJECT_INFO_0 { pSignerFileInfo: &mut file_info },
        };
        let mut store_info = SIGNER_CERT_STORE_INFO {
            cbSize: size_of::<SIGNER_CERT_STORE_INFO>() as u32,
            pSigningCert: cert,
            dwCertPolicy: SIGNER_CERT_POLICY_CHAIN_NO_ROOT,
            hCertStore: HCERTSTORE::default(),
        };
        let signer = SIGNER_CERT {
            cbSize: size_of::<SIGNER_CERT>() as u32,
            dwCertChoice: SIGNER_CERT_STORE,
            Anonymous: SIGNER_CERT_0 { pCertStoreInfo: &mut store_info },
            hwnd: HWND::default(),
        };
        let signature = SIGNER_SIGNATURE_INFO {
            cbSize: size_of::<SIGNER_SIGNATURE_INFO>() as u32,
            algidHash: CALG_SHA_256,
            dwAttrChoice: SIGNER_NO_ATTR,
            ..Default::default()
        };
        let mut context_data: *mut SIGNER_CONTEXT = std::ptr::null_mut();
        let context = &raw mut context_data;
        // Kedua struktur saling menunjuk, jadi hanya diakses lewat pointer mentah.
        let mut sip_data = AppxSipClientData { params: std::ptr::null_mut(), state: std::ptr::null_mut() };
        let mut params_data = SignerSignEx2Params {
            flags: 0,
            subject: &subject,
            cert: &signer,
            signature: &signature,
            provider: std::ptr::null(),
            timestamp_flags: 0,
            algorithm_oid: PCSTR::null(),
            timestamp_url: PCWSTR::null(),
            attributes: std::ptr::null(),
            sip_data: std::ptr::null(),
            context,
            crypto_policy: std::ptr::null(),
            reserved: std::ptr::null(),
        };
        let (sip, params) = (&raw mut sip_data, &raw mut params_data);
        (*sip).params = params;
        (*params).sip_data = sip.cast_const().cast();
        let result = SignerSignEx2(
            SIGNER_SIGN_FLAGS(0),
            &subject,
            &signer,
            &signature,
            None,
            None,
            PCSTR::null(),
            PCWSTR::null(),
            None,
            Some(sip.cast_const().cast()),
            context,
            None,
            None,
        );
        if !(*sip).state.is_null() {
            drop(IUnknown::from_raw((*sip).state));
        }
        if !(*context).is_null() {
            let _ = SignerFreeSignerContext(*context);
        }
        result
    }
}

/// Percayai sertifikat (bagian publiknya saja) untuk paket di komputer ini.
fn trust(encoded: &[u8]) -> Result<()> {
    let store = Store::open(false)?;
    unsafe { CertAddEncodedCertificateToStore(Some(store.0), X509_ASN_ENCODING, encoded, CERT_STORE_ADD_REPLACE_EXISTING, None) }
}

/// Daftar "Orang Terpercaya" milik komputer (LocalMachine\TrustedPeople), tempat Windows mencari
/// penerbit paket yang dipasang di luar Microsoft Store.
struct Store(HCERTSTORE);

impl Store {
    fn open(read_only: bool) -> Result<Self> {
        let mut flags = CERT_SYSTEM_STORE_LOCAL_MACHINE;
        if read_only {
            flags |= CERT_STORE_READONLY_FLAG.0;
        }
        let store = unsafe {
            CertOpenStore(
                CERT_STORE_PROV_SYSTEM_W,
                CERT_QUERY_ENCODING_TYPE(0),
                None,
                CERT_OPEN_STORE_FLAGS(flags),
                Some(w!("TrustedPeople").as_ptr().cast()),
            )?
        };
        Ok(Self(store))
    }

    /// Sertifikat notch berikutnya sesudah `previous` (yang ikut dilepas); null kalau tidak ada.
    fn find(&self, previous: Option<*const CERT_CONTEXT>) -> *const CERT_CONTEXT {
        // Nama yang dicari tanpa "CN=", karena pencarian ini memakai nama yang terbaca.
        let name = HSTRING::from(SUBJECT.trim_start_matches("CN="));
        let found = unsafe {
            CertFindCertificateInStore(self.0, X509_ASN_ENCODING, 0, CERT_FIND_SUBJECT_STR, Some(name.as_ptr().cast()), previous)
        };
        // Pencarian ini mencocokkan sebagian nama: pastikan namanya persis sama.
        if !found.is_null() && !exact_subject(found) {
            return self.find(Some(found));
        }
        found
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = unsafe { CertCloseStore(Some(self.0), 0) };
    }
}

fn exact_subject(cert: *const CERT_CONTEXT) -> bool {
    use windows::Win32::Security::Cryptography::{CERT_NAME_SIMPLE_DISPLAY_TYPE, CertGetNameStringW};
    let mut buffer = [0u16; 128];
    let len = unsafe { CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, None, Some(&mut buffer)) };
    let name = String::from_utf16_lossy(&buffer[..(len as usize).saturating_sub(1)]);
    name == SUBJECT.trim_start_matches("CN=")
}

/// Salinan konteks sertifikat (untuk tes).
#[cfg(test)]
fn duplicate(cert: *const CERT_CONTEXT) -> *const CERT_CONTEXT {
    unsafe { windows::Win32::Security::Cryptography::CertDuplicateCertificateContext(Some(cert)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Penerbit di manifest paket dan di manifest notch.exe harus sama persis, kalau tidak
    /// Windows tidak memberi identitas.
    #[test]
    fn publishers_match() {
        let exe_manifest = include_str!("../packaging/notch.exe.manifest");
        assert!(MANIFEST.contains(&format!("Publisher=\"{SUBJECT}\"")));
        assert!(exe_manifest.contains(&format!("publisher=\"{SUBJECT}\"")));
        assert!(MANIFEST.contains(&format!("Name=\"{PACKAGE_NAME}\"")));
        assert!(exe_manifest.contains(&format!("packageName=\"{PACKAGE_NAME}\"")));
    }

    #[test]
    fn manifest_follows_app_version() {
        let text = manifest("0.10.0");
        let identity = &text[text.find("<Identity").unwrap()..];
        assert!(identity[..identity.find("/>").unwrap()].contains("Version=\"0.10.0.0\""), "{identity}");
        // Hanya versi paket yang berubah, bukan atribut lain yang kebetulan bernama Version.
        assert_eq!(text.matches("0.10.0.0").count(), 1);
    }

    #[test]
    fn file_uris() {
        assert_eq!(file_uri(Path::new(r"C:\Users\Budi Santoso\AppData\Local\Programs\NotchWindows")), "file:///C:/Users/Budi%20Santoso/AppData/Local/Programs/NotchWindows");
    }

    /// Paket dibuat dan ditandatangani dengan kunci sekali pakai, tanpa mengubah apa pun di
    /// komputer (tidak ada sertifikat yang dipercaya, tidak ada kunci yang disimpan). Hasilnya
    /// diperiksa dengan PowerShell: tanda tangannya utuh, hanya penerbitnya belum dipercaya.
    #[test]
    #[ignore]
    fn builds_and_signs_package() {
        let _ = unsafe { windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED) };
        let dir = std::env::temp_dir().join("notch-uji-identitas");
        std::fs::create_dir_all(&dir).unwrap();
        let package = dir.join(PACKAGE_FILE);
        // Paket hanya dibuat sebagai file baru (lihat `build_package`).
        let _ = std::fs::remove_file(&package);
        build_package(&package, &manifest("1.2.3")).unwrap();
        assert!(build_package(&package, &manifest("1.2.3")).is_err(), "file yang sudah ada tidak boleh ditimpa");
        let _ = std::fs::remove_file(&package);
        build_package(&package, &manifest("1.2.3")).unwrap();
        let key = Key::new().unwrap();
        let name = key.name.clone();
        let cert = Certificate::new(&key).unwrap();
        sign(&package, cert.0).unwrap();
        let copy = duplicate(cert.0);
        assert!(exact_subject(copy));
        let _ = unsafe { CertFreeCertificateContext(Some(copy)) };
        drop(cert);
        key.delete().unwrap();
        // Kuncinya benar-benar hilang dari penyimpanan kunci Windows.
        unsafe {
            let mut provider = NCRYPT_PROV_HANDLE::default();
            NCryptOpenStorageProvider(&mut provider, MS_KEY_STORAGE_PROVIDER, 0).unwrap();
            let mut handle = NCRYPT_KEY_HANDLE::default();
            let reopened = windows::Win32::Security::Cryptography::NCryptOpenKey(provider, &mut handle, &name, CERT_KEY_SPEC(0), NCRYPT_FLAGS(0));
            let _ = NCryptFreeObject(NCRYPT_HANDLE(provider.0));
            assert!(reopened.is_err(), "kunci masih ada");
        }
        println!("paket uji: {}", package.display());
    }
}
