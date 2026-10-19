//! Exercise the shipped TSX app through its public controls, without a probe.
use super::common::Harness;
use gpui_kit::test::TestQueryExt;

pub(super) fn run(ui: &mut Harness) {
    let bundle = std::fs::read_to_string("target/nsh-base-task-app/bundle.js")
        .expect("build the original examples/app with the Base NSH CLI first");
    ui.load(&bundle);
    eprintln!("task app: awaiting original bundle controls");
    ui.wait_window(|window, _| {
        window
            .try_find_by_label("Task name")
            .is_some_and(|input| input.visible())
            && window
                .try_find_by_label("Create task")
                .is_some_and(|button| button.disabled() == Some(true))
    });
    ui.window(|window, _| {
        assert_eq!(window.find_by_label("Task name").value(), Some(""));
        assert_eq!(window.find_by_label("Priority").checked(), Some(false));
    });

    eprintln!("task app: typing task name");
    ui.click("Task name");
    ui.input("Review NSH controls");
    ui.wait_window(|window, _| {
        window.find_by_label("Task name").value() == Some("Review NSH controls")
    });
    eprintln!("task app: checking Priority");
    ui.click("Priority");
    ui.wait_window(|window, _| window.find_by_label("Priority").checked() == Some(true));
    ui.screenshot("task-before.png");
    eprintln!("task app: creating task");
    ui.click("Create task");
    wait_reset(ui);
    ui.screenshot("task-after.png");

    // Input's public submit event invokes the app's same task-creation handler.
    eprintln!("task app: keyboard submission");
    ui.input("Check keyboard submission");
    ui.wait_window(|window, _| {
        window.find_by_label("Task name").value() == Some("Check keyboard submission")
    });
    ui.key("enter");
    wait_reset(ui);
    ui.screenshot("task-keyboard.png");

    // The app accepts 80 characters and vetoes the next edit. NSH's controlled
    // acknowledgement must restore the original native InputState value.
    eprintln!("task app: 80-character boundary");
    let limit = "x".repeat(80);
    ui.input(&limit);
    ui.wait_window(|window, _| window.find_by_label("Task name").value() == Some(limit.as_str()));
    ui.input("!");
    ui.wait_window(|window, _| window.find_by_label("Task name").value() == Some(limit.as_str()));
    ui.screenshot("task-limit.png");
    println!(
        "verified original TSX task app: pointer creation, input/checkbox reset, disabled button, Enter submit and 80-character veto; task summary is recorded in screenshots for visual verification"
    );
}

fn wait_reset(ui: &mut Harness) {
    ui.wait_window(|window, _| {
        window.find_by_label("Task name").value() == Some("")
            && window.find_by_label("Task name").focused() == Some(true)
            && window.find_by_label("Priority").checked() == Some(false)
            && window.find_by_label("Create task").disabled() == Some(true)
    });
}
