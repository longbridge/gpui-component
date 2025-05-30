// filepath: todo-app-gpui/todo-app-gpui/src/views/todo_form.rs
use gpui::*;
use crate::models::todo_item::TodoItem;

pub struct TodoForm {
    title: String,
    completed: bool,
}

impl TodoForm {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            completed: false,
        }
    }

    pub fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div().p_4().child(
            v_flex()
                .gap_2()
                .child(
                    TextInput::new(&self.title)
                        .placeholder("Enter todo title")
                        .on_change(cx.listener(|this, value: &str| {
                            this.title = value.to_string();
                        })),
                )
                .child(
                    Checkbox::new("completed")
                        .checked(self.completed)
                        .label("Completed")
                        .on_click(cx.listener(|this, checked: &bool| {
                            this.completed = *checked;
                        })),
                )
                .child(
                    Button::new("Submit")
                        .on_click(cx.listener(|this, _, cx| {
                            let todo_item = TodoItem::new(this.title.clone(), this.completed);
                            // Handle submission logic here
                            cx.notify();
                        })),
                ),
        )
    }
}