use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, MouseButton, Role,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

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
            include_str!("../verify_datetime.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |v| {
        v["fixture"] == json!("datetime")
    });
    click(cx, handle, "2026-10-19");
    poll(cx, handle, probe, errors, |v| {
        v["calendar"] == json!("2026-10-19") && v["changes"] == json!(1)
    });
    // A disabled day must not emit a change.
    click(cx, handle, "2026-10-20");
    cx.run_until_parked();
    assert_eq!(probe.borrow()["changes"], json!(1));
    cx.update_window(handle, |_, window, cx| {
        let position = window.find("hour").bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
        window.press("up", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["time"] == json!("10:30:15") && v["changes"] == json!(2)
    });
    // Controlled replacement must preserve the selected minute segment and not
    // feed a host property update back as a user edit.
    cx.update_window(handle, |_, window, cx| window.press("right", cx))
        .unwrap();
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("reset"));
    poll(cx, handle, probe, errors, |v| {
        v["calendar"] == json!("2026-10-22")
            && v["picked"] == json!("2026-10-23")
            && v["time"] == json!("14:45:30")
    });
    assert_eq!(probe.borrow()["changes"], json!(2));
    cx.update_window(handle, |_, window, cx| window.press("up", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["time"] == json!("14:46:30") && v["changes"] == json!(3)
    });
    cx.update_window(handle, |_, window, cx| {
        let position = window.find_all_by_role(Role::ComboBox)[0].bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let visible = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find_by_label("Next week")
                    .is_some_and(|item| item.visible())
            })
            .unwrap();
        if visible {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "DatePicker preset popup did not open"
        );
        std::thread::yield_now();
    }
    click(cx, handle, "Next week");
    poll(cx, handle, probe, errors, |v| {
        v["picked"] == json!("2026-10-25T00:00:00") && v["changes"] == json!(4)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("navigate"));
    poll(cx, handle, probe, errors, |v| {
        v["navigation"]["year"] == json!(2026) && v["navigation"]["month"] == json!(11)
    });
    assert_eq!(
        probe.borrow()["changes"],
        json!(4),
        "navigation must not change the selected date"
    );
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("range"));
    poll(cx, handle, probe, errors, |v| {
        v["range"] == json!(true) && v["calendar"] == json!(["2026-11-10", null])
    });
    click(cx, handle, "2026-11-12");
    poll(cx, handle, probe, errors, |v| {
        v["calendar"] == json!(["2026-11-10", "2026-11-12"]) && v["changes"] == json!(5)
    });
    cx.update_window(handle, |_, window, cx| {
        let position = window.find_all_by_role(Role::ComboBox)[0].bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let visible = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find_by_label("November range")
                    .is_some_and(|item| item.visible())
            })
            .unwrap();
        if visible {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "range DatePicker popup did not open"
        );
        std::thread::yield_now();
    }
    click(cx, handle, "November range");
    poll(cx, handle, probe, errors, |v| {
        v["picked"] == json!(["2026-11-14T00:00:00", "2026-11-16T00:00:00"])
            && v["changes"] == json!(6)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("twelve"));
    poll(cx, handle, probe, errors, |v| v["cycle"] == json!("h12"));
    assert_eq!(probe.borrow()["changes"], json!(6));
    cx.update_window(handle, |_, window, cx| {
        let position = window.find("period").bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
        window.press("up", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["time"] == json!("02:46:00") && v["changes"] == json!(7)
    });
    // The owner can veto edits. Acknowledgements repeat the previous prop,
    // so a transport-level equality cache must not suppress their restoration.
    let accepted_calendar = probe.borrow()["calendar"].clone();
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("veto-calendar"));
    poll(cx, handle, probe, errors, |value| {
        value["veto"] == json!(true) && value["changes"] == json!(8)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("calendar-snapshot"));
    poll(cx, handle, probe, errors, |value| {
        value["navigation"]["value"] == accepted_calendar
    });
    cx.update_window(handle, |_, window, cx| window.press("up", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["changes"] == json!(9)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("time-snapshot"));
    poll(cx, handle, probe, errors, |value| {
        value["timeNative"]["value"] == json!("02:46:00")
    });
    let accepted_picker = probe.borrow()["picked"].clone();
    cx.update_window(handle, |_, window, cx| {
        let position = window.find_all_by_role(Role::ComboBox)[0].bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let visible = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find_by_label("December range")
                    .is_some_and(|item| item.visible())
            })
            .unwrap();
        if visible {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "veto DatePicker popup did not open"
        );
        std::thread::yield_now();
    }
    click(cx, handle, "December range");
    poll(cx, handle, probe, errors, |value| {
        value["changes"] == json!(10)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("picker-snapshot"));
    poll(cx, handle, probe, errors, |value| {
        value["dateNative"]["value"] == accepted_picker
    });
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/datetime.png")
        .unwrap();
    println!(
        "verified calendar selection, disabled dates, retained segmented keyboard editing and controlled date/time replacement"
    );
}
