use rquickjs::{
    Ctx, Object, Result, Value,
    module::{Declarations, Exports, ModuleDef},
};

pub(super) fn install(ctx: &Ctx<'_>) -> Result<()> {
    ctx.globals()
        .set("console", super::super::host::log_object(ctx)?)
}

pub(super) struct ConsoleModule;

impl ModuleDef for ConsoleModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("default")?;
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let console: Object = ctx.globals().get("console")?;
        exports.export("default", Value::from_object(console))?;
        Ok(())
    }
}
