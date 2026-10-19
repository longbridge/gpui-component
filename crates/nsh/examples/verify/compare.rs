//! Run the shipped Component TSX comparison app through its public controls.
use crate::verification::click;
use gpui_kit::{
    AnyWindowHandle, Entity, HeadlessAppContext, Window,
    test::{TestQueryExt, TestWindowExt},
};
use gpui_kit_nsh::nsh::{self, AppView, Bundle};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

pub(super) fn run(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    view: &Entity<AppView>,
    errors: &Rc<RefCell<Vec<String>>>,
) {
    let bundle = std::fs::read_to_string("target/nsh-component-compare/bundle.js")
        .expect("build examples/compare with the Component CLI first");
    cx.update(|cx| view.read(cx).load(nsh::App::Js(Bundle::source(bundle))));
    wait(cx, handle, errors, |window| {
        window
            .try_find_by_label("Task name")
            .is_some_and(|input| input.visible())
            && window
                .try_find_by_label("Create task")
                .is_some_and(|button| button.disabled() == Some(true))
    });
    cx.update_window(handle, |_, window, _| {
        assert_eq!(window.find_by_label("Task name").value(), Some(""));
        assert_eq!(window.find_by_label("Priority").checked(), Some(false));
    })
    .unwrap();
    screenshot(cx, handle, "initial");
    click(cx, handle, "Task name");
    input(cx, handle, "Review NSH controls");
    wait(cx, handle, errors, |window| {
        window.find_by_label("Task name").value() == Some("Review NSH controls")
            && window.find_by_label("Create task").disabled() != Some(true)
    });
    click(cx, handle, "Priority");
    wait(cx, handle, errors, |window| {
        window.find_by_label("Priority").checked() == Some(true)
    });
    screenshot(cx, handle, "before");
    click(cx, handle, "Create task");
    wait_reset(cx, handle, errors);
    screenshot(cx, handle, "after");
    input(cx, handle, "Check keyboard submission");
    wait(cx, handle, errors, |window| {
        window.find_by_label("Task name").value() == Some("Check keyboard submission")
            && window.find_by_label("Create task").disabled() != Some(true)
    });
    cx.update_window(handle, |_, window, cx| window.press("enter", cx))
        .unwrap();
    wait_reset(cx, handle, errors);
    screenshot(cx, handle, "keyboard");
    let limit = "x".repeat(80);
    input(cx, handle, &limit);
    wait(cx, handle, errors, |window| {
        window.find_by_label("Task name").value() == Some(limit.as_str())
    });
    input(cx, handle, "!");
    wait(cx, handle, errors, |window| {
        window.find_by_label("Task name").value() == Some(limit.as_str())
    });
    screenshot(cx, handle, "limit");
    println!(
        "verified original Component compare TSX bundle: pointer creation, Input/Checkbox reset, disabled button, Input focus, Enter submit and 80-character controlled veto; summaries recorded in screenshots"
    );
}
fn wait_reset(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    errors: &Rc<RefCell<Vec<String>>>,
) {
    wait(cx, handle, errors, |window| {
        window.find_by_label("Task name").value() == Some("")
            && window.find_by_label("Task name").focused() == Some(true)
            && window.find_by_label("Priority").checked() == Some(false)
            && window.find_by_label("Create task").disabled() == Some(true)
    });
}
fn input(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, text: &str) {
    cx.update_window(handle, |_, window, cx| window.input(text, cx))
        .unwrap();
}
fn wait(
    cx: &mut HeadlessAppContext,
    handle: AnyWindowHandle,
    errors: &Rc<RefCell<Vec<String>>>,
    mut predicate: impl FnMut(&Window) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        cx.run_until_parked();
        assert!(
            errors.borrow().is_empty(),
            "NSH errors: {:?}",
            errors.borrow()
        );
        let ready = cx
            .update_window(handle, |_, window, cx| {
                window.render_frame(cx);
                predicate(window)
            })
            .unwrap();
        if ready {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Component compare controls did not reach their expected state"
        );
        std::thread::yield_now();
    }
}
fn screenshot(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, name: &str) {
    std::fs::create_dir_all("target/nsh-component-compare/screenshots").unwrap();
    cx.capture_screenshot(handle)
        .unwrap()
        .save(format!(
            "target/nsh-component-compare/screenshots/{name}.png"
        ))
        .unwrap();
}
