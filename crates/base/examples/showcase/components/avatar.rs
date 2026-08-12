use super::*;

impl BaseShowcase {
    pub(in super::super) fn avatar(&self) -> impl IntoElement {
        div().flex().items_start().gap_2().children(
            [
                ("AM", 0xf5f5f5),
                ("JL", 0xe5e5e5),
                ("SK", 0xd4d4d4),
                ("+3", 0xffffff),
            ]
            .into_iter()
            .map(|(initials, background)| {
                Avatar::new()
                    .size(px(34.))
                    .overflow_hidden()
                    .border_1()
                    .border_color(rgb(0xa3a3a3))
                    .fallback(
                        AvatarFallback::new()
                            .flex()
                            .size_8()
                            .items_center()
                            .justify_center()
                            .bg(rgb(background))
                            .text_xs()
                            .text_color(rgb(0x262626))
                            .child(initials),
                    )
            }),
        )
    }
}
