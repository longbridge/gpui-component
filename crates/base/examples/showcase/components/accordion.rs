use super::*;

impl BaseShowcase {
    pub(in super::super) fn accordion(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let items = [
            (
                "What is GPUI Base?",
                "Unstyled, accessible primitives for building native GPUI interfaces.",
            ),
            (
                "Can I bring my own theme?",
                "Yes. Every visual detail remains application-owned.",
            ),
            (
                "Does it support keyboard input?",
                "Focus, activation, and semantic state are built into the primitives.",
            ),
        ];

        Accordion::new("example-accordion")
            .w(px(270.))
            .border_t_1()
            .border_color(super::example_rgb(0xd4d4d4))
            .children(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(index, (question, answer))| {
                        let open = self.accordion_items[index];
                        let entity = cx.entity().downgrade();
                        AccordionItem::new()
                            .open(open)
                            .header(AccordionHeader::new(
                                AccordionTrigger::new(format!("accordion-trigger-{index}"))
                                    .on_change(move |next, _, _, cx| {
                                        _ = entity.update(cx, |this, cx| {
                                            this.accordion_items[index] = next;
                                            cx.notify();
                                        });
                                    })
                                    .w_full()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .h_7()
                                    .border_b_1()
                                    .border_color(super::example_rgb(0xd4d4d4))
                                    .text_xs()
                                    .child(question)
                                    .child(
                                        div()
                                            .text_color(super::example_rgb(0x737373))
                                            .child(if open { "−" } else { "+" }),
                                    ),
                            ))
                            .panel(
                                AccordionPanel::new()
                                    .px_1()
                                    .py_1()
                                    .border_b_1()
                                    .border_color(super::example_rgb(0xd4d4d4))
                                    .text_xs()
                                    .text_color(super::example_rgb(0x525252))
                                    .child(answer),
                            )
                    }),
            )
    }
}
