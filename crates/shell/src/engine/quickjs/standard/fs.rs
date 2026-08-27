use rquickjs::{
    Ctx, Object, Result, Value,
    function::Func,
    module::{Declarations, Exports, ModuleDef},
};

const EXPORTS: &[&str] = &[
    "readFile",
    "writeFile",
    "readdir",
    "exists",
    "unlink",
    "rmdir",
    "mkdir",
];

pub(super) struct FsModule;

impl ModuleDef for FsModule {
    fn declare(declarations: &Declarations) -> Result<()> {
        declarations.declare("default")?;
        for name in EXPORTS {
            declarations.declare(*name)?;
        }
        Ok(())
    }

    fn evaluate<'js>(ctx: &Ctx<'js>, exports: &Exports<'js>) -> Result<()> {
        let fs = Object::new(ctx.clone())?;
        fs.set("readFile", Func::from(super::super::host::read_file))?;
        fs.set("writeFile", Func::from(super::super::host::write_file))?;
        fs.set("readdir", Func::from(super::super::host::list_dir))?;
        fs.set("exists", Func::from(super::super::host::exists))?;
        fs.set("unlink", Func::from(super::super::host::remove_file))?;
        fs.set("rmdir", Func::from(super::super::host::remove_dir))?;
        fs.set("mkdir", Func::from(super::super::host::mkdir))?;
        exports.export("default", Value::from_object(fs.clone()))?;
        for name in EXPORTS {
            exports.export(*name, fs.get::<_, Value>(*name)?)?;
        }
        Ok(())
    }
}
