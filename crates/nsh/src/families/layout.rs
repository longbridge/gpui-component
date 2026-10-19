//! Structural controls with NSH-owned content and controlled selection.
use crate::bridge::{children, slot};
use crate::registration::native_leaf;
use gpui_kit::component::{
    Side,
    accordion::Accordion,
    breadcrumb::{Breadcrumb, BreadcrumbItem},
    collapsible::Collapsible,
    description_list::{DescriptionItem, DescriptionList},
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    },
    form::{Field, Form},
    group_box::{GroupBox, GroupBoxVariant, GroupBoxVariants},
    setting::{
        SettingGroup as ComponentSettingGroup, SettingItem, SettingPage as ComponentSettingPage,
        Settings,
    },
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarFooter, SidebarHeader, SidebarMenu, SidebarMenuItem,
    },
    status_bar::StatusBar,
    tab::{Tab, TabBar, TabVariant},
    toolbar::{Toolbar, ToolbarGroup},
};
use gpui_kit::{
    AnyElement, App, Axis, FocusHandle, InteractiveElement, IntoElement, ParentElement, Styled,
    Window, div, prelude::FluentBuilder,
};
use nsh_core::ui::{Children, Event, JsObject, PropEnum, PropError, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum LayoutAxis {
    Vertical,
    Horizontal,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum SidebarSide {
    Left,
    Right,
}
impl SidebarSide {
    fn native(self) -> Side {
        match self {
            Self::Left => Side::Left,
            Self::Right => Side::Right,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum GroupBoxStyle {
    Normal,
    Fill,
    Outline,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum EmptyMediaStyle {
    Default,
    Icon,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum NativeTabVariant {
    Pill,
    Segmented,
    Outline,
    Underline,
    Folder,
}
impl NativeTabVariant {
    fn native(self) -> TabVariant {
        match self {
            Self::Pill => TabVariant::Pill,
            Self::Segmented => TabVariant::Segmented,
            Self::Outline => TabVariant::Outline,
            Self::Underline => TabVariant::Underline,
            Self::Folder => TabVariant::Folder,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum SidebarCollapse {
    Icon,
    Offcanvas,
    None,
}
impl SidebarCollapse {
    fn native(self) -> SidebarCollapsible {
        match self {
            Self::Icon => SidebarCollapsible::Icon,
            Self::Offcanvas => SidebarCollapsible::Offcanvas,
            Self::None => SidebarCollapsible::None,
        }
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct LayoutItem {
    id: String,
    label: String,
    #[prop(default)]
    disabled: bool,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SidebarItem {
    id: String,
    label: String,
    #[prop(default)]
    disabled: bool,
    open: Option<bool>,
    #[prop(default)]
    items: Vec<SidebarItem>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SidebarOpenChange {
    id: String,
    open: bool,
}
#[derive(Clone, Event)]
pub(crate) enum SelectionEvent {
    Change(String),
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
#[derive(Clone, Event)]
pub(crate) enum AccordionEvent {
    Change(Vec<String>),
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
#[derive(Clone, Event)]
pub(crate) enum SidebarEvent {
    Change(String),
    OpenChange(SidebarOpenChange),
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
#[derive(Clone, Event)]
pub(crate) enum LayoutError {
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
#[derive(Default)]
struct NavigationFocus {
    handles: BTreeMap<String, FocusHandle>,
    active: Vec<FocusHandle>,
}
impl NavigationFocus {
    fn update(&mut self, items: impl IntoIterator<Item = (String, bool)>, cx: &mut App) {
        let items: Vec<_> = items.into_iter().collect();
        self.handles
            .retain(|id, _| items.iter().any(|(key, _)| key == id));
        self.active.clear();
        for (id, enabled) in items {
            let handle = self
                .handles
                .entry(id)
                .or_insert_with(|| cx.focus_handle().tab_stop(true));
            if enabled {
                self.active.push(handle.clone());
            }
        }
    }
}
fn validate_ids<'a>(ids: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.is_empty() {
            return Err("item ids must be nonempty strings".into());
        }
        if !seen.insert(id) {
            return Err("item ids must be unique".into());
        }
    }
    Ok(())
}
fn content(context: &nsh_platform_gpui::ComponentContext, name: &str) -> AnyElement {
    slot(context, name).unwrap_or_else(|| div().into_any_element())
}
native_leaf!(NativeCollapsible(CollapsibleProps, context, cx, p) =>
    Collapsible::new()
        .motion_id("reveal")
        .open(p.open)
        .children(children(context))
        .content(content(context, "content"));
{
    open: bool => skip;
    children: Children => skip;
});
native_leaf!(NativeSidebarHeader(SidebarHeaderProps, context, cx, p) =>
    SidebarHeader::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeSidebarFooter(SidebarFooterProps, context, cx, p) =>
    SidebarFooter::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeStatusBar(StatusBarProps, context, cx, p) =>
    StatusBar::new()
        .when_some(slot(context, "left"), |this, child| this.left(child))
        .when_some(slot(context, "right"), |this, child| this.right(child))
        .children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeToolbar(ToolbarProps, context, cx, p) =>
    Toolbar::new("toolbar")
        .disabled(p.disabled)
        .contents(children(context));
{
    disabled: bool => skip;
    children: Children => skip;
});
native_leaf!(NativeToolbarGroup(ToolbarGroupProps, context, cx, p) =>
    ToolbarGroup::new("toolbar-group")
        .label(p.label.clone().unwrap_or_default())
        .content(div().children(children(context)));
{
    label: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeField(FieldProps, context, cx, p) =>
    Field::new()
        .label(p.label.clone().unwrap_or_default())
        .description(p.description.clone().unwrap_or_default())
        .required(p.required)
        .visible(p.visible.unwrap_or(true))
        .children(children(context));
{
    label: Option<String> => skip;
    description: Option<String> => skip;
    required: bool => skip;
    visible: Option<bool> => skip;
    children: Children => skip;
});
native_leaf!(NativeGroupBox(GroupBoxProps, context, cx, p) =>
    GroupBox::new()
        .id("group")
        .with_variant(match p.variant {
            Some(GroupBoxStyle::Fill) => GroupBoxVariant::Fill,
            Some(GroupBoxStyle::Outline) => GroupBoxVariant::Outline,
            _ => GroupBoxVariant::Normal,
        })
        .when_some(
            slot(context, "title").or_else(|| p.title.clone().map(IntoElement::into_any_element)),
            |this, title| this.title(title),
        )
        .when_some(slot(context, "footer"), |this, footer| this.footer(footer))
        .children(children(context));
{
    title: Option<String> => skip;
    variant: Option<GroupBoxStyle> => skip;
    children: Children => skip;
});
native_leaf!(NativeEmptyContent(EmptyContentProps, context, cx, p) =>
    EmptyContent::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeEmptyTitle(EmptyTitleProps, context, cx, p) =>
    EmptyTitle::new()
        .children(children(context))
        .child(p.text.clone().unwrap_or_default());
{
    text: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeEmptyDescription(EmptyDescriptionProps, context, cx, p) =>
    EmptyDescription::new()
        .children(children(context))
        .child(p.text.clone().unwrap_or_default());
{
    text: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeEmptyMedia(EmptyMediaProps, context, cx, p) =>
    EmptyMedia::new()
        .with_variant(match p.variant {
            Some(EmptyMediaStyle::Icon) => EmptyMediaVariant::Icon,
            _ => EmptyMediaVariant::Default,
        })
        .children(children(context));
{
    variant: Option<EmptyMediaStyle> => skip;
    children: Children => skip;
});
native_leaf!(NativeEmptyHeader(EmptyHeaderProps, context, cx, p) =>
    empty_header(context, p.title.as_deref(), p.description.as_deref());
{
    title: Option<String> => skip;
    description: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeEmpty(EmptyProps, context, cx, p) =>
    Empty::new()
        .header(empty_header(
            context,
            p.title.as_deref(),
            p.description.as_deref(),
        ))
        .content(EmptyContent::new().children(children(context)));
{
    title: Option<String> => skip;
    description: Option<String> => skip;
    children: Children => skip;
});

fn empty_header(
    context: &nsh_platform_gpui::ComponentContext,
    title: Option<&str>,
    description: Option<&str>,
) -> EmptyHeader {
    EmptyHeader::new()
        .title(EmptyTitle::new().child(title.unwrap_or_default().to_owned()))
        .description(EmptyDescription::new().child(description.unwrap_or_default().to_owned()))
        .when_some(slot(context, "media"), |this, media| {
            this.media(EmptyMedia::new().child(media))
        })
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct TabsProps {
    items: Vec<LayoutItem>,
    value: String,
    variant: Option<NativeTabVariant>,
    #[prop(default)]
    menu: bool,
    children: Children,
}
struct NativeTabs<const CONTENT: bool> {
    context: TypedComponentContext<SelectionEvent>,
    focus: NavigationFocus,
}
impl<const CONTENT: bool> TypedNativeComponent for NativeTabs<CONTENT> {
    type Props = TabsProps;
    type Event = SelectionEvent;
    fn validate_props(&self, p: &TabsProps) -> Result<(), PropError> {
        validate_ids(p.items.iter().map(|item| item.id.as_str())).map_err(PropError)
    }
    fn render(&mut self, p: &TabsProps, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.focus.update(
            p.items.iter().map(|item| (item.id.clone(), !item.disabled)),
            cx,
        );
        let ids = p
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let index = ids
            .iter()
            .position(|id| *id == p.value)
            .unwrap_or(usize::MAX);
        let context = self.context.clone();
        let bar = TabBar::new("tabs")
            .with_variant(p.variant.unwrap_or(NativeTabVariant::Underline).native())
            .selected_index(index)
            .menu(p.menu)
            .children(p.items.iter().map(|item| {
                Tab::new()
                    .track_focus(&self.focus.handles[&item.id])
                    .label(item.label.clone())
                    .disabled(item.disabled)
            }))
            .when_some(slot(&context, "prefix"), |this, child| this.prefix(child))
            .when_some(slot(&context, "suffix"), |this, child| this.suffix(child))
            .on_click(move |index, _, _| {
                if let Some(id) = ids.get(*index) {
                    context.emit(SelectionEvent::Change(id.clone()));
                }
            });
        Some(if CONTENT {
            div()
                .child(bar)
                .child(content(&self.context, &p.value))
                .into_any_element()
        } else {
            bar.into_any_element()
        })
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.active.first().cloned()
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus.active.clone()
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct AccordionProps {
    items: Vec<LayoutItem>,
    value: Vec<String>,
    #[prop(default)]
    multiple: bool,
    bordered: Option<bool>,
    #[prop(default)]
    disabled: bool,
    children: Children,
}
struct NativeAccordion {
    context: TypedComponentContext<AccordionEvent>,
    focus: NavigationFocus,
}
impl TypedNativeComponent for NativeAccordion {
    type Props = AccordionProps;
    type Event = AccordionEvent;
    fn validate_props(&self, p: &AccordionProps) -> Result<(), PropError> {
        validate_ids(p.items.iter().map(|item| item.id.as_str())).map_err(PropError)
    }
    fn render(&mut self, p: &AccordionProps, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        self.focus.update(
            p.items
                .iter()
                .map(|item| (item.id.clone(), !item.disabled && !p.disabled)),
            cx,
        );
        let ids = p
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        let context = self.context.clone();
        Some(
            p.items
                .iter()
                .fold(
                    Accordion::new("accordion")
                        .multiple(p.multiple)
                        .bordered(p.bordered.unwrap_or(true))
                        .disabled(p.disabled),
                    |accordion, item| {
                        accordion.item(|part| {
                            part.track_focus(&self.focus.handles[&item.id])
                                .accessibility_label(item.label.clone())
                                .title(item.label.clone())
                                .open(p.value.contains(&item.id))
                                .disabled(item.disabled)
                                .child(content(&self.context, &item.id))
                        })
                    },
                )
                .on_toggle_click(move |indices, _, _| {
                    let mut indices = indices.to_vec();
                    indices.sort_unstable();
                    context.emit(AccordionEvent::Change(
                        indices
                            .iter()
                            .filter_map(|index| ids.get(*index).cloned())
                            .collect(),
                    ));
                })
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.active.first().cloned()
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus.active.clone()
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct SidebarProps {
    items: Vec<SidebarItem>,
    value: String,
    #[prop(default)]
    collapsed: bool,
    side: Option<SidebarSide>,
    collapsible: Option<SidebarCollapse>,
    children: Children,
}
struct NativeSidebar {
    context: TypedComponentContext<SidebarEvent>,
    focus: NavigationFocus,
    open_states: Rc<RefCell<BTreeMap<String, bool>>>,
}
fn sidebar_entries(
    items: &[SidebarItem],
    visible: bool,
    states: &BTreeMap<String, bool>,
    entries: &mut Vec<(String, bool)>,
) {
    for item in items {
        entries.push((item.id.clone(), visible && !item.disabled));
        let open = item
            .open
            .unwrap_or_else(|| states.get(&item.id).copied().unwrap_or(false));
        sidebar_entries(&item.items, visible && open, states, entries);
    }
}
impl TypedNativeComponent for NativeSidebar {
    type Props = SidebarProps;
    type Event = SidebarEvent;
    fn validate_props(&self, p: &SidebarProps) -> Result<(), PropError> {
        let mut entries = vec![];
        sidebar_entries(&p.items, true, &self.open_states.borrow(), &mut entries);
        validate_ids(entries.iter().map(|(id, _)| id.as_str())).map_err(PropError)
    }
    fn render(&mut self, p: &SidebarProps, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let mut entries = vec![];
        sidebar_entries(&p.items, true, &self.open_states.borrow(), &mut entries);
        if p.collapsed {
            for (id, visible) in &mut entries {
                *visible = p.collapsible != Some(SidebarCollapse::Offcanvas)
                    && p.items.iter().any(|item| item.id == *id && !item.disabled);
            }
        }
        self.open_states
            .borrow_mut()
            .retain(|id, _| entries.iter().any(|(key, _)| key == id));
        self.focus.update(entries, cx);
        let sidebar = Sidebar::new("sidebar")
            .side(p.side.unwrap_or(SidebarSide::Left).native())
            .collapsible(p.collapsible.unwrap_or(SidebarCollapse::Icon).native())
            .collapsed(p.collapsed)
            .when_some(slot(&self.context, "header"), |this, child| {
                this.header(child)
            })
            .when_some(slot(&self.context, "footer"), |this, child| {
                this.footer(child)
            })
            .child(SidebarMenu::new().children(p.items.iter().map(|item| {
                sidebar_item(
                    item,
                    &self.context,
                    &p.value,
                    &self.focus.handles,
                    &self.open_states,
                )
            })));
        let handles = self.focus.active.clone();
        Some(
            div()
                .size_full()
                .child(sidebar)
                .on_key_down(move |event, window, cx| {
                    let Some(current) = handles.iter().position(|handle| handle.is_focused(window))
                    else {
                        return;
                    };
                    let next = match event.keystroke.key.as_str() {
                        "down" => (current + 1) % handles.len(),
                        "up" => (current + handles.len() - 1) % handles.len(),
                        "home" => 0,
                        "end" => handles.len() - 1,
                        _ => return,
                    };
                    window.focus(&handles[next], cx);
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.active.first().cloned()
    }
    fn focus_handles(&self) -> Vec<FocusHandle> {
        self.focus.active.clone()
    }
}
fn sidebar_item(
    item: &SidebarItem,
    context: &TypedComponentContext<SidebarEvent>,
    selected: &str,
    handles: &BTreeMap<String, FocusHandle>,
    states: &Rc<RefCell<BTreeMap<String, bool>>>,
) -> SidebarMenuItem {
    let id = item.id.clone();
    let event_context = context.clone();
    let open_context = context.clone();
    let open_id = id.clone();
    let states_copy = states.clone();
    SidebarMenuItem::new(item.label.clone())
        .track_focus(&handles[&id])
        .active(id == selected)
        .disable(item.disabled)
        .default_open(item.open.unwrap_or(false))
        .when_some(item.open, |this, open| this.open(open))
        .on_open_change(move |open, _, _| {
            states_copy.borrow_mut().insert(open_id.clone(), open);
            open_context.emit(SidebarEvent::OpenChange(SidebarOpenChange {
                id: open_id.clone(),
                open,
            }));
            open_context.redraw();
        })
        .children(
            item.items
                .iter()
                .map(|child| sidebar_item(child, context, selected, handles, states)),
        )
        .on_click(move |_, _, _| event_context.emit(SidebarEvent::Change(id.clone())))
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct BreadcrumbProps {
    items: Vec<LayoutItem>,
    children: Children,
}
struct NativeBreadcrumb {
    context: TypedComponentContext<SelectionEvent>,
}
impl TypedNativeComponent for NativeBreadcrumb {
    type Props = BreadcrumbProps;
    type Event = SelectionEvent;
    fn validate_props(&self, p: &BreadcrumbProps) -> Result<(), PropError> {
        validate_ids(p.items.iter().map(|item| item.id.as_str())).map_err(PropError)
    }
    fn render(&mut self, p: &BreadcrumbProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            Breadcrumb::new()
                .children(p.items.iter().map(|item| {
                    let context = self.context.clone();
                    let id = item.id.clone();
                    BreadcrumbItem::new(item.label.clone())
                        .disabled(item.disabled)
                        .on_click(move |_, _, _| context.emit(SelectionEvent::Change(id.clone())))
                }))
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct SidebarToggleProps {
    #[prop(default)]
    collapsed: bool,
    side: Option<SidebarSide>,
    children: Children,
}
#[derive(Clone, Event)]
pub(crate) enum SidebarToggleEvent {
    Change(bool),
}
struct NativeSidebarToggle {
    context: TypedComponentContext<SidebarToggleEvent>,
}
impl TypedNativeComponent for NativeSidebarToggle {
    type Props = SidebarToggleProps;
    type Event = SidebarToggleEvent;
    fn render(
        &mut self,
        p: &SidebarToggleProps,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        let context = self.context.clone();
        let next = !p.collapsed;
        Some(
            gpui_kit::component::sidebar::SidebarToggleButton::new()
                .side(p.side.unwrap_or(SidebarSide::Left).native())
                .collapsed(p.collapsed)
                .on_click(move |_, _, _| context.emit(SidebarToggleEvent::Change(next)))
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct FormField {
    id: String,
    label: String,
    description: Option<String>,
    #[prop(default)]
    required: bool,
    #[prop(default)]
    hidden: bool,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct FormProps {
    items: Vec<FormField>,
    layout: Option<LayoutAxis>,
    columns: Option<usize>,
    children: Children,
}
struct NativeForm {
    context: TypedComponentContext<LayoutError>,
}
impl TypedNativeComponent for NativeForm {
    type Props = FormProps;
    type Event = LayoutError;
    fn validate_props(&self, p: &FormProps) -> Result<(), PropError> {
        validate_ids(p.items.iter().map(|item| item.id.as_str())).map_err(PropError)
    }
    fn render(&mut self, p: &FormProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        Some(
            Form::new()
                .layout(if p.layout == Some(LayoutAxis::Horizontal) {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                })
                .columns(p.columns.unwrap_or(1).max(1))
                .children(p.items.iter().map(|item| {
                    Field::new()
                        .label(item.label.clone())
                        .description(item.description.clone().unwrap_or_default())
                        .required(item.required)
                        .visible(!item.hidden)
                        .child(content(&self.context, &item.id))
                }))
                .when_some(slot(&self.context, "footer"), |this, footer| {
                    this.footer(footer)
                })
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct DescriptionEntry {
    #[prop(default)]
    id: String,
    #[prop(default)]
    label: String,
    value: Option<String>,
    span: Option<usize>,
    #[prop(default)]
    separator: bool,
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct DescriptionListProps {
    items: Vec<DescriptionEntry>,
    layout: Option<LayoutAxis>,
    columns: Option<usize>,
    bordered: Option<bool>,
    children: Children,
}
struct NativeDescriptionList {
    context: TypedComponentContext<LayoutError>,
}
impl TypedNativeComponent for NativeDescriptionList {
    type Props = DescriptionListProps;
    type Event = LayoutError;
    fn validate_props(&self, p: &DescriptionListProps) -> Result<(), PropError> {
        validate_ids(
            p.items
                .iter()
                .filter(|item| !item.separator)
                .map(|item| item.id.as_str()),
        )
        .map_err(PropError)
    }
    fn render(
        &mut self,
        p: &DescriptionListProps,
        _: &mut Window,
        _: &mut App,
    ) -> Option<AnyElement> {
        Some(
            DescriptionList::new()
                .columns(p.columns.unwrap_or(1).max(1))
                .bordered(p.bordered.unwrap_or(true))
                .layout(if p.layout == Some(LayoutAxis::Vertical) {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                })
                .children(p.items.iter().map(|item| {
                    if item.separator {
                        DescriptionItem::Separator
                    } else {
                        DescriptionItem::new(item.label.clone())
                            .value(slot(&self.context, &item.id).unwrap_or_else(|| {
                                item.value.clone().unwrap_or_default().into_any_element()
                            }))
                            .span(item.span.unwrap_or(1).max(1))
                    }
                }))
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SettingEntry {
    id: String,
    #[prop(default)]
    disabled: bool,
    #[prop(default)]
    keywords: Vec<String>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SettingGroup {
    label: Option<String>,
    description: Option<String>,
    items: Vec<SettingEntry>,
}
#[derive(Clone, PartialEq, JsObject)]
pub(crate) struct SettingPage {
    label: String,
    description: Option<String>,
    groups: Vec<SettingGroup>,
}
native_leaf!(NativeSettings(SettingsProps,context,cx,p)=>Settings::new("settings").pages(p.items.iter().map(|page|{
    ComponentSettingPage::new(page.label.clone()).description(page.description.clone().unwrap_or_default()).groups(page.groups.iter().map(|group|{
        ComponentSettingGroup::new().title(group.label.clone().unwrap_or_default()).description(group.description.clone().unwrap_or_default()).items(group.items.iter().map(|item|{
            let id=item.id.clone();let context=(*context).clone();SettingItem::render(move|_,_,_|content(context.raw(),&id)).disabled(item.disabled).keywords(item.keywords.clone())
        }))
    }))
}));{items:Vec<SettingPage> => skip;children:Children => skip;});

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeCollapsible>(
        "gpui-component.Collapsible",
        "Collapsible",
        NativeCollapsible::new,
    );
    registry.register::<NativeSidebarHeader>(
        "gpui-component.SidebarHeader",
        "SidebarHeader",
        NativeSidebarHeader::new,
    );
    registry.register::<NativeSidebarFooter>(
        "gpui-component.SidebarFooter",
        "SidebarFooter",
        NativeSidebarFooter::new,
    );
    registry.register::<NativeStatusBar>(
        "gpui-component.StatusBar",
        "StatusBar",
        NativeStatusBar::new,
    );
    registry.register::<NativeToolbar>("gpui-component.Toolbar", "Toolbar", NativeToolbar::new);
    registry.register::<NativeToolbarGroup>(
        "gpui-component.ToolbarGroup",
        "ToolbarGroup",
        NativeToolbarGroup::new,
    );
    registry.register::<NativeField>("gpui-component.Field", "Field", NativeField::new);
    registry.register::<NativeGroupBox>("gpui-component.GroupBox", "GroupBox", NativeGroupBox::new);
    registry.register::<NativeEmptyContent>(
        "gpui-component.EmptyContent",
        "EmptyContent",
        NativeEmptyContent::new,
    );
    registry.register::<NativeEmptyTitle>(
        "gpui-component.EmptyTitle",
        "EmptyTitle",
        NativeEmptyTitle::new,
    );
    registry.register::<NativeEmptyDescription>(
        "gpui-component.EmptyDescription",
        "EmptyDescription",
        NativeEmptyDescription::new,
    );
    registry.register::<NativeEmptyMedia>(
        "gpui-component.EmptyMedia",
        "EmptyMedia",
        NativeEmptyMedia::new,
    );
    registry.register::<NativeEmptyHeader>(
        "gpui-component.EmptyHeader",
        "EmptyHeader",
        NativeEmptyHeader::new,
    );
    registry.register::<NativeEmpty>("gpui-component.Empty", "Empty", NativeEmpty::new);
    registry.register::<NativeSettings>("gpui-component.Settings", "Settings", NativeSettings::new);
    registry.register::<NativeTabs<false>>("gpui-component.TabBar", "TabBar", |context| {
        NativeTabs {
            context,
            focus: NavigationFocus::default(),
        }
    });
    registry.register::<NativeTabs<true>>("gpui-component.Tabs", "Tabs", |context| NativeTabs {
        context,
        focus: NavigationFocus::default(),
    });
    registry.register::<NativeAccordion>("gpui-component.Accordion", "Accordion", |context| {
        NativeAccordion {
            context,
            focus: NavigationFocus::default(),
        }
    });
    registry.register::<NativeSidebar>("gpui-component.Sidebar", "Sidebar", |context| {
        NativeSidebar {
            context,
            focus: NavigationFocus::default(),
            open_states: Rc::new(RefCell::new(BTreeMap::new())),
        }
    });
    registry.register::<NativeBreadcrumb>("gpui-component.Breadcrumb", "Breadcrumb", |context| {
        NativeBreadcrumb { context }
    });
    registry.register::<NativeForm>("gpui-component.Form", "Form", |context| NativeForm {
        context,
    });
    registry.register::<NativeDescriptionList>(
        "gpui-component.DescriptionList",
        "DescriptionList",
        |context| NativeDescriptionList { context },
    );
    registry.register::<NativeSidebarToggle>(
        "gpui-component.SidebarToggleButton",
        "SidebarToggleButton",
        |context| NativeSidebarToggle { context },
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_duplicate_nested_navigation_identity() {
        let items = vec![SidebarItem {
            id: "root".into(),
            label: "Root".into(),
            disabled: false,
            open: None,
            items: vec![SidebarItem {
                id: "root".into(),
                label: "Nested".into(),
                disabled: false,
                open: None,
                items: vec![],
            }],
        }];
        let mut entries = vec![];
        sidebar_entries(&items, true, &BTreeMap::new(), &mut entries);
        assert!(validate_ids(entries.iter().map(|(id, _)| id.as_str())).is_err());
        assert!(validate_ids(["a", "b"]).is_ok());
    }
}
