use super::common::Harness;
use gpui_kit::{
    MouseButton, point, px,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use serde_json::json;
fn color(ui: &mut Harness, expected: [u8; 3]) {
    let actual = ui.pixel("Controlled priority", point(px(268.), px(8.)));
    assert!(
        actual[..3]
            .iter()
            .zip(expected)
            .all(|(a, e)| a.abs_diff(e) <= 2),
        "live NSH child background must update, got {actual:?}, expected {expected:?}"
    );
}
pub(super) fn run(ui: &mut Harness) {
    ui.load(include_str!("checkbox.js"));
    ui.poll(|value| value["scene"] == json!("checkbox"));
    ui.window(|window, _| {
        assert_eq!(
            window.find_by_label("Controlled priority").indeterminate(),
            Some(true)
        );
    });
    color(ui, [0xfe, 0xf3, 0xc7]);
    ui.click("Controlled priority");
    ui.poll(|value| {
        value["changes"] == json!(1)
            && value["checked"] == json!(true)
            && value["mixed"] == json!(false)
    });
    color(ui, [0xdc, 0xfc, 0xe7]);
    ui.window(|window, cx| window.press("enter", cx));
    ui.poll(|value| value["changes"] == json!(2) && value["checked"] == json!(false));
    color(ui, [0xfe, 0xe2, 0xe2]);
    ui.window(|window, cx| window.press("space", cx));
    ui.poll(|value| value["changes"] == json!(3) && value["checked"] == json!(true));
    ui.click("Veto mixed changes");
    ui.poll(|value| {
        value["allow"] == json!(false)
            && value["mixed"] == json!(true)
            && value["focused"] == json!(true)
    });
    ui.window(|window, cx| window.press("space", cx));
    ui.poll(|value| value["changes"] == json!(4));
    assert_eq!(ui.snapshot()["requested"], json!(true));
    assert_eq!(ui.snapshot()["checked"], json!(true));
    assert_eq!(ui.snapshot()["mixed"], json!(true));
    ui.window(|window, _| {
        assert_eq!(
            window.find_by_label("Controlled priority").indeterminate(),
            Some(true)
        )
    });
    ui.window(|window, cx| window.press("enter", cx));
    ui.poll(|value| value["changes"] == json!(5) && value["mixed"] == json!(true));
    ui.click("Accept checkbox changes");
    ui.poll(|value| value["allow"] == json!(true) && value["focused"] == json!(true));
    ui.window(|window, cx| window.press("space", cx));
    ui.poll(|value| value["changes"] == json!(6) && value["mixed"] == json!(false));
    ui.window(|window, _| {
        assert_eq!(
            window.find_by_label("Controlled priority").checked(),
            Some(true)
        )
    });
    ui.click("Disable checkbox");
    ui.poll(|value| value["disabled"] == json!(true));
    ui.click("Controlled priority");
    ui.window(|window, cx| window.press("space", cx));
    ui.key("enter");
    ui.settle();
    assert_eq!(ui.snapshot()["changes"], json!(6));
    ui.click("Enable checkbox");
    ui.poll(|value| value["disabled"] == json!(false));
    ui.click("Controlled priority");
    ui.poll(|value| value["changes"] == json!(7) && value["checked"] == json!(false));
    let blurs = ui.snapshot()["blurs"].as_u64().unwrap();
    ui.click("Disable checkbox");
    ui.poll(|v| {
        v["disabled"] == json!(true)
            && v["focused"] == json!(false)
            && v["blurs"] == json!(blurs + 1)
    });
    ui.key("tab");
    ui.settle();
    ui.window(|window, _| {
        assert_eq!(
            window.find_by_label("Enable checkbox").focused(),
            Some(true)
        )
    });
    ui.key("tab");
    ui.settle();
    ui.window(|window, _| {
        assert_eq!(
            window.find_by_label("Veto mixed changes").focused(),
            Some(true),
            "Tab must skip disabled Checkbox; actual owners {:?}",
            [
                "Controlled priority",
                "Veto mixed changes",
                "Accept checkbox changes",
                "Disable checkbox",
                "Enable checkbox",
                "Remove on next change",
                "Restore checkbox"
            ]
            .map(|label| (label, window.find_by_label(label).focused()))
        )
    });
    assert_eq!(ui.snapshot()["changes"], json!(7));
    ui.click("Enable checkbox");
    ui.poll(|v| v["disabled"] == json!(false));
    let focuses = ui.snapshot()["focuses"].as_u64().unwrap();
    ui.key("tab");
    ui.poll(|v| v["focused"] == json!(true) && v["focuses"] == json!(focuses + 1));
    ui.key("space");
    ui.poll(|v| v["changes"] == json!(8) && v["checked"] == json!(true));
    color(ui, [0xdc, 0xfc, 0xe7]);
    let center = ui.window(|window, _| {
        window
            .find_by_label("Controlled priority")
            .bounds()
            .center()
    });
    ui.click("Remove on next change");
    ui.poll(|v| v["focused"] == json!(true));
    ui.key("space");
    ui.poll(|v| v["visible"] == json!(false) && v["changes"] == json!(9));
    ui.settle();
    ui.window(|window, cx| {
        assert!(window.try_find_by_label("Controlled priority").is_none());
        window.pointer_move(center, None, cx);
        window.pointer_down(center, MouseButton::Left, cx);
        window.pointer_up(center, MouseButton::Left, cx);
    });
    ui.key("space");
    ui.key("enter");
    ui.settle();
    assert_eq!(
        ui.snapshot()["changes"],
        json!(9),
        "unmounted Checkbox must not emit stale changes"
    );
    ui.click("Restore checkbox");
    ui.poll(|v| v["visible"] == json!(true));
    ui.click("Controlled priority");
    ui.poll(|v| v["changes"] == json!(10));
    ui.settle();
    assert_eq!(
        ui.snapshot()["changes"],
        json!(10),
        "remount must install exactly one live handler"
    );
    assert_eq!(ui.snapshot()["errors"], json!([]));
    ui.screenshot("checkbox.png");
}
