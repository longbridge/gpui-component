//! Real native text-only footer parts: retained focus and keyboard activation.
use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, App, Entity, HeadlessAppContext, InputEvent, KeyDownEvent, KeyUpEvent,
    Keystroke, Window,
    component::WindowExt,
    test::{TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn physical_press(window: &mut Window, cx: &mut App, key: &str) {
    window.render_frame(cx);
    let keystroke = Keystroke::parse(key).unwrap();
    window.dispatch_event(
        KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        }
        .to_platform_input(),
        cx,
    );
    window.dispatch_event(KeyUpEvent { keystroke }.to_platform_input(), cx);
    window.render_frame(cx);
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
            include_str!("../verify_dialog_parts.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |v| {
        v["fixture"] == "dialog_parts"
    });
    // Establish native Rust's Enter promise before asserting the same adapter behavior.
    let baseline = Rc::new(Cell::new((0, 0)));
    cx.update_window(handle, |_, window, cx| {
        let focus = cx.focus_handle();
        let button_focus = focus.clone();
        let counts = baseline.clone();
        let native = window.open_dialog_with_handle(cx, move |dialog, _, _| {
            let ok = counts.clone();
            let cancel = counts.clone();
            dialog
                .title("Native close baseline")
                .footer(
                    gpui_kit::component::dialog::DialogClose::new().trigger(|button| {
                        button
                            .track_focus(&button_focus)
                            .label("Native close baseline")
                    }),
                )
                .on_ok(move |_, _, _| {
                    let (ok_count, cancel_count) = ok.get();
                    ok.set((ok_count + 1, cancel_count));
                    true
                })
                .on_cancel(move |_, _, _| {
                    let (ok_count, cancel_count) = cancel.get();
                    cancel.set((ok_count, cancel_count + 1));
                    true
                })
        });
        window.render_frame(cx);
        window.focus(&focus, cx);
        window.render_frame(cx);
        assert!(
            focus.is_focused(window),
            "native Close baseline has real focus"
        );
        physical_press(window, cx, "enter");
        assert_eq!(
            baseline.get(),
            (1, 0),
            "native Dialog Enter invokes its default Confirm action even on Close"
        );
        assert!(!native.is_open(window, cx));
    })
    .unwrap();

    let mut confirms = 0;
    let mut cancels = 0;
    for (index, (part, label, key)) in [
        ("action", "Save text part", "enter"),
        ("action", "Save text part", "space"),
        ("cancel", "Cancel text part", "enter"),
        ("cancel", "Cancel text part", "space"),
    ]
    .into_iter()
    .enumerate()
    {
        // An independently owned host dialog must survive each native part's close.
        let (host, host_focus) = cx
            .update_window(handle, |_, window, cx| {
                let host = window
                    .open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Protected host"));
                (
                    host,
                    window.focused(cx).expect("host dialog receives focus"),
                )
            })
            .unwrap();
        commands
            .borrow()
            .as_ref()
            .unwrap()
            .emit("command", json!("open"));
        poll(cx, handle, probe, errors, |v| v["open"] == true);
        // All four named slots mount ordinary native NSH subtrees.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            let mounted = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    [
                        "Named header action",
                        "Named title content",
                        "Named body content",
                        label,
                    ]
                    .into_iter()
                    .all(|name| {
                        window
                            .try_find_by_label(name)
                            .is_some_and(|node| node.visible())
                    })
                })
                .unwrap();
            if mounted {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "named dialog slots did not mount"
            );
            std::thread::yield_now();
        }
        click(cx, handle, "Named header action");
        poll(cx, handle, probe, errors, |v| {
            v["headerClicks"] == index + 1 && v["open"] == true
        });
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            // Focus is requested after the live slot has had a chance to mount.
            commands
                .borrow()
                .as_ref()
                .unwrap()
                .emit("command", json!(part));
            cx.run_until_parked();
            let focused = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    window
                        .try_find_by_label(label)
                        .is_some_and(|node| node.focused() == Some(true))
                })
                .unwrap();
            if focused {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "text-only {part} must expose actual native Button focus"
            );
            std::thread::yield_now();
        }
        cx.update_window(handle, |_, window, cx| physical_press(window, cx, key))
            .unwrap();
        if part == "action" || key == "enter" {
            confirms += 1;
        } else {
            cancels += 1;
        }
        poll(cx, handle, probe, errors, |v| {
            v["open"] == false
                && v["confirms"] == confirms
                && v["cancels"] == cancels
                && v["closes"] == confirms + cancels
        });
        cx.update_window(handle, |_, window, cx| {
            assert!(
                host.is_open(window, cx),
                "footer action must only remove its owned dialog"
            );
            assert_eq!(
                window.focused(cx),
                Some(host_focus),
                "native part restores its live host opener"
            );
            host.close(window, cx);
        })
        .unwrap();
    }
    println!(
        "verified text-only DialogAction/DialogClose native focus, Enter/Space, single events and scoped host protection"
    );
}
