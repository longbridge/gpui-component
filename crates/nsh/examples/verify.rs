//! Checks the NSH event loop and actual GPUI controls together in an offscreen window.
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/alert.rs"]
mod alert;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/collections.rs"]
mod collections;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/compare.rs"]
mod compare;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/component_focus.rs"]
mod component_focus;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/content.rs"]
mod content;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/datetime.rs"]
mod datetime;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/dialog_parts.rs"]
mod dialog_parts;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/display.rs"]
mod display;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/dock.rs"]
mod dock;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/extra_controls.rs"]
mod extra_controls;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/inputs.rs"]
mod inputs;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/layout.rs"]
mod layout;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/menus.rs"]
mod menus;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/overlay_stack.rs"]
mod overlay_stack;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/overlays.rs"]
mod overlays;
#[cfg(any(target_os = "macos", target_os = "linux"))]
#[path = "verify/surfaces.rs"]
mod surfaces;
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() {
    verification::run();
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {
    panic!("GPUI offscreen verification requires the macOS or Linux headless renderer");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod verification {
    use std::{
        cell::RefCell,
        rc::Rc,
        sync::Arc,
        time::{Duration, Instant},
    };

    use gpui_kit::{
        AnyWindowHandle, App, AppContext, Bounds, Focusable, HeadlessAppContext, MouseButton, Role,
        Window, WindowBounds, WindowOptions, px, size,
        test::{TestEventExt, TestQueryExt, TestWindowExt},
    };
    use gpui_kit_nsh::nsh::{self, AppOptions, AppView, Bundle, ComponentContext, NativeComponent};
    use serde_json::{Value, json};

    struct Probe(Rc<RefCell<Value>>);

    impl NativeComponent for Probe {
        fn set_prop(&mut self, name: &str, value: Option<&Value>, _: &mut Window, _: &mut App) {
            if name == "snapshot" {
                *self.0.borrow_mut() = value.cloned().unwrap_or(Value::Null);
            }
        }
    }

    pub(super) fn poll(
        cx: &mut HeadlessAppContext,
        handle: AnyWindowHandle,
        probe: &Rc<RefCell<Value>>,
        errors: &Rc<RefCell<Vec<String>>>,
        predicate: impl Fn(&Value) -> bool,
    ) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            cx.run_until_parked();
            let rendered = cx
                .update_window(handle, |_, window, cx| {
                    window.render_frame(cx);
                    window
                        .find_all_by_label("Increment")
                        .iter()
                        .any(|element| element.visible() && element.bounds().size.width > px(0.))
                })
                .unwrap();
            assert!(
                errors.borrow().is_empty(),
                "NSH errors: {:?}",
                errors.borrow()
            );
            assert!(
                probe.borrow()["error"].is_null(),
                "adapter error: {}",
                probe.borrow()
            );
            if rendered && predicate(&probe.borrow()) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "NSH did not reach the expected state: {}",
                probe.borrow()
            );
            // The NSH engine is a real worker thread, outside TestDispatcher.
            std::thread::yield_now();
        }
    }

    pub(super) fn click(cx: &mut HeadlessAppContext, handle: AnyWindowHandle, label: &str) {
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            let position = window.find_by_label(label).bounds().center();
            window.pointer_move(position, None, cx);
            window.pointer_down(position, MouseButton::Left, cx);
            window.pointer_up(position, MouseButton::Left, cx);
        })
        .unwrap();
        cx.run_until_parked();
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
    }

    pub(super) fn run() {
        let probe = Rc::new(RefCell::new(Value::Null));
        let commands = Rc::new(RefCell::new(None::<ComponentContext>));
        let errors = Rc::new(RefCell::new(Vec::new()));
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            Arc::new(gpui_kit::assets::Assets),
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        nsh::set_ui_sets(vec![gpui_kit_nsh::ui_set()]);
        // Verify settled surfaces deterministically; animation timing has its own
        // component tests and is not driven by NSH's engine clock.
        cx.update(|cx| cx.set_reduce_motion(true));
        let components = gpui_kit_nsh::components();
        let snapshot = probe.clone();
        let command_sink = commands.clone();
        components.register(
            "verify.probe",
            Rc::new(move |context| {
                *command_sink.borrow_mut() = Some(context);
                Box::new(Probe(snapshot.clone()))
            }),
        );
        let error_log = errors.clone();
        let (handle, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Default::default(),
                        size: size(px(640.), px(700.)),
                    })),
                    show: false,
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
                        view.on_error(move |error| error_log.borrow_mut().push(error.to_owned()));
                        view.load(nsh::App::Js(Bundle::source(
                            include_str!("verify.js").to_owned(),
                        )));
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
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["count"] == json!(0)
        });
        std::fs::create_dir_all("target/nsh-verify").unwrap();
        if let Ok(family) = std::env::var("NSH_VERIFY_FAMILY") {
            match family.as_str() {
                "overlays" => {
                    super::overlays::run(&mut cx, handle, &view, &probe, &errors, &commands)
                }
                "inputs" => super::inputs::run(&mut cx, handle, &view, &probe, &errors),
                "datetime" => {
                    super::datetime::run(&mut cx, handle, &view, &probe, &errors, &commands)
                }
                "dock" => super::dock::run(&mut cx, handle, &view, &probe, &errors, &commands),
                "content" => super::content::run(&mut cx, handle, &view, &probe, &errors),
                "layout" => super::layout::run(&mut cx, handle, &view, &probe, &errors),
                "display" => {
                    super::display::run(&mut cx, handle, &view, &probe, &errors, &commands)
                }
                "collections" => super::collections::run(&mut cx, handle, &view, &probe, &errors),
                "compare" => super::compare::run(&mut cx, handle, &view, &errors),
                "component_focus" => {
                    super::component_focus::run(&mut cx, handle, &view, &probe, &errors, &commands)
                }
                "extra_controls" => {
                    super::extra_controls::run(&mut cx, handle, &view, &probe, &errors, &commands)
                }
                "surfaces" => super::surfaces::run(&mut cx, handle, &view, &probe, &errors),
                "menus" => super::menus::run(&mut cx, handle, &view, &probe, &errors, &commands),
                "alert" => super::alert::run(&mut cx, handle, &view, &probe, &errors, &commands),
                "overlay_stack" => super::overlay_stack::run(&mut cx, handle, &view),
                "dialog_parts" => {
                    super::dialog_parts::run(&mut cx, handle, &view, &probe, &errors, &commands)
                }
                _ => panic!("unknown verification family: {family}"),
            }
            cx.update(|cx| view.read(cx).stop());
            return;
        }
        cx.capture_screenshot(handle)
            .unwrap()
            .save("target/nsh-verify/initial.png")
            .unwrap();
        click(&mut cx, handle, "Increment");
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["count"] == json!(1)
        });

        click(&mut cx, handle, "Name");
        cx.update_window(handle, |_, window, cx| window.input("Ada", cx))
            .unwrap();
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["name"] == json!("Ada")
        });
        cx.update_window(handle, |_, window, cx| {
            window.press("home", cx);
            window.input("X", cx);
        })
        .unwrap();
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["name"] == json!("XAda")
        });
        cx.update_window(handle, |_, window, cx| window.input("Y", cx))
            .unwrap();
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["name"] == json!("XYAda")
        });

        click(&mut cx, handle, "Shared flag");
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["checked"] == json!(false)
        });
        cx.update_window(handle, |_, window, _| {
            assert_eq!(window.find_by_label("Mirrored flag").checked(), Some(false));
        })
        .unwrap();

        click(&mut cx, handle, "Fruit");
        cx.update_window(handle, |_, window, cx| window.render_frame(cx))
            .unwrap();
        cx.update_window(handle, |_, window, _| {
            let trigger = window.find_by_label("Fruit");
            assert_eq!(trigger.expanded(), Some(true));
            assert!(
                window
                    .find_all_by_role(Role::TextInput)
                    .iter()
                    .any(|element| {
                        element.visible() && element.bounds().origin.y >= trigger.bounds().bottom()
                    }),
                "the dropdown search input must be visible outside the native trigger box"
            );
        })
        .unwrap();
        cx.capture_screenshot(handle)
            .unwrap()
            .save("target/nsh-verify/select-open.png")
            .unwrap();
        cx.update_window(handle, |_, window, cx| {
            window.press("down", cx);
            window.press("enter", cx);
        })
        .unwrap();
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["selection"] == json!("orange")
        });
        click(&mut cx, handle, "Reorder options");
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["itemOrder"] == json!(["banana", "orange", "apple"])
        });
        cx.update_window(handle, |_, window, _| {
            assert_eq!(window.find_by_label("Fruit").value(), Some("Orange"));
        })
        .unwrap();
        click(&mut cx, handle, "Toggle search");
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["searchable"] == json!(false)
        });
        click(&mut cx, handle, "Fruit");
        cx.update_window(handle, |_, window, cx| {
            let trigger = window.find_by_label("Fruit");
            assert_eq!(trigger.expanded(), Some(true));
            assert_eq!(trigger.value(), Some("Orange"));
            assert!(
                !window
                    .find_all_by_role(Role::TextInput)
                    .iter()
                    .any(|element| {
                        element.visible() && element.bounds().origin.y >= trigger.bounds().bottom()
                    })
            );
            window.press("escape", cx);
        })
        .unwrap();

        cx.update_window(handle, |_, window, cx| {
            let bounds = window.find_by_label("Amount").bounds();
            let position =
                bounds.origin + gpui_kit::point(bounds.size.width * 0.75, bounds.size.height * 0.5);
            window.pointer_move(position, None, cx);
            window.pointer_down(position, MouseButton::Left, cx);
            window.pointer_up(position, MouseButton::Left, cx);
        })
        .unwrap();
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["releases"] == json!(1)
        });
        assert!(
            probe.borrow()["amount"]
                .as_f64()
                .is_some_and(|value| value > 60. && value < 90.)
        );

        let edits = probe.borrow()["edits"].clone();
        let selections = probe.borrow()["selections"].clone();
        click(&mut cx, handle, "Reset controls");
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["name"] == json!("Reset")
                && value["selection"] == json!("apple")
                && value["amount"] == json!(25)
        });
        assert_eq!(
            probe.borrow()["edits"],
            edits,
            "owner updates must not emit user input events"
        );
        assert_eq!(
            probe.borrow()["selections"],
            selections,
            "owner updates must not emit selection events"
        );
        click(&mut cx, handle, "Unmount controls");
        poll(&mut cx, handle, &probe, &errors, |value| {
            value["visible"] == json!(false)
        });
        cx.update_window(handle, |_, window, _| {
            assert!(window.try_find_by_label("Name").is_none());
            assert!(window.try_find_by_label("Fruit").is_none());
        })
        .unwrap();
        super::overlays::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::alert::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::overlay_stack::run(&mut cx, handle, &view);
        super::dialog_parts::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::inputs::run(&mut cx, handle, &view, &probe, &errors);
        super::datetime::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::dock::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::content::run(&mut cx, handle, &view, &probe, &errors);
        super::layout::run(&mut cx, handle, &view, &probe, &errors);
        super::display::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::component_focus::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::collections::run(&mut cx, handle, &view, &probe, &errors);
        super::extra_controls::run(&mut cx, handle, &view, &probe, &errors, &commands);
        super::surfaces::run(&mut cx, handle, &view, &probe, &errors);
        super::menus::run(&mut cx, handle, &view, &probe, &errors, &commands);
        cx.update(|cx| view.read(cx).stop());
        println!(
            "verified NSH signal round trips, Input echo selection, shared checked state, Select identity, Slider events, owner sync and unmount"
        );
    }
}
