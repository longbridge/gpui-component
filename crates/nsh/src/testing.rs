//! Drives controls in a GPUI test window and records what they send to JS.

use std::{cell::RefCell, rc::Rc};

pub(crate) use gpui_kit::test::TestWindowExt;
use gpui_kit::{
    App, AppContext as _, Bounds, Context, ElementId, Entity, FocusHandle, IntoElement,
    ParentElement as _, Pixels, Point, Render, Size, Styled as _, TestAppContext, Window,
    WindowBounds, WindowHandle, WindowOptions, base::Root, div, prelude::FluentBuilder as _, px,
    size,
};
use serde_json::Value;

use crate::catalog::{Mounted, Sink};

/// The events a control sent, in order.
#[derive(Default)]
struct Recorder {
    events: RefCell<Vec<(String, Value)>>,
}

impl Sink for Recorder {
    fn emit(&self, name: &str, data: Value) {
        self.events.borrow_mut().push((name.to_owned(), data));
    }

    fn redraw(&self) {}
}

/// The view holding the control under test.
pub(crate) struct Host {
    mounted: Mounted,
    /// The natural size, measured in the next prepaint when `None`.
    measured: Option<Size<Pixels>>,
    /// The box width it is measured at; `None` measures at max-content.
    measure_width: Option<Pixels>,
}

impl Render for Host {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let host = cx.entity().downgrade();
        let width = self.measure_width;
        div()
            .size_full()
            .p_4()
            .child(self.mounted.build(None, window, cx))
            // Measuring lays the element out as a root, which GPUI allows
            // only while drawing, as in NSH's surface.
            .when(self.measured.is_none(), |this| {
                this.child(gpui_kit::canvas(
                    move |_, window, cx| {
                        let _ = host.update(cx, |host, cx| {
                            host.measured = Some(host.mounted.measure(width, window, cx));
                        });
                    },
                    |_, _, _, _| {},
                ))
            })
    }
}

/// One control of [`crate::base::catalog`] in an 800×600 test window.
pub(crate) struct Fixture {
    window: WindowHandle<Root>,
    host: Entity<Host>,
    recorder: Rc<Recorder>,
    /// The id the control's props give it ([`crate::Props::id`]).
    pub(crate) id: ElementId,
}

impl Fixture {
    /// Opens a window showing control `name` with `props` (a JSON object).
    pub(crate) fn new(cx: &mut TestAppContext, name: &str, props: Value) -> Self {
        let control = crate::base::catalog()
            .find(name)
            .unwrap_or_else(|| panic!("no control `{name}`"));
        let recorder = Rc::new(Recorder::default());
        let mut mounted = Mounted::new(control, recorder.clone());
        if let Value::Object(props) = props {
            for (prop, value) in props {
                mounted.set_prop(&prop, Some(&value));
            }
        }
        let id = mounted.id();
        let (window, host) = cx.update(|cx| {
            gpui_kit::init(cx);
            let (window, host) = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(800.), px(600.)),
                    })),
                    ..Default::default()
                },
                cx,
                |_, cx| {
                    cx.new(|_| Host {
                        mounted,
                        measured: None,
                        measure_width: None,
                    })
                },
            )
            .expect("open a test window");
            // GPUI sends focus changes only to an active window.
            let _ = window.update(cx, |_, window, _| window.activate_window());
            (window.downcast::<Root>().expect("Base Root"), host)
        });
        let fixture = Self {
            window,
            host,
            recorder,
            id,
        };
        fixture.update(cx, |_, _| {});
        fixture
    }

    /// Draws a frame, runs `f` with the window, then lets what `f` started
    /// finish and draws again so its result shows.
    pub(crate) fn update<R>(
        &self,
        cx: &mut TestAppContext,
        f: impl FnOnce(&mut Window, &mut App) -> R,
    ) -> R {
        let result = cx
            .update_window(self.window.into(), |_, window, cx| {
                window.render_frame(cx);
                f(window, cx)
            })
            .expect("the test window is open");
        cx.run_until_parked();
        cx.update_window(self.window.into(), |_, window, cx| {
            window.refresh();
            window.render_frame(cx);
        })
        .expect("the test window is open");
        result
    }

    /// Sets prop `name` as JS would (`None` removes it), and draws.
    pub(crate) fn set_prop(&self, cx: &mut TestAppContext, name: &str, value: Option<Value>) {
        cx.update(|cx| {
            self.host.update(cx, |host, cx| {
                host.mounted.set_prop(name, value.as_ref());
                cx.notify();
            })
        });
        self.update(cx, |_, _| {});
    }

    /// The events sent since the last call, as `(name, data)`.
    pub(crate) fn take_events(&self) -> Vec<(String, Value)> {
        std::mem::take(&mut *self.recorder.events.borrow_mut())
    }

    /// The control's natural size with its current props, as NSH's layout
    /// gets it.
    pub(crate) fn measure(&self, cx: &mut TestAppContext) -> Size<Pixels> {
        self.measure_at(cx, None)
    }

    /// The control's natural size in a box `width` wide (`None`:
    /// max-content), as NSH's layout gets it once the box has a width.
    pub(crate) fn measure_at(
        &self,
        cx: &mut TestAppContext,
        width: Option<Pixels>,
    ) -> Size<Pixels> {
        cx.update(|cx| {
            self.host.update(cx, |host, cx| {
                host.measured = None;
                host.measure_width = width;
                cx.notify();
            })
        });
        self.update(cx, |_, _| {});
        cx.update(|cx| self.host.read(cx).measured)
            .expect("the control was measured")
    }

    /// The focus handle a [`crate::Control::focus`] control reports.
    pub(crate) fn focus_handle(&self, cx: &mut TestAppContext) -> Option<FocusHandle> {
        cx.update(|cx| self.host.read(cx).mounted.focus_handle())
    }
}
