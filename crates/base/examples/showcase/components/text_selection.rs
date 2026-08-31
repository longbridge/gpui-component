use gpui::{Context, IntoElement, ParentElement as _, Styled as _, Window};
#[cfg(test)]
use gpui_base::ElementExt as _;
use gpui_base::{SelectableText, TextSelection};

use super::*;

const PRODUCT_PARAGRAPH: &str = "Selection should feel like a natural part of reading a product brief. Start in this paragraph, continue into the next renderer, and GPUI preserves the document order while every frame supplies fresh geometry for the same stable selection handle.";
const IMPLEMENTATION_PARAGRAPH: &str = "This second paragraph is deliberately long enough to wrap in the showcase. Drag across the boundary to see one continuous highlight, then use the platform copy shortcut to confirm that the copied result follows the visible reading order rather than renderer ownership.";
const INTERNATIONAL_PARAGRAPH: &str = "International text should remain predictable when a line mixes café, déjà vu, Kraków, naïve, and résumé. Resize the window or drag across several wrapped lines; UTF-8 byte ranges still map back to the correct glyphs without splitting a character.";

impl BaseShowcase {
    pub(in super::super) fn text_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.text_selection_active = TextSelection::has_selection(window, cx);
        self.text_selection_text = TextSelection::selected_text(window, cx);

        let active = self.text_selection_active;
        let selected_text = if active {
            self.text_selection_text.clone()
        } else {
            "Drag across any paragraphs to select text.".to_owned()
        };
        let entity = cx.entity().downgrade();

        let footer = div()
            .id("text-selection-footer")
            .h(px(150.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .bg(super::example_rgb(0xf5f5f5))
            .border_1()
            .border_color(super::example_rgb(0xe5e5e5))
            .child(
                div()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(if active {
                        "Selection active"
                    } else {
                        "No selection"
                    }),
            )
            .child(
                div()
                    .id("text-selection-preview")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .text_color(super::example_rgb(0x525252))
                    .child(selected_text),
            )
            .child(
                Button::new("clear-text-selection")
                    .h_7()
                    .px_2()
                    .flex()
                    .items_center()
                    .justify_center()
                    .self_start()
                    .border_1()
                    .border_color(super::example_rgb(0x171717))
                    .child("Clear selection")
                    .on_click(move |_, window, cx| {
                        TextSelection::clear(window, cx);
                        _ = entity.update(cx, |this, cx| {
                            this.text_selection_active = false;
                            this.text_selection_text.clear();
                            cx.notify();
                        });
                    }),
            );
        #[cfg(test)]
        let footer = {
            let bounds = self.text_selection_footer_bounds.clone();
            footer.on_prepaint(move |value, _, _| *bounds.borrow_mut() = Some(value))
        };

        div()
            .id("text-selection-example")
            .w(px(620.))
            .max_w_full()
            .h(px(520.))
            .max_h_full()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .id("text-selection-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.text_selection_scroll)
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .child(
                        div()
                            .text_lg()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(
                                SelectableText::with_handle(
                                    "selection-heading",
                                    self.text_selection_handles[0].clone(),
                                    "Text selection across renderers",
                                )
                                .document_order(0),
                            ),
                    )
                    .child(
                        div()
                            .text_color(super::example_rgb(0x525252))
                            .line_height(px(22.))
                            .child(
                                SelectableText::with_handle(
                                    "selection-product",
                                    self.text_selection_handles[1].clone(),
                                    PRODUCT_PARAGRAPH,
                                )
                                .document_order(1),
                            ),
                    )
                    .child(
                        div()
                            .text_color(super::example_rgb(0x525252))
                            .line_height(px(22.))
                            .child(
                                SelectableText::with_handle(
                                    "selection-implementation",
                                    self.text_selection_handles[2].clone(),
                                    IMPLEMENTATION_PARAGRAPH,
                                )
                                .document_order(2),
                            ),
                    )
                    .child(
                        div()
                            .text_color(super::example_rgb(0x525252))
                            .line_height(px(22.))
                            .child(
                                SelectableText::with_handle(
                                    "selection-international",
                                    self.text_selection_handles[3].clone(),
                                    INTERNATIONAL_PARAGRAPH,
                                )
                                .document_order(3),
                            ),
                    ),
            )
            .child(footer)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{TestAppContext, point, px};

    use crate::showcase::BaseShowcase;

    #[gpui::test]
    fn text_selection_footer_stays_fixed_when_document_scrolls(cx: &mut TestAppContext) {
        let (view, window) =
            cx.add_window_view(|window, cx| BaseShowcase::new("text-selection", window, cx));
        window.update(|window, cx| window.draw(cx).clear(cx));

        let (footer_bounds, scroll) = view.read_with(window, |view, _| {
            (
                view.text_selection_footer_bounds
                    .borrow()
                    .expect("footer should be painted"),
                view.text_selection_scroll.clone(),
            )
        });
        scroll.set_offset(point(px(0.), px(-80.)));
        view.update(window, |_, cx| cx.notify());
        window.update(|window, cx| window.draw(cx).clear(cx));

        let scrolled_footer_bounds = view.read_with(window, |view, _| {
            view.text_selection_footer_bounds
                .borrow()
                .expect("footer should be painted after scrolling")
        });
        assert_eq!(scrolled_footer_bounds, footer_bounds);
    }
}
