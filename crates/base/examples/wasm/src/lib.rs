use gpui::{Application, ApplicationHandle};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

#[path = "../../showcase/mod.rs"]
#[allow(dead_code)]
mod showcase;

thread_local! {
    static APPLICATION: RefCell<Option<ApplicationHandle>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn run(component: Option<String>) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    tracing_wasm::set_as_global_default();
    #[cfg(target_family = "wasm")]
    gpui_platform::web_init();
    let handle = showcase::run_embedded(
        web_application(),
        component.unwrap_or_else(|| "overview".to_owned()),
    );
    APPLICATION.with(|application| *application.borrow_mut() = Some(handle));
    Ok(())
}

#[cfg(target_family = "wasm")]
fn web_application() -> Application {
    gpui_platform::single_threaded_web()
}

#[cfg(not(target_family = "wasm"))]
fn web_application() -> Application {
    gpui_platform::application()
}
