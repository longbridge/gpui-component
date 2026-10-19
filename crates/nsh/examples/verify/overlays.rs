use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

use gpui_kit::{
    AnyWindowHandle, App, Entity, HeadlessAppContext, Window,
    component::{WindowExt, dialog::Confirm},
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};

use crate::verification::{click, poll};

fn settle(cx: &mut HeadlessAppContext, handle: AnyWindowHandle) {
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.render_frame(cx))
        .unwrap();
}

#[track_caller]
fn wait_window(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    predicate: impl Fn(&mut Window, &mut App) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        settle(cx, handle);
        if cx
            .update_window(handle, |_, window, cx| predicate(window, cx))
            .unwrap()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "window overlays did not reach their expected state"
        );
        std::thread::yield_now();
    }
}

fn wait_notifications(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        // Unmount closes asynchronously; its native exit timer can start after
        // the signal acknowledgement. Advance that clock until dismissal ends.
        cx.advance_clock(Duration::from_millis(50));
        settle(cx, handle);
        if cx
            .update_window(handle, |_, window, cx| {
                window.notifications(cx).len() == count
            })
            .unwrap()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "notifications did not finish dismissal"
        );
        std::thread::yield_now();
    }
}

fn command(commands: &Rc<RefCell<Option<ComponentContext>>>, name: &str) {
    commands
        .borrow()
        .as_ref()
        .expect("the NSH probe is mounted")
        .emit("command", json!(name));
}

fn image(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, name: &str) -> Vec<u8> {
    settle(cx, handle);
    let image = cx.capture_screenshot(handle).unwrap();
    image.save(format!("target/nsh-verify/{name}.png")).unwrap();
    image.into_raw()
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
            include_str!("../verify_overlays.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("overlays")
    });

    // Modal cancellation crosses back into the NSH signal exactly once.
    click(cx, handle, "Open dialog…");
    wait_window(cx, handle, |window, _| {
        window
            .try_find("dialog")
            .is_some_and(|dialog| dialog.visible())
    });
    image(cx, handle, "dialog");
    cx.update_window(handle, |_, window, cx| window.press("escape", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["dialog"] == json!(false) && value["cancels"] == json!(1)
    });
    assert_eq!(probe.borrow()["dialogCloses"], json!(1));
    cx.run_until_parked();
    cx.advance_clock(Duration::from_secs(1));
    wait_window(cx, handle, |window, _| {
        window.try_find("dialog").is_none()
            && window.find_by_label("Open dialog…").focused() == Some(true)
    });

    click(cx, handle, "Open dialog…");
    wait_window(cx, handle, |window, _| window.try_find("dialog").is_some());
    cx.update_window(handle, |_, window, cx| {
        window.within("dialog").click("ok", cx)
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["dialog"] == json!(false) && value["confirms"] == json!(1)
    });
    assert_eq!(probe.borrow()["dialogCloses"], json!(2));
    cx.advance_clock(Duration::from_secs(1));
    settle(cx, handle);

    click(cx, handle, "Open dialog…");
    wait_window(cx, handle, |window, _| window.try_find("dialog").is_some());
    command(commands, "close-dialog");
    poll(cx, handle, probe, errors, |value| {
        value["dialog"] == json!(false)
    });
    wait_window(cx, handle, |window, _| window.try_find("dialog").is_none());
    assert_eq!(
        probe.borrow()["dialogCloses"],
        json!(2),
        "owner sync must not emit user close events"
    );

    click(cx, handle, "Open dialog…");
    wait_window(cx, handle, |window, _| window.try_find("dialog").is_some());
    let before = image(cx, handle, "dialog-before-update");
    command(commands, "change-title");
    poll(cx, handle, probe, errors, |value| {
        value["title"] == json!("Updated dialog")
    });
    assert!(
        before != image(cx, handle, "dialog-after-update"),
        "open dialog props must rebuild the visible content"
    );

    let closed = Rc::new(Cell::new(0));
    let observed = closed.clone();
    let (host, focus, lower_focus, _closed) = cx
        .update_window(handle, |_, window, cx| {
            let lower_focus = window.focused(cx).expect("lower native dialog has focus");
            let host =
                window.open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Host dialog"));
            let closed = host.on_closed(window, cx, move |_, _| observed.set(observed.get() + 1));
            (host, window.focused(cx).unwrap(), lower_focus, closed)
        })
        .unwrap();
    wait_window(cx, handle, |window, _| {
        window.is_action_available_in(&Confirm { secondary: false }, &lower_focus)
    });
    command(commands, "unmount-overlays");
    poll(cx, handle, probe, errors, |value| {
        value["mounted"] == json!(false)
    });
    wait_window(cx, handle, |window, cx| {
        // Confirm is installed on every native dialog dispatch node. An
        // absent focus node resolves to the root, which has no Confirm handler.
        host.is_open(window, cx)
            && !window.is_action_available_in(&Confirm { secondary: false }, &lower_focus)
    });
    cx.update_window(handle, |_, window, cx| {
        assert_eq!(
            window.focused(cx).as_ref(),
            Some(&focus),
            "removing the lower NSH dialog must not steal host focus"
        );
        host.close(window, cx);
        host.close(window, cx);
    })
    .unwrap();
    settle(cx, handle);
    assert_eq!(
        closed.get(),
        1,
        "a scoped close must notify once even when repeated"
    );
    cx.update_window(handle, |_, window, _| {
        assert_eq!(window.find_by_label("Open dialog…").focused(), Some(true))
    })
    .unwrap();

    command(commands, "mount-overlays");
    poll(cx, handle, probe, errors, |value| {
        value["mounted"] == json!(true)
    });
    click(cx, handle, "Open sheet…");
    wait_window(cx, handle, |window, _| {
        window
            .try_find("sheet-content")
            .is_some_and(|sheet| sheet.visible())
    });
    image(cx, handle, "sheet");
    command(commands, "replace-sheet");
    poll(cx, handle, probe, errors, |value| {
        value["sheet"] == json!(false) && value["replacement"] == json!(true)
    });
    assert_eq!(probe.borrow()["sheetCloses"], json!(1));
    command(commands, "unmount-first-sheet");
    poll(cx, handle, probe, errors, |value| {
        value["firstSheetMounted"] == json!(false)
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert_eq!(
            f32::from(window.find("sheet-content").bounds().size.width),
            260.
        );
    })
    .unwrap();

    let (host, focus) = cx
        .update_window(handle, |_, window, cx| {
            let host =
                window.open_dialog_with_handle(cx, |dialog, _, _| dialog.title("Host above sheet"));
            (host, window.focused(cx).unwrap())
        })
        .unwrap();
    command(commands, "unmount-overlays");
    poll(cx, handle, probe, errors, |value| {
        value["mounted"] == json!(false)
    });
    wait_window(cx, handle, |window, cx| {
        host.is_open(window, cx) && window.try_find("sheet-content").is_none()
    });
    cx.update_window(handle, |_, window, cx| {
        assert_eq!(window.focused(cx).as_ref(), Some(&focus));
        host.close(window, cx);
    })
    .unwrap();
    settle(cx, handle);
    cx.update_window(handle, |_, window, _| {
        assert_eq!(window.find_by_label("Open sheet…").focused(), Some(true))
    })
    .unwrap();

    command(commands, "mount-overlays");
    poll(cx, handle, probe, errors, |value| {
        value["mounted"] == json!(true)
    });
    click(cx, handle, "Show notifications");
    poll(cx, handle, probe, errors, |value| {
        value["note1"] == json!(true) && value["note2"] == json!(true)
    });
    wait_window(cx, handle, |window, cx| window.notifications(cx).len() == 2);
    cx.advance_clock(Duration::from_secs(1));
    image(cx, handle, "notifications");
    command(commands, "unmount-first-note");
    poll(cx, handle, probe, errors, |value| {
        value["firstNoteMounted"] == json!(false)
    });
    cx.run_until_parked();
    cx.advance_clock(Duration::from_secs(1));
    wait_notifications(cx, handle, 1);
    assert_eq!(
        probe.borrow()["noteCloses"],
        json!(0),
        "unmount must not emit a stale close event"
    );
    cx.update_window(handle, |_, window, cx| {
        let position = window
            .within("notification")
            .find("close")
            .bounds()
            .center();
        window.pointer_move(position, None, cx);
        window.render_frame(cx);
        window.within("notification").click("close", cx)
    })
    .unwrap();
    cx.run_until_parked();
    cx.advance_clock(Duration::from_secs(1));
    poll(cx, handle, probe, errors, |value| {
        value["note2"] == json!(false)
    });
    assert_eq!(probe.borrow()["noteCloses"], json!(1));
    wait_notifications(cx, handle, 0);
    println!(
        "verified window overlays, controlled close, scoped unmount, sheet replacement, focus chains and independent notifications"
    );
}
