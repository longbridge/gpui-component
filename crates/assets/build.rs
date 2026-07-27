use std::{env, path::Path};

fn main() {
    // Publish the absolute path of `assets/icons` so dependents that need
    // the icons at *build time* (notably `gpui-component`, whose `IconName`
    // enum is generated at proc-macro expansion time) can find them without
    // a sibling-crate reference. Cargo turns the `cargo:icons-dir=...` line
    // below into the `DEP_GPUI_COMPONENT_DEFAULT_ICONS_ICONS_DIR` env var in
    // every dependent's build script — see the `links` field in our
    // `Cargo.toml` for the full mechanism.
    //
    // Within this crate, `assets/icons` is also consumed at *runtime* by
    // `RustEmbed` (see `src/native_assets.rs`), which finds the files
    // relative to our own `CARGO_MANIFEST_DIR`. So this build script does
    // not need to copy or move anything — it only advertises the location.
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set by cargo");
    let icons_dir = Path::new(&manifest_dir).join("assets/icons");

    // Sanity-check that the directory we're advertising actually exists.
    // Bail loudly if it's missing — silently publishing a bad path would
    // give dependents a confusing "failed to read directory" later.
    if !icons_dir.is_dir() {
        panic!(
            "expected default icons at {}, but the directory is missing",
            icons_dir.display(),
        );
    }

    println!("cargo:icons-dir={}", icons_dir.display());

    // Rerun if the icon set changes (rename/add/remove). Per-SVG watching
    // isn't needed because the dependent reads from this advertised path
    // at *its own* expansion time, not via cached build-script output.
    println!("cargo:rerun-if-changed=assets/icons");

    // Also rerun if anyone fiddles with this script itself.
    println!("cargo:rerun-if-changed=build.rs");
}
