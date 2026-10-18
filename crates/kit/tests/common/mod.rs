//! Fixtures use the production window entry point with deterministic test bounds.
#![allow(dead_code)]
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, Bounds, Context, Entity, Pixels, Point, Render, Size,
    TestAppContext, Window, WindowBounds, WindowHandle, WindowOptions, base::Root,
};

pub fn open_window<V: Render>(
    cx: &mut TestAppContext,
    size: Option<Size<Pixels>>,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> (WindowHandle<Root>, Entity<V>) {
    cx.update(|cx| {
        let bounds = size
            .map(|size| Bounds {
                origin: Point::default(),
                size,
            })
            .unwrap_or_else(|| Bounds::maximized(None, cx));
        let (window, content) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            cx,
            build,
        )
        .expect("open test window");
        (window.downcast::<Root>().expect("Base Root"), content)
    })
}

pub fn update_content<V: Render, R>(
    window: impl Into<AnyWindowHandle>,
    content: &Entity<V>,
    cx: &mut TestAppContext,
    update: impl FnOnce(&mut V, &mut Window, &mut Context<V>) -> R,
) -> gpui_kit::Result<R> {
    cx.update_window(window.into(), |_, window, cx| {
        content.update(cx, |view, cx| update(view, window, cx))
    })
}

/// The editable semantic descendant, not the frame that also scopes addons.
pub fn input(
    window: &mut Window,
    frame: impl Into<gpui_kit::ElementId>,
) -> gpui_kit::test::ElementSnapshot {
    input_in(&window.within(frame))
}

pub fn input_in(scope: &gpui_kit::test::ScopedWindow<'_>) -> gpui_kit::test::ElementSnapshot {
    let mut nodes = scope.elements().into_iter().filter(|node| {
        matches!(
            node.role(),
            Some(
                gpui_kit::Role::TextInput
                    | gpui_kit::Role::MultilineTextInput
                    | gpui_kit::Role::PasswordInput
            )
        )
    });
    let input = nodes.next().expect("one semantic text input in its frame");
    assert!(
        nodes.next().is_none(),
        "multiple semantic text inputs in one frame"
    );
    input
}
