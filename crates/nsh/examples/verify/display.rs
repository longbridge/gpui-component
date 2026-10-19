//! Real NSH signals and real GPUI Component activation, including group keyboard events.
use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, Focusable, HeadlessAppContext,
    test::{TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

fn wait_focus(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, label: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let focused = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find_by_label(label)
                    .is_some_and(|element| element.focused() == Some(true))
            })
            .unwrap();
        if focused {
            return;
        }
        if Instant::now() >= deadline {
            let observed = cx
                .update_window(handle, |_, window, _| {
                    [
                        "Increment",
                        "Emphasis",
                        "Bold",
                        "Italic",
                        "Red",
                        "Blue",
                        "Small",
                        "Large",
                        "Copy fixture",
                        "Export",
                        "Export options",
                    ]
                    .map(|name| (name, window.try_find_by_label(name)))
                })
                .unwrap();
            panic!("NSH Tab did not focus {label}: {observed:#?}");
        }
        std::thread::yield_now();
    }
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
            include_str!("../verify_display.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |p| {
        p["fixture"] == json!("display")
    });
    let command = |name: &str| {
        commands
            .borrow()
            .as_ref()
            .unwrap()
            .emit("command", json!(name))
    };
    // Production hosts initially focus AppView. Component Button deliberately
    // keeps pointer activation from moving keyboard focus, so enter by Tab.
    cx.update_window(handle, |_, window, cx| {
        window.focus(&view.focus_handle(cx), cx)
    })
    .unwrap();
    cx.update_window(handle, |_, window, cx| window.press("tab", cx))
        .unwrap();
    wait_focus(cx, handle, "Increment");
    click(cx, handle, "Increment");
    poll(cx, handle, probe, errors, |p| p["changes"] == json!(1));
    for label in [
        "Emphasis",
        "Bold",
        "Italic",
        "Red",
        "Blue",
        "Small",
        "Large",
        "Copy fixture",
        "Export",
        "Export options",
    ] {
        cx.update_window(handle, |_, window, cx| window.press("tab", cx))
            .unwrap();
        wait_focus(cx, handle, label);
        if label == "Blue" {
            cx.update_window(handle, |_, window, cx| window.press("space", cx))
                .unwrap();
            poll(cx, handle, probe, errors, |p| p["radio"] == json!("blue"));
        }
        if label == "Copy fixture" {
            cx.update_window(handle, |_, window, cx| window.press("space", cx))
                .unwrap();
            poll(cx, handle, probe, errors, |p| p["copyCount"] == json!(1));
        }
    }
    cx.update_window(handle, |_, window, cx| window.press("shift-tab", cx))
        .unwrap();
    wait_focus(cx, handle, "Export");
    command("reset");
    poll(cx, handle, probe, errors, |p| p["radio"] == json!("red"));
    command("clear-count");
    poll(cx, handle, probe, errors, |p| p["changes"] == json!(0));
    click(cx, handle, "Emphasis");
    poll(cx, handle, probe, errors, |p| p["toggle"] == json!(true));
    // The same retained toggle consumes Space and emits one controlled update.
    cx.update_window(handle, |_, window, cx| window.press("space", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |p| {
        p["toggle"] == json!(false) && p["changes"] == json!(2)
    });
    click(cx, handle, "Bold");
    poll(cx, handle, probe, errors, |p| {
        p["toggles"] == json!(["bold"])
    });
    cx.update_window(handle, |_, window, cx| window.press("space", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |p| {
        p["toggles"] == json!([]) && p["changes"] == json!(4)
    });
    click(cx, handle, "Bold");
    poll(cx, handle, probe, errors, |p| {
        p["toggles"] == json!(["bold"])
    });
    command("reorder");
    poll(cx, handle, probe, errors, |p| {
        p["order"] == json!(["mono", "italic", "bold"])
    });
    assert_eq!(probe.borrow()["toggles"], json!(["bold"]));
    wait_focus(cx, handle, "Bold");
    cx.update_window(handle, |_, window, cx| window.press("shift-tab", cx))
        .unwrap();
    wait_focus(cx, handle, "Italic");
    cx.update_window(handle, |_, window, cx| window.press("tab", cx))
        .unwrap();
    wait_focus(cx, handle, "Bold");
    click(cx, handle, "Blue");
    poll(cx, handle, probe, errors, |p| p["radio"] == json!("blue"));
    click(cx, handle, "Small");
    poll(cx, handle, probe, errors, |p| {
        p["buttons"] == json!(["small"])
    });
    for label in ["Red", "Blue", "Small"] {
        cx.update_window(handle, |_, window, cx| window.press("tab", cx))
            .unwrap();
        wait_focus(cx, handle, label);
    }
    cx.update_window(handle, |_, window, cx| window.press("space", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |p| p["buttons"] == json!([]));
    click(cx, handle, "Large");
    poll(cx, handle, probe, errors, |p| {
        p["buttons"] == json!(["large"])
    });
    // Rejected requests leave the supplied value authoritative, even when it
    // equals the constructor's initial value. Accepted requests then round trip.
    for expected in [1, 2] {
        cx.update_window(handle, |_, window, cx| {
            window.within("rating").click(1usize, cx)
        })
        .unwrap();
        poll(cx, handle, probe, errors, |p| {
            p["ratingRequests"].as_array().unwrap().len() == expected
        });
    }
    assert_eq!(probe.borrow()["ratingRequests"], json!([0, 0]));
    assert_eq!(probe.borrow()["rating"], json!(6));
    command("accept-rating");
    poll(cx, handle, probe, errors, |p| {
        p["acceptRating"] == json!(true)
    });
    cx.update_window(handle, |_, window, cx| {
        window.within("rating").click(1usize, cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |p| p["rating"] == json!(0));
    cx.update_window(handle, |_, window, cx| {
        window.within("rating").click(1usize, cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |p| p["rating"] == json!(1));
    cx.run_until_parked();
    cx.advance_clock(Duration::from_secs(3));
    cx.run_until_parked();
    click(cx, handle, "Copy fixture");
    poll(cx, handle, probe, errors, |p| {
        p["copied"] == json!("NSH clipboard payload") && p["copyCount"] == json!(2)
    });
    click(cx, handle, "Export");
    click(cx, handle, "Export options");
    click(cx, handle, "Export PDF");
    poll(cx, handle, probe, errors, |p| p["exported"] == json!("pdf"));
    let count = probe.borrow()["changes"].clone();
    command("disable");
    poll(cx, handle, probe, errors, |p| p["disabled"] == json!(true));
    click(cx, handle, "Emphasis");
    click(cx, handle, "Bold");
    click(cx, handle, "Red");
    click(cx, handle, "Small");
    cx.update_window(handle, |_, window, cx| {
        window.within("rating").click(1usize, cx)
    })
    .unwrap();
    // Drain a complete worker round trip before checking for leaked callbacks.
    command("reset");
    poll(cx, handle, probe, errors, |p| {
        p["radio"] == json!("red") && p["toggles"] == json!([]) && p["buttons"] == json!([])
    });
    assert_eq!(
        probe.borrow()["changes"],
        count,
        "disabled controls and owner updates must not emit user changes"
    );
    command("enable");
    poll(cx, handle, probe, errors, |p| p["disabled"] == json!(false));
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/display-controls.png")
        .unwrap();
    click(cx, handle, "Export options");
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find_by_label("Export PDF").is_some());
    })
    .unwrap();
    command("displays");
    poll(cx, handle, probe, errors, |p| {
        p["mode"] == json!("displays")
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window.try_find_by_label("Export PDF").is_none(),
            "unmounting DropdownButton must dismiss its menu"
        );
        assert_eq!(
            window.find_by_label("Download").role(),
            Some(gpui_kit::Role::ProgressIndicator)
        );
        assert_eq!(
            window.find_by_label("Indexing").role(),
            Some(gpui_kit::Role::ProgressIndicator)
        );
    })
    .unwrap();
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/display-components.png")
        .unwrap();
    println!(
        "verified display controls, group pointer/keyboard activation, stable option identity, disabled suppression, clipboard callbacks and owner updates"
    );
}
