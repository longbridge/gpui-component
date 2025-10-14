use gpui::{
    App, AppContext, Context, Entity, Focusable, InteractiveElement, ParentElement, Render, Styled,
    Window, px,
};

use gpui_component::{
    ActiveTheme as _, IconName, ListItem, StyledExt as _, TreeItem, TreeState, dock::PanelControl,
    h_flex, label::Label, tree, v_flex,
};

use crate::{Story, section};

pub struct TreeStory {
    tree_state: Entity<TreeState>,
    selected_item: Option<TreeItem>,
    focus_handle: gpui::FocusHandle,
}

impl TreeStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let tree_state = cx.new(|_| {
            TreeState::new().items(vec![
                TreeItem::new("src")
                    .expanded(true)
                    .child(TreeItem::new("main.rs"))
                    .child(TreeItem::new("lib.rs"))
                    .child(
                        TreeItem::new("components")
                            .child(TreeItem::new("utils").child(TreeItem::new("mod.rs")))
                            .child(TreeItem::new("label.rs"))
                            .child(TreeItem::new("button.rs"))
                            .child(TreeItem::new("input.rs"))
                            .child(TreeItem::new("theme.rs"))
                            .child(TreeItem::new("colors.rs"))
                            .child(TreeItem::new("dropdown.rs"))
                            .child(TreeItem::new("menu.rs"))
                            .child(TreeItem::new("popover.rs"))
                            .child(TreeItem::new("tree.rs"))
                            .child(TreeItem::new("mod.rs")),
                    ),
                TreeItem::new("Cargo.toml"),
                TreeItem::new("Cargo.lock").disabled(true),
                TreeItem::new("README.md"),
            ])
        });

        Self {
            tree_state,
            selected_item: None,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Story for TreeStory {
    fn title() -> &'static str {
        "Tree"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }

    fn zoomable() -> Option<PanelControl> {
        None
    }
}

impl Focusable for TreeStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TreeStory {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let view = cx.entity();
        v_flex().track_focus(&self.focus_handle).gap_5().child(
            section("File tree")
                .v_flex()
                .max_w_md()
                .child(
                    tree(
                        &self.tree_state,
                        move |ix, entry, _selected, _window, cx| {
                            view.update(cx, |_, cx| {
                                let item = entry.item();
                                let icon = if !entry.is_folder() {
                                    IconName::File
                                } else if entry.is_expanded() {
                                    IconName::FolderOpen
                                } else {
                                    IconName::Folder
                                };

                                ListItem::new(ix)
                                    .w_full()
                                    .rounded(cx.theme().radius)
                                    .px_3()
                                    .pl(px(10.) * entry.depth() + px(12.))
                                    .child(h_flex().gap_2().child(icon).child(item.label.clone()))
                                    .on_click(cx.listener({
                                        let item = item.clone();
                                        move |this, _, _window, cx| {
                                            this.selected_item = Some(item.clone());
                                            cx.notify();
                                        }
                                    }))
                            })
                        },
                    )
                    .p_1()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .h(px(640.)),
                )
                .children(
                    self.selected_item
                        .as_ref()
                        .map(|item| Label::new("Selected Item").secondary(item.label.clone())),
                ),
        )
    }
}
