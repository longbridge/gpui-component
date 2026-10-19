use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, MouseButton, Window,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

fn wait_for(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    description: &str,
    predicate: impl Fn(&Window) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        if cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                predicate(window)
            })
            .unwrap()
        {
            return;
        }
        if Instant::now() >= deadline {
            shot(cx, handle, "surface-failure");
            panic!("surface did not reach {description}");
        }
        std::thread::yield_now();
    }
}
fn wait_label(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, label: &str) {
    wait_for(cx, handle, label, |window| {
        window
            .try_find_by_label(label)
            .is_some_and(|element| element.visible())
    });
}
fn hover(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, label: &str) {
    cx.update_window(handle, |_, window, cx| {
        let position = window.find_by_label(label).bounds().center();
        window.pointer_move(position, None, cx);
        window.render_frame(cx);
    })
    .unwrap();
}
fn press(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, key: &str) {
    cx.update_window(handle, |_, window, cx| window.press(key, cx))
        .unwrap();
}
fn shot(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, name: &str) {
    cx.capture_screenshot(handle)
        .unwrap()
        .save(format!("target/nsh-verify/{name}.png"))
        .unwrap();
}

pub(super) fn run(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    view: &Entity<AppView>,
    probe: &Rc<RefCell<Value>>,
    errors: &Rc<RefCell<Vec<String>>>,
) {
    cx.update(|cx| {
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_surfaces.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("surfaces")
    });
    click(cx, handle, "Open popover");
    poll(cx, handle, probe, errors, |value| value["open"] == true);
    wait_label(cx, handle, "Popover item");
    shot(cx, handle, "popover");
    click(cx, handle, "Popover item");
    poll(cx, handle, probe, errors, |value| {
        value["presses"] == 1 && value["open"] == false
    });
    click(cx, handle, "Increment");
    poll(cx, handle, probe, errors, |value| value["roundtrip"] == 1);
    assert_eq!(
        probe.borrow()["changes"],
        json!(1),
        "owner close echoed a user change"
    );
    // Desktop Buttons preserve prior focus on mouse activation. Establish
    // keyboard focus explicitly before checking keyboard restoration.
    click(cx, handle, "Focus popover");
    wait_for(cx, handle, "popover keyboard focus", |window| {
        window.find_by_label("Open popover").focused() == Some(true)
    });
    press(cx, handle, "space");
    poll(cx, handle, probe, errors, |value| value["open"] == true);
    press(cx, handle, "escape");
    poll(cx, handle, probe, errors, |value| value["open"] == false);
    wait_for(cx, handle, "popover focus restoration", |window| {
        window.find_by_label("Open popover").focused() == Some(true)
    });
    press(cx, handle, "space");
    poll(cx, handle, probe, errors, |value| value["open"] == true);
    wait_label(cx, handle, "Popover item");
    press(cx, handle, "escape");
    poll(cx, handle, probe, errors, |value| value["open"] == false);

    click(cx, handle, "Open menu");
    wait_label(cx, handle, "Copy item");
    hover(cx, handle, "Disabled submenu");
    cx.run_until_parked();
    cx.advance_clock(Duration::from_millis(100));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window.try_find_by_label("Blocked item").is_none(),
            "disabled submenu opened"
        );
    })
    .unwrap();
    click(cx, handle, "Unavailable");
    click(cx, handle, "Copy item");
    poll(cx, handle, probe, errors, |value| {
        value["selected"] == "copy" && value["menuOpen"] == false && value["selections"] == 1
    });
    click(cx, handle, "Open menu");
    hover(cx, handle, "More");
    wait_label(cx, handle, "Paste item");
    press(cx, handle, "right");
    press(cx, handle, "enter");
    poll(cx, handle, probe, errors, |value| {
        value["selected"] == "paste" && value["menuOpen"] == false && value["selections"] == 2
    });

    click(cx, handle, "Switch trigger");
    poll(cx, handle, probe, errors, |value| value["custom"] == true);
    wait_label(cx, handle, "Custom menu trigger");
    press(cx, handle, "tab");
    wait_for(
        cx,
        handle,
        "custom trigger without a stale fallback tab stop",
        |window| window.find_by_label("Custom menu trigger").focused() == Some(true),
    );
    cx.update_window(handle, |_, window, _| {
        assert!(
            window.try_find_by_label("Open menu").is_none(),
            "custom trigger was nested in an extra Button"
        )
    })
    .unwrap();
    press(cx, handle, "space");
    poll(cx, handle, probe, errors, |value| value["menuOpen"] == true);
    wait_label(cx, handle, "Copy item");
    press(cx, handle, "escape");
    poll(cx, handle, probe, errors, |value| {
        value["menuOpen"] == false
    });

    cx.update_window(handle, |_, window, cx| {
        let position = window.find_by_label("Context trigger").bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Right, cx);
        window.pointer_up(position, MouseButton::Right, cx);
    })
    .unwrap();
    wait_label(cx, handle, "Copy item");
    click(cx, handle, "Copy item");
    poll(cx, handle, probe, errors, |value| value["selections"] == 3);

    hover(cx, handle, "Hover card");
    cx.run_until_parked();
    cx.advance_clock(Duration::from_millis(100));
    wait_label(cx, handle, "Hover item");
    click(cx, handle, "Hover item");
    poll(cx, handle, probe, errors, |value| value["presses"] == 2);
    hover(cx, handle, "Increment");
    cx.run_until_parked();
    cx.advance_clock(Duration::from_millis(100));
    poll(cx, handle, probe, errors, |value| value["hover"] == false);
    hover(cx, handle, "Tooltip trigger");
    cx.run_until_parked();
    cx.advance_clock(Duration::from_secs(1));
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.advance_clock(Duration::from_secs(1));
    wait_for(cx, handle, "tooltip popup", |window| {
        window
            .try_find_by_label("Tooltip live content")
            .is_some_and(|element| element.visible())
    });
    shot(cx, handle, "tooltip");
    hover(cx, handle, "Increment");
    cx.run_until_parked();
    cx.advance_clock(Duration::from_secs(1));
    wait_for(cx, handle, "tooltip dismissal", |window| {
        !window
            .try_find_by_label("Tooltip live content")
            .is_some_and(|element| element.visible())
    });

    wait_label(cx, handle, "Native commands");
    click(cx, handle, "Reject invalid items");
    poll(cx, handle, probe, errors, |value| {
        value["validationError"]
            .as_str()
            .is_some_and(|error| error.contains("unique"))
    });
    assert_eq!(probe.borrow()["selections"], json!(3));
    println!(
        "verified live popover control/focus, dropdown/context menu activation, disabled items, nested menu keyboard selection, custom trigger Tab, hover-card content and tooltip lifecycle; NativeMenu validation only (OS tracking requires a native window)"
    );
}
