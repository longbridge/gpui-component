//! Real NSH children, retained dock panels, persistence and splitter gestures.
use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, MouseButton, point, px,
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
            include_str!("../verify_dock.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("dock")
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let visible = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                [
                    "Alpha action",
                    "Beta action",
                    "Left split action",
                    "Right split action",
                ]
                .iter()
                .all(|label| {
                    window
                        .try_find_by_label(label)
                        .is_some_and(|element| element.visible())
                })
            })
            .unwrap();
        if visible {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "named NSH dock/splitter slots did not render"
        );
        std::thread::yield_now();
    }
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/dock-initial.png")
        .unwrap();
    click(cx, handle, "Alpha action");
    click(cx, handle, "Beta action");
    poll(cx, handle, probe, errors, |value| {
        value["panelCount"] == json!(11)
    });
    click(cx, handle, "Left split action");
    click(cx, handle, "Right split action");
    poll(cx, handle, probe, errors, |value| {
        value["splitCount"] == json!(11)
    });
    click(cx, handle, "Dock editor");
    cx.update_window(handle, |_, window, cx| window.input("Ada", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["text"] == json!("Ada")
    });
    cx.update_window(handle, |_, window, cx| {
        window.press("home", cx);
        window.input("X", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["text"] == json!("XAda")
    });
    let command = |name: &str| {
        commands
            .borrow()
            .as_ref()
            .expect("probe mounted")
            .emit("command", json!(name))
    };
    command("save");
    poll(cx, handle, probe, errors, |value| {
        value["saved"].is_object()
    });
    command("zoom");
    poll(cx, handle, probe, errors, |value| {
        value["zoomed"] == json!(true)
    });
    command("unzoom");
    poll(cx, handle, probe, errors, |value| {
        value["zoomed"] == json!(false)
    });
    let before_close = probe.borrow()["changes"].as_u64().unwrap();
    command("close");
    poll(cx, handle, probe, errors, |value| {
        value["changes"].as_u64().unwrap_or(0) > before_close
    });
    let before_restore = probe.borrow()["changes"].as_u64().unwrap();
    command("restore");
    poll(cx, handle, probe, errors, |value| {
        value["changes"].as_u64().unwrap_or(0) > before_restore
    });
    // The restored registry returns the same NSH panel entities and live slots.
    click(cx, handle, "Beta action");
    poll(cx, handle, probe, errors, |value| {
        value["panelCount"] == json!(21)
    });
    command("resize");
    poll(cx, handle, probe, errors, |value| {
        value["resized"]
            .as_array()
            .is_some_and(|sizes| !sizes.is_empty())
    });
    // Drive the actual splitter handle, then release; resizing must reach NSH.
    let before = probe.borrow()["resized"][0].as_f64().unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let bounds = window.find_by_label("Left split action").bounds();
        let start = point(bounds.left() + px(before as f32), bounds.top() + px(70.));
        let end = point(start.x + px(35.), start.y);
        window.pointer_move(start, None, cx);
        window.pointer_down(start, MouseButton::Left, cx);
        window.pointer_move(
            point(start.x + px(12.), start.y),
            Some(MouseButton::Left),
            cx,
        );
        window.pointer_move(end, Some(MouseButton::Left), cx);
        window.pointer_up(end, MouseButton::Left, cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["resized"][0]
            .as_f64()
            .is_some_and(|size| (size - before).abs() > 5.)
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let start = window.find_by_label("Standalone resize").bounds().center();
        window.pointer_move(start, None, cx);
        window.pointer_down(start, MouseButton::Left, cx);
        window.pointer_move(
            point(start.x - px(12.), start.y),
            Some(MouseButton::Left),
            cx,
        );
        window.render_frame(cx);
        let end = point(start.x - px(50.), start.y);
        window.pointer_move(end, Some(MouseButton::Left), cx);
        window.pointer_up(end, MouseButton::Left, cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["handleDelta"]
            .as_f64()
            .is_some_and(|delta| delta < -20.)
    });
    let before_drag = probe.borrow()["changes"].as_u64().unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let tabs = window.find_all_by_role(gpui_kit::Role::Tab);
        assert_eq!(tabs.len(), 2, "both dock tab groups are observed");
        let source = tabs[1].bounds().center();
        let target = tabs[0].bounds().center();
        window.pointer_move(source, None, cx);
        window.pointer_down(source, MouseButton::Left, cx);
        window.pointer_move(
            point(source.x + px(12.), source.y),
            Some(MouseButton::Left),
            cx,
        );
        window.render_frame(cx);
        window.pointer_move(target, Some(MouseButton::Left), cx);
        window.render_frame(cx);
        window.pointer_up(target, MouseButton::Left, cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["changes"].as_u64().unwrap_or(0) > before_drag
    });
    command("select-alpha");
    poll(cx, handle, probe, errors, |value| {
        value["active"] == json!("alpha")
    });
    command("focus-editor");
    // Refocusing through the native command leaves the retained caret alone.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let focused = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find_by_label("Dock editor")
                    .is_some_and(|element| element.focused() == Some(true))
            })
            .unwrap();
        if focused {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "restored dock editor did not regain focus"
        );
        std::thread::yield_now();
    }
    cx.update_window(handle, |_, window, cx| window.input("Y", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["text"] == json!("XYAda")
    });
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/dock-resizable.png")
        .unwrap();
    println!(
        "verified live NSH dock/splitter content, panel actions, zoom, save/restore and native resize/tab drags"
    );
}
