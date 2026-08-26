use rquickjs::{
    Ctx, Object, Result, Value,
    function::Func,
    module::{Declarations, Exports, ModuleDef},
};

const EXPORTS: &[&str] = &["platform", "arch", "EOL"];

pub(super) struct OsModule;

impl ModuleDef for OsModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("default")?;
        for name in EXPORTS {
            declarations.declare(*name)?;
        }
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let os = Object::new(ctx.clone())?;
        os.set("platform", Func::from(|| std::env::consts::OS))?;
        os.set("arch", Func::from(|| std::env::consts::ARCH))?;
        os.set("EOL", if cfg!(windows) { "\r\n" } else { "\n" })?;
        exports.export("default", Value::from_object(os.clone()))?;
        for name in EXPORTS {
            exports.export(*name, os.get::<_, Value>(*name)?)?;
        }
        Ok(())
    }
}
