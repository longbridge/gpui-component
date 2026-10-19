use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, App, Entity, HeadlessAppContext, MouseButton, Role, Window,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
fn command(commands: &Rc<RefCell<Option<ComponentContext>>>, name: &str) {
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!(name));
}
fn wait_window(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    predicate: impl Fn(&mut Window, &mut App) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        if cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                predicate(window, cx)
            })
            .unwrap()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "color/stepper window did not reach expected state"
        );
        std::thread::yield_now();
    }
}
fn click_step(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, index: usize) {
    cx.update_window(handle, |_, window, cx| {
        let bounds = window.find(("trigger", index)).bounds();
        let position = bounds.center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
}
// Component's existing featured-colors row is observed without replacing the
// Base swatches. Click their native layout, then assert the resulting events.
fn click_featured(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    index: usize,
    count: usize,
) {
    cx.update_window(handle, |_, window, cx| {
        let bounds = window.find("featured-colors").bounds();
        let size = bounds.size.height;
        // The native row uses h_5 swatches and gap_1, both in the same rem scale.
        let gap = size / 5.;
        assert!(bounds.size.width >= size * count as f32 + gap * (count - 1) as f32);
        let position = gpui_kit::point(
            bounds.left() + size / 2. + (size + gap) * index as f32,
            bounds.center().y,
        );
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
}
pub(super) fn run(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    view: &Entity<AppView>,
    probe: &Rc<RefCell<Value>>,
    errors: &Rc<RefCell<Vec<String>>>,
    commands: &Rc<RefCell<Option<ComponentContext>>>,
) {
    cx.update(|cx| {
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_extra_controls.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |v| {
        v["fixture"] == json!("extra_controls")
    });
    click(cx, handle, "Step slot action");
    poll(cx, handle, probe, errors, |v| v["slotClicks"] == json!(1));
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/stepper-slots.png")
        .unwrap();
    click_step(cx, handle, 1);
    poll(cx, handle, probe, errors, |v| v["step"] == json!(1));
    let step_changes = probe.borrow()["stepChanges"].clone();
    click_step(cx, handle, 2);
    cx.run_until_parked();
    assert_eq!(
        probe.borrow()["stepChanges"],
        step_changes,
        "disabled step must not activate"
    );
    command(commands, "disable-steps");
    poll(cx, handle, probe, errors, |v| {
        v["allDisabled"] == json!(true)
    });
    click_step(cx, handle, 0);
    cx.run_until_parked();
    assert_eq!(
        probe.borrow()["stepChanges"],
        step_changes,
        "disabled Stepper must not activate"
    );
    command(commands, "focus-picker");
    poll(cx, handle, probe, errors, |value| {
        value["native"]["focused"] == json!(true)
    });
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["open"] == json!(true) && v["opens"] == json!(1)
    });
    wait_window(cx, handle, |window, _| {
        window
            .try_find("featured-colors")
            .is_some_and(|item| item.visible())
    });
    cx.update_window(handle, |_, window, _| {
        let triggers = window.find_all("trigger");
        let trigger = triggers.last().unwrap().bounds();
        let colors = window.find("featured-colors").bounds();
        assert!(
            colors.bottom() <= trigger.origin.y,
            "bottom-left anchor must open above its trigger"
        );
    })
    .unwrap();
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/color-picker-open.png")
        .unwrap();
    click_featured(cx, handle, 1, 3);
    poll(cx, handle, probe, errors, |v| {
        v["value"] == json!("#00FF00") && v["changes"] == json!(1) && v["open"] == json!(false)
    });
    cx.update_window(handle, |_, window, cx| {
        let position = window.find_all("trigger")[0].bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    wait_window(cx, handle, |window, _| {
        window
            .try_find("featured-colors")
            .is_some_and(|item| item.visible())
    });
    click_featured(cx, handle, 0, 2);
    poll(cx, handle, probe, errors, |v| {
        v["background"] == json!("#0000FF") && v["selectChanges"] == json!(1)
    });
    command(commands, "reset");
    poll(cx, handle, probe, errors, |v| {
        v["value"] == json!("#FF000080") && v["background"] == json!("#FFFF00")
    });
    assert_eq!(probe.borrow()["changes"], json!(1));
    assert_eq!(probe.borrow()["selectChanges"], json!(1));
    command(commands, "preview");
    poll(cx, handle, probe, errors, |v| {
        v["native"]["preview"] == json!("#00FF00")
    });
    assert_eq!(probe.borrow()["native"]["value"], json!("#FF000080"));
    assert_eq!(probe.borrow()["changes"], json!(1));
    command(commands, "clear-preview");
    poll(cx, handle, probe, errors, |v| {
        v["native"]["preview"] == v["native"]["value"]
    });
    command(commands, "hsla");
    poll(cx, handle, probe, errors, |v| v["changes"] == json!(2));
    command(commands, "snapshot");
    poll(cx, handle, probe, errors, |v| {
        (v["native"]["hsla"]["h"].as_f64().unwrap_or(-1.) - 0.4321).abs() < 0.00001
    });
    assert!(
        (probe.borrow()["native"]["hsla"]["a"].as_f64().unwrap() - 0.7654).abs() < 0.00001,
        "hex echo must preserve native HSLA precision"
    );
    let opens = probe.borrow()["opens"].clone();
    let tabs = probe.borrow()["tabs"].clone();
    command(commands, "open-tab");
    poll(cx, handle, probe, errors, |v| {
        v["open"] == json!(true) && v["tab"] == json!(1)
    });
    wait_window(cx, handle, |window, _| {
        window.find_all_by_role(Role::Slider).len() == 4
    });
    assert_eq!(
        probe.borrow()["opens"],
        opens,
        "controlled opening must not emit user feedback"
    );
    assert_eq!(probe.borrow()["tabs"], tabs);
    cx.update_window(handle, |_, window, cx| {
        let sliders = window.find_all_by_role(Role::Slider);
        let bounds = sliders[3].bounds();
        let position = gpui_kit::point(bounds.left() + bounds.size.width * 0.4, bounds.center().y);
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["changes"].as_u64().unwrap_or(0) > 2
    });
    command(commands, "snapshot");
    poll(cx, handle, probe, errors, |v| {
        (v["native"]["hsla"]["a"].as_f64().unwrap_or(-1.) - 0.4).abs() < 0.03
    });
    assert!(
        (probe.borrow()["native"]["hsla"]["h"].as_f64().unwrap() - 0.4321).abs() < 0.00001,
        "slider echo must preserve untouched HSLA channels"
    );
    let committed_changes = probe.borrow()["changes"].as_u64().unwrap() + 1;
    command(commands, "commit-hex");
    poll(cx, handle, probe, errors, |v| {
        v["changes"] == json!(committed_changes) && v["open"] == json!(false)
    });
    assert!(
        probe.borrow()["value"].as_str().unwrap().ends_with("44"),
        "RGBA shorthand must retain alpha"
    );
    let before_hex_edit = probe.borrow()["changes"].as_u64().unwrap();
    let committed_value = probe.borrow()["value"].clone();
    command(commands, "open-tab");
    poll(cx, handle, probe, errors, |value| {
        value["open"] == json!(true)
    });
    wait_window(cx, handle, |window, _| {
        window
            .find_all_by_role(Role::TextInput)
            .iter()
            .any(|input| input.visible())
    });
    cx.update_window(handle, |_, window, cx| {
        let position = window.find_all_by_role(Role::TextInput)[0]
            .bounds()
            .center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
        window.press("secondary-a", cx);
        window.input("#00FFFF", cx);
    })
    .unwrap();
    command(commands, "snapshot");
    poll(cx, handle, probe, errors, |value| {
        value["native"]["preview"] == json!("#00FFFF")
    });
    assert_eq!(
        probe.borrow()["changes"],
        json!(before_hex_edit),
        "typing previews without committing"
    );
    assert_eq!(probe.borrow()["value"], committed_value);
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["changes"] == json!(before_hex_edit + 1)
            && value["value"] == json!("#00FFFF")
            && value["open"] == json!(false)
    });
    let accepted_picker = probe.borrow()["value"].clone();
    let rejected_changes = probe.borrow()["changes"].as_u64().unwrap() + 1;
    command(commands, "veto-picker");
    poll(cx, handle, probe, errors, |value| {
        value["veto"] == json!(true) && value["changes"] == json!(rejected_changes)
    });
    command(commands, "snapshot");
    poll(cx, handle, probe, errors, |value| {
        value["native"]["value"] == accepted_picker
    });
    assert_eq!(probe.borrow()["value"], accepted_picker);
    command(commands, "select-snapshot");
    poll(cx, handle, probe, errors, |value| {
        value["native"]["value"] != accepted_picker
    });
    let accepted_background = probe.borrow()["native"]["value"].clone();
    let rejected_select_changes = probe.borrow()["selectChanges"].as_u64().unwrap() + 1;
    command(commands, "veto-select");
    poll(cx, handle, probe, errors, |value| {
        value["selectChanges"] == json!(rejected_select_changes)
    });
    command(commands, "select-snapshot");
    poll(cx, handle, probe, errors, |value| {
        value["native"]["value"] == accepted_background
    });
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/extra-controls.png")
        .unwrap();
    println!(
        "verified ColorPicker/ColorSelect palette, anchor, HSLA precision, preview, alpha, controlled config and live Stepper slots/disabled activation"
    );
}
