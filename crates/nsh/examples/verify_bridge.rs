//! Runs real NSH children, named slots, retained editing and subtree unmounting.
#[cfg(not(feature = "test-support"))]
fn main() {
    panic!("run verify_bridge with --features test-support");
}
#[cfg(feature = "test-support")]
fn main() {
    use gpui_kit::{
        test::{TestEventExt, TestQueryExt, TestWindowExt},
        *,
    };
    use gpui_kit_nsh::nsh::{AppOptions, AppView, Bundle, ComponentContext, NativeComponent};
    use serde_json::{Value, json};
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::Arc,
        time::{Duration, Instant},
    };
    struct Host(ComponentContext);
    impl NativeComponent for Host {
        fn render(&mut self, _: &mut Window, _: &mut App) -> Option<AnyElement> {
            self.0.set_size(440., 260.);
            Some(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .children(gpui_kit_nsh::bridge::slot(&self.0, "header"))
                    .children(gpui_kit_nsh::bridge::children(&self.0))
                    .into_any_element(),
            )
        }
        fn set_prop(&mut self, _: &str, _: Option<&Value>, _: &mut Window, _: &mut App) {}
    }
    struct Probe(Rc<RefCell<Value>>);
    impl NativeComponent for Probe {
        fn set_prop(&mut self, name: &str, value: Option<&Value>, _: &mut Window, _: &mut App) {
            if name == "snapshot" {
                *self.0.borrow_mut() = value.cloned().unwrap_or(Value::Null);
            }
        }
    }
    let probe = Rc::new(RefCell::new(Value::Null));
    let probe_context = Rc::new(RefCell::new(None::<ComponentContext>));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let components = gpui_kit_nsh::components();
    components.register("verify.host", Rc::new(|context| Box::new(Host(context))));
    let snapshot = probe.clone();
    let context_sink = probe_context.clone();
    components.register(
        "verify.probe",
        Rc::new(move |context| {
            *context_sink.borrow_mut() = Some(context);
            Box::new(Probe(snapshot.clone()))
        }),
    );
    let mut cx = HeadlessAppContext::with_platform(
        gpui_kit::platform::current_platform(true).text_system(),
        Arc::new(gpui_kit::assets::Assets),
        gpui_kit::platform::current_headless_renderer,
    );
    gpui_kit_nsh::nsh::set_ui_sets(vec![gpui_kit_nsh::ui_set()]);
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.set_reduce_motion(true));
    let bundle_path = std::env::args().nth(1);
    let verify_overlay = bundle_path.is_some();
    let bundle = match bundle_path {
        Some(path) => {
            // SAFETY: this verifier accepts a developer-built local fixture.
            unsafe { Bundle::read_trusted(std::path::Path::new(&path)) }.unwrap()
        }
        None => Bundle::source(include_str!("verify_bridge.js").to_owned()),
    };
    let error_log = errors.clone();
    let (handle, view) = cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                show: false,
                window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                    Default::default(),
                    size(px(640.), px(480.)),
                ))),
                ..Default::default()
            },
            cx,
            |window, cx| {
                cx.new(|cx| {
                    let view = AppView::new(
                        AppOptions {
                            components,
                            ..Default::default()
                        },
                        window,
                        cx,
                    );
                    view.on_error(move |e| error_log.borrow_mut().push(e.to_owned()));
                    view.load(gpui_kit_nsh::nsh::App::Js(bundle));
                    view
                })
            },
        )
        .unwrap()
    });
    let handle: AnyWindowHandle = handle.into();
    cx.update_window(handle, |_, window, cx| {
        window.activate_window();
        window.focus(&view.focus_handle(cx), cx);
    })
    .unwrap();
    let poll = |cx: &mut HeadlessAppContext, expected: Value| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            cx.update_window(handle, |_, window, cx| window.render_frame(cx))
                .unwrap();
            assert!(
                errors.borrow().is_empty(),
                "NSH errors: {:?}",
                errors.borrow()
            );
            if *probe.borrow() == expected {
                break;
            }
            if Instant::now() >= deadline {
                std::fs::create_dir_all("target/nsh-verify").unwrap();
                cx.capture_screenshot(handle)
                    .unwrap()
                    .save("target/nsh-verify/bridge-failure.png")
                    .unwrap();
                panic!("snapshot: {}; expected: {}", probe.borrow(), expected);
            }
            std::thread::yield_now();
        }
    };
    let click = |cx: &mut HeadlessAppContext, label: &str| {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let target = window.find_by_label(label);
            assert!(target.visible(), "{label} not visible");
            let point = target.bounds().center();
            window.pointer_move(point, None, cx);
            window.pointer_down(point, MouseButton::Left, cx);
            window.pointer_up(point, MouseButton::Left, cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
    };
    poll(&mut cx, json!({"count":0,"name":"","show":true}));
    click(&mut cx, "Slot increment");
    poll(&mut cx, json!({"count":1,"name":"","show":true}));
    click(&mut cx, "Child input");
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        let target = window.find_by_label("Child input");
        eprintln!(
            "NSH bridge Input bounds={:?} focused={:?}",
            target.bounds(),
            target.focused()
        );
    })
    .unwrap();
    cx.update_window(handle, |_, window, cx| window.input("Ada", cx))
        .unwrap();
    poll(&mut cx, json!({"count":1,"name":"Ada","show":true}));
    cx.update_window(handle, |_, window, cx| window.press("home", cx))
        .unwrap();
    probe_context
        .borrow()
        .as_ref()
        .unwrap()
        .emit("advance", Value::Null);
    poll(&mut cx, json!({"count":2,"name":"Ada","show":true}));
    cx.update_window(handle, |_, window, cx| {
        window.input("X", cx);
    })
    .unwrap();
    poll(&mut cx, json!({"count":2,"name":"XAda","show":true}));
    click(&mut cx, "Child increment");
    poll(&mut cx, json!({"count":3,"name":"XAda","show":true}));
    cx.update_window(handle, |_, window, cx| {
        let child = window.find_by_label("Child increment").bounds();
        let point = point(child.origin.x + px(40.), child.origin.y - px(29.));
        window.pointer_move(point, None, cx);
        window.pointer_down(point, MouseButton::Left, cx);
        window.pointer_up(point, MouseButton::Left, cx);
    })
    .unwrap();
    poll(&mut cx, json!({"count":4,"name":"XAda","show":true}));
    click(&mut cx, "Unmount children");
    poll(&mut cx, json!({"count":4,"name":"XAda","show":false}));
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        assert!(window.try_find_by_label("Child input").is_none());
        assert!(window.try_find_by_label("Child increment").is_none());
        assert!(window.find_by_label("Slot increment").visible());
    })
    .unwrap();
    if verify_overlay {
        // Enter the opener through actual keyboard navigation. Closing the
        // dialog must restore this original focus, including after NSH presses.
        cx.update_window(handle, |_, window, cx| {
            window.focus(&view.focus_handle(cx), cx);
        })
        .unwrap();
        let mut opener_focused = false;
        for _ in 0..32 {
            cx.update_window(handle, |_, window, cx| window.press("tab", cx))
                .unwrap();
            cx.run_until_parked();
            opener_focused = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    window.find_by_label("Open bridge dialog").focused() == Some(true)
                })
                .unwrap();
            if opener_focused {
                break;
            }
        }
        assert!(opener_focused, "keyboard did not reach dialog opener");
        cx.update_window(handle, |_, window, cx| window.press("space", cx))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            let ready = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    window
                        .try_find_by_label("Overlay geometry anchor")
                        .is_some_and(|element| {
                            element.visible() && element.bounds().size.width > px(100.)
                        })
                })
                .unwrap();
            assert!(
                errors.borrow().is_empty(),
                "NSH overlay errors: {:?}",
                errors.borrow()
            );
            if ready {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "default Slot did not negotiate Component dialog width"
            );
            std::thread::yield_now();
        }
        cx.update_window(handle, |_, window, cx| {
            let anchor = window.find_by_label("Overlay geometry anchor").bounds();
            let point = point(anchor.origin.x + px(40.), anchor.bottom() + px(21.));
            window.pointer_move(point, None, cx);
            window.pointer_down(point, MouseButton::Left, cx);
            window.pointer_up(point, MouseButton::Left, cx);
        })
        .unwrap();
        poll(&mut cx, json!({"count":5,"name":"XAda","show":false}));
        click(&mut cx, "Overlay native increment");
        poll(&mut cx, json!({"count":6,"name":"XAda","show":false}));
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.find_by_label("Overlay native increment").visible());
        })
        .unwrap();
        std::fs::create_dir_all("target/nsh-verify").unwrap();
        cx.capture_screenshot(handle)
            .unwrap()
            .save("target/nsh-verify/bridge-dialog.png")
            .unwrap();
        cx.update_window(handle, |_, window, cx| window.press("escape", cx))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            let closed = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    window
                        .try_find_by_label("Overlay native increment")
                        .is_none_or(|element| !element.visible())
                })
                .unwrap();
            if closed {
                break;
            }
            assert!(Instant::now() < deadline, "dialog did not finish closing");
        }
        cx.update_window(handle, |_, window, _| {
            assert_eq!(
                window.find_by_label("Open bridge dialog").focused(),
                Some(true),
                "closing the dialog restores its opener"
            );
        })
        .unwrap();
        click(&mut cx, "Veto input");
        cx.update_window(handle, |_, window, _| {
            eprintln!(
                "NSH bridge veto focused={:?}",
                window.find_by_label("Veto input").focused()
            );
        })
        .unwrap();
        cx.update_window(handle, |_, window, cx| window.input("Z", cx))
            .unwrap();
        poll(&mut cx, json!({"count":7,"name":"XAda","show":false}));
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find_by_label("Veto input").value(), Some("Locked"));
        })
        .unwrap();
        click(&mut cx, "Veto checkbox");
        poll(&mut cx, json!({"count":8,"name":"XAda","show":false}));
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find_by_label("Veto checkbox").checked(), Some(true));
        })
        .unwrap();
    }
    view.update(&mut cx, |view, _| view.stop());
    println!(
        "NSH bridge verified: real nested buttons, named slot events, reactive controlled input, subtree unmount"
    );
}
