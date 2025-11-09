use gpui::{
    anchored, deferred, div, prelude::FluentBuilder as _, px, AnyElement, App, AppContext, Bounds,
    Context, Corner, DismissEvent, DispatchPhase, Element, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, GlobalElementId, Hitbox, InteractiveElement as _, IntoElement,
    KeyBinding, LayoutId, ManagedView, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    Render, RenderOnce, Style, StyleRefinement, Styled, Window,
};
use std::{cell::RefCell, rc::Rc};

use crate::{actions::Cancel, Selectable, StyledExt as _};

const CONTEXT: &str = "Popover";

pub(crate) fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Cancel, Some(CONTEXT))])
}

enum PopoverContent<M: ManagedView> {
    Element(AnyElement),
    View(Entity<M>),
}

impl<M> RenderOnce for PopoverContent<M>
where
    M: ManagedView,
{
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        match self {
            PopoverContent::Element(element) => element.into_any_element(),
            PopoverContent::View(view) => view.into_any_element(),
        }
    }
}

/// The content of the popover.
struct PopoverContentContainer {
    style: StyleRefinement,
    focus_handle: FocusHandle,
    content: Rc<dyn Fn(&mut Window, &mut Context<Self>) -> AnyElement>,
}

impl PopoverContentContainer {
    /// Create a new PopoverContent.
    fn new<B, E>(_: &mut Window, cx: &mut App, content: B) -> Self
    where
        E: IntoElement,
        B: Fn(&mut Window, &mut Context<Self>) -> E + 'static,
    {
        let focus_handle = cx.focus_handle();

        Self {
            style: StyleRefinement::default(),
            focus_handle,
            content: Rc::new(move |window, cx| content(window, cx).into_any_element()),
        }
    }

    /// Build a new PopoverContent.
    fn build<B, E>(window: &mut Window, cx: &mut App, content: B) -> Entity<Self>
    where
        E: IntoElement,
        B: Fn(&mut Window, &mut Context<Self>) -> E + 'static,
    {
        cx.new(|cx| Self::new(window, cx, content))
    }
}
impl EventEmitter<DismissEvent> for PopoverContentContainer {}

impl Render for PopoverContentContainer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .refine_style(&self.style)
            .track_focus(&self.focus_handle)
            .key_context(CONTEXT)
            .on_action(cx.listener(|_, _: &Cancel, _, cx| {
                cx.propagate();
                cx.emit(DismissEvent);
            }))
            .child(self.content.clone()(window, cx))
    }
}

/// A popover element that can be triggered by a button or any other element.
pub struct Popover<M: ManagedView> {
    id: ElementId,
    style: StyleRefinement,
    anchor: Corner,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<Rc<dyn Fn(&mut Window, &mut App) -> Entity<M> + 'static>>,
    /// Style for trigger element.
    /// This is used for hotfix the trigger element style to support w_full.
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
}

impl<M> Popover<M>
where
    M: ManagedView,
{
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Corner::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            mouse_button: MouseButton::Left,
            appearance: true,
        }
    }

    /// Set the anchor corner of the popover, default is `Corner::TopLeft`.
    pub fn anchor(mut self, anchor: Corner) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let selected = trigger.is_selected();
            trigger.selected(selected || is_open).into_any_element()
        }));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: &StyleRefinement) -> Self {
        self.trigger_style = Some(style.clone());
        self
    }

    /// Set the content of the popover.
    pub fn content(mut self, content: impl Into<PopoverContent<M>>) -> Self {
        let content: PopoverContent<M> = content.into();
        self.content = Some(Rc::new(move |window, cx| mathch content {
            PopoverContent::render(content, window, cx).into_entity(window, cx)
        }));
        self
    }

    /// Set whether the popover no style, default is `false`.
    ///
    /// If no style:
    ///
    /// - The popover will not have a bg, border, shadow, or padding.
    /// - The click out of the popover will not dismiss it.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    fn render_trigger(&mut self, open: bool, window: &mut Window, cx: &mut App) -> AnyElement {
        let Some(trigger) = self.trigger.take() else {
            return div().into_any_element();
        };

        (trigger)(open, window, cx)
    }

    fn resolved_corner(&self, bounds: Bounds<Pixels>) -> Point<Pixels> {
        bounds.corner(match self.anchor {
            Corner::TopLeft => Corner::BottomLeft,
            Corner::TopRight => Corner::BottomRight,
            Corner::BottomLeft => Corner::TopLeft,
            Corner::BottomRight => Corner::TopRight,
        })
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut PopoverElementState<M>, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<PopoverElementState<M>, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.unwrap().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<M> Styled for Popover<M>
where
    M: ManagedView,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<M> IntoElement for Popover<M>
where
    M: ManagedView,
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

pub struct PopoverElementState<M> {
    trigger_layout_id: Option<LayoutId>,
    popover_layout_id: Option<LayoutId>,
    popover_element: Option<AnyElement>,
    trigger_element: Option<AnyElement>,
    content_view: Rc<RefCell<Option<Entity<M>>>>,
    /// Trigger bounds for positioning the popover.
    trigger_bounds: Option<Bounds<Pixels>>,
}

impl<M> Default for PopoverElementState<M> {
    fn default() -> Self {
        Self {
            trigger_layout_id: None,
            popover_layout_id: None,
            popover_element: None,
            trigger_element: None,
            content_view: Rc::new(RefCell::new(None)),
            trigger_bounds: None,
        }
    }
}

pub struct PrepaintState {
    hitbox: Hitbox,
    /// Trigger bounds for limit a rect to handle mouse click.
    trigger_bounds: Option<Bounds<Pixels>>,
}

impl<M: ManagedView> Element for Popover<M> {
    type RequestLayoutState = PopoverElementState<M>;
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();

        // FIXME: Remove this and find a better way to handle this.
        // Apply trigger style, for support w_full for trigger.
        //
        // If remove this, the trigger will not support w_full.
        if let Some(trigger_style) = self.trigger_style.clone() {
            if let Some(width) = trigger_style.size.width {
                style.size.width = width;
            }
            if let Some(display) = trigger_style.display {
                style.display = display;
            }
        }

        let popover_style = self.style.clone();

        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |view, element_state, window, cx| {
                let mut popover_layout_id = None;
                let mut popover_element = None;
                let mut is_open = false;

                if let Some(content_view) = element_state.content_view.borrow_mut().as_mut() {
                    is_open = true;

                    let mut anchored = anchored()
                        .snap_to_window_with_margin(px(8.))
                        .anchor(view.anchor);
                    if let Some(trigger_bounds) = element_state.trigger_bounds {
                        anchored = anchored.position(view.resolved_corner(trigger_bounds));
                    }

                    let mut element = {
                        let content_view_mut = element_state.content_view.clone();
                        let anchor = view.anchor;
                        let appearance = view.appearance;
                        deferred(
                            anchored.child(
                                div()
                                    .size_full()
                                    .occlude()
                                    .tab_group()
                                    .when(appearance, |this| this.popover_style(cx).p_4())
                                    .map(|this| match anchor {
                                        Corner::TopLeft | Corner::TopRight => this.top_1(),
                                        Corner::BottomLeft | Corner::BottomRight => this.bottom_1(),
                                    })
                                    .child(content_view.clone())
                                    .when(appearance, |this| {
                                        this.on_mouse_down_out(move |_, window, _| {
                                            // Update the element_state.content_view to `None`,
                                            // so that the `paint`` method will not paint it.
                                            *content_view_mut.borrow_mut() = None;
                                            window.refresh();
                                        })
                                    })
                                    .refine_style(&popover_style),
                            ),
                        )
                        .with_priority(1)
                        .into_any()
                    };

                    popover_layout_id = Some(element.request_layout(window, cx));
                    popover_element = Some(element);
                }

                let mut trigger_element = view.render_trigger(is_open, window, cx);
                let trigger_layout_id = trigger_element.request_layout(window, cx);

                let layout_id = window.request_layout(
                    style,
                    Some(trigger_layout_id).into_iter().chain(popover_layout_id),
                    cx,
                );

                (
                    layout_id,
                    PopoverElementState {
                        trigger_layout_id: Some(trigger_layout_id),
                        popover_layout_id,
                        popover_element,
                        trigger_element: Some(trigger_element),
                        ..Default::default()
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&gpui::GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _bounds: gpui::Bounds<gpui::Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(element) = &mut request_layout.trigger_element {
            element.prepaint(window, cx);
        }
        if let Some(element) = &mut request_layout.popover_element {
            element.prepaint(window, cx);
        }

        let trigger_bounds = request_layout
            .trigger_layout_id
            .map(|id| window.layout_bounds(id));

        // Prepare the popover, for get the bounds of it for open window size.
        let _ = request_layout
            .popover_layout_id
            .map(|id| window.layout_bounds(id));

        let hitbox = window.insert_hitbox(
            trigger_bounds.unwrap_or_default(),
            gpui::HitboxBehavior::Normal,
        );

        PrepaintState {
            trigger_bounds,
            hitbox,
        }
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.with_element_state(
            id.unwrap(),
            window,
            cx,
            |this, element_state, window, cx| {
                element_state.trigger_bounds = prepaint.trigger_bounds;

                if let Some(mut element) = request_layout.trigger_element.take() {
                    element.paint(window, cx);
                }

                if let Some(mut element) = request_layout.popover_element.take() {
                    element.paint(window, cx);
                    return;
                }

                // When mouse click down in the trigger bounds, open the popover.
                let Some(content_build) = this.content.take() else {
                    return;
                };
                let old_content_view = element_state.content_view.clone();
                let hitbox_id = prepaint.hitbox.id;
                let mouse_button = this.mouse_button;
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble
                        && event.button == mouse_button
                        && hitbox_id.is_hovered(window)
                    {
                        cx.stop_propagation();
                        window.prevent_default();

                        let new_content_view = (content_build)(window, cx);
                        let old_content_view1 = old_content_view.clone();

                        let previous_focus_handle = window.focused(cx);

                        window
                            .subscribe(
                                &new_content_view,
                                cx,
                                move |dialog, _: &DismissEvent, window, cx| {
                                    if dialog.focus_handle(cx).contains_focused(window, cx) {
                                        if let Some(previous_focus_handle) =
                                            previous_focus_handle.as_ref()
                                        {
                                            window.focus(previous_focus_handle);
                                        }
                                    }
                                    *old_content_view1.borrow_mut() = None;

                                    window.refresh();
                                },
                            )
                            .detach();

                        window.focus(&new_content_view.focus_handle(cx));
                        *old_content_view.borrow_mut() = Some(new_content_view);
                        window.refresh();
                    }
                });
            },
        );
    }
}
