//! Real NSH engine and unstyled Base controls; Component is not a dependency.
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/button.rs"]
mod button;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/checkbox.rs"]
mod checkbox;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/common.rs"]
mod common;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/composition.rs"]
mod composition;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/input.rs"]
mod input;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/input_lifecycle.rs"]
mod input_lifecycle;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/input_reconciliation.rs"]
mod input_reconciliation;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/task_app.rs"]
mod task_app;
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() {
    let family = std::env::var("NSH_BASE_VERIFY_FAMILY").unwrap_or_else(|_| "all".into());
    assert!(
        [
            "all",
            "button",
            "checkbox",
            "input",
            "input_lifecycle",
            "input_reconciliation",
            "task_app",
            "composition"
        ]
        .contains(&family.as_str()),
        "unknown Base test family {family}"
    );
    let mut ui = common::Harness::new(
        gpui_kit_nsh_base::components(),
        vec![nsh_ui_mobile::registry(), gpui_kit_nsh_base::ui_set()],
    );
    if family == "all" || family == "button" {
        button::run(&mut ui);
    }
    if family == "all" || family == "checkbox" {
        checkbox::run(&mut ui);
    }
    if family == "all" || family == "input" {
        input::run(&mut ui);
    }
    if family == "all" || family == "input_lifecycle" {
        input_lifecycle::run(&mut ui);
    }
    if family == "all" || family == "input_reconciliation" {
        input_reconciliation::run(&mut ui);
    }
    if family == "all" || family == "task_app" {
        task_app::run(&mut ui);
    }
    if family == "all" || family == "composition" {
        composition::run(&mut ui);
    }
    ui.stop();
    println!("verified real NSH → Base {family} public UI interactions");
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {
    panic!("Base UI verification requires macOS or Linux headless renderer");
}
