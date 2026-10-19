use crate::verification::{click, poll};
use gpui_kit::{AnyWindowHandle, Entity, HeadlessAppContext, test::TestWindowExt};
use gpui_kit_nsh::nsh::{self, AppView, Bundle};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
pub(super) fn run(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    view: &Entity<AppView>,
    probe: &Rc<RefCell<Value>>,
    errors: &Rc<RefCell<Vec<String>>>,
) {
    cx.update(|cx| {
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_inputs.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |v| {
        v["fixture"] == json!("inputs")
    });
    click(cx, handle, "Notes");
    cx.update_window(handle, |_, window, cx| {
        window.input("Two lines\nof notes", cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["textarea"] == json!("Two lines\nof notes")
    });
    click(cx, handle, "Select editor text");
    poll(cx, handle, probe, errors, |v| {
        v["focused"] == json!("editor")
    });
    cx.update_window(handle, |_, window, cx| window.input("Z", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| v["editor"] == json!("Zc"));
    cx.update_window(handle, |_, window, cx| window.input("Y", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| v["editor"] == json!("ZYc"));
    click(cx, handle, "Change language");
    poll(cx, handle, probe, errors, |v| {
        v["language"] == json!("javascript")
    });
    click(cx, handle, "Source code");
    cx.update_window(handle, |_, window, cx| {
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-z"
            } else {
                "ctrl-z"
            },
            cx,
        )
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| v["editor"] != json!("ZYc"));
    click(cx, handle, "Quantity");
    cx.update_window(handle, |_, window, cx| window.press("up", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| v["number"] == json!("4"));
    click(cx, handle, "Focus code");
    poll(cx, handle, probe, errors, |v| v["focused"] == json!("otp"));
    cx.update_window(handle, |_, window, cx| {
        for key in ["1", "2", "3", "4"] {
            window.press(key, cx);
        }
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["otp"] == json!("1234") && v["complete"] == json!(1)
    });
    cx.update_window(handle, |_, window, cx| window.press("5", cx))
        .unwrap();
    cx.run_until_parked();
    assert_eq!(probe.borrow()["otp"], json!("1234"));
    click(cx, handle, "Recipient");
    cx.update_window(handle, |_, window, cx| {
        window.press("end", cx);
        window.press("backspace", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["recipient"] == json!("To ") && v["content"]["tokens"] == json!([])
    });
    cx.update_window(handle, |_, window, cx| window.input("X", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["recipient"] == json!("To X")
    });
    cx.update_window(handle, |_, window, cx| window.input("Y", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["recipient"] == json!("To XY")
    });
    click(cx, handle, "Grouped notes");
    cx.update_window(handle, |_, window, cx| {
        window.input("Grouped\ntextarea", cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["groupValue"] == json!("Grouped\ntextarea")
    });
    click(cx, handle, "Switch group mode");
    poll(cx, handle, probe, errors, |v| {
        v["groupMode"] == json!("input") && v["groupValue"] == json!("Single")
    });
    click(cx, handle, "Grouped notes");
    cx.update_window(handle, |_, window, cx| window.input("!", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["groupValue"]
            .as_str()
            .is_some_and(|value| value.contains('!'))
    });
    click(cx, handle, "Enable language services");
    poll(cx, handle, probe, errors, |v| v["services"] == json!(true));
    click(cx, handle, "Source code");
    cx.update_window(handle, |_, window, cx| {
        window.press(
            if cfg!(target_os = "macos") {
                "cmd-a"
            } else {
                "ctrl-a"
            },
            cx,
        );
        window.input("pri", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |v| {
        v["requests"]
            .as_array()
            .is_some_and(|methods| methods.contains(&json!("textDocument/completion")))
    });
    // The real completion provider consumes the asynchronous NSH response;
    // selecting its menu entry edits through the original editor engine.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let visible = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find("completion-menu")
                    .is_some_and(|entry| entry.visible())
            })
            .unwrap();
        if visible {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "NSH completion response did not produce a native completion item"
        );
        std::thread::yield_now();
    }
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |v| v["editor"] == json!("print"));
    println!(
        "verified Textarea multiline, Editor controlled selection and undo across language changes, NumberInput stepping, OTP focus/completion"
    );
}
