use std::rc::{Rc, Weak};

use gpui::{
    div, prelude::FluentBuilder as _, px, AlignItems, AnyElement, AnyView, Axis, Div, Element,
    ElementId, FocusHandle, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, Styled, WindowContext,
};

use crate::{h_flex, v_flex, ActiveTheme as _, AxisExt, FocusableCycle, Sizable, Size, StyledExt};

/// Create a new form with a vertical layout.
pub fn v_form() -> Form {
    Form::vertical()
}

/// Create a new form with a horizontal layout.
pub fn h_form() -> Form {
    Form::horizontal()
}

/// Create a new form field.
pub fn form_field() -> FormField {
    FormField::new()
}

#[derive(IntoElement)]
pub struct Form {
    fields: Vec<FormField>,
    props: FieldProps,
}

impl Form {
    fn new() -> Self {
        Self {
            props: FieldProps::default(),
            fields: Vec::new(),
        }
    }

    /// Creates a new form with a horizontal layout.
    pub fn horizontal() -> Self {
        Self::new().layout(Axis::Horizontal)
    }

    /// Creates a new form with a vertical layout.
    pub fn vertical() -> Self {
        Self::new().layout(Axis::Vertical)
    }

    /// Set the layout for the form, default is `Axis::Vertical`.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.props.layout = layout;
        self
    }

    /// Set the width of the labels in the form. Default is `px(100.)`.
    pub fn label_width(mut self, width: Pixels) -> Self {
        self.props.label_width = Some(width);
        self
    }

    pub fn child(mut self, field: impl Into<FormField>) -> Self {
        self.fields.push(field.into());
        self
    }
}

impl Sizable for Form {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.props.size = size.into();
        self
    }
}

impl FocusableCycle for Form {
    fn cycle_focus_handles(&self, _: &mut WindowContext) -> Vec<FocusHandle>
    where
        Self: Sized,
    {
        self.fields
            .iter()
            .filter_map(|item| item.focus_handle.clone())
            .collect()
    }
}

pub enum FieldBuilder {
    String(SharedString),
    Element(Rc<dyn Fn(&mut WindowContext) -> AnyElement>),
    View(AnyView),
}

impl Default for FieldBuilder {
    fn default() -> Self {
        Self::String(SharedString::default())
    }
}

impl From<AnyView> for FieldBuilder {
    fn from(view: AnyView) -> Self {
        Self::View(view)
    }
}

impl RenderOnce for FieldBuilder {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        match self {
            FieldBuilder::String(value) => value.into_any_element(),
            FieldBuilder::Element(builder) => builder(cx),
            FieldBuilder::View(view) => view.into_any(),
        }
    }
}

impl From<&'static str> for FieldBuilder {
    fn from(value: &'static str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for FieldBuilder {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<SharedString> for FieldBuilder {
    fn from(value: SharedString) -> Self {
        Self::String(value)
    }
}

#[derive(Clone, Copy)]
struct FieldProps {
    size: Size,
    label_width: Option<Pixels>,
    layout: Axis,
}

impl Default for FieldProps {
    fn default() -> Self {
        Self {
            label_width: Some(px(100.)),
            layout: Axis::Vertical,
            size: Size::default(),
        }
    }
}

#[derive(IntoElement)]
pub struct FormField {
    id: ElementId,
    form: Weak<Form>,
    label: Option<FieldBuilder>,
    focus_handle: Option<FocusHandle>,
    description: Option<FieldBuilder>,
    /// Used to render the actual form field, e.g.: TextInput, Switch...
    child: Div,
    visible: bool,
    required: bool,
    /// Alignment of the form field.
    align_items: Option<AlignItems>,
    props: FieldProps,
}

impl FormField {
    pub fn new() -> Self {
        Self {
            id: 0.into(),
            form: Weak::new(),
            label: None,
            description: None,
            child: div(),
            visible: true,
            required: false,
            focus_handle: None,
            align_items: None,
            props: FieldProps::default(),
        }
    }

    /// Sets the label for the form field.
    pub fn label(mut self, label: impl Into<FieldBuilder>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the label for the form field using a function.
    pub fn label_fn<F, E>(mut self, label: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut WindowContext) -> E + 'static,
    {
        self.label = Some(FieldBuilder::Element(Rc::new(move |cx| {
            label(cx).into_any_element()
        })));
        self
    }

    /// Sets the description for the form field.
    pub fn description(mut self, description: impl Into<FieldBuilder>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the description for the form field using a function.
    pub fn description_fn<F, E>(mut self, description: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut WindowContext) -> E + 'static,
    {
        self.description = Some(FieldBuilder::Element(Rc::new(move |cx| {
            description(cx).into_any_element()
        })));
        self
    }

    /// Set the visibility of the form field, default is `true`.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set the required status of the form field, default is `false`.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set the focus handle for the form field.
    ///
    /// If not set, the form field will not be focusable.
    pub fn track_focus(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }

    pub fn parent(mut self, form: &Rc<Form>) -> Self {
        self.form = Rc::downgrade(form);
        self
    }

    /// Set the properties for the form field.
    ///
    /// This is internal API for sync props from From.
    fn props(mut self, ix: usize, props: FieldProps) -> Self {
        self.id = ix.into();
        self.props = props;
        self
    }

    /// Align the form field items to the start, this is the default.
    pub fn items_start(mut self) -> Self {
        self.align_items = Some(AlignItems::Start);
        self
    }

    /// Align the form field items to the end.
    pub fn items_end(mut self) -> Self {
        self.align_items = Some(AlignItems::End);
        self
    }

    /// Align the form field items to the center.
    pub fn items_center(mut self) -> Self {
        self.align_items = Some(AlignItems::Center);
        self
    }
}
impl ParentElement for FormField {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.child.extend(elements);
    }
}
impl RenderOnce for FormField {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let layout = self.props.layout;

        let label_width = if layout.is_vertical() {
            None
        } else {
            self.props.label_width
        };

        #[inline]
        fn wrap_div(layout: Axis) -> Div {
            if layout.is_vertical() {
                v_flex()
            } else {
                h_flex()
            }
        }

        fn wrap_label(label_width: Option<Pixels>) -> Div {
            h_flex().when_some(label_width, |this, width| this.w(width).flex_shrink_0())
        }

        let gap = match self.props.size {
            Size::XSmall | Size::Small => px(4.),
            _ => px(8.),
        };

        v_flex()
            .flex_1()
            .gap(gap)
            .child(
                // This warp for aligning the Label + Input
                wrap_div(layout)
                    .id(self.id)
                    .gap(gap)
                    .when_some(self.align_items, |this, align| {
                        this.map(|this| match align {
                            AlignItems::Start => this.items_start(),
                            AlignItems::End => this.items_end(),
                            AlignItems::Center => this.items_center(),
                            AlignItems::Baseline => this.items_baseline(),
                            _ => this,
                        })
                    })
                    // Label
                    .child(
                        wrap_label(label_width)
                            .text_sm()
                            .font_medium()
                            .gap_1()
                            .items_center()
                            .when_some(self.label, |this, label| {
                                this.child(label.render(cx)).when(self.required, |this| {
                                    this.child(div().text_color(cx.theme().danger).child("*"))
                                })
                            }),
                    )
                    .child(div().w_full().child(self.child)),
            )
            .child(
                // Other
                wrap_div(layout)
                    .gap(gap)
                    .when(layout.is_horizontal(), |this| {
                        this.child(
                            // Empty for spacing to align with the input
                            wrap_label(label_width),
                        )
                    })
                    .when(self.description.is_some(), |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(self.description.unwrap().render(cx)),
                        )
                    }),
            )
    }
}

impl RenderOnce for Form {
    fn render(self, _: &mut WindowContext) -> impl IntoElement {
        let props = self.props;

        let gap = match props.size {
            Size::XSmall | Size::Small => px(8.),
            Size::Large => px(16.),
            _ => px(12.),
        };

        v_flex().w_full().gap(gap).children(
            self.fields
                .into_iter()
                .enumerate()
                .map(|(ix, field)| field.props(ix, props)),
        )
    }
}