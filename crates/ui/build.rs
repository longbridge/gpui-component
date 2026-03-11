use std::{fs, path::Path};

fn main() {
    // Tell Cargo to rerun this build script if any SVG file in assets/icons changes.
    let icons_dir = Path::new("../assets/assets/icons");

    // Watch the icons directory itself.
    println!("cargo:rerun-if-changed=../assets/assets/icons");

    // Watch each SVG file in the directory.
    if let Ok(entries) = fs::read_dir(icons_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("svg") {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}
