use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, MouseButton, point, px,
    test::{TestEventExt, TestWindowExt},
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
            include_str!("../verify_alert.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == "alert"
    });
    click(cx, handle, "Open alert");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let ready = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find("dialog")
                    .is_some_and(|dialog| dialog.visible())
            })
            .unwrap();
        if ready {
            break;
        }
        assert!(Instant::now() < deadline, "alert did not open");
        std::thread::yield_now();
    }
    cx.update_window(handle, |_, window, cx| {
        window.within("dialog").click("ok", cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| value["confirms"] == 1);
    assert_eq!(
        probe.borrow()["open"],
        true,
        "confirm veto keeps the alert open"
    );
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("remove-close"));
    poll(cx, handle, probe, errors, |value| {
        value["closeButton"] == false
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window.within("dialog").try_find("close").is_none(),
            "removing the prop restores Alert's close-button default"
        );
        let position = point(px(3.), px(3.));
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("allow"));
    poll(cx, handle, probe, errors, |value| value["allow"] == true);
    assert_eq!(
        probe.borrow()["open"],
        true,
        "alert backdrop never dismisses it"
    );
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/alert.png")
        .unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.within("dialog").click("ok", cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["open"] == false && value["confirms"] == 2
    });
    assert_eq!(probe.borrow()["closes"], 1);
    assert_eq!(probe.borrow()["cancels"], 0);
    // Generated exports acknowledge the accepted open getter, including a veto.
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("veto-close"));
    poll(cx, handle, probe, errors, |value| value["veto"] == true);
    click(cx, handle, "Open alert");
    poll(cx, handle, probe, errors, |value| value["open"] == true);
    cx.update_window(handle, |_, window, cx| window.press("escape", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["open"] == true && value["cancels"] == 1 && value["closes"] == 2
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        let reopened = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                window
                    .try_find("dialog")
                    .is_some_and(|dialog| dialog.visible())
            })
            .unwrap();
        if reopened {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "controlled open veto must restore the actual native alert"
        );
        std::thread::yield_now();
    }
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("accept-close"));
    poll(cx, handle, probe, errors, |value| value["veto"] == false);
    cx.update_window(handle, |_, window, cx| window.press("escape", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["open"] == false && value["cancels"] == 2 && value["closes"] == 3
    });
    cx.run_until_parked();
    assert_eq!(
        probe.borrow()["closes"],
        3,
        "controlled acknowledgement does not echo openChange"
    );
    println!(
        "verified native AlertDialog header, footer, confirmation veto, close-button reset and modal backdrop"
    );
}
