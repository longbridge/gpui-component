use super::common::Harness;
use gpui_kit::{
    point, px,
    test::{TestQueryExt, TestWindowExt},
};
use serde_json::json;
pub(super) fn run(ui: &mut Harness) {
    ui.load(include_str!("../verify_button.js"));
    ui.poll(|value| value["ready"] == json!(true));
    ui.screenshot("button-before.png");
    ui.click("Run base action");
    ui.poll(|value| value["presses"] == json!(1));
    let color = ui.pixel("Run base action", point(px(12.), px(12.)));
    assert!(
        color[0].abs_diff(0x25) <= 2
            && color[1].abs_diff(0x63) <= 2
            && color[2].abs_diff(0xeb) <= 2,
        "NSH must paint the Base button background, observed {color:?}"
    );
    ui.screenshot("button.png");
    // Keyboard activation uses the same real Base control and fires once.
    ui.window(|window, cx| window.press("enter", cx));
    ui.poll(|value| value["presses"] == json!(2));
    ui.window(|window, cx| window.press("tab", cx));
    ui.poll(|value| value["focused"] == json!("next") && value["blurs"] == json!(1));
    ui.window(|window, cx| window.press("shift-tab", cx));
    ui.poll(|value| value["focused"] == json!("main"));
    // The owner disables the focused action in response to its third press.
    ui.window(|window, cx| window.press("space", cx));
    ui.poll(|value| value["presses"] == json!(3) && value["disabled"] == json!(true));
    ui.window(|window, cx| {
        window.press("enter", cx);
        window.press("space", cx);
    });
    ui.click("Run base action");
    ui.click("Disabled base action");
    ui.settle();
    assert_eq!(
        ui.snapshot()["presses"],
        json!(3),
        "disabled pointer and keys must not activate"
    );
    ui.click("Next base action");
    ui.poll(|value| value["secondary"] == json!(1) && value["disabled"] == json!(false));
    ui.click("Run base action");
    ui.poll(|value| value["presses"] == json!(4));
    ui.load(include_str!("button_focus.js"));
    ui.poll(|value| value["scene"] == json!("button-focus"));
    ui.click("Index two");
    ui.poll(|value| value["focused"] == json!("Index two"));
    ui.key("tab");
    ui.poll(|value| value["focused"] == json!("Index three"));
    ui.key("tab");
    ui.poll(|value| value["focused"] == json!("Index zero"));
    ui.key("shift-tab");
    ui.poll(|value| value["focused"] == json!("Index three"));
    ui.click("Skipped action");
    ui.key("space");
    ui.poll(|value| value["presses"] == json!(["Index two", "Skipped action", "Skipped action"]));
    ui.click("Negative action");
    ui.key("enter");
    ui.poll(|value| {
        value["presses"]
            == json!([
                "Index two",
                "Skipped action",
                "Skipped action",
                "Negative action",
                "Negative action"
            ])
    });
    ui.load(include_str!("button_lifecycle.js"));
    ui.poll(|value| value["scene"] == json!("button-lifecycle"));
    ui.click("Focus transient");
    ui.poll(|value| value["focused"] == json!(true));
    ui.key("enter");
    ui.poll(|value| value["presses"] == json!(1));
    ui.click("Corrupt button props");
    ui.poll(|value| {
        value["errors"]
            .as_array()
            .is_some_and(|errors| errors.len() == 1)
    });
    assert!(
        ui.snapshot()["errors"][0]
            .as_str()
            .is_some_and(|error| error.contains("disabled"))
    );
    ui.click("Transient action");
    ui.poll(|value| value["presses"] == json!(2));
    assert_eq!(
        ui.snapshot()["errors"].as_array().unwrap().len(),
        1,
        "invalid candidate must not poison later updates"
    );
    ui.click("Restore button props");
    ui.click("Arm focused unmount");
    ui.poll(|value| value["focused"] == json!(true));
    ui.key("enter");
    ui.poll(|value| value["shown"] == json!(false) && value["presses"] == json!(3));
    ui.wait_window(|window, _| window.try_find_by_label("Transient action").is_none());
    ui.key("space");
    ui.settle();
    assert_eq!(
        ui.snapshot()["presses"],
        json!(3),
        "unmounted native handlers must not receive keys"
    );
    ui.click("Remount transient");
    ui.poll(|value| value["shown"] == json!(true));
    ui.click("Focus transient");
    ui.poll(|value| value["focused"] == json!(true));
    ui.key("enter");
    ui.poll(|value| value["presses"] == json!(4));
    assert_eq!(ui.snapshot()["errors"].as_array().unwrap().len(), 1);
    ui.screenshot("button-recovery.png");
}
