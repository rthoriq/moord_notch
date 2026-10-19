//! Tanam manifest Windows ke notch.exe (lihat packaging/notch.exe.manifest).

fn main() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging").join("notch.exe.manifest");
    println!("cargo:rerun-if-changed=packaging/notch.exe.manifest");
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTINPUT:{}", manifest.display());
}
