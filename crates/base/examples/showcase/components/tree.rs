use super::*;
use gpui::StyleRefinement;

impl BaseShowcase {
    pub(in super::super) fn tree(&self) -> impl IntoElement {
        Tree::new(&self.tree)
            .w(px(260.))
            .h(px(188.))
            .list_style(StyleRefinement::default().flex_grow_1().size_full())
            .relative()
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0xd4d4d4))
            .py_1()
            .item(|_, entry, state, _, _| {
                div()
                    .h(px(28.))
                    .mx_1()
                    .px_2()
                    .flex()
                    .items_center()
                    .when(state.is_selected(), |this| this.bg(rgb(0xf0f0f0)))
                    .child(format!(
                        "{}{} {}",
                        "   ".repeat(entry.depth()),
                        if entry.is_folder() {
                            if entry.is_expanded() { "⌄" } else { "›" }
                        } else {
                            "·"
                        },
                        entry.item().label
                    ))
                    .into_any_element()
            })
    }
}
