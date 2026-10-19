//! Kit-enabled host for `nsh run --target gpui`.
fn main() -> std::process::ExitCode {
    gpui_kit_nsh::host::development(std::env::args().skip(1))
}
