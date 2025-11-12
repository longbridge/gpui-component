use gpui::{
    px, App, Axis, IntoElement, ParentElement, Pixels, Rems, RenderOnce, StyleRefinement, Styled,
    Window,
};

use crate::{
    form::{Field, FieldProps},
    v_flex, Sizable, Size,
};

/// A form element that contains multiple form fields.
#[derive(IntoElement)]
pub struct Form {
    style: StyleRefinement,
    fields: Vec<Field>,
    props: FieldProps,
}

impl Form {
    fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
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

    /// Add a child to the form.
    pub fn child(mut self, field: impl Into<Field>) -> Self {
        self.fields.push(field.into());
        self
    }

    /// Add multiple children to the form.
    pub fn children(mut self, fields: impl IntoIterator<Item = Field>) -> Self {
        self.fields.extend(fields);
        self
    }

    /// Set the column count for the form.
    ///
    /// Default is 1.
    pub fn columns(mut self, columns: usize) -> Self {
        self.props.columns = columns;
        self
    }
}

impl Styled for Form {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Form {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.props.size = size.into();
        self
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

        v_flex()
            .w_full()
            .gap_x(gap * 3.)
            .gap_y(gap)
            .grid()
            .grid_cols(props.columns as u16)
            .children(
                self.fields
                    .into_iter()
                    .enumerate()
                    .map(|(ix, field)| field.props(ix, props)),
            )
    }
}
