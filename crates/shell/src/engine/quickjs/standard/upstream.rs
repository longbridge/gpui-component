//! Compile-time checks for the privileged LLRT modules that Shell adapts.
//!
//! Registering these upstream modules directly would grant ambient filesystem,
//! process and network authority. Referencing their public entry points here
//! keeps the pinned integration compiling while the public modules are backed
//! by Shell's policy-aware adapters.

use rquickjs::{Ctx, Result, module::ModuleDef};

pub(super) fn assert_compatible() {
    fn module<M: ModuleDef>() {}

    module::<llrt_console::ConsoleModule>();
    module::<llrt_fs::FsModule>();
    module::<llrt_fs::FsPromisesModule>();
    module::<llrt_net::NetModule>();
    module::<llrt_os::OsModule>();
    module::<llrt_process::ProcessModule>();

    let _: for<'js> fn(&Ctx<'js>) -> Result<()> = llrt_console::init;
    let _: for<'js> fn(&Ctx<'js>) -> Result<()> = llrt_fetch::init;
    let _: for<'js> fn(&Ctx<'js>) -> Result<()> = llrt_process::init;
}
