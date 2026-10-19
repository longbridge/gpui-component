//! Thin access to the real NSH engine and public GPUI test interfaces.
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Entity, Focusable, HeadlessAppContext,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, Styled, Window,
    WindowBounds, WindowOptions, div, px, size,
    test::{TestEventExt, TestQueryExt, TestWindowExt},
};
use nsh_platform_gpui::{
    self as nsh, AppOptions, AppView, Bundle, ComponentContext, NativeComponent,
};
use serde_json::Value;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

struct Probe(Rc<RefCell<Value>>);
impl NativeComponent for Probe {
    fn set_prop(&mut self, name: &str, value: Option<&Value>, _: &mut Window, _: &mut App) {
        if name == "snapshot" {
            *self.0.borrow_mut() = value.cloned().unwrap_or(Value::Null);
        }
    }
}

// Fixture-only native container exercises the public live slot/children seam.
struct CompositionHost {
    context: ComponentContext,
}
impl NativeComponent for CompositionHost {
    fn set_prop(&mut self, _: &str, _: Option<&Value>, _: &mut Window, _: &mut App) {}
    fn render(&mut self, _: &mut Window, _: &mut App) -> Option<gpui_kit::AnyElement> {
        Some(
            div()
                .id("verify-host")
                .flex()
                .flex_col()
                .size_full()
                .children(self.context.slot("header"))
                .children(self.context.children())
                .into_any_element(),
        )
    }
}

pub(crate) struct Harness {
    cx: HeadlessAppContext,
    handle: AnyWindowHandle,
    view: Entity<AppView>,
    probe: Rc<RefCell<Value>>,
    errors: Rc<RefCell<Vec<String>>>,
}
impl Harness {
    pub(crate) fn new(components: nsh::Components, ui_sets: Vec<nsh_core::ui::Registry>) -> Self {
        Self::with_assets(components, ui_sets, Arc::new(()))
    }
    pub(crate) fn with_assets(
        components: nsh::Components,
        ui_sets: Vec<nsh_core::ui::Registry>,
        assets: Arc<dyn gpui_kit::AssetSource>,
    ) -> Self {
        let probe = Rc::new(RefCell::new(Value::Null));
        let errors = Rc::new(RefCell::new(Vec::new()));
        let mut cx = HeadlessAppContext::with_platform(
            gpui_kit::platform::current_platform(true).text_system(),
            assets,
            gpui_kit::platform::current_headless_renderer,
        );
        cx.update(gpui_kit::init);
        cx.update(|cx| cx.set_reduce_motion(true));
        nsh::set_ui_sets(ui_sets);
        let snapshot = probe.clone();
        components.register(
            "verify.probe",
            Rc::new(move |_| Box::new(Probe(snapshot.clone()))),
        );
        components.register(
            "verify.host",
            Rc::new(|context| Box::new(CompositionHost { context })),
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
                        view
                    })
                },
            )
            .unwrap()
        });
        cx.update_window(handle, |_, window, cx| {
            window.activate_window();
            window.focus(&view.focus_handle(cx), cx);
        })
        .unwrap();
        std::fs::create_dir_all("target/nsh-base-verify").unwrap();
        Self {
            cx,
            handle,
            view,
            probe,
            errors,
        }
    }
    pub(crate) fn load(&mut self, source: &str) {
        *self.probe.borrow_mut() = Value::Null;
        self.cx.update(|cx| {
            self.view
                .read(cx)
                .load(nsh::App::Js(Bundle::source(source.to_owned())))
        });
    }
    pub(crate) fn poll(&mut self, predicate: impl Fn(&Value) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.settle();
            assert!(
                self.errors.borrow().is_empty(),
                "NSH engine errors: {:?}",
                self.errors.borrow()
            );
            if predicate(&self.probe.borrow()) {
                return;
            }
            if Instant::now() >= deadline {
                self.screenshot("failure.png");
                eprintln!(
                    "Observed native controls: {:#?}",
                    self.window(|window, _| window.elements())
                );
                panic!(
                    "Base UI did not reach expected state: {}",
                    self.probe.borrow()
                );
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub(crate) fn wait_window(&mut self, mut predicate: impl FnMut(&mut Window, &mut App) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            self.settle();
            assert!(
                self.errors.borrow().is_empty(),
                "NSH engine errors: {:?}",
                self.errors.borrow()
            );
            if self.window(|window, cx| predicate(window, cx)) {
                return;
            }
            if Instant::now() >= deadline {
                self.screenshot("failure.png");
                eprintln!(
                    "Observed native controls: {:#?}",
                    self.window(|window, _| window.elements())
                );
                panic!("Base UI window predicate timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    pub(crate) fn settle(&mut self) {
        self.cx.run_until_parked();
        self.window(|window, cx| window.render_frame(cx));
    }
    pub(crate) fn snapshot(&self) -> Value {
        self.probe.borrow().clone()
    }
    pub(crate) fn window<R>(&mut self, update: impl FnOnce(&mut Window, &mut App) -> R) -> R {
        self.cx
            .update_window(self.handle, |_, window, cx| update(window, cx))
            .unwrap()
    }
    pub(crate) fn click(&mut self, label: &str) {
        self.window(|window, cx| {
            window.render_frame(cx);
            let target = window.find_by_label(label);
            assert!(target.visible(), "{label} must be visible");
            assert!(
                target.bounds().size.width > px(0.) && target.bounds().size.height > px(0.),
                "{label} must have real NSH layout"
            );
            let position = target.bounds().center();
            window.pointer_move(position, None, cx);
            window.pointer_down(position, MouseButton::Left, cx);
            window.pointer_up(position, MouseButton::Left, cx);
        });
    }
    pub(crate) fn key(&mut self, key: &str) {
        self.window(|window, cx| window.press(key, cx));
    }
    pub(crate) fn input(&mut self, text: &str) {
        self.window(|window, cx| window.input(text, cx));
    }
    pub(crate) fn screenshot(&mut self, name: &str) {
        self.settle();
        self.cx
            .capture_screenshot(self.handle)
            .unwrap()
            .save(format!("target/nsh-base-verify/{name}"))
            .unwrap();
    }
    pub(crate) fn pixel(&mut self, label: &str, offset: Point<Pixels>) -> [u8; 4] {
        self.settle();
        let (x, y) = self.window(|window, _| {
            let bounds = window.find_by_label(label).bounds();
            let scale = window.scale_factor();
            (
                ((f32::from(bounds.left() + offset.x)) * scale) as u32,
                ((f32::from(bounds.top() + offset.y)) * scale) as u32,
            )
        });
        self.cx
            .capture_screenshot(self.handle)
            .unwrap()
            .get_pixel(x, y)
            .0
    }
    /// Pixel bounds relative to an observed control, in logical pixels.
    pub(crate) fn color_bounds(
        &mut self,
        label: &str,
        rgb: [u8; 3],
        tolerance: u8,
    ) -> Option<(f32, f32, f32, f32)> {
        self.settle();
        let (bounds, scale) =
            self.window(|window, _| (window.find_by_label(label).bounds(), window.scale_factor()));
        let image = self.cx.capture_screenshot(self.handle).unwrap();
        let left = (f32::from(bounds.left()) * scale).floor().max(0.) as u32;
        let top = (f32::from(bounds.top()) * scale).floor().max(0.) as u32;
        let right = ((f32::from(bounds.right()) * scale).ceil().max(0.) as u32).min(image.width());
        let bottom =
            ((f32::from(bounds.bottom()) * scale).ceil().max(0.) as u32).min(image.height());
        let mut found: Option<(u32, u32, u32, u32)> = None;
        for y in top..bottom {
            for x in left..right {
                let pixel = image.get_pixel(x, y).0;
                if pixel[3] > 0
                    && (0..3).all(|channel| pixel[channel].abs_diff(rgb[channel]) <= tolerance)
                {
                    found = Some(match found {
                        None => (x, y, x, y),
                        Some((min_x, min_y, max_x, max_y)) => {
                            (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                        }
                    });
                }
            }
        }
        found.map(|(min_x, min_y, max_x, max_y)| {
            (
                min_x as f32 / scale - f32::from(bounds.left()),
                min_y as f32 / scale - f32::from(bounds.top()),
                (max_x - min_x + 1) as f32 / scale,
                (max_y - min_y + 1) as f32 / scale,
            )
        })
    }
    pub(crate) fn stop(&mut self) {
        self.cx.update(|cx| self.view.read(cx).stop());
    }
}
