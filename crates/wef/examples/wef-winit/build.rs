use embed_manifest::{embed_manifest, new_manifest};

fn main() {
    if std::env::var_os("CARGO_CFG_WINDOWS").is_some() {
        embed_manifest(new_manifest("Wef.Example.Winit")).expect("unable to embed manifest file");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
