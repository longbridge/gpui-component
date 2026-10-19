use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, Menu, MenuItem, MouseButton, OwnedMenu,
    OwnedMenuItem,
    component::GlobalState,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle, ComponentContext};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
fn restore_menu(menu: OwnedMenu) -> Menu {
    Menu {
        name: menu.name,
        disabled: menu.disabled,
        items: menu
            .items
            .into_iter()
            .map(|item| match item {
                OwnedMenuItem::Separator => MenuItem::Separator,
                OwnedMenuItem::Submenu(menu) => MenuItem::Submenu(restore_menu(menu)),
                OwnedMenuItem::SystemMenu(menu) => MenuItem::os_submenu(menu.name, menu.menu_type),
                OwnedMenuItem::Action {
                    name,
                    action,
                    os_action,
                    checked,
                    disabled,
                } => MenuItem::Action {
                    name: name.into(),
                    action,
                    os_action,
                    checked,
                    disabled,
                },
            })
            .collect(),
    }
}
fn menu_names(cx: &mut HeadlessAppContext) -> Vec<String> {
    cx.update(|cx| {
        GlobalState::global(cx)
            .app_menus()
            .iter()
            .map(|menu| menu.name.to_string())
            .collect()
    })
}
fn wait_menus(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, names: &[&str]) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        if menu_names(cx) == names {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "application menus did not reach {names:?}"
        );
        std::thread::yield_now();
    }
}
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
        assert!(Instant::now() < deadline, "focus did not reach {label}");
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
    let original = cx.update(|cx| GlobalState::global(cx).app_menus().to_vec());
    let original_platform = cx.update(|cx| cx.get_menus());
    cx.update(|cx| {
        GlobalState::global_mut(cx).set_app_menus(vec![Menu::new("Host baseline").owned()]);
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_menus.js").to_owned(),
        )));
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("menus")
    });
    wait_menus(cx, handle, &["File", "Unavailable"]);
    click(cx, handle, "Unavailable");
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            !window
                .try_find_by_label("Must stay hidden")
                .is_some_and(|item| item.visible())
        );
    })
    .unwrap();
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("focus-start"));
    wait_focus(cx, handle, "Increment");
    cx.update_window(handle, |_, window, cx| window.press("tab", cx))
        .unwrap();
    wait_focus(cx, handle, "Open popup");
    cx.update_window(handle, |_, window, cx| window.press("tab", cx))
        .unwrap();
    wait_focus(cx, handle, "File");
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["chosen"] == json!(["a:run-a"])
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("mount-b"));
    poll(cx, handle, probe, errors, |value| value["b"] == json!(true));
    wait_menus(cx, handle, &["Edit"]);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        // Both mounted bars present the application-global menu. Exercise one
        // visible bar while retaining both owners for the lease test.
        let matches = window.find_all_by_label("Edit");
        assert_eq!(matches.len(), 2);
        let position = matches[0].bounds().center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    click(cx, handle, "Run B");
    poll(cx, handle, probe, errors, |value| {
        value["chosen"] == json!(["a:run-a", "b:run-b"])
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("unmount-b"));
    poll(cx, handle, probe, errors, |value| {
        value["b"] == json!(false)
    });
    wait_menus(cx, handle, &["File", "Unavailable"]);
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("focus-open"));
    wait_focus(cx, handle, "Open popup");
    cx.update_window(handle, |_, window, cx| window.press("space", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["opened"] == json!(true)
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window
                .try_find_by_label("Live custom menu row")
                .is_some_and(|item| item.visible()),
            "PopupMenu slot must be a live NSH child"
        );
    })
    .unwrap();
    cx.capture_screenshot(handle)
        .unwrap()
        .save("target/nsh-verify/menus-popup.png")
        .unwrap();
    cx.update_window(handle, |_, window, cx| {
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["chosen"] == json!(["a:run-a", "b:run-b", "popup:custom"])
            && value["closed"] == json!(1)
    });
    wait_focus(cx, handle, "Open popup");
    // The raw native menu has one stable entity; live item updates rebuild its
    // data without replacing focus or subscribing a second time.
    click(cx, handle, "Open popup");
    poll(cx, handle, probe, errors, |value| {
        value["opened"] == json!(true)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("update"));
    poll(cx, handle, probe, errors, |value| {
        value["version"] == json!(1)
    });
    click(cx, handle, "Popup action 1");
    poll(cx, handle, probe, errors, |value| {
        value["chosen"] == json!(["a:run-a", "b:run-b", "popup:custom", "popup:plain"])
            && value["closed"] == json!(2)
    });
    click(cx, handle, "Open popup");
    poll(cx, handle, probe, errors, |value| {
        value["opened"] == json!(true)
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("owner-close"));
    poll(cx, handle, probe, errors, |value| {
        value["opened"] == json!(false)
    });
    assert_eq!(probe.borrow()["closed"], json!(2));
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("unmount-a"));
    poll(cx, handle, probe, errors, |value| {
        value["a"] == json!(false)
    });
    wait_menus(cx, handle, &["Host baseline"]);
    // A host takeover while an NSH lease is alive must survive its later drop.
    cx.update(|cx| {
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_menus.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["a"] == json!(true) && value["chosen"] == json!([])
    });
    wait_menus(cx, handle, &["File", "Unavailable"]);
    cx.update(|cx| {
        GlobalState::global_mut(cx).set_app_menus(vec![Menu::new("Host takeover").owned()])
    });
    commands
        .borrow()
        .as_ref()
        .unwrap()
        .emit("command", json!("unmount-a"));
    poll(cx, handle, probe, errors, |value| {
        value["a"] == json!(false)
    });
    wait_menus(cx, handle, &["Host takeover"]);
    if cx.update(|cx| cx.get_menus().is_some()) {
        cx.update(|cx| {
            view.read(cx).load(nsh::App::Js(Bundle::source(
                include_str!("../verify_menus.js").to_owned(),
            )))
        });
        poll(cx, handle, probe, errors, |value| {
            value["a"] == json!(true) && value["chosen"] == json!([])
        });
        wait_menus(cx, handle, &["File", "Unavailable"]);
        cx.update(|cx| cx.set_menus(vec![Menu::new("Native host takeover")]));
        click(cx, handle, "File");
        click(cx, handle, "Run A");
        commands
            .borrow()
            .as_ref()
            .unwrap()
            .emit("command", json!("update"));
        poll(cx, handle, probe, errors, |value| {
            value["version"] == json!(1)
        });
        assert_eq!(
            probe.borrow()["chosen"],
            json!([]),
            "native host takeover must deactivate stale NSH actions"
        );
        commands
            .borrow()
            .as_ref()
            .unwrap()
            .emit("command", json!("unmount-a"));
        poll(cx, handle, probe, errors, |value| {
            value["a"] == json!(false)
        });
        wait_menus(cx, handle, &["Native host takeover"]);
        cx.update(|cx| {
            assert_eq!(
                cx.get_menus().unwrap()[0].name.as_ref(),
                "Native host takeover"
            )
        });
    }
    cx.update(|cx| {
        GlobalState::global_mut(cx).set_app_menus(original);
        if let Some(original) = original_platform {
            cx.set_menus(original.into_iter().map(restore_menu));
        }
    });
    println!(
        "verified AppMenuBar actions, scoped global menu leases, host takeover, PopupMenu keyboard/slots/rebuild and controlled dismissal"
    );
}
