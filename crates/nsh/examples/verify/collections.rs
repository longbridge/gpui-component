use crate::verification::{click, poll};
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, MouseButton, Role,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle};
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
) {
    cx.update(|cx| {
        view.read(cx).load(nsh::App::Js(Bundle::source(
            include_str!("../verify_collections.js").to_owned(),
        )))
    });
    poll(cx, handle, probe, errors, |value| {
        value["fixture"] == json!("collections")
    });
    click(cx, handle, "Beta");
    poll(cx, handle, probe, errors, |value| {
        value["listId"] == json!("b") && value["listConfirm"]["id"] == json!("b")
    });
    click(cx, handle, "Focus collection");
    poll(cx, handle, probe, errors, |value| {
        value["focus"] == json!("list")
    });
    cx.update_window(handle, |_, window, cx| window.press("down", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["listId"] == json!("a")
    });
    cx.update_window(handle, |_, window, cx| window.press("up", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["listId"] == json!("b")
    });
    let selections = probe.borrow()["selections"].clone();
    click(cx, handle, "Reorder items");
    poll(cx, handle, probe, errors, |value| {
        value["reversed"] == json!(true)
    });
    assert_eq!(probe.borrow()["listId"], json!("b"));
    assert_eq!(
        probe.borrow()["selections"],
        selections,
        "owner reorder must not emit a user selection"
    );
    click(cx, handle, "Focus collection");
    poll(cx, handle, probe, errors, |value| {
        value["focus"] == json!("list")
    });
    cx.update_window(handle, |_, window, cx| window.input("Alpha", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["query"] == json!("Alpha")
    });
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(
            window.find_all_by_role(Role::TextInput).is_empty(),
            "hiding search removes the input"
        );
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["listConfirm"]["id"] == json!("a")
    });
    click(cx, handle, "Show table");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("table")
    });
    click(cx, handle, "Focus collection");
    poll(cx, handle, probe, errors, |value| {
        value["focus"] == json!("table")
    });
    cx.update_window(handle, |_, window, cx| window.press("down", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["tableSelection"].get("row").is_some()
    });
    click(cx, handle, "Sort by Name");
    poll(cx, handle, probe, errors, |value| {
        value["sort"]["column"] == json!("name")
    });
    click(cx, handle, "Cell action");
    poll(cx, handle, probe, errors, |value| {
        value["cellCount"] == json!(1)
    });
    click(cx, handle, "Export table");
    poll(cx, handle, probe, errors, |value| {
        value["exported"]["rows"]
            .as_array()
            .is_some_and(|rows| rows.len() == 2)
    });
    click(cx, handle, "Show tree");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("tree")
    });
    click(cx, handle, "Folder");
    poll(cx, handle, probe, errors, |value| {
        value["expanded"]["id"] == json!("root") && value["expanded"]["expanded"] == json!(true)
    });
    click(cx, handle, "Before collection");
    wait_focus(cx, handle, "Before collection");
    cx.update_window(handle, |_, window, cx| window.press("tab", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| window.press("down", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["treeId"] == json!("leaf")
    });
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["treeConfirm"] == json!("leaf")
    });
    click(cx, handle, "Show command");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("command")
    });
    click(cx, handle, "Focus collection");
    poll(cx, handle, probe, errors, |value| {
        value["focus"] == json!("command")
    });
    cx.update_window(handle, |_, window, cx| window.input("bravo", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["commandQuery"] == json!("bravo") && value["commandSearchable"] == json!(false)
    });
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["commandConfirm"] == json!("b") && value["commandConfirms"] == json!(1)
    });
    click(cx, handle, "After collection");
    wait_focus(cx, handle, "After collection");
    cx.update_window(handle, |_, window, cx| window.press("shift-tab", cx))
        .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["commandConfirms"] == json!(2)
    });
    click(cx, handle, "Show combobox");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("combobox")
    });
    // The real Combobox input owns the dropdown click target.
    cx.update_window(handle, |_, window, cx| {
        let position = window
            .find_all_by_role(Role::ComboBox)
            .into_iter()
            .find(|element| element.visible())
            .expect("mounted combobox")
            .bounds()
            .center();
        window.pointer_move(position, None, cx);
        window.pointer_down(position, MouseButton::Left, cx);
        window.pointer_up(position, MouseButton::Left, cx);
    })
    .unwrap();
    cx.run_until_parked();
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["values"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
    });
    cx.update_window(handle, |_, window, cx| {
        window.press("down", cx);
        window.press("enter", cx);
    })
    .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["values"]
            .as_array()
            .is_some_and(|values| values.len() == 2)
    });
    cx.update_window(handle, |_, window, cx| window.press("escape", cx))
        .unwrap();
    poll(cx, handle, probe, errors, |value| {
        value["comboConfirm"]
            .as_array()
            .is_some_and(|values| !values.is_empty())
    });
    click(cx, handle, "Show pagination");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("pagination")
    });
    click(cx, handle, "2");
    poll(cx, handle, probe, errors, |value| value["page"] == json!(2));
    click(cx, handle, "Show table parts");
    poll(cx, handle, probe, errors, |value| {
        value["phase"] == json!("parts")
    });
    click(cx, handle, "Compound cell");
    poll(cx, handle, probe, errors, |value| {
        value["partCount"] == json!(1)
    });
    click(cx, handle, "Standalone list item");
    poll(cx, handle, probe, errors, |value| {
        value["partCount"] == json!(2)
    });
    println!(
        "verified retained list search/reorder, DataTable selection/sort/live JSX cell/export, tree expansion/keyboard, command keywords, multiple combobox, pagination, compound Table cells and standalone ListItem"
    );
}

// NodeRef commands cross the real JS worker. A click queues the anchor's focus
// command; start keyboard traversal only once GPUI has actually received it.
fn wait_focus(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, label: &str) {
    cx.update_window(handle,|_,window,cx| {
        eprintln!("collection Tab anchor {label}: focus before barrier={:?}, controls={:?}, command lists={:?}",window.focused(cx),["Before collection","After collection","Focus collection","Export table"].map(|name|(name,window.try_find_by_label(name).and_then(|element|element.focused()))),window.find_all_by_role(Role::ListBox).iter().map(|element|(element.bounds(),element.focused())).collect::<Vec<_>>());
    }).unwrap();
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
        assert!(
            Instant::now() < deadline,
            "collection traversal anchor {label} did not receive its requested focus"
        );
        std::thread::yield_now();
    }
}
