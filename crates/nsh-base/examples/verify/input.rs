use super::common::Harness;
use gpui_kit::test::TestQueryExt;
use serde_json::json;

pub(super) fn run(ui: &mut Harness) {
    ui.load(include_str!("input.js"));
    ui.poll(|value| value["scene"] == json!("input"));
    // Focus goes through the public ref command, then real IME text entry.
    ui.click("Focus task");
    ui.poll(|value| value["focused"] == json!(true));
    ui.key("home");
    ui.input("X");
    ui.poll(|value| value["value"] == json!("Xabc") && value["changes"] == json!(1));
    // An accepted controlled echo must preserve the insertion point.
    ui.input("Y");
    ui.poll(|value| value["value"] == json!("XYabc") && value["changes"] == json!(2));
    assert_eq!(ui.snapshot()["errors"], json!([]));
    ui.screenshot("input-echo.png");
    ui.click("Reject edits");
    ui.poll(|v| v["allow"] == json!(false) && v["focused"] == json!(true));
    ui.input("!");
    ui.poll(|v| v["changes"] == json!(3));
    assert_eq!(ui.snapshot()["value"], json!("XYabc"));
    ui.click("Accept edits");
    ui.poll(|v| v["allow"] == json!(true));
    ui.input("Z");
    ui.poll(|v| v["value"] == json!("XYZabc") && v["changes"] == json!(4));
    ui.key("enter");
    ui.poll(|v| v["submits"].as_array().is_some_and(|all| all.len() == 1));
    assert_eq!(
        ui.snapshot()["submits"][0],
        json!({"secondary": false, "shift": false})
    );
    ui.click("Readonly task");
    ui.poll(|v| v["readonly"] == json!(true));
    ui.input("blocked");
    ui.key("backspace");
    ui.settle();
    assert_eq!(ui.snapshot()["changes"], json!(4));
    ui.click("Editable task again");
    ui.poll(|v| v["readonly"] == json!(false));
    ui.click("Select task text");
    ui.poll(|v| v["focused"] == json!(true));
    ui.input("replacement");
    // The test IME sends one native edit per character.
    ui.poll(|v| v["value"] == json!("replacement") && v["changes"] == json!(15));
    ui.click("Disable task");
    ui.poll(|v| v["disabled"] == json!(true));
    ui.input("blocked");
    ui.key("backspace");
    ui.settle();
    assert_eq!(ui.snapshot()["changes"], json!(15));
    ui.click("Enable task");
    ui.poll(|v| v["disabled"] == json!(false) && v["focused"] == json!(true));
    ui.key("end");
    ui.input("!");
    ui.poll(|v| v["value"] == json!("replacement!") && v["changes"] == json!(16));
    navigation(ui);
    style(ui);
}

fn navigation(ui: &mut Harness) {
    ui.load(include_str!("input_navigation.js"));
    ui.poll(|v| v["scene"] == json!("input-navigation"));
    // Excluding an input from Tab must not exclude it from native IME ownership.
    ui.click("Navigation excluded");
    ui.poll(|v| v["focused"] == json!("excluded"));
    ui.key("end");
    ui.input("X");
    ui.poll(|v| v["values"]["excluded"] == json!("excludedX"));
    ui.click("Focus navigation negative");
    ui.poll(|v| v["focused"] == json!("negative"));
    ui.key("end");
    ui.input("Y");
    ui.poll(|v| v["values"]["negative"] == json!("negativeY"));
    ui.click("Navigation negative");
    ui.key("end");
    ui.input("Z");
    ui.poll(|v| v["values"]["negative"] == json!("negativeYZ"));
    ui.click("Focus navigation excluded");
    ui.poll(|v| v["focused"] == json!("excluded"));
    ui.key("end");
    ui.input("W");
    ui.poll(|v| v["values"]["excluded"] == json!("excludedXW"));
    ui.click("Focus navigation two");
    ui.poll(|v| v["focused"] == json!("two"));
    eprintln!("Input navigation: focused two, forward Tab to three");
    ui.key("tab");
    ui.poll(|v| v["focused"] == json!("three"));
    assert_navigation_focus(ui, "three");
    eprintln!("Input navigation: focused three, forward Tab to zero");
    ui.key("tab");
    ui.poll(|v| v["focused"] == json!("zero"));
    assert_navigation_focus(ui, "zero");
    eprintln!("Input navigation: focused zero, reverse Tab to three");
    ui.key("shift-tab");
    ui.poll(|v| v["focused"] == json!("three"));
    assert_navigation_focus(ui, "three");
    ui.key("shift-tab");
    ui.poll(|v| v["focused"] == json!("two"));
    assert_navigation_focus(ui, "two");
    assert_eq!(ui.snapshot()["errors"], json!([]));
    ui.screenshot("input-navigation.png");
}

fn assert_navigation_focus(ui: &mut Harness, expected: &str) {
    ui.settle();
    ui.window(|window, _| {
        for name in ["two", "three", "zero", "negative", "excluded"] {
            assert_eq!(
                window
                    .find_by_label(&format!("Navigation {name}"))
                    .focused(),
                Some(name == expected),
                "native Input focus must follow real Tab navigation: {name}"
            );
        }
    });
}

fn style(ui: &mut Harness) {
    ui.load(include_str!("input_style.js"));
    ui.poll(|v| v["scene"] == json!("input-style"));
    let before = ui
        .color_bounds("Styled native input", [0x10, 0x20, 0x30], 12)
        .expect("native glyphs must inherit the NSH foreground color");
    assert!(
        before.0 >= 10. && before.2 > 40. && before.3 > 10.,
        "real text must use NSH content insets and font: {before:?}"
    );
    ui.screenshot("input-style-before.png");
    ui.click("Change inherited input style");
    ui.poll(|v| v["changed"] == json!(true));
    let after = ui
        .color_bounds("Styled native input", [0x8a, 0x14, 0x28], 12)
        .expect("updated NSH ancestor color must reach the native glyphs");
    assert!(
        after.0 >= before.0 + 4.,
        "native text must follow updated NSH padding: {before:?} → {after:?}"
    );
    assert!(
        after.2 > before.2 + 4. && after.3 > before.3 + 2.,
        "native text must follow the updated inherited font rather than fall back to its default: {before:?} → {after:?}"
    );
    assert!(
        ui.color_bounds("Styled native input", [0x10, 0x20, 0x30], 12)
            .is_none(),
        "the old native text color must not survive the reactive style update"
    );
    assert_eq!(ui.snapshot()["value"], json!("NSH Kit"));
    assert_eq!(ui.snapshot()["errors"], json!([]));
    ui.screenshot("input-style-after.png");
}
