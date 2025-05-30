// filepath: todo-app-gpui/todo-app-gpui/src/views/todo_main_view.rs
use gpui::*;
use crate::models::todo_item::TodoItem;

pub struct TodoMainView {
    todos: Vec<TodoItem>,
}

impl TodoMainView {
    pub fn new() -> Self {
        Self {
            todos: Vec::new(),
        }
    }

    pub fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let todo_list = self.todos.iter().map(|todo| {
            div()
                .child(todo.title.clone())
                .child(if todo.completed {
                    div().child("✓").text_color(cx.theme().success)
                } else {
                    div().child("✗").text_color(cx.theme().error)
                })
        });

        div()
            .p_4()
            .child(h1().child("Todo List"))
            .child(v_flex().children(todo_list))
            .child(Button::new("Add Todo").on_click(cx.listener(|_, _, _| {
                // Logic to add a new todo item
            })))
    }

    pub fn add_todo(&mut self, title: String) {
        let new_todo = TodoItem::new(title);
        self.todos.push(new_todo);
    }
}