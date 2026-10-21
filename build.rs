//! Tanam manifest Windows ke notch.exe (lihat packaging/notch.exe.manifest), beserta ikon dan
//! keterangan versinya (nama "NotchWindows" di Task Manager dan properti file).

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = root.join("packaging").join("notch.exe.manifest");
    println!("cargo:rerun-if-changed=packaging/notch.exe.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());

    // Ikon dan versi lewat rc.exe dari Windows SDK, yang sudah ikut terpasang bersama alat build
    // MSVC. Tanpa rc.exe, notch.exe tetap dibuat, hanya tanpa ikon.
    println!("cargo:rerun-if-changed=packaging/notch.ico");
    let Some(rc) = find_rc() else {
        println!("cargo:warning=rc.exe (Windows SDK) tidak ditemukan; notch.exe dibuat tanpa ikon");
        return;
    };
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let script = out.join("notch.rc");
    let resources = out.join("notch.res");
    std::fs::write(&script, resource_script(&root.join("packaging").join("notch.ico"))).expect("notch.rc");
    let built = Command::new(rc).arg("/nologo").arg("/fo").arg(&resources).arg(&script).status();
    if built.is_ok_and(|status| status.success()) {
        println!("cargo:rustc-link-arg-bins={}", resources.display());
    } else {
        println!("cargo:warning=ikon notch.exe gagal dibuat");
    }
}

fn resource_script(icon: &Path) -> String {
    let version = env!("CARGO_PKG_VERSION");
    let numbers: Vec<&str> = version.split('.').chain(["0"; 4]).take(4).collect();
    let numbers = numbers.join(",");
    let icon = icon.display().to_string().replace('\\', "\\\\");
    format!(
        r#"1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "042104b0"
    BEGIN
      VALUE "CompanyName", "NotchWindows (open source)"
      VALUE "FileDescription", "NotchWindows"
      VALUE "FileVersion", "{version}"
      VALUE "InternalName", "notch"
      VALUE "LegalCopyright", "MIT License"
      VALUE "OriginalFilename", "notch.exe"
      VALUE "ProductName", "NotchWindows"
      VALUE "ProductVersion", "{version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x421, 1200
  END
END
"#
    )
}

/// rc.exe dari Windows SDK terbaru yang terpasang.
fn find_rc() -> Option<PathBuf> {
    let kits = PathBuf::from(std::env::var_os("ProgramFiles(x86)")?).join("Windows Kits").join("10").join("bin");
    let mut found: Vec<PathBuf> = std::fs::read_dir(kits)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path().join("x64").join("rc.exe")))
        .filter(|rc| rc.exists())
        .collect();
    found.sort();
    found.pop()
}
