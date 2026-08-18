use gpui::AnyElement;
use gpui::Corners;
use gpui::InteractiveElement;
use gpui::ParentElement;
use gpui::{App, Axis, Edges, ElementId, IntoElement, Window};
use gpui::{
    RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled, div,
    prelude::FluentBuilder as _,
};
use std::{cell::Cell, rc::Rc};

use crate::{
    Disableable, Selectable as _, Sizable, Size, StyledExt,
    button::{Button, ButtonVariant, ButtonVariants},
    menu::DropdownMenuPopover,
};

/// A member of a [`ButtonGroup`]: a plain [`Button`], or a button that opens a
/// dropdown menu.
///
/// Both forms convert on their own, so a group takes either one directly:
///
/// ```ignore
/// ButtonGroup::new("save")
///     .child(Button::new("save").label("Save"))
///     .child(
///         Button::new("save-options")
///             .dropdown_caret(true)
///             .dropdown_menu(|menu, _, _| menu.menu("Save as…", Box::new(SaveAs))),
///     )
/// ```
pub struct ButtonGroupChild(ButtonGroupChildKind);

// Both variants are a Button-sized builder — boxing the popover would trade a
// move for an allocation and leave the enum just as large.
#[allow(clippy::large_enum_variant)]
enum ButtonGroupChildKind {
    Button(Button),
    Menu(DropdownMenuPopover<Button>),
}

impl From<Button> for ButtonGroupChild {
    fn from(button: Button) -> Self {
        Self(ButtonGroupChildKind::Button(button))
    }
}

impl From<DropdownMenuPopover<Button>> for ButtonGroupChild {
    fn from(menu: DropdownMenuPopover<Button>) -> Self {
        Self(ButtonGroupChildKind::Menu(menu))
    }
}

impl ButtonGroupChild {
    /// Rebuilds the member's button, reaching through the popover for a member
    /// that opens a menu.
    fn map_button(self, f: impl FnOnce(Button) -> Button) -> Self {
        Self(match self.0 {
            ButtonGroupChildKind::Button(button) => ButtonGroupChildKind::Button(f(button)),
            ButtonGroupChildKind::Menu(menu) => ButtonGroupChildKind::Menu(menu.map_trigger(f)),
        })
    }

    fn is_selected(&self) -> bool {
        match &self.0 {
            ButtonGroupChildKind::Button(button) => button.is_selected(),
            ButtonGroupChildKind::Menu(menu) => menu.is_selected(),
        }
    }

    /// Whether clicking this member opens a menu, in which case the group must
    /// leave its click handling alone.
    fn opens_menu(&self) -> bool {
        matches!(self.0, ButtonGroupChildKind::Menu(_))
    }

    fn into_any_element(self) -> AnyElement {
        match self.0 {
            ButtonGroupChildKind::Button(button) => button.into_any_element(),
            ButtonGroupChildKind::Menu(menu) => menu.into_any_element(),
        }
    }
}

/// A ButtonGroup element, to wrap multiple buttons in a group.
#[derive(IntoElement)]
pub struct ButtonGroup {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<ButtonGroupChild>,
    pub(super) multiple: bool,
    pub(super) disabled: bool,
    pub(super) layout: Axis,
    attached: bool,
    toggle: bool,

    // The button props
    pub(super) compact: bool,
    pub(super) outline: bool,
    pub(super) variant: Option<ButtonVariant>,
    pub(super) size: Option<Size>,

    on_click: Option<Box<dyn Fn(&Vec<usize>, &mut Window, &mut App) + 'static>>,
}

impl Disableable for ButtonGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ButtonGroup {
    /// Creates a new ButtonGroup.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            variant: None,
            size: None,
            compact: false,
            outline: false,
            multiple: false,
            disabled: false,
            layout: Axis::Horizontal,
            attached: true,
            toggle: true,
            on_click: None,
        }
    }

    /// Adds a button as a child to the ButtonGroup.
    pub fn child(mut self, child: impl Into<ButtonGroupChild>) -> Self {
        let disabled = self.disabled;
        self.children
            .push(child.into().map_button(|child| child.disabled(disabled)));
        self
    }

    /// Adds multiple buttons as children to the ButtonGroup.
    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<ButtonGroupChild>>,
    ) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Whether the members share their inner edges, default: true.
    ///
    /// With `false` every member keeps the borders and radii it would have on
    /// its own, which is how a ghost group reads as separate buttons.
    pub(crate) fn attached(mut self, attached: bool) -> Self {
        self.attached = attached;
        self
    }

    /// Stops the group from advertising its members as toggle buttons.
    ///
    /// A group is a toggle control by default, so every member reports its
    /// pressed state. A split button is not: its halves run an action and open
    /// a menu, and `selected` there is presentation only.
    pub(crate) fn no_toggle(mut self) -> Self {
        self.toggle = false;
        self
    }

    /// With the multiple selection mode, default is false (single selection).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set the layout of the button group. Default is `Axis::Horizontal`.
    pub fn layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    /// With the compact mode for the ButtonGroup.
    ///
    /// See also: [`Button::compact()`]
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// With the outline mode for the ButtonGroup.
    ///
    /// See also: [`Button::outline()`]
    pub fn outline(mut self) -> Self {
        self.outline = true;
        self
    }

    /// Sets the on_click handler for the ButtonGroup.
    ///
    /// The handler first argument is a vector of the selected button indices.
    ///
    /// The `&Vec<usize>` is the indices of the clicked (selected in `multiple` mode) buttons.
    /// For example: `[0, 2, 3]` is means the first, third and fourth buttons are clicked.
    ///
    /// ```ignore
    /// ButtonGroup::new("size-button")
    ///    .child(Button::new("large").label("Large").selected(self.size == Size::Large))
    ///    .child(Button::new("medium").label("Medium").selected(self.size == Size::Medium))
    ///    .child(Button::new("small").label("Small").selected(self.size == Size::Small))
    ///    .on_click(cx.listener(|view, clicks: &Vec<usize>, _, cx| {
    ///        if clicks.contains(&0) {
    ///            view.size = Size::Large;
    ///        } else if clicks.contains(&1) {
    ///            view.size = Size::Medium;
    ///        } else if clicks.contains(&2) {
    ///            view.size = Size::Small;
    ///        }
    ///        cx.notify();
    ///    }))
    /// ```
    pub fn on_click(
        mut self,
        handler: impl Fn(&Vec<usize>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl Sizable for ButtonGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl Styled for ButtonGroup {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl ButtonVariants for ButtonGroup {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl RenderOnce for ButtonGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let children_len = self.children.len();
        let mut selected_ixs: Vec<usize> = Vec::new();
        let state = Rc::new(Cell::new(None));

        for (ix, child) in self.children.iter().enumerate() {
            if child.is_selected() {
                selected_ixs.push(ix);
            }
        }

        let vertical = self.layout == Axis::Vertical;
        let (attached, toggle) = (self.attached, self.toggle);
        let (size, variant) = (self.size, self.variant);
        let (compact, outline) = (self.compact, self.outline);
        let has_group_click = self.on_click.is_some();

        div()
            .id(self.id)
            .flex()
            .when(vertical, |this| this.flex_col().justify_center())
            .when(!vertical, |this| this.items_center())
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(child_index, child)| {
                        let state = Rc::clone(&state);
                        let selected = child.is_selected();
                        // A member that opens a menu keeps its own click
                        // handling: the popover listens for the press itself.
                        let child_click = has_group_click && !child.opens_menu();

                        child
                            .map_button(|child| {
                                // A group is a toggle control by default, so every
                                // child advertises its pressed state.
                                let child = child.when(toggle, |this| this.toggled(selected));
                                let child = if children_len == 1 || !attached {
                                    child
                                } else if child_index == 0 {
                                    // First
                                    child
                                        .border_corners(Corners {
                                            top_left: true,
                                            top_right: vertical,
                                            bottom_left: !vertical,
                                            bottom_right: false,
                                        })
                                        .border_edges(Edges {
                                            left: true,
                                            top: true,
                                            right: true,
                                            bottom: true,
                                        })
                                } else if child_index == children_len - 1 {
                                    // Last
                                    child
                                        .border_edges(Edges {
                                            left: vertical,
                                            top: !vertical,
                                            right: true,
                                            bottom: true,
                                        })
                                        .border_corners(Corners {
                                            top_left: false,
                                            top_right: !vertical,
                                            bottom_left: vertical,
                                            bottom_right: true,
                                        })
                                } else {
                                    // Middle
                                    child
                                        .border_corners(Corners {
                                            top_left: false,
                                            top_right: false,
                                            bottom_left: false,
                                            bottom_right: false,
                                        })
                                        .border_edges(Edges {
                                            left: vertical,
                                            top: !vertical,
                                            right: true,
                                            bottom: true,
                                        })
                                }
                                .when_some(size, |this, size| this.with_size(size))
                                .when_some(variant, |this, variant| this.with_variant(variant))
                                .when(compact, |this| this.compact())
                                .when(outline, |this| this.outline())
                                .when(child_click, |this| {
                                    this.on_click(move |_, _, _| {
                                        state.set(Some(child_index));
                                    })
                                });

                                child
                            })
                            .into_any_element()
                    }),
            )
            .when_some(
                self.on_click.filter(|_| !self.disabled),
                move |this, on_click| {
                    this.on_click(move |_, window, cx| {
                        let mut selected_ixs = selected_ixs.clone();
                        if let Some(ix) = state.get() {
                            if self.multiple {
                                if let Some(pos) = selected_ixs.iter().position(|&i| i == ix) {
                                    selected_ixs.remove(pos);
                                } else {
                                    selected_ixs.push(ix);
                                }
                            } else {
                                selected_ixs.clear();
                                selected_ixs.push(ix);
                            }
                        }

                        on_click(&selected_ixs, window, cx);
                    })
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{
        Axis, Context, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Render, TestAppContext,
        VisualTestContext, point, px,
    };
    use std::{cell::Cell, rc::Rc};

    struct GroupHarness {
        multiple: bool,
        install_group_callback: bool,
        child_clicks: Rc<Cell<usize>>,
        group_changes: Rc<std::cell::RefCell<Vec<Vec<usize>>>>,
    }

    impl Render for GroupHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let child_clicks = self.child_clicks.clone();
            let mut group = ButtonGroup::new("group")
                .multiple(self.multiple)
                .size(px(120.))
                .child(
                    Button::new("one")
                        .label("One")
                        .on_click(move |_, _, _| child_clicks.set(child_clicks.get() + 1)),
                )
                .child(Button::new("two").label("Two").selected(true));
            if self.install_group_callback {
                let changes = self.group_changes.clone();
                group = group.on_click(move |next, _, _| changes.borrow_mut().push(next.clone()));
            }
            group
        }
    }

    fn group_harness(
        cx: &mut TestAppContext,
        multiple: bool,
        install_group_callback: bool,
    ) -> (
        &mut VisualTestContext,
        Rc<Cell<usize>>,
        Rc<std::cell::RefCell<Vec<Vec<usize>>>>,
    ) {
        cx.update(crate::init);
        let child_clicks = Rc::new(Cell::new(0));
        let changes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let (_, cx) = cx.add_window_view({
            let child_clicks = child_clicks.clone();
            let group_changes = changes.clone();
            move |_, _| GroupHarness {
                multiple,
                install_group_callback,
                child_clicks,
                group_changes,
            }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        (cx, child_clicks, changes)
    }

    fn activate_key(cx: &mut VisualTestContext, key: &str) {
        let keystroke = Keystroke::parse(key).unwrap();
        cx.simulate_event(KeyDownEvent {
            keystroke: keystroke.clone(),
            is_held: false,
            prefer_character_input: false,
        });
        cx.simulate_event(KeyUpEvent { keystroke });
    }

    fn click_first(cx: &mut VisualTestContext) {
        cx.simulate_click(point(px(10.), px(60.)), Modifiers::default());
    }

    /// A member that opens a menu is styled through its popover, and is left
    /// out of the group's click handling.
    #[test]
    fn a_menu_member_is_recognized_and_styled_through_its_popover() {
        use crate::{Selectable as _, menu::DropdownMenu as _};

        assert!(!ButtonGroupChild::from(Button::new("plain")).opens_menu());

        let menu = ButtonGroupChild::from(
            Button::new("menu")
                .dropdown_caret(true)
                .dropdown_menu(|menu, _, _| menu),
        );
        assert!(menu.opens_menu());
        assert!(
            menu.map_button(|button| button.selected(true))
                .is_selected()
        );
    }

    #[gpui::test]
    fn legacy_group_callback_overrides_the_child_callback(cx: &mut TestAppContext) {
        let (cx, child_clicks, changes) = group_harness(cx, false, true);
        click_first(cx);
        assert_eq!(child_clicks.get(), 0);
        assert_eq!(changes.borrow().as_slice(), &[vec![0]]);
    }

    #[gpui::test]
    fn legacy_child_callback_survives_without_a_group_callback(cx: &mut TestAppContext) {
        let (cx, child_clicks, changes) = group_harness(cx, false, false);
        click_first(cx);
        assert_eq!(child_clicks.get(), 1);
        assert!(changes.borrow().is_empty());
    }

    #[gpui::test]
    fn legacy_single_and_multiple_results_use_the_rendered_selection(cx: &mut TestAppContext) {
        let (cx, _, single) = group_harness(cx, false, true);
        click_first(cx);
        assert_eq!(single.borrow().as_slice(), &[vec![0]]);

        let (cx, _, multiple) = group_harness(cx, true, true);
        click_first(cx);
        assert_eq!(multiple.borrow().as_slice(), &[vec![1, 0]]);
    }

    #[gpui::test]
    fn legacy_keyboard_click_does_not_reach_the_group_callback(cx: &mut TestAppContext) {
        let (cx, _, changes) = group_harness(cx, false, true);
        cx.update(|window, cx| window.focus_next(cx));
        activate_key(cx, "enter");
        assert!(changes.borrow().is_empty());
    }

    #[gpui::test]
    fn legacy_disabled_state_depends_on_builder_order(cx: &mut TestAppContext) {
        struct DisabledOrderHarness(Rc<Cell<usize>>);

        impl Render for DisabledOrderHarness {
            fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let first = self.0.clone();
                let second = self.0.clone();
                crate::v_flex()
                    .child(
                        ButtonGroup::new("disabled-before-child")
                            .size(px(120.))
                            .disabled(true)
                            .child(
                                Button::new("first")
                                    .label("First")
                                    .on_click(move |_, _, _| first.set(first.get() + 1)),
                            ),
                    )
                    .child(
                        ButtonGroup::new("disabled-after-child")
                            .size(px(120.))
                            .child(
                                Button::new("second")
                                    .label("Second")
                                    .on_click(move |_, _, _| second.set(second.get() + 1)),
                            )
                            .disabled(true),
                    )
            }
        }

        cx.update(crate::init);
        let clicks = Rc::new(Cell::new(0));
        let (_, cx) = cx.add_window_view({
            let clicks = clicks.clone();
            move |_, _| DisabledOrderHarness(clicks)
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(point(px(10.), px(60.)), Modifiers::default());
        assert_eq!(clicks.get(), 0);
        cx.simulate_click(point(px(10.), px(180.)), Modifiers::default());
        assert_eq!(clicks.get(), 1);
    }

    #[gpui::test]
    fn test_button_group_builder(_cx: &mut gpui::TestAppContext) {
        let group = ButtonGroup::new("complex-group")
            .child(Button::new("btn1").label("One"))
            .child(Button::new("btn2").label("Two"))
            .child(Button::new("btn3").label("Three"))
            .primary()
            .large()
            .outline()
            .compact()
            .multiple(true)
            .layout(Axis::Vertical)
            .disabled(false)
            .on_click(|_, _, _| {});

        assert_eq!(group.children.len(), 3);
        assert_eq!(group.variant, Some(ButtonVariant::Primary));
        assert_eq!(group.size, Some(Size::Large));
        assert!(group.outline);
        assert!(group.compact);
        assert!(group.multiple);
        assert_eq!(group.layout, Axis::Vertical);
        assert!(!group.disabled);
        assert!(group.on_click.is_some());
    }
}
