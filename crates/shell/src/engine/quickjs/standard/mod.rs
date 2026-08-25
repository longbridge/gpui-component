//! LLRT-backed, authority-free Standard Runtime modules.
//!
//! Privileged modules live behind Shell adapters; this module only registers
//! implementations that cannot bypass the active [`crate::Policy`].

use rquickjs::{Ctx, Result, loader::BuiltinResolver, loader::ModuleLoader};

mod console;
mod fs;
mod os;
mod process;

const NAMES: &[&str] = &[
    "buffer",
    "console",
    "crypto",
    "fs",
    "fs/promises",
    "os",
    "path",
    "process",
    "url",
    "zlib",
];

pub(super) fn resolver() -> BuiltinResolver {
    NAMES
        .iter()
        .fold(BuiltinResolver::default(), |resolver, name| {
            resolver.with_module(*name)
        })
}

pub(super) fn loader() -> ModuleLoader {
    ModuleLoader::default()
        .with_module("buffer", llrt_buffer::BufferModule)
        .with_module("console", console::ConsoleModule)
        .with_module("crypto", llrt_crypto::CryptoModule)
        .with_module("fs", fs::FsModule)
        .with_module("fs/promises", fs::FsModule)
        .with_module("os", os::OsModule)
        .with_module("path", llrt_path::PathModule)
        .with_module("process", process::ProcessModule)
        .with_module("url", llrt_url::UrlModule)
        .with_module("zlib", llrt_zlib::ZlibModule)
}

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    // Order is significant: URL and Crypto consume Buffer-compatible byte
    // classes installed by the first initializer.
    llrt_buffer::init(ctx)?;
    llrt_url::init(ctx)?;
    llrt_crypto::init(ctx)?;
    console::install(ctx)?;
    super::sandbox::install_process(ctx)?;
    Ok(())
}
