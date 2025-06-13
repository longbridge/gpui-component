use super::todo_thread_edit::TodoThreadEdit;
use crate::app::AppState;
use crate::models::todo_item::{TodoItem as TodoItemModel, TodoPriority};
use crate::ui::{views::todo_thread::TodoThreadChat, AppExt};
use gpui::prelude::*;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonGroup, ButtonVariants},
    indicator::Indicator,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    popup_menu::PopupMenu,
    tab::TabBar,
    *,
};
use std::time::Duration;

actions!(
    list_story,
    [
        SelectedTodo,
        Open,
        Edit,
        Completed,
        Pause,
        Clone,
        Star,
        Delete,
        RefreshList
    ]
);

#[derive(Clone)]
pub struct ToggleCompletion {
    pub id: u32,
}

impl_actions!(list_story, [ToggleCompletion]);

/// UI中显示的Todo项，基于业务模型
#[derive(Clone)]
pub struct TodoDisplayItem {
    pub model: TodoItemModel,
}

impl TodoDisplayItem {
    pub fn new(model: TodoItemModel) -> Self {
        Self { model }
    }

    pub fn priority_color(&self) -> Hsla {
        match self.model.priority {
            TodoPriority::Low => blue_500(),
            TodoPriority::Medium => yellow_500(),
            TodoPriority::High => orange_500(),
            TodoPriority::Urgent => red_500(),
        }
    }

    pub fn priority_icon(&self) -> IconName {
        match self.model.priority {
            TodoPriority::Low => IconName::ArrowDown,
            TodoPriority::Medium => IconName::Minus,
            TodoPriority::High => IconName::ArrowUp,
            TodoPriority::Urgent => IconName::AlertTriangle,
        }
    }
}

#[derive(IntoElement)]
pub struct TodoItem {
    base: ListItem,
    ix: usize,
    item: TodoDisplayItem,
    selected: bool,
}

impl TodoItem {
    pub fn new(id: impl Into<ElementId>, item: TodoDisplayItem, ix: usize, selected: bool) -> Self {
        TodoItem {
            item,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl RenderOnce for TodoItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let selected = self.selected;
        let completed = self.item.model.completed;
        let text_color = if self.selected {
            cx.theme().accent_foreground
        } else if completed {
            cx.theme().foreground.opacity(0.5)
        } else {
            cx.theme().foreground
        };

        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix % 2 == 0 {
            cx.theme().list
        } else {
            cx.theme().list_even
        };

        // 格式化时间显示
        let created_time = self.item.model.created_at.format("%m/%d %H:%M").to_string();

        self.base
            .px_3()
            .py_2()
            .overflow_x_hidden()
            .bg(bg_color)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        // 左侧内容区域
                        h_flex()
                            .items_start()
                            .gap_3()
                            .flex_1()
                            .child(
                                // 完成状态指示器
                                Button::new(format!("toggle-{}", self.item.model.id))
                                    .icon(if completed {
                                        IconName::CheckCircle
                                    } else {
                                        IconName::Circle
                                    })
                                    .ghost()
                                    .compact()
                                    .size(px(20.))
                                    .text_color(if completed {
                                        green_500()
                                    } else {
                                        cx.theme().muted_foreground
                                    })
                                    .on_click({
                                        let todo_id = self.item.model.id;
                                        move |_, _, cx| {
                                            cx.dispatch_action(Box::new(ToggleCompletion { id: todo_id }));
                                        }
                                    })
                            )
                            .child(
                                // 主要内容
                                v_flex()
                                    .gap_1()
                                    .flex_1()
                                    .child(
                                        // 标题行
                                        h_flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                Label::new(self.item.model.title.clone())
                                                    .text_sm()
                                                    .when(completed, |label| {
                                                        label.line_through()
                                                    })
                                            )
                                            .child(
                                                // 优先级指示器
                                                Icon::new(self.item.priority_icon())
                                                    .xsmall()
                                                    .text_color(self.item.priority_color())
                                            )
                                    )
                                    .when_some(self.item.model.description.as_ref(), |this, desc| {
                                        this.child(
                                            Label::new(desc.clone())
                                                .text_xs()
                                                .text_color(text_color.opacity(0.7))
                                                .line_clamp(2)
                                        )
                                    })
                                    .child(
                                        // 时间信息
                                        h_flex()
                                            .gap_2()
                                            .text_xs()
                                            .text_color(text_color.opacity(0.5))
                                            .child(
                                                h_flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(Icon::new(IconName::Calendar).xsmall())
                                                    .child(Label::new(created_time))
                                            )
                                    )
                            )
                    )
                    .child(
                        // 右侧状态指示器
                        h_flex()
                            .items_center()
                            .gap_2()
                            .when(completed, |this| {
                                this.child(
                                    Icon::new(IconName::CheckCircle2)
                                        .small()
                                        .text_color(green_500())
                                )
                            })
                    )
            )
    }
}

#[derive(Debug)]
pub struct TodoListDelegate {
    todos: Vec<TodoDisplayItem>,
    matched_todos: Vec<TodoDisplayItem>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
}

impl TodoListDelegate {
    pub fn new() -> Self {
        Self {
            todos: Vec::new(),
            matched_todos: Vec::new(),
            selected_index: None,
            confirmed_index: None,
        }
    }

    pub fn load_todos(&mut self, cx: &mut App) {
        let app_state = AppState::global(cx);
        if let Ok(todos) = app_state.todo_service.get_all_todos() {
            self.todos = todos.into_iter().map(TodoDisplayItem::new).collect();
            self.matched_todos = self.todos.clone();
        }
    }

    pub fn filter_todos(&mut self, filter: TodoFilter) {
        match filter {
            TodoFilter::All => {
                self.matched_todos = self.todos.clone();
            }
            TodoFilter::Pending => {
                self.matched_todos = self.todos
                    .iter()
                    .filter(|todo| !todo.model.completed)
                    .cloned()
                    .collect();
            }
            TodoFilter::Completed => {
                self.matched_todos = self.todos
                    .iter()
                    .filter(|todo| todo.model.completed)
                    .cloned()
                    .collect();
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TodoFilter {
    All,
    Pending,
    Completed,
}

impl ListDelegate for TodoListDelegate {
    type Item = TodoItem;

    fn items_count(&self) -> usize {
        self.matched_todos.len()
    }

    fn confirmed_index(&self) -> Option<usize> {
        self.confirmed_index
    }

    fn perform_search(&mut self, _query: &str, _cx: &mut App) {
        // 搜索功能可以在这里实现
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_item(
        &self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(todo) = self.matched_todos.get(ix) {
            return Some(TodoItem::new(ix, todo.clone(), ix, selected));
        }
        None
    }

    fn context_menu(
        &self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &Window,
        _cx: &App,
    ) -> PopupMenu {
        menu.external_link_icon(true)
            .menu("打开", Box::new(Open))
            .menu("编辑", Box::new(Edit))
            .separator()
            .menu_with_icon("克隆", IconName::Copy, Box::new(Clone))
            .menu_with_icon("暂停", IconName::Pause, Box::new(Pause))
            .menu_with_icon("完成", IconName::Done, Box::new(Completed))
            .menu_with_icon("关注", IconName::Star, Box::new(Star))
            .separator()
            .menu_with_icon("删除", IconName::Trash, Box::new(Delete))
    }
}

pub struct TodoList {
    focus_handle: FocusHandle,
    todo_list: View<List<TodoListDelegate>>,
    active_tab_ix: usize,
    current_filter: TodoFilter,
}

impl TodoList {
    pub fn new(cx: &mut Window) -> View<Self> {
        cx.new(|cx| {
            let mut delegate = TodoListDelegate::new();
            delegate.load_todos(cx);
            
            let list_view = cx.new(|cx| {
                List::new(delegate, cx)
                    .with_sizing(ListSizing::Auto)
                    .with_selection(true)
            });

            Self {
                focus_handle: cx.focus_handle(),
                todo_list: list_view,
                active_tab_ix: 0,
                current_filter: TodoFilter::All,
            }
        })
    }

    fn set_active_tab(&mut self, ix: usize, _window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        
        let filter = match ix {
            0 => TodoFilter::All,
            1 => TodoFilter::Pending,
            2 => TodoFilter::Completed,
            _ => TodoFilter::All,
        };
        
        self.current_filter = filter;
        
        self.todo_list.update(cx, |list, cx| {
            list.delegate_mut().filter_todos(filter);
            cx.notify();
        });
    }

    fn refresh_todos(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.todo_list.update(cx, |list, cx| {
            list.delegate_mut().load_todos(cx);
            list.delegate_mut().filter_todos(self.current_filter);
            cx.notify();
        });
    }

    fn toggle_completion(&mut self, action: &ToggleCompletion, _window: &mut Window, cx: &mut Context<Self>) {
        let app_state = AppState::global(cx);
        if let Err(e) = app_state.todo_service.toggle_todo_completion(action.id) {
            eprintln!("切换完成状态失败: {}", e);
        } else {
            self.refresh_todos(_window, cx);
        }
    }

    fn open_todo(&mut self, _action: &Open, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.todo_list.read(cx).delegate().selected_index {
            if let Some(todo) = self.todo_list.read(cx).delegate().matched_todos.get(selected_ix) {
                TodoThreadChat::open(todo.model.id, cx);
            }
        }
    }

    fn edit_todo(&mut self, _action: &Edit, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.todo_list.read(cx).delegate().selected_index {
            if let Some(todo) = self.todo_list.read(cx).delegate().matched_todos.get(selected_ix) {
                TodoThreadEdit::edit(todo.model.id, cx);
            }
        }
    }

    fn clone_todo(&mut self, _action: &Clone, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_ix) = self.todo_list.read(cx).delegate().selected_index {
            if let Some(todo) = self.todo_list.read(cx).delegate().matched_todos.get(selected_ix) {
                let app_state = AppState::global(cx);
                let new_title = format!("{} (副本)", todo.model.title);
                if let Err(e) = app_state.todo_service.create_todo_with_details(
                    new_title,
                    todo.model.description.clone(),
                    todo.model.priority.clone(),
                ) {
                    eprintln!("克隆Todo失败: {}", e);
                } else {
                    self.refresh_todos(_window, cx);
                }
            }
        }
    }
}

impl Focusable for TodoList {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TodoList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::toggle_completion))
            .on_action(cx.listener(Self::open_todo))
            .on_action(cx.listener(Self::edit_todo))
            .on_action(cx.listener(Self::clone_todo))
            .size_full()
            .gap_4()
            .child(
                // 顶部工具栏
                h_flex()
                    .gap_2()
                    .flex_nowrap()
                    .child(
                        TabBar::new("todo-list-tabs")
                            .w_full()
                            .segmented()
                            .selected_index(self.active_tab_ix)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .children(vec!["全部", "进行中", "已完成"]),
                    )
                    .child(
                        Button::new("refresh-button")
                            .icon(IconName::RefreshCW)
                            .size(px(24.))
                            .compact()
                            .ghost()
                            .on_click(cx.listener(|this, _ev, window, cx| {
                                this.refresh_todos(window, cx);
                            })),
                    )
                    .child(
                        Button::new("add-todo-button")
                            .icon(IconName::Plus)
                            .size(px(24.))
                            .compact()
                            .ghost()
                            .on_click(cx.listener(|_this, _ev, _window, cx| {
                                TodoThreadEdit::add(cx);
                            })),
                    ),
            )
            .child(
                // 待办事项列表
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(self.todo_list.clone()),
            )
    }
}
