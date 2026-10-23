use super::*;

impl BaseShowcase {
    pub(in super::super) fn table(&self) -> impl IntoElement {
        Table::new("example-table")
            .w_72()
            .text_xs()
            .border_1()
            .border_color(super::example_rgb(0xe5e7eb))
            .overflow_hidden()
            .child(
                TableHeader::new("header").child(
                    TableRow::new("header-row", 1)
                        .flex()
                        .bg(super::example_rgb(0xf5f5f5))
                        .child(
                            TableHead::new("name-head", 1)
                                .w(px(124.))
                                .px_2()
                                .py_1()
                                .child("Component"),
                        )
                        .child(
                            TableHead::new("status-head", 2)
                                .w(px(84.))
                                .px_2()
                                .py_1()
                                .child("Status"),
                        )
                        .child(
                            TableHead::new("version-head", 3)
                                .w(px(92.))
                                .px_2()
                                .py_1()
                                .child("Version"),
                        ),
                ),
            )
            .child(
                TableBody::new("body").children(
                    [
                        ("gpui-base", "Stable", "0.4.1"),
                        ("gpui-component", "Active", "0.4.1"),
                        ("story-web", "Preview", "0.2.8"),
                        ("gpui-web", "Beta", "0.1.0"),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(ix, (name, status, version))| {
                        TableRow::new(("body-row", ix), ix)
                            .flex()
                            .border_t_1()
                            .border_color(super::example_rgb(0xe5e7eb))
                            .child(
                                TableCell::new("name", 1)
                                    .w(px(124.))
                                    .px_2()
                                    .py_1()
                                    .child(name),
                            )
                            .child(
                                TableCell::new(("status", ix), 2)
                                    .w(px(84.))
                                    .px_2()
                                    .py_1()
                                    .child(
                                        div()
                                            .px_1()
                                            .border_1()
                                            .border_color(super::example_rgb(0xd4d4d4))
                                            .child(status),
                                    ),
                            )
                            .child(
                                TableCell::new(("version", ix), 3)
                                    .w(px(92.))
                                    .px_2()
                                    .py_1()
                                    .text_color(super::example_rgb(0x737373))
                                    .child(version),
                            )
                    }),
                ),
            )
    }
}
