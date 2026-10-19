use super::common::Harness;
use gpui_kit::{
    MouseButton, point, px,
    test::{TestEventExt, TestQueryExt},
};
use serde_json::json;
pub(super) fn run(ui: &mut Harness) {
    ui.load(include_str!("composition.js"));
    ui.poll(|value| value["scene"] == json!("composition"));
    ui.window(|window, _| {
        let outer = window.find_by_label("Outer native host").bounds();
        let inner = window.find_by_label("Inner native action").bounds();
        assert_eq!(
            inner.size.width,
            px(160.),
            "NSH owns the nested native width"
        );
        assert_eq!(
            inner.size.height,
            px(56.),
            "NSH owns the nested native height"
        );
        assert!(inner.left() >= outer.left() && inner.right() <= outer.right());
        assert!(inner.top() >= outer.top() && inner.bottom() <= outer.bottom());
    });
    let color = ui.pixel("Inner native action", point(px(12.), px(12.)));
    assert!(
        color[0].abs_diff(0xb9) <= 2
            && color[1].abs_diff(0x1c) <= 2
            && color[2].abs_diff(0x1c) <= 2,
        "NSH paints direct-native child backgrounds: {color:?}"
    );
    ui.click("Inner native action");
    ui.poll(|value| value["presses"] == json!(1));
    ui.screenshot("composition.png");
    ui.load(include_str!("composition_live.js"));
    ui.poll(|value| value["scene"] == json!("composition-live"));
    // A real interactive NSH subtree consumes its own pointer exactly once.
    ui.click("Outer press region");
    ui.poll(|value| value["child"] == json!(1));
    assert_eq!(ui.snapshot()["outer"], json!(0));
    ui.click("Header 0");
    ui.poll(|value| value["header"] == json!(1));
    ui.wait_window(|window, _| {
        window.try_find_by_label("Header 1").is_some()
            && window.try_find_by_label("Header 0").is_none()
    });
    ui.click("Body 0");
    ui.poll(|value| value["body"] == json!(1));
    ui.click("Header 1");
    ui.click("Body 1");
    ui.poll(|value| value["header"] == json!(2) && value["body"] == json!(2));
    let old_header = ui.window(|window, _| window.find_by_label("Header 2").bounds().center());
    ui.screenshot("composition-live.png");
    ui.click("Unmount live subtrees");
    ui.poll(|value| value["shown"] == json!(false));
    ui.wait_window(|window, _| {
        window.try_find_by_label("Header 2").is_none()
            && window.try_find_by_label("Body 2").is_none()
    });
    ui.window(|window, cx| {
        window.pointer_move(old_header, None, cx);
        window.pointer_down(old_header, MouseButton::Left, cx);
        window.pointer_up(old_header, MouseButton::Left, cx);
    });
    ui.settle();
    assert_eq!(ui.snapshot()["header"], json!(2));
    assert_eq!(ui.snapshot()["body"], json!(2));
    ui.screenshot("composition-unmounted.png");
    ui.click("Remount live subtrees");
    ui.poll(|value| value["shown"] == json!(true));
    ui.click("Header 2");
    ui.poll(|value| value["header"] == json!(3));
    assert_eq!(ui.snapshot()["outer"], json!(0));
}
