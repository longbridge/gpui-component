use std::path::PathBuf;

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let command = arguments.next();
    let directory = arguments.next().map(PathBuf::from);

    if command.as_deref() != Some(std::ffi::OsStr::new("types"))
        || directory.is_none()
        || arguments.next().is_some()
    {
        eprintln!("usage: gpui-component-shell types <application-directory>");
        std::process::exit(2);
    }

    if let Err(error) = gpui_component_shell::write_type_declarations(directory.unwrap()) {
        eprintln!("gpui-component-shell: {error:#}");
        std::process::exit(1);
    }
}
