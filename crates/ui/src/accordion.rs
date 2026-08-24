use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc, time::Duration};

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, ContentMask, Element, ElementId, GlobalElementId,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, Style, StyleRefinement, Styled,
    Window, div, percentage, prelude::FluentBuilder as _, px, relative, rems, size,
};

use crate::{ActiveTheme as _, Icon, IconName, Sizable, Size, StyledExt as _, h_flex};
use gpui_base::{
    Accordion as BaseAccordion, AccordionHeader as BaseAccordionHeader,
    AccordionItem as BaseAccordionItem, AccordionPanel as BaseAccordionPanel, AccordionTrigger,
    Spring, spring,
};

/// Expand and collapse motion.
///
/// Critically damped, so the panel height never overshoots the measured content
/// height. A panel toggled again mid-flight reverses from its current velocity
/// instead of restarting.
const PANEL_SPRING: Spring = Spring::new(Duration::from_millis(200));

/// Accordion element.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    style: StyleRefinement,
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
            style: StyleRefinement::default(),
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

impl Styled for Accordion {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_indices = Rc::new(RefCell::new(HashSet::new()));
        let multiple = self.multiple;
        let last_ix = self.children.len().saturating_sub(1);

        BaseAccordion::new(self.id)
            .v_flex()
            .size_full()
            // The bordered accordion is a single rounded card, the items are
            // joined by their separators.
            .when(self.bordered, |this| {
                this.border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius_lg)
                    .overflow_hidden()
            })
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, accordion)| {
                        if accordion.open {
                            open_indices.borrow_mut().insert(ix);
                        }

                        accordion
                            .index(ix)
                            .last(ix == last_ix)
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .on_toggle_click({
                                let open_indices = open_indices.clone();
                                move |open, _, _| {
                                    let mut open_indices = open_indices.borrow_mut();
                                    if *open {
                                        if !multiple {
                                            open_indices.clear();
                                        }
                                        open_indices.insert(ix);
                                    } else {
                                        open_indices.remove(&ix);
                                    }
                                }
                            })
                    }),
            )
            .when_some(
                self.on_toggle_click.filter(|_| !self.disabled),
                |this, on_toggle| {
                    this.on_click(move |_, window, cx| {
                        let open_indices =
                            open_indices.borrow().iter().copied().collect::<Vec<_>>();
                        on_toggle(&open_indices, window, cx)
                    })
                },
            )
    }
}

/// The content of an [`AccordionItem`], animating its height on toggle.
///
/// It stays in the tree while closed, clipped to zero height, to be able to
/// animate the collapse and to keep its natural height measured.
struct AnimatedAccordionPanel {
    id: ElementId,
    progress: f32,
    child: AnyElement,
}

#[derive(Clone, Copy, Default)]
struct AnimatedAccordionPanelState {
    /// The natural height measured in the last prepaint.
    height: Option<Pixels>,
}

impl AnimatedAccordionPanel {
    fn new(id: impl Into<ElementId>, progress: f32, child: AnyElement) -> Self {
        Self {
            id: id.into(),
            progress,
            child,
        }
    }
}

impl IntoElement for AnimatedAccordionPanel {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for AnimatedAccordionPanel {
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
        let height = window.with_element_state(
            global_id.expect("AnimatedAccordionPanel must have an id"),
            |state: Option<AnimatedAccordionPanelState>, _| {
                let state = state.unwrap_or_default();
                (state.height, state)
            },
        );

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        match height {
            // Not measured yet, let the content lay itself out.
            None if self.progress > 0. => {}
            None => style.size.height = px(0.).into(),
            Some(height) => style.size.height = (height * self.progress).into(),
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
        // Lay out at the natural height, `bounds` clips the visible part.
        let available = size(
            AvailableSpace::Definite(bounds.size.width),
            AvailableSpace::MinContent,
        );
        let measured = self.child.layout_as_root(available, window, cx);

        let changed = window.with_element_state(
            global_id.expect("AnimatedAccordionPanel must have an id"),
            |state: Option<AnimatedAccordionPanelState>, _| {
                let mut state = state.unwrap_or_default();
                let changed = state.height != Some(measured.height);
                state.height = Some(measured.height);
                (changed, state)
            },
        );

        // The measured height is only used by the next `request_layout`, ask
        // for that frame, or the new height never gets painted.
        if changed {
            window.request_animation_frame();
        }

        // Masked here too, so the hidden content takes no mouse events.
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
    last: bool,
    style: StyleRefinement,
    hover_style: Option<StyleRefinement>,
    title_style: StyleRefinement,
    content_style: StyleRefinement,
    icon: Option<Icon>,
    title: AnyElement,
    children: Vec<AnyElement>,
    open: bool,
    size: Size,
    disabled: bool,
    on_toggle_click: Option<Arc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl AccordionItem {
    /// Create a new AccordionItem.
    pub fn new() -> Self {
        Self {
            index: 0,
            last: false,
            style: StyleRefinement::default(),
            hover_style: None,
            title_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            icon: None,
            title: SharedString::default().into_any_element(),
            children: Vec::new(),
            open: false,
            disabled: false,
            on_toggle_click: None,
            size: Size::default(),
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

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set extra style for the title row.
    pub fn title_style(mut self, style: StyleRefinement) -> Self {
        self.title_style = style;
        self
    }

    /// Set the style of the title row while the mouse is over it.
    ///
    /// There is no hover style by default. The title row is the part that
    /// toggles the item, so the hover feedback belongs there, not on the
    /// whole item.
    pub fn hover(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.hover_style = Some(f(StyleRefinement::default()));
        self
    }

    /// Set extra style for the content below the title.
    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
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

impl Styled for AccordionItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AccordionItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_size = match self.size {
            Size::XSmall => rems(0.8125),
            Size::Large => rems(1.0),
            _ => rems(0.875),
        };
        let progress = spring(
            (self.index, "accordion-panel"),
            if self.open { 1. } else { 0. },
            PANEL_SPRING,
            window,
            cx,
        );
        let trigger = AccordionTrigger::new(("trigger", self.index))
            .open(self.open)
            .disabled(self.disabled)
            .h_flex()
            .justify_between()
            .gap_3()
            .font_medium()
            .map(|this| match self.size {
                Size::XSmall => this.py_1().px_1p5(),
                Size::Small => this.py_1p5().px_2(),
                Size::Large => this.py_3().px_4(),
                _ => this.py_2().px_3(),
            })
            .when(self.open, |this| this.text_color(cx.theme().foreground))
            .refine_style(&self.title_style)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .map(|this| match self.size {
                        Size::XSmall | Size::Small => this.gap_1(),
                        _ => this.gap_2(),
                    })
                    .when_some(self.icon, |this, icon| {
                        this.child(icon.with_size(self.size))
                    })
                    .child(self.title),
            )
            .when(!self.disabled, |this| {
                this.when_some(self.hover_style, |this, hover_style| {
                    this.hover(move |this| this.refine_style(&hover_style))
                })
                .child(
                    Icon::new(IconName::ChevronDown)
                        .xsmall()
                        .flex_none()
                        .text_color(cx.theme().muted_foreground)
                        .rotate(percentage(if self.open { 0.5 } else { 0. })),
                )
                .when_some(self.on_toggle_click, |this, on_toggle_click| {
                    this.on_change(move |open, _, window, cx| {
                        on_toggle_click(&open, window, cx);
                    })
                })
            });

        div().flex_1().child(
            BaseAccordionItem::new()
                .open(self.open)
                .disabled(self.disabled)
                .header(
                    BaseAccordionHeader::new(trigger)
                        .id(("header", self.index))
                        .w_full(),
                )
                .panel(
                    BaseAccordionPanel::new()
                        .id(("panel", self.index))
                        .open(self.open)
                        .keep_mounted(true)
                        .w_full()
                        .child(AnimatedAccordionPanel::new(
                            ("content", self.index),
                            progress,
                            div()
                                .map(|this| match self.size {
                                    Size::XSmall => this.pb_1().px_1p5(),
                                    Size::Small => this.pb_1p5().px_2(),
                                    Size::Large => this.pb_3().px_4(),
                                    _ => this.pb_2().px_3(),
                                })
                                .refine_style(&self.content_style)
                                .children(self.children)
                                .into_any_element(),
                        )),
                )
                .v_flex()
                .w_full()
                .bg(cx.theme().tokens.accordion)
                .overflow_hidden()
                .when(!self.last, |this| {
                    this.border_b_1().border_color(cx.theme().border)
                })
                .text_size(text_size)
                .refine_style(&self.style),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Context, Render, TestAppContext, div, px};

    use super::*;

    struct Harness;

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            Accordion::new("accordion-layout")
                .w(px(240.))
                .h(px(100.))
                .item(|item| {
                    item.open(true)
                        .title(div().debug_selector(|| "first-title".into()).child("First"))
                        .child(div().debug_selector(|| "first-content".into()).h(px(60.)))
                })
                .item(|item| {
                    item.title(
                        div()
                            .debug_selector(|| "second-title".into())
                            .child("Second"),
                    )
                })
        }
    }

    #[gpui::test]
    fn expanded_panel_keeps_content_between_its_header_and_the_next_item(cx: &mut TestAppContext) {
        cx.update(crate::theme::init);
        let (_, cx) = cx.add_window_view(|_, _| Harness);
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let first = cx.debug_bounds("first-title").unwrap();
        let content = cx.debug_bounds("first-content").unwrap();
        let second = cx.debug_bounds("second-title").unwrap();
        assert!(first.origin.y < content.origin.y);
        assert!(content.origin.y + content.size.height <= second.origin.y);
    }
}
