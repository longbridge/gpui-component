use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext,
    test::{TestQueryExt, TestWindowExt},
};
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
            include_str!("../verify_content.js").to_owned(),
        )))
    });
    for index in 0..19 {
        poll(cx, handle, probe, errors, |value| {
            value["fixture"] == json!("content") && value["index"] == json!(index)
        });
        assert!(
            probe.borrow()["error"].is_null(),
            "content adapter failed: {:?}",
            probe.borrow()
        );
        let name = probe.borrow()["name"].as_str().unwrap().to_owned();
        if name == "MarkdownSlots" || name == "InlineMarkdownSlots" {
            let label = if name == "MarkdownSlots" {
                "Markdown slot action"
            } else {
                "Inline slot action"
            };
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                cx.run_until_parked();
                let present = cx
                    .update_window(handle, |_, window, cx| {
                        window.render_frame(cx);
                        window.try_find_by_label(label).is_some()
                    })
                    .unwrap();
                if present {
                    break;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "live Markdown slot did not render"
                );
                std::thread::yield_now();
            }
            click(cx, handle, label);
            poll(cx, handle, probe, errors, |value| {
                value["slotClicks"]
                    == if name == "MarkdownSlots" {
                        json!(1)
                    } else {
                        json!(2)
                    }
            });
        }
        if name == "Diff" {
            click(cx, handle, "after");
            poll(cx, handle, probe, errors, |value| {
                value["line"]["path"] == json!("demo.txt") && value["line"]["line"] == json!(1)
            });
            assert_eq!(probe.borrow()["line"]["side"], json!("modified"));
        }
        cx.capture_screenshot(handle)
            .unwrap()
            .save(format!("target/nsh-verify/content-{name}.png"))
            .unwrap();
        click(cx, handle, "Next content");
    }
    println!(
        "verified NSH native rich text, code, diff, 7 chart families, low-level Plot, Image and unsupported speech presentation"
    );
}
