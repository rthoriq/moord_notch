//! Bagikan file lewat jendela Bagikan Windows (Berbagi di Sekitar ke PC lain, email, aplikasi),
//! padanan AirDrop di Mac. Dipakai saat file dilepas di zona "Bagikan" notch. File tidak
//! dikirim ke mana pun oleh notch; pengguna sendiri yang memilih tujuan di jendela Windows itu.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use windows::ApplicationModel::DataTransfer::{DataRequestedEventArgs, DataTransferManager};
use windows::Foundation::TypedEventHandler;
use windows::Storage::{IStorageItem, StorageFile, StorageFolder};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Shell::IDataTransferManagerInterop;
use windows::core::{HSTRING, Interface, Ref, Result, factory};
use windows_collections::IIterable;

/// Jendela Bagikan milik jendela notch; dipasang sekali, isinya diganti tiap kali membagikan.
pub struct Share {
    hwnd: HWND,
    interop: IDataTransferManagerInterop,
    manager: DataTransferManager,
    token: i64,
    /// File yang sedang dibagikan; dibaca saat Windows meminta isinya.
    paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl Share {
    pub fn new(hwnd: HWND) -> Result<Self> {
        let interop = factory::<DataTransferManager, IDataTransferManagerInterop>()?;
        let manager: DataTransferManager = unsafe { interop.GetForWindow(hwnd)? };
        let paths: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let shared = paths.clone();
        // Windows meminta isinya saat jendela Bagikan dibuka.
        let handler = TypedEventHandler::new(move |_, args: Ref<DataRequestedEventArgs>| {
            let Some(args) = args.as_ref() else { return Ok(()) };
            let paths = shared.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let items: Vec<IStorageItem> = paths.iter().filter_map(|path| storage_item(path).ok()).collect();
            let title = match items.as_slice() {
                [one] => one.Name()?.to_string(),
                many => format!("{} file", many.len()),
            };
            let data = args.Request()?.Data()?;
            data.Properties()?.SetTitle(&HSTRING::from(title))?;
            let items: Vec<Option<IStorageItem>> = items.into_iter().map(Some).collect();
            data.SetStorageItemsReadOnly(&IIterable::from(items))?;
            Ok(())
        });
        let token = manager.DataRequested(&handler)?;
        Ok(Self { hwnd, interop, manager, token, paths })
    }

    /// Buka jendela Bagikan untuk `paths` (file atau folder).
    pub fn show(&self, paths: &[PathBuf]) -> Result<()> {
        let paths: Vec<PathBuf> = paths.iter().filter(|path| path.exists()).cloned().collect();
        if paths.is_empty() {
            return Ok(());
        }
        *self.paths.lock().unwrap_or_else(|e| e.into_inner()) = paths;
        unsafe { self.interop.ShowShareUIForWindow(self.hwnd) }
    }
}

impl Drop for Share {
    fn drop(&mut self) {
        let _ = self.manager.RemoveDataRequested(self.token);
    }
}

fn storage_item(path: &std::path::Path) -> Result<IStorageItem> {
    let name = HSTRING::from(path.as_os_str());
    if path.is_dir() {
        StorageFolder::GetFolderFromPathAsync(&name)?.join()?.cast()
    } else {
        StorageFile::GetFileFromPathAsync(&name)?.join()?.cast()
    }
}
