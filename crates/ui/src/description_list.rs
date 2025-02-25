use gpui::{
    div, prelude::FluentBuilder as _, px, relative, AnyElement, Axis, DefiniteLength, Empty,
    IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window,
};

use crate::{h_flex, text::Text, v_flex, ActiveTheme as _, AxisExt, Sizable, Size};

/// A description list.
#[derive(IntoElement)]
pub struct DescriptionList {
    items: Vec<DescriptionItem>,
    size: Size,
    layout: Axis,
    label_width: DefiniteLength,
    bordered: bool,
    columns: usize,
}

/// Description item.
pub enum DescriptionItem {
    Item {
        label: Text,
        value: AnyElement,
        span: usize,
    },
    Divider,
}

impl DescriptionItem {
    /// Create a new description item, with a label.
    ///
    /// The value is an empty element.
    pub fn new(label: impl Into<Text>) -> Self {
        DescriptionItem::Item {
            label: label.into(),
            value: Empty.into_any_element(),
            span: 1,
        }
    }

    /// Set the string value of the item.
    ///
    /// This method only works for [`DescriptionItem::Item`].
    pub fn value(mut self, value: impl Into<SharedString>) -> Self {
        let new_value: SharedString = value.into();
        if let DescriptionItem::Item { value, .. } = &mut self {
            *value = new_value.into_any_element();
        }
        self
    }

    /// Set the element value of the item.
    pub fn child(mut self, value: impl Into<AnyElement>) -> Self {
        let new_value: AnyElement = value.into();
        if let DescriptionItem::Item { value, .. } = &mut self {
            *value = new_value;
        }
        self
    }

    /// Set the span of the item.
    ///
    /// This method only works for [`DescriptionItem::Item`].
    pub fn span(mut self, span: usize) -> Self {
        let val = span;
        if let DescriptionItem::Item { span, .. } = &mut self {
            *span = val;
        }
        self
    }

    fn _label(&self) -> Option<&Text> {
        match self {
            DescriptionItem::Item { label, .. } => Some(label),
            _ => None,
        }
    }

    fn _span(&self) -> Option<usize> {
        match self {
            DescriptionItem::Item { span, .. } => Some(*span),
            _ => None,
        }
    }
}

impl DescriptionList {
    /// Create a new description list with the default layout (Horizontal).
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            layout: Axis::Horizontal,
            label_width: px(120.).into(),
            size: Size::default(),
            bordered: true,
            columns: 3,
        }
    }

    /// Create a vertical description list.
    pub fn vertical() -> Self {
        Self::new().layout(Axis::Vertical)
    }

    /// Create a horizontal description list, the default.
    pub fn horizontal() -> Self {
        Self::new().layout(Axis::Horizontal)
    }

    /// Set the width of the label, only works for horizontal layout.
    ///
    /// Default is `120px`.
    pub fn label_width(mut self, label_width: DefiniteLength) -> Self {
        self.label_width = label_width;
        self
    }

    /// Set the layout of the description list.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// Set the border of the description list, default is `true`.
    ///
    /// `Horizontal` layout only.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set the number of columns in the description list, default is `3`.
    ///
    /// A value between `1` and `10` is allowed.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.clamp(1, 10);
        self
    }

    /// Add a [`DescriptionItem::Item`] to the list.
    pub fn child(
        mut self,
        label: impl Into<Text>,
        value: impl Into<AnyElement>,
        span: usize,
    ) -> Self {
        self.items.push(DescriptionItem::Item {
            label: label.into(),
            value: value.into(),
            span,
        });
        self
    }

    /// Add children to the list.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<DescriptionItem>>,
    ) -> Self {
        self.items
            .extend(children.into_iter().map(Into::into).collect::<Vec<_>>());
        self
    }

    /// Add a divider to the list.
    pub fn divider(mut self) -> Self {
        self.items.push(DescriptionItem::Divider);
        self
    }

    fn group_item_rows(items: Vec<DescriptionItem>, columns: usize) -> Vec<Vec<DescriptionItem>> {
        let mut rows = vec![];
        let mut current_span = 0;
        for item in items.into_iter() {
            let span = item._span().unwrap_or(columns);
            if rows.is_empty() {
                rows.push(vec![]);
            }
            if current_span + span > columns {
                rows.push(vec![]);
                current_span = 0;
            }
            let last_group = rows.last_mut().unwrap();
            last_group.push(item);
            current_span += span;
        }
        // Remove last empty rows if it exists
        while let Some(last_group) = rows.last() {
            if !last_group.is_empty() {
                break;
            }

            rows.pop();
        }

        rows
    }
}

impl Sizable for DescriptionList {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for DescriptionList {
    fn render(self, _: &mut Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let gap = if self.layout.is_vertical() {
            match self.size {
                Size::XSmall | Size::Small => px(2.),
                Size::Medium => px(4.),
                Size::Large => px(8.),
                _ => px(4.),
            }
        } else {
            px(0.)
        };
        let (mut padding_x, padding_y) = match self.size {
            Size::XSmall | Size::Small => (px(4.), px(2.)),
            Size::Medium => (px(8.), px(4.)),
            Size::Large => (px(12.), px(6.)),
            _ => (px(8.), px(4.)),
        };
        if !self.layout.is_horizontal() {
            padding_x = px(0.);
        }

        let label_width = if self.layout.is_horizontal() {
            Some(self.label_width)
        } else {
            None
        };
        let bordered = self.bordered && self.layout.is_horizontal();

        let items_len = self.items.len();
        let columns = self.columns;

        // Group items by columns
        let rows = Self::group_item_rows(self.items, self.columns);

        v_flex()
            .gap(gap)
            .when(bordered, |this| {
                this.rounded(padding_x)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .children(rows.into_iter().enumerate().map(|(ix, items)| {
                let is_last = ix == items_len - 1;
                h_flex()
                    .when(bordered && !is_last, |this| {
                        this.border_b_1().border_color(cx.theme().border)
                    })
                    .children({
                        items.into_iter().map(|item| {
                            let item_width = if let Some(span) = item._span() {
                                relative(span as f32 / columns as f32)
                            } else {
                                relative(1.)
                            };

                            match item {
                                DescriptionItem::Item { label, value, .. } => {
                                    let el = if self.layout.is_vertical() {
                                        v_flex()
                                    } else {
                                        h_flex()
                                    };

                                    el.h_full()
                                        .w(item_width)
                                        .child(
                                            div()
                                                .text_color(
                                                    cx.theme().description_list_label_foreground,
                                                )
                                                .text_sm()
                                                .h_full()
                                                .map(|this| match label_width {
                                                    Some(label_width) => this
                                                        .w(label_width)
                                                        .px(padding_x)
                                                        .py(padding_y)
                                                        .flex_shrink_0()
                                                        .when(bordered, |this| {
                                                            this.border_r_1()
                                                                .border_color(cx.theme().border)
                                                                .bg(cx
                                                                    .theme()
                                                                    .description_list_label)
                                                        }),
                                                    None => this.flex_1(),
                                                })
                                                .child(label.clone()),
                                        )
                                        .child(
                                            div().flex_1().px(padding_x).py(padding_y).child(value),
                                        )
                                }
                                _ => div().h(px(2.)),
                            }
                        })
                    })
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::DescriptionItem;

    #[test]
    fn test_group_item_rows() {
        let items = vec![
            DescriptionItem::new("test1"),
            DescriptionItem::new("test2").span(2),
            DescriptionItem::new("test3"),
            DescriptionItem::new("test4"),
            DescriptionItem::new("test5"),
            DescriptionItem::new("test6").span(3),
            DescriptionItem::new("test7"),
        ];
        let rows = super::DescriptionList::group_item_rows(items, 3);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].len(), 2);
        assert_eq!(rows[0][0]._label(), Some("test1"));
        assert_eq!(rows[0][1]._label(), Some("test2"));
        assert_eq!(rows[1].len(), 3);
        assert_eq!(rows[1][0]._label(), Some("test3"));
        assert_eq!(rows[1][1]._label(), Some("test4"));
        assert_eq!(rows[1][2]._label(), Some("test5"));
        assert_eq!(rows[2].len(), 1);
        assert_eq!(rows[2][0]._label(), Some("test6"));
        assert_eq!(rows[3].len(), 1);
        assert_eq!(rows[3][0]._label(), Some("test7"));
    }
}
