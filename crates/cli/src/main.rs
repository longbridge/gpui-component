use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("error: cannot determine current directory: {error}");
            return ExitCode::FAILURE;
        }
    };
    match gpui_component_cli::run(env::args().skip(1), &cwd) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
