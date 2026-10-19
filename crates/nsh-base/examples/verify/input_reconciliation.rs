//! Controlled edits after a public external value reset, using real key input.
use super::common::Harness;
use gpui_kit::test::TestQueryExt;
use serde_json::json;

pub(super) fn run(ui: &mut Harness) {
    ui.load(include_str!("input_reconciliation.js"));
    ui.poll(|v| v["scene"] == json!("input-reconciliation"));
    ui.click("Reconciled task");
    ui.poll(|v| v["focused"] == json!(true));
    ui.input("S");
    ui.poll(|v| v["changes"] == json!(1));

    ui.click("Reset task externally");
    ui.poll(|v| v["resets"] == json!(1) && v["focused"] == json!(true));
    ui.wait_window(|window, _| window.find_by_label("Reconciled task").value() == Some("wxyz"));
    ui.key("home");
    ui.key("right");
    ui.key("right");
    ui.input("!");
    ui.poll(|v| v["changes"] == json!(2));
    ui.wait_window(|window, _| window.find_by_label("Reconciled task").value() == Some("wxyz"));
    assert_eq!(ui.snapshot()["value"], json!("wxyz"));
    ui.click("Accept reconciled edits");
    ui.poll(|v| v["allow"] == json!(true) && v["focused"] == json!(true));
    // A veto restores both the external value and the native insertion point.
    ui.input("Q");
    ui.poll(|v| v["value"] == json!("wxQyz") && v["changes"] == json!(3));
    assert_eq!(ui.snapshot()["errors"], json!([]));
    ui.click("Reset Unicode task externally");
    ui.poll(|v| v["resets"] == json!(2) && v["focused"] == json!(true));
    ui.wait_window(|window, _| window.find_by_label("Reconciled task").value() == Some("甲乙丙丁"));
    ui.key("home");
    ui.key("right");
    ui.key("shift-right");
    ui.key("shift-right");
    ui.input("Z");
    ui.poll(|v| v["changes"] == json!(4));
    ui.wait_window(|window, _| window.find_by_label("Reconciled task").value() == Some("甲乙丙丁"));
    ui.click("Accept reconciled edits");
    ui.poll(|v| v["allow"] == json!(true) && v["focused"] == json!(true));
    ui.input("K");
    ui.poll(|v| v["value"] == json!("甲K丁") && v["changes"] == json!(5));

    ui.click("Reject invalid batch on next edit");
    ui.poll(|v| v["allow"] == json!(false) && v["focused"] == json!(true));
    ui.input("!");
    ui.poll(|v| {
        v["changes"] == json!(6) && v["errors"].as_array().is_some_and(|all| all.len() == 1)
    });
    ui.wait_window(|window, _| {
        let input = window.find_by_label("Reconciled task");
        input.value() == Some("甲K丁")
            && input.disabled() != Some(true)
            && input.focused() == Some(true)
    });
    ui.settle();
    assert_eq!(ui.snapshot()["errors"].as_array().unwrap().len(), 1);
    ui.click("Recover valid task props");
    ui.poll(|v| {
        v["value"] == json!("甲K丁") && v["allow"] == json!(true) && v["focused"] == json!(true)
    });
    ui.input("R");
    ui.poll(|v| v["value"] == json!("甲KR丁") && v["changes"] == json!(7));
    assert_eq!(ui.snapshot()["errors"].as_array().unwrap().len(), 1);
    ui.screenshot("input-reconciliation.png");
    println!(
        "verified external Input reset, Unicode selection veto and rejected mixed-prop batch recovery"
    );
}
