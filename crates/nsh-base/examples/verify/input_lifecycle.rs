use super::common::Harness;
use gpui_kit::{
    MouseButton,
    test::{TestEventExt, TestQueryExt},
};
use serde_json::json;

pub(super) fn run(ui: &mut Harness) {
    ui.load(include_str!("input_lifecycle.js"));
    ui.poll(|v| v["scene"] == json!("input-lifecycle"));
    ui.click("Focus lifecycle input");
    ui.poll(|v| v["focused"] == json!(true));
    ui.key("end");
    ui.input("X");
    ui.poll(|v| v["value"] == json!("abcX") && v["changes"] == json!(1));
    ui.click("Update presentation");
    ui.poll(|v| v["presentation"] == json!(true) && v["focused"] == json!(true));
    #[cfg(target_os = "macos")]
    ui.key("cmd-z");
    #[cfg(not(target_os = "macos"))]
    ui.key("ctrl-z");
    ui.poll(|v| v["value"] == json!("abc") && v["changes"] == json!(2));
    assert_eq!(ui.snapshot()["errors"], json!([]));
    eprintln!("Input lifecycle: accepted echo and unrelated style/placeholder preserve real Undo");

    let center = ui.window(|window, _| window.find_by_label("Lifecycle input").bounds().center());
    ui.click("Remove on next edit");
    ui.poll(|v| v["focused"] == json!(true));
    ui.input("Q");
    ui.poll(|v| v["visible"] == json!(false) && v["changes"] == json!(3));
    ui.settle();
    ui.window(|window, cx| {
        assert!(window.try_find_by_label("Lifecycle input").is_none());
        window.pointer_move(center, None, cx);
        window.pointer_down(center, MouseButton::Left, cx);
        window.pointer_up(center, MouseButton::Left, cx);
    });
    ui.input("Z");
    ui.key("backspace");
    ui.settle();
    assert_eq!(ui.snapshot()["changes"], json!(3));
    ui.click("Command removed input");
    ui.poll(|v| v["staleCommands"] == json!(1));
    ui.input("Y");
    ui.settle();
    assert_eq!(
        ui.snapshot()["changes"],
        json!(3),
        "disposed ref must not reactivate old native editor"
    );
    ui.click("Restore lifecycle input");
    ui.poll(|v| v["visible"] == json!(true));
    ui.click("Focus lifecycle input");
    ui.poll(|v| v["focused"] == json!(true));
    ui.key("end");
    let before = ui.snapshot()["value"].as_str().unwrap().to_owned();
    ui.input("R");
    ui.poll(|v| v["value"] == json!(format!("{before}R")) && v["changes"] == json!(4));
    ui.settle();
    assert_eq!(
        ui.snapshot()["changes"],
        json!(4),
        "remounted editor must emit exactly once"
    );
    assert_eq!(ui.snapshot()["errors"], json!([]));
    ui.screenshot("input-lifecycle.png");
}
