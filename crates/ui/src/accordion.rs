use std::{
    cell::RefCell,
    collections::HashSet,
    rc::Rc,
    sync::Arc,
    time::{Duration, Instant},
};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, Element, ElementId, GlobalElementId,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, ParentElement, Pixels,
    RenderOnce, Role, SharedString, StatefulInteractiveElement as _, Style, Styled, Window, div,
    percentage, prelude::FluentBuilder as _, px, relative, rems, size,
};

use crate::{
    ActiveTheme as _, Icon, IconName, Sizable, Size, StyledExt as _, animation::ease_out_cubic,
    h_flex, v_flex,
};

/// Duration of the expand/collapse animation.
const ANIMATION_DURATION: Duration = Duration::from_millis(200);

/// Accordion element.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    multiple: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    children: Vec<AccordionItem>,
    on_toggle_click: Option<Arc<dyn Fn(&[usize], &mut Window, &mut App) + Send + Sync>>,
}

impl Accordion {
    /// Create a new Accordion with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            multiple: false,
            size: Size::default(),
            bordered: true,
            children: Vec::new(),
            disabled: false,
            on_toggle_click: None,
        }
    }

    /// Set whether multiple accordion items can be opened simultaneously, default: false
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set whether the accordion items have borders, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set whether the accordion is disabled, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Adds an AccordionItem to the Accordion.
    pub fn item<F>(mut self, child: F) -> Self
    where
        F: FnOnce(AccordionItem) -> AccordionItem,
    {
        let item = child(AccordionItem::new());
        self.children.push(item);
        self
    }

    /// Sets the on_toggle_click callback for the AccordionGroup.
    ///
    /// The first argument `Vec<usize>` is the indices of the open accordions.
    pub fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&[usize], &mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl Sizable for Accordion {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let open_ixs = Rc::new(RefCell::new(HashSet::new()));
        let is_multiple = self.multiple;
        let last_ix = self.children.len().saturating_sub(1);

        v_flex()
            .id(self.id)
            .size_full()
            .when(self.bordered, |this| this.gap_y_2())
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, accordion)| {
                        if accordion.open {
                            open_ixs.borrow_mut().insert(ix);
                        }

                        accordion
                            .index(ix)
                            .last(ix == last_ix)
                            .with_size(self.size)
                            .bordered(self.bordered)
                            .disabled(self.disabled)
                            .on_toggle_click({
                                let open_ixs = Rc::clone(&open_ixs);
                                move |open, _, _| {
                                    let mut open_ixs = open_ixs.borrow_mut();
                                    if *open {
                                        if !is_multiple {
                                            open_ixs.clear();
                                        }
                                        open_ixs.insert(ix);
                                    } else {
                                        open_ixs.remove(&ix);
                                    }
                                }
                            })
                    }),
            )
            .when_some(
                self.on_toggle_click.filter(|_| !self.disabled),
                move |this, on_toggle_click| {
                    let open_ixs = Rc::clone(&open_ixs);
                    this.on_click(move |_, window, cx| {
                        let open_ixs: Vec<usize> = open_ixs.borrow().iter().map(|&ix| ix).collect();

                        on_toggle_click(&open_ixs, window, cx);
                    })
                },
            )
    }
}

/// The content of an [`AccordionItem`], animating its height on toggle.
///
/// The content stays in the element tree while closed (clipped to zero height),
/// so that the collapse can be animated, and so that its natural height is
/// still measured for the next expand.
struct AccordionContent {
    id: ElementId,
    open: bool,
    child: AnyElement,
}

#[derive(Clone, Copy)]
struct AccordionContentState {
    /// The `open` of the last frame, to detect a toggle.
    open: bool,
    /// The progress at the time the current animation started.
    from: f32,
    /// `None` when the content is settled at its target height.
    started_at: Option<Instant>,
    /// The natural height measured in the last prepaint.
    height: Option<Pixels>,
}

impl AccordionContentState {
    fn new(open: bool) -> Self {
        Self {
            open,
            from: if open { 1. } else { 0. },
            started_at: None,
            height: None,
        }
    }

    /// The progress of the animation, from 0. (closed) to 1. (open).
    ///
    /// This finishes the animation when the duration has passed, so it takes
    /// `&mut self`.
    fn progress(&mut self) -> f32 {
        let target = if self.open { 1. } else { 0. };
        let Some(started_at) = self.started_at else {
            return target;
        };

        let t = started_at.elapsed().as_secs_f32() / ANIMATION_DURATION.as_secs_f32();
        if t >= 1. {
            self.started_at = None;
            return target;
        }

        self.from + (target - self.from) * ease_out_cubic(t)
    }
}

impl AccordionContent {
    fn new(id: impl Into<ElementId>, open: bool, child: AnyElement) -> Self {
        Self {
            id: id.into(),
            open,
            child,
        }
    }
}

impl IntoElement for AccordionContent {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for AccordionContent {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let open = self.open;
        let (progress, height) = window.with_element_state(
            global_id.expect("AccordionContent must have an id"),
            |state: Option<AccordionContentState>, window| {
                let mut state = state.unwrap_or_else(|| AccordionContentState::new(open));

                if state.open != open {
                    state.from = state.progress();
                    state.open = open;
                    state.started_at = Some(Instant::now());
                }

                let progress = state.progress();
                if state.started_at.is_some() {
                    window.request_animation_frame();
                }

                ((progress, state.height), state)
            },
        );

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        match height {
            // Before the first measure there is nothing to animate to, let the
            // content lay itself out to get its natural height measured.
            None if open => {}
            None => style.size.height = px(0.).into(),
            Some(height) => style.size.height = (height * progress).into(),
        }

        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // Lay the content out at its natural height, the visible part is the
        // one clipped by `bounds`.
        let available = size(
            AvailableSpace::Definite(bounds.size.width),
            AvailableSpace::MinContent,
        );
        let measured = self.child.layout_as_root(available, window, cx);

        window.with_element_state(
            global_id.expect("AccordionContent must have an id"),
            |state: Option<AccordionContentState>, _| {
                let mut state = state.unwrap_or_else(|| AccordionContentState::new(self.open));
                state.height = Some(measured.height);
                ((), state)
            },
        );

        // Mask here as well as in paint, to keep the hidden content from
        // taking mouse events.
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.child.prepaint_at(bounds.origin, window, cx);
        });
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            self.child.paint(window, cx);
        });
    }
}

/// An Accordion is a vertically stacked list of items, each of which can be expanded to reveal the content associated with it.
#[derive(IntoElement)]
pub struct AccordionItem {
    index: usize,
    /// The last item does not need a separator under it.
    last: bool,
    icon: Option<Icon>,
    title: AnyElement,
    children: Vec<AnyElement>,
    open: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    on_toggle_click: Option<Arc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl AccordionItem {
    /// Create a new AccordionItem.
    pub fn new() -> Self {
        Self {
            index: 0,
            last: false,
            icon: None,
            title: SharedString::default().into_any_element(),
            children: Vec::new(),
            open: false,
            disabled: false,
            on_toggle_click: None,
            size: Size::default(),
            bordered: true,
        }
    }

    /// Set the icon for the accordion item.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the title for the accordion item.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = title.into_any_element();
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    fn last(mut self, last: bool) -> Self {
        self.last = last;
        self
    }

    fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl ParentElement for AccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for AccordionItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for AccordionItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_size = match self.size {
            Size::XSmall => rems(0.875),
            Size::Small => rems(0.875),
            _ => rems(1.0),
        };

        div().flex_1().child(
            v_flex()
                .w_full()
                .bg(cx.theme().tokens.accordion)
                .overflow_hidden()
                .map(|this| {
                    if self.bordered {
                        this.border_1()
                            .rounded(cx.theme().radius)
                            .border_color(cx.theme().border)
                    } else if self.last {
                        this
                    } else {
                        // Without borders the items are separated by a single
                        // line under each of them, below the content.
                        this.border_b_1().border_color(cx.theme().border)
                    }
                })
                .text_size(text_size)
                .child(
                    h_flex()
                        .id(self.index)
                        .role(Role::Button)
                        .aria_expanded(self.open)
                        .justify_between()
                        .gap_3()
                        .font_medium()
                        .map(|this| match self.size {
                            Size::XSmall => this.py_1().px_2(),
                            Size::Small => this.py_1p5().px_2p5(),
                            Size::Large => this.py_3().px_4(),
                            _ => this.py_2p5().px_3(),
                        })
                        .when(self.open, |this| this.text_color(cx.theme().foreground))
                        .child(
                            h_flex()
                                .items_center()
                                .map(|this| match self.size {
                                    Size::XSmall => this.gap_1(),
                                    Size::Small => this.gap_1(),
                                    _ => this.gap_2(),
                                })
                                .when_some(self.icon, |this, icon| {
                                    this.child(
                                        icon.with_size(self.size)
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                })
                                .child(self.title),
                        )
                        .when(!self.disabled, |this| {
                            this.hover(|this| this.bg(cx.theme().tokens.accordion_hover))
                                .child(
                                    Icon::new(IconName::ChevronDown)
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground)
                                        .rotate(percentage(if self.open { 0.5 } else { 0. })),
                                )
                                .when_some(self.on_toggle_click, |this, on_toggle_click| {
                                    this.on_click({
                                        move |_, window, cx| {
                                            on_toggle_click(&!self.open, window, cx);
                                        }
                                    })
                                })
                        }),
                )
                .child(AccordionContent::new(
                    ("content", self.index),
                    self.open,
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .map(|this| match self.size {
                            Size::XSmall => this.px_2().pb_2(),
                            Size::Small => this.px_2p5().pb_2p5(),
                            Size::Large => this.px_4().pb_4(),
                            _ => this.px_3().pb_3(),
                        })
                        .children(self.children)
                        .into_any_element(),
                )),
        )
    }
}
