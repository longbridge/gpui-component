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
            include_str!("../verify_layout.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("layout")
    });
    click(cx, handle, "Child 0");
    poll(cx, handle, probe, errors, |value| {
        value["presses"] == json!(1)
    });
    click(cx, handle, "Child 1");
    poll(cx, handle, probe, errors, |value| {
        value["presses"] == json!(2)
    });
    click(cx, handle, "Second tab");
    poll(cx, handle, probe, errors, |value| {
        value["selected"] == json!("second")
    });
    cx.update_window(handle, |_, window, _| {
        assert!(window.find_by_label("Second content").visible());
        assert!(window.try_find_by_label("First content").is_none());
    })
    .unwrap();
    click(cx, handle, "Disabled tab");
    cx.run_until_parked();
    assert_eq!(probe.borrow()["changes"], json!(1));
    cx.update_window(handle, |_, window, cx| {
        window.press("right", cx);
        window.press("space", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["selected"] == json!("first")
    });
    cx.update_window(handle, |_, window, cx| {
        window.press("left", cx);
        window.press("space", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["selected"] == json!("second")
    });
    // Both slot inputs remain mounted in NSH; only the selected slot may
    // contribute a logical Tab stop after previously rendering the other.
    cx.update_window(handle, |_, window, cx| window.press("tab", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["focused"] == json!("second")
    });
    click(cx, handle, "Details");
    poll(cx, handle, probe, errors, |value| {
        value["open"] == json!(["details"])
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.find_by_label("Expanded content").visible());
    })
    .unwrap();
    cx.update_window(handle, |_, window, cx| window.press("space", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["open"] == json!([])
    });
    cx.update_window(handle, |_, window, cx| window.press("space", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["open"] == json!(["details"])
    });
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/layout.png")
        .unwrap();
    cx.update(|cx| {
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_layout_systems.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("layout-systems")
    });
    click(cx, handle, "Next slide");
    poll(cx, handle, probe, errors, |value| {
        value["selected"] == json!(1)
    });
    click(cx, handle, "Questionnaire");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("survey")
    });
    click(cx, handle, "First answer");
    poll(cx, handle, probe, errors, |value| {
        value["survey"]["answer"]["choices"] == json!(["a"])
    });
    click(cx, handle, "Submit");
    poll(cx, handle, probe, errors, |value| {
        value["submitted"]["answer"]["choices"] == json!(["a"])
    });
    click(cx, handle, "Messages");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("messages")
    });
    click(cx, handle, "message-19");
    poll(cx, handle, probe, errors, |value| {
        value["clicks"] == json!(1)
    });
    click(cx, handle, "Navigation");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("navigation")
    });
    click(cx, handle, "Projects");
    poll(cx, handle, probe, errors, |value| {
        value["destination"] == json!("projects")
    });
    cx.update_window(handle, |_, window, cx| {
        window.press("up", cx);
        window.press("space", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["destination"] == json!("home")
    });
    click(cx, handle, "Virtual list");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("virtual")
    });
    click(cx, handle, "virtual-row-b");
    poll(cx, handle, probe, errors, |value| {
        value["clicks"] == json!(2)
    });
    println!(
        "verified native layout selection, disabled tab, accordion, live NSH children, carousel, questionnaire, message scrolling, virtual rows and navigation"
    );
}
