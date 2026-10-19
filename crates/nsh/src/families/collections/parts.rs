use super::schema::{CollectionSize as PartSize, TableAlignment as Align};
use gpui_kit::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Window,
    component::{
        ChildElement, Disableable, Selectable, Sizable, Size,
        list::{ListItem, ListSeparatorItem},
        searchable_list::SearchableListItemElement,
        table::{
            TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow,
        },
    },
};
use nsh_core::ui::{Children, Event, NoEvent, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};

// Each native NSH child owns its size and identity. This adapter lets the
// typed native Table containers host a real NSH subtree as a child.
#[derive(IntoElement)]
pub(super) struct Child(pub(super) AnyElement);
impl Sizable for Child {
    fn with_size(self, _: impl Into<Size>) -> Self {
        self
    }
}
impl ChildElement for Child {
    fn with_ix(self, _: usize) -> Self {
        self
    }
}
impl RenderOnce for Child {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.0
    }
}

#[derive(Clone, PartialEq, Props)]
pub(super) struct TablePartProps {
    children: Children,
    #[prop(default = 1)]
    col_span: usize,
    #[prop(default)]
    align: Align,
    #[prop(default)]
    size: PartSize,
}
pub(super) struct TablePart<const PART: u8> {
    context: TypedComponentContext<NoEvent>,
}
impl<const PART: u8> TablePart<PART> {
    pub(super) fn new(context: TypedComponentContext<NoEvent>) -> Self {
        Self { context }
    }
}
impl<const PART: u8> TypedNativeComponent for TablePart<PART> {
    type Props = TablePartProps;
    type Event = NoEvent;
    fn render(&mut self, p: &TablePartProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let children = crate::bridge::children(&self.context);
        let size = p.size.resolve();
        Some(match PART {
            0 => TableHeader::new()
                .with_size(size)
                .children(children.into_iter().map(Child))
                .into_any_element(),
            1 => TableBody::new()
                .with_size(size)
                .children(children.into_iter().map(Child))
                .into_any_element(),
            2 => TableFooter::new()
                .with_size(size)
                .children(children.into_iter().map(Child))
                .into_any_element(),
            3 => TableRow::new()
                .with_size(size)
                .children(children.into_iter().map(Child))
                .into_any_element(),
            4 => {
                let head = TableHead::new()
                    .col_span(p.col_span.max(1))
                    .with_size(size)
                    .children(children);
                match p.align {
                    Align::Center => head.text_center(),
                    Align::Right => head.text_right(),
                    Align::Left => head,
                }
                .into_any_element()
            }
            5 => {
                let cell = TableCell::new()
                    .col_span(p.col_span.max(1))
                    .with_size(size)
                    .children(children);
                match p.align {
                    Align::Center => cell.text_center(),
                    Align::Right => cell.text_right(),
                    Align::Left => cell,
                }
                .into_any_element()
            }
            _ => TableCaption::new()
                .with_size(size)
                .children(children)
                .into_any_element(),
        })
    }
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct ListItemProps {
    children: Children,
    label: Option<String>,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    selected: bool,
}
#[derive(Clone, Event)]
pub(super) enum ListItemEvent {
    Press,
}
pub(super) struct NativeListItem {
    context: TypedComponentContext<ListItemEvent>,
}
impl NativeListItem {
    pub(super) fn new(context: TypedComponentContext<ListItemEvent>) -> Self {
        Self { context }
    }
}
impl TypedNativeComponent for NativeListItem {
    type Props = ListItemProps;
    type Event = ListItemEvent;
    fn render(&mut self, p: &ListItemProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let children = crate::bridge::children(&self.context);
        let label: SharedString = p.label.clone().unwrap_or_default().into();
        let context = self.context.clone();
        Some(
            ListItem::new("item")
                .disabled(p.disabled)
                .selected(p.selected)
                .accessibility_label(label.clone())
                .children(if children.is_empty() {
                    vec![label.into_any_element()]
                } else {
                    children
                })
                .on_click(move |_, _, _| context.emit(ListItemEvent::Press))
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct SeparatorProps {
    children: Children,
}
pub(super) struct NativeSeparator {
    context: TypedComponentContext<NoEvent>,
}
impl NativeSeparator {
    pub(super) fn new(context: TypedComponentContext<NoEvent>) -> Self {
        Self { context }
    }
}
impl TypedNativeComponent for NativeSeparator {
    type Props = SeparatorProps;
    type Event = NoEvent;
    fn render(&mut self, _: &SeparatorProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            ListSeparatorItem::new()
                .children(crate::bridge::children(&self.context))
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, Props)]
pub(super) struct SearchableItemProps {
    children: Children,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    selected: bool,
    #[prop(default)]
    checked: bool,
    #[prop(default)]
    size: PartSize,
}
pub(super) struct NativeSearchableItem {
    context: TypedComponentContext<NoEvent>,
}
impl NativeSearchableItem {
    pub(super) fn new(context: TypedComponentContext<NoEvent>) -> Self {
        Self { context }
    }
}
impl TypedNativeComponent for NativeSearchableItem {
    type Props = SearchableItemProps;
    type Event = NoEvent;
    fn render(
        &mut self,
        p: &SearchableItemProps,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        Some(
            SearchableListItemElement::new(0)
                .disabled(p.disabled)
                .selected(p.selected)
                .checked(p.checked)
                .with_size(p.size.resolve())
                .children(crate::bridge::children(&self.context))
                .into_any_element(),
        )
    }
}
