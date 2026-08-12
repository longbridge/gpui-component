use super::*;

impl BaseShowcase {
    pub(in super::super) fn tree(&self) -> impl IntoElement {
        Tree::new(&self.tree)
            .w(px(260.))
            .h(px(188.))
            .text_size(px(13.))
            .border_1()
            .border_color(rgb(0xd4d4d4))
            .py_1()
            .item(|_, entry, state, _, _| {
                div()
                    .mx_1()
                    .px_2()
                    .py_1()
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
