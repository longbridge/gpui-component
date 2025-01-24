use gpui::*;
use story::{Assets, TableStory};

pub struct Example {
    table: View<TableStory>,
}

impl Example {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let table = TableStory::view(cx);

        Self { table }
    }

    fn view(cx: &mut WindowContext) -> View<Self> {
        cx.new_view(Self::new)
    }
}

impl Render for Example {
    fn render(&mut self, _: &mut ViewContext<Self>) -> impl IntoElement {
        div().p_4().size_full().child(self.table.clone())
    }
}

fn main() {
    let app = App::new().with_assets(Assets);

    app.run(move |cx| {
        story::init(cx);
        cx.activate(true);

        story::create_new_window("Table Example", Example::view, cx);
    });
}
