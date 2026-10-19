//! Mixed Component tab order uses real generated native bindings and existing host.
use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, Focusable, HeadlessAppContext,
    test::{TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};

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
            include_str!("component_focus.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |p| {
        p["fixture"] == json!("component-focus")
    });
    cx.update_window(handle, |_, window, cx| {
        window.focus(&view.focus_handle(cx), cx)
    })
    .unwrap();
    for expected in [
        "Increment",
        "Focus checkbox three",
        "Focus radio zero",
        "Focus switch zero",
        "Increment",
    ] {
        cx.update_window(handle, |_, window, cx| window.press("tab", cx))
            .unwrap();
        // Poll the native focus instead of relying on JS focus events absent from these APIs.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            let focused = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    window.find_by_label(expected).focused() == Some(true)
                })
                .unwrap();
            if focused {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "mixed Component Tab order expected {expected}"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    for label in [
        "Focus button negative",
        "Focus checkbox excluded",
        "Focus radio negative",
        "Focus switch excluded",
    ] {
        let before = probe.borrow()["events"].as_u64().unwrap();
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
        })
        .unwrap();
        click(cx, handle, label);
        poll(cx, handle, probe, errors, |p| {
            p["events"] == json!(before + 1)
        });
        commands
            .borrow()
            .as_ref()
            .unwrap()
            .emit("command", json!(label));
        poll(cx, handle, probe, errors, |p| {
            p["commanded"] == json!(label)
        });
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(
                window.find_by_label(label).focused(),
                Some(true),
                "excluded controls remain ref-focusable"
            );
        })
        .unwrap();
        cx.update_window(handle, |_, window, cx| window.press("space", cx))
            .unwrap();
        poll(cx, handle, probe, errors, |p| {
            p["events"] == json!(before + 2)
        });
    }
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/component-focus.png")
        .unwrap();
    println!(
        "verified mixed Component positive Tab order, excluded negative/tabStop controls, pointer/ref focus and keyboard activation"
    );
}
