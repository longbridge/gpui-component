use super::*;

impl BaseShowcase {
    pub(in super::super) fn select(
        &self,
        combobox: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = !self.checked;
        let selected = self.selected_tab.min(3);
        let labels = ["GPUI", "React", "SwiftUI", "Vue"];
        let entity = cx.entity().downgrade();
        let trigger = div()
            .h(px(30.))
            .px_2()
            .text_size(px(13.))
            .flex()
            .items_center()
            .justify_between()
            .border_1()
            .border_color(rgb(0x171717))
            .child(labels[selected])
            .child(if open { "⌃" } else { "⌄" });
        let options = div()
            .mt_1()
            .p_1()
            .border_1()
            .border_color(rgb(0x171717))
            .bg(rgb(0xffffff))
            .children(labels.into_iter().enumerate().map(|(ix, label)| {
                let entity = entity.clone();
                div()
                    .id(("select-option", ix))
                    .px_2()
                    .py_1()
                    .flex()
                    .justify_between()
                    .hover(|this| this.bg(rgb(0xf5f5f5)))
                    .child(label)
                    .when(ix == selected, |this| this.child("✓"))
                    .on_click(move |_, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.selected_tab = ix;
                            this.checked = true;
                            cx.notify();
                        });
                    })
            }));
        if combobox {
            Combobox::new("example-combobox")
                .open(open)
                .w(px(220.))
                .child(trigger)
                .when(open, |this| this.child(options))
                .into_any_element()
        } else {
            Select::new("example-select")
                .open(open)
                .on_open_change({
                    let entity = entity.clone();
                    move |next, _, cx| {
                        _ = entity.update(cx, |this, cx| {
                            this.checked = !next;
                            cx.notify();
                        });
                    }
                })
                .accessibility_label("Framework")
                .w(px(220.))
                .child(trigger)
                .when(open, |this| this.child(options))
                .into_any_element()
        }
    }
}
