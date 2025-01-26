use gpui::*;
use story::{Assets, TableStory};

pub struct Example {
    table: Entity<TableStory>,
}

impl Example {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let table = TableStory::view(cx);

        Self { table }
    }

    fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(Self::new)
    }
}

impl Render for Example {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
