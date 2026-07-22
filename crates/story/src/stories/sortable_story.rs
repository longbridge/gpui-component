use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render, Styled,
    Window, px,
};
use gpui_component::{
    ActiveTheme as _,
    h_flex,
    label::Label,
    sortable::{Sortable, SortableState},
    v_flex,
};

use crate::section;

#[derive(Clone)]
struct DemoItem {
    id: usize,
    label: String,
    color: gpui::Hsla,
}

pub struct SortableStory {
    focus_handle: gpui::FocusHandle,
    list_a: Entity<SortableState<DemoItem>>,
    list_b: Entity<SortableState<DemoItem>>,
}

impl super::Story for SortableStory {
    fn title() -> &'static str {
        "Sortable"
    }

    fn description() -> &'static str {
        "Reorderable list with cross-list drag-and-drop."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl SortableStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let items_a = vec![
            DemoItem {
                id: 1,
                label: "Design review".into(),
                color: gpui::hsla(0.6, 0.7, 0.65, 1.0),
            },
            DemoItem {
                id: 2,
                label: "Write tests".into(),
                color: gpui::hsla(0.3, 0.7, 0.55, 1.0),
            },
            DemoItem {
                id: 3,
                label: "Fix CI pipeline".into(),
                color: gpui::hsla(0.0, 0.7, 0.65, 1.0),
            },
            DemoItem {
                id: 4,
                label: "Update docs".into(),
                color: gpui::hsla(0.1, 0.7, 0.6, 1.0),
            },
        ];

        let items_b = vec![
            DemoItem {
                id: 5,
                label: "Ship v2.0".into(),
                color: gpui::hsla(0.8, 0.7, 0.65, 1.0),
            },
            DemoItem {
                id: 6,
                label: "Customer demo".into(),
                color: gpui::hsla(0.5, 0.7, 0.55, 1.0),
            },
        ];

        let list_a = cx.new(|_| SortableState::new(items_a));
        let list_b = cx.new(|_| SortableState::new(items_b));

        Self {
            focus_handle: cx.focus_handle(),
            list_a,
            list_b,
        }
    }
}

fn render_card(item: &DemoItem, _ix: usize, _window: &Window, cx: &App) -> gpui::AnyElement {
    let bg = item.color.opacity(0.15);
    let border = item.color.opacity(0.4);
    let text_color = cx.theme().foreground;

    gpui::div()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(bg)
        .border_1()
        .border_color(border)
        .text_color(text_color)
        .text_sm()
        .cursor_grab()
        .child(item.label.clone())
        .into_any_element()
}

impl Focusable for SortableStory {
    fn focus_handle(&self, _: &App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SortableStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_6().child(
            section("Cross-list drag and drop").child(
                h_flex()
                    .gap_6()
                    .items_start()
                    .child(
                        v_flex()
                            .gap_2()
                            .w(px(240.))
                            .child(Label::new("To Do"))
                            .child(
                                Sortable::new(
                                    "list-a",
                                    self.list_a.clone(),
                                    |item: &DemoItem| {
                                        gpui::ElementId::NamedInteger(
                                            "demo".into(),
                                            item.id as u64,
                                        )
                                    },
                                    render_card,
                                )
                                .gap(px(6.))
                                .on_reorder({
                                    move |from, to, _window, _cx| {
                                        eprintln!("List A: reorder {from} → {to}");
                                    }
                                })
                                .on_insert({
                                    move |item, at, _source, _window, _cx| {
                                        eprintln!(
                                            "List A: inserted '{}' at {at}",
                                            item.label
                                        );
                                    }
                                }),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .w(px(240.))
                            .child(Label::new("Done"))
                            .child(
                                Sortable::new(
                                    "list-b",
                                    self.list_b.clone(),
                                    |item: &DemoItem| {
                                        gpui::ElementId::NamedInteger(
                                            "demo".into(),
                                            item.id as u64,
                                        )
                                    },
                                    render_card,
                                )
                                .gap(px(6.))
                                .on_reorder({
                                    move |from, to, _window, _cx| {
                                        eprintln!("List B: reorder {from} → {to}");
                                    }
                                })
                                .on_insert({
                                    move |item, at, _source, _window, _cx| {
                                        eprintln!(
                                            "List B: inserted '{}' at {at}",
                                            item.label
                                        );
                                    }
                                }),
                            ),
                    ),
            ),
        )
    }
}
