use gpui::*;
use gpui_component::Selectable;
use story::{Assets, LabelStory};

pub struct Example {
    root: Entity<LabelStory>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root = LabelStory::view(window, cx);

        Self { root }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        
        // div()
        //     .p_4()
        //     .id("example")
        //     .overflow_y_scroll()
        //     .size_full()
        //     .child(self.root.clone())

        div()
            .size_full()
            .bg(rgb(0xFFFFFF))
            .flex()
            .justify_center()
            .items_center()
            .text_3xl()
            .child(
               
                InteractiveText::new("interactive_text_id", StyledText::new("Text")).on_click(
                    vec![1..3],
                    |_range_index, _window, _app| {
                        println!("Clicked {}",_range_index);
                    },
                ),
            )
    }
}

fn main() {
    Application::new().with_assets(Assets).run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("List Example", Example::view, cx);
    });
}
