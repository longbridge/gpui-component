use crate::{
    text::Text, v_flex, ActiveTheme, Disableable, IconName, Selectable, Sizable, Size,
    StyledExt as _,
};
use gpui::{
    div, prelude::FluentBuilder as _, px, relative, rems, svg, AnyElement, App, Div, ElementId,
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
    Window,
};

/// A Checkbox element.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    base: Div,
    label: Option<Text>,
    children: Vec<AnyElement>,
    checked: bool,
    disabled: bool,
    size: Size,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            label: None,
            children: Vec::new(),
            checked: false,
            disabled: false,
            size: Size::default(),
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<Text>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl InteractiveElement for Checkbox {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.base.interactivity()
    }
}
impl StatefulInteractiveElement for Checkbox {}

impl Styled for Checkbox {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.base.style()
    }
}

impl Disableable for Checkbox {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Selectable for Checkbox {
    fn element_id(&self) -> &ElementId {
        &self.id
    }

    fn selected(self, selected: bool) -> Self {
        self.checked(selected)
    }
}

impl ParentElement for Checkbox {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Checkbox {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Checkbox {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (color, icon_color) = if self.disabled {
            (
                cx.theme().primary.opacity(0.5),
                cx.theme().primary_foreground.opacity(0.5),
            )
        } else {
            (cx.theme().primary, cx.theme().primary_foreground)
        };
        let radius = (cx.theme().radius / 2.).min(px(6.));

        div().child(
            self.base
                .id(self.id)
                .h_flex()
                .gap_2()
                .items_start()
                .line_height(relative(1.))
                .text_color(cx.theme().foreground)
                .map(|this| match self.size {
                    Size::XSmall => this.text_xs(),
                    Size::Small => this.text_sm(),
                    Size::Medium => this.text_base(),
                    Size::Large => this.text_lg(),
                    _ => this,
                })
                .child(
                    v_flex()
                        .relative()
                        .map(|this| match self.size {
                            Size::XSmall => this.size_3(),
                            Size::Small => this.size_3p5(),
                            Size::Medium => this.size_4(),
                            Size::Large => this.size(rems(1.125)),
                            _ => this.size_4(),
                        })
                        .flex_shrink_0()
                        .border_1()
                        .border_color(color)
                        .rounded(radius)
                        .map(|this| match self.checked {
                            false => this.bg(cx.theme().transparent),
                            _ => this.bg(color),
                        })
                        .child(
                            svg()
                                .absolute()
                                .top_px()
                                .left_px()
                                .map(|this| match self.size {
                                    Size::XSmall => this.size_2(),
                                    Size::Small => this.size_2p5(),
                                    Size::Medium => this.size_3(),
                                    Size::Large => this.size_3p5(),
                                    _ => this.size_3(),
                                })
                                .text_color(icon_color)
                                .map(|this| match self.checked {
                                    true => this.path(IconName::Check.path()),
                                    _ => this,
                                }),
                        ),
                )
                .when(self.label.is_some() || !self.children.is_empty(), |this| {
                    this.child(
                        v_flex()
                            .w_full()
                            .line_height(relative(1.2))
                            .gap_1()
                            .map(|this| {
                                if let Some(label) = self.label {
                                    this.child(
                                        div()
                                            .size_full()
                                            .text_color(cx.theme().foreground)
                                            .line_height(relative(1.))
                                            .child(label),
                                    )
                                } else {
                                    this
                                }
                            })
                            .children(self.children),
                    )
                })
                .when(self.disabled, |this| {
                    this.text_color(cx.theme().muted_foreground)
                })
                .when_some(
                    self.on_click.filter(|_| !self.disabled),
                    |this, on_click| {
                        this.on_click(move |_, window, cx| {
                            cx.stop_propagation();
                            let checked = !self.checked;
                            on_click(&checked, window, cx);
                        })
                    },
                ),
        )
    }
}
