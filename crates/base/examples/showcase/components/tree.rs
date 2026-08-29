use super::*;
use gpui::{Image, ImageFormat, StyleRefinement, img};
use std::sync::Arc;

const CHEVRON_RIGHT_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m6 3.5 4.5 4.5L6 12.5" stroke="#171717" stroke-width="1.5" stroke-linecap="square" stroke-linejoin="miter"/></svg>"##;
const CHEVRON_DOWN_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="none"><path d="m3.5 6 4.5 4.5L12.5 6" stroke="#171717" stroke-width="1.5" stroke-linecap="square" stroke-linejoin="miter"/></svg>"##;

impl BaseShowcase {
    pub(in super::super) fn tree(&self) -> impl IntoElement {
        Tree::new(&self.tree)
            .w_64()
            .h_48()
            .list_style(StyleRefinement::default().flex_grow_1().size_full())
            .relative()
            .text_sm()
            .border_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .py_1()
            .item(|_, entry, state, _, _| {
                let depth = entry.depth();
                let icon = entry.is_folder().then(|| {
                    let bytes = if entry.is_expanded() {
                        CHEVRON_DOWN_SVG
                    } else {
                        CHEVRON_RIGHT_SVG
                    };
                    img(Arc::new(Image::from_bytes(
                        ImageFormat::Svg,
                        bytes.to_vec(),
                    )))
                    .size_3()
                    .flex_none()
                });
                div()
                    .h_8()
                    .mx_1()
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(state.is_selected(), |this| {
                        this.bg(super::example_rgb(0xf0f0f0))
                    })
                    .when(depth > 0, |this| {
                        this.child(div().flex_none().w(px(depth as f32 * 12.)))
                    })
                    .child(
                        div()
                            .size_3()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .children(icon),
                    )
                    .child(entry.item().label.clone())
                    .into_any_element()
            })
    }
}
