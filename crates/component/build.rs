use std::env;

fn main() {
    // `gpui-kit-assets` exposes the absolute path of its default
    // icons directory via cargo's `links` mechanism (see its `Cargo.toml`
    // and `build.rs`). We receive that path as
    // `DEP_GPUI_KIT_DEFAULT_ICONS_ICONS_DIR` here, then re-publish
    // it as a `rustc-env` so it's visible to the
    // `icon_named!("$GPUI_KIT_DEFAULT_ICONS_DIR")` proc-macro call
    // in `src/icon.rs` at expansion time. This is what lets the default
    // `IconName` enum be generated from the assets crate's icon set
    // without a sibling-crate reference, which would otherwise break
    // `cargo vendor` and `cargo publish`.
    //
    // Cargo only propagates `DEP_<name>_<key>` through *regular*
    // dependencies, not through build-deps — see the `dependencies`
    // (not `build-dependencies`) entry for `gpui-kit-assets` in
    // `Cargo.toml`.
    let icons_dir = env::var("DEP_GPUI_KIT_DEFAULT_ICONS_ICONS_DIR").expect(
        "DEP_GPUI_KIT_DEFAULT_ICONS_ICONS_DIR is set by gpui-kit-assets's \
         build.rs via its `links` field; make sure the regular dependency on \
         gpui-kit-assets is intact in Cargo.toml",
    );

    println!("cargo:rustc-env=GPUI_KIT_DEFAULT_ICONS_DIR={icons_dir}");

    // Rerun if the icons directory we point at changes. The assets crate's
    // build.rs already declares the same `rerun-if-changed`, but cargo
    // invalidates each build script independently, so this keeps us in
    // lockstep.
    println!("cargo:rerun-if-changed={icons_dir}");
    println!("cargo:rerun-if-changed=build.rs");
}
