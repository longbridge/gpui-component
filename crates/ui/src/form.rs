use std::rc::{Rc, Weak};

use gpui::{
    div, prelude::FluentBuilder as _, px, AlignItems, AnyElement, AnyView, App, Axis, Div, Element,
    ElementId, FocusHandle, InteractiveElement as _, IntoElement, ParentElement, Pixels, Rems,
    RenderOnce, SharedString, Styled, Window,
};

use crate::{h_flex, v_flex, ActiveTheme as _, AxisExt, Sizable, Size, StyledExt};

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

#[derive(Clone, Copy)]
struct FieldProps {
    size: Size,
    label_width: Option<Pixels>,
    label_text_size: Option<Rems>,
    layout: Axis,
    /// Field gap
    gap: Option<Pixels>,
}

impl Default for FieldProps {
    fn default() -> Self {
        Self {
            label_width: Some(px(140.)),
            label_text_size: None,
            layout: Axis::Vertical,
            size: Size::default(),
            gap: None,
        }
    }
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

    /// Set the text size of the labels in the form. Default is `None`.
    pub fn label_text_size(mut self, size: Rems) -> Self {
        self.props.label_text_size = Some(size);
        self
    }

    /// Set the gap between the form fields.
    pub fn gap(mut self, gap: Pixels) -> Self {
        self.props.gap = Some(gap);
        self
    }

    /// Add a child to the form.
    pub fn child(mut self, field: impl Into<FormField>) -> Self {
        self.fields.push(field.into());
        self
    }

    /// Add multiple children to the form.
    pub fn children(mut self, fields: impl IntoIterator<Item = FormField>) -> Self {
        self.fields.extend(fields);
        self
    }
}

impl Sizable for Form {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.props.size = size.into();
        self
    }
}

pub enum FieldBuilder {
    String(SharedString),
    Element(Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>),
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        match self {
            FieldBuilder::String(value) => value.into_any_element(),
            FieldBuilder::Element(builder) => builder(window, cx),
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

#[derive(IntoElement)]
pub struct FormField {
    id: ElementId,
    form: Weak<Form>,
    label: Option<FieldBuilder>,
    no_label_indent: bool,
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
            no_label_indent: false,
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

    /// Sets not indent with the label width (in Horizontal layout).
    ///
    /// Sometimes you want to align the input form left (Default is align after the label width in Horizontal layout).
    ///
    /// This is only work when the `label` is not set.
    pub fn no_label_indent(mut self) -> Self {
        self.no_label_indent = true;
        self
    }

    /// Sets the label for the form field using a function.
    pub fn label_fn<F, E>(mut self, label: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.label = Some(FieldBuilder::Element(Rc::new(move |window, cx| {
            label(window, cx).into_any_element()
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
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.description = Some(FieldBuilder::Element(Rc::new(move |window, cx| {
            description(window, cx).into_any_element()
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let layout = self.props.layout;

        let label_width = if layout.is_vertical() {
            None
        } else {
            self.props.label_width
        };
        let has_label = !self.no_label_indent;

        #[inline]
        fn wrap_div(layout: Axis) -> Div {
            if layout.is_vertical() {
                v_flex()
            } else {
                h_flex()
            }
        }

        #[inline]
        fn wrap_label(label_width: Option<Pixels>) -> Div {
            div().when_some(label_width, |this, width| this.w(width).flex_shrink_0())
        }

        let gap = match self.props.gap {
            Some(v) => v,
            None => match self.props.size {
                Size::Large => px(8.),
                Size::XSmall | Size::Small => px(4.),
                _ => px(4.),
            },
        };
        let inner_gap = if layout.is_horizontal() {
            gap
        } else {
            gap / 2.
        };

        v_flex()
            .flex_1()
            .gap(gap / 2.)
            .child(
                // This warp for aligning the Label + Input
                wrap_div(layout)
                    .id(self.id)
                    .gap(inner_gap)
                    .when_some(self.align_items, |this, align| {
                        this.map(|this| match align {
                            AlignItems::Start => this.items_start(),
                            AlignItems::End => this.items_end(),
                            AlignItems::Center => this.items_center(),
                            AlignItems::Baseline => this.items_baseline(),
                            _ => this,
                        })
                    })
                    .when(has_label, |this| {
                        // Label
                        this.child(
                            wrap_label(label_width)
                                .text_sm()
                                .when_some(self.props.label_text_size, |this, size| {
                                    this.text_size(size)
                                })
                                .font_medium()
                                .flex()
                                .flex_row()
                                .gap_1()
                                .items_center()
                                .when_some(self.label, |this, builder| {
                                    this.child(builder.render(window, cx)).when(
                                        self.required,
                                        |this| {
                                            this.child(
                                                div().text_color(cx.theme().danger).child("*"),
                                            )
                                        },
                                    )
                                }),
                        )
                    })
                    .child(div().flex_1().overflow_x_hidden().child(self.child)),
            )
            .child(
                // Other
                wrap_div(layout)
                    .gap(inner_gap)
                    .when(has_label && layout.is_horizontal(), |this| {
                        this.child(
                            // Empty for spacing to align with the input
                            wrap_label(label_width),
                        )
                    })
                    .when_some(self.description, |this, builder| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(builder.render(window, cx)),
                        )
                    }),
            )
    }
}
impl RenderOnce for Form {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let props = self.props;

        let gap = match props.size {
            Size::XSmall | Size::Small => px(6.),
            Size::Large => px(12.),
            _ => px(8.),
        };

        v_flex().w_full().gap(gap).children(
            self.fields
                .into_iter()
                .enumerate()
                .map(|(ix, field)| field.props(ix, props)),
        )
    }
}
