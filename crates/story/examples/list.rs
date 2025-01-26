use gpui::*;
use story::{Assets, ListStory};

pub struct Example {
    root: Entity<ListStory>,
}

impl Example {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let root = ListStory::view(cx);

        Self { root }
    }

    fn view(cx: &mut WindowContext) -> Entity<Self> {
        cx.new_view(Self::new)
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.root.clone())
    }
}

fn main() {
    let app = App::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("List Example", Example::view, cx);
    });
}
