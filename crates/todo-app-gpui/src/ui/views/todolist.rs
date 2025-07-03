use std::collections::HashMap;

use super::todo_thread_edit::TodoThreadEdit;
use crate::ui::views::todo_thread::TodoThreadChat;
use crate::{config::todo_item::*, ui::views::todo_thread_edit::Save as TodoSaved};
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

actions!(
    todolist,
    [New, Open, Edit, Completed, Redo, Pause, Clone, Follow, Delete]
);

#[derive(IntoElement)]
pub struct TodoItem {
    base: ListItem,
    ix: usize,
    item: Todo,
    selected: bool,
}

impl TodoItem {
    pub fn new(id: impl Into<ElementId>, item: Todo, ix: usize, selected: bool) -> Self {
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
        let text_color = if self.selected {
            cx.theme().accent_foreground
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

        // 检查任务是否已完成
        let is_completed = self.item.status == TodoStatus::Done;

        let description_color = if is_completed {
            text_color.opacity(0.3)
        } else {
            text_color.opacity(0.8)
        };
        self.base
            .h_16()
            .bg(bg_color)
            .text_color(text_color)
            .text_sm()
            .child(
                h_flex()
                    .size_full() // 水平弹性布局
                    .items_center() // 垂直居中对齐
                    .justify_between() // 两端对齐
                    .gap_1() // 间距 2 单位
                    //.text_color(text_color) // 设置文本颜色
                    .child(
                        //左侧
                        v_flex()
                            .size_full()
                            .gap_1()
                            .items_center()
                            .justify_end()
                            .overflow_x_hidden()
                            .child(
                                div().text_ellipsis().child(
                                    Label::new(self.item.description.clone())
                                        .text_color(description_color)
                                        .when(is_completed, |mut this| {
                                            let style = this
                                                .text_style()
                                                .get_or_insert_with(Default::default);
                                            style.strikethrough = Some(StrikethroughStyle {
                                                thickness: px(1.),
                                                color: Some(Hsla::black()),
                                            });
                                            this.italic()
                                        }),
                                ),
                            )
                            .child(
                                h_flex()
                                    .size_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    //.text_color(text_color)
                                    .child(
                                        //todo信息
                                        h_flex()
                                            .items_center()
                                            .justify_start()
                                            .gap_2()
                                            .when(self.item.status == TodoStatus::Alert, |div| {
                                                div.child(
                                                    Icon::new(IconName::TriangleAlert)
                                                        .xsmall()
                                                        .text_color(yellow_500()),
                                                )
                                            })
                                            .child(Icon::new(IconName::Paperclip).xsmall())
                                            .child(if is_completed {
                                                // 已完成任务显示完成图标
                                                Icon::new(IconName::CircleCheck)
                                                    .xsmall()
                                                    .text_color(green_500())
                                            } else if self.item.status == TodoStatus::InProgress {
                                                // 进行中任务显示刷新图标
                                                Icon::new(IconName::RefreshCW).xsmall()
                                            } else {
                                                // 待办任务显示计时器图标
                                                Icon::new(IconName::TimerReset)
                                                    .xsmall()
                                                    .text_color(green_500())
                                            })
                                            .child(
                                                Label::new("10/01 17:36")
                                                    .whitespace_nowrap()
                                                    .text_xs()
                                                    .text_color(text_color.opacity(0.95)),
                                            ),
                                    )
                                    .when(
                                        !selected || self.item.status == TodoStatus::Done,
                                        |this| {
                                            this.child(
                                                // 模型信息 - 为已完成任务降低透明度
                                                h_flex()
                                                    .items_center()
                                                    .justify_end()
                                                    .gap_2()
                                                    .when(is_completed, |div| div.opacity(0.5))
                                                    .child(Icon::new(IconName::Ear).xsmall())
                                                    .child(Icon::new(IconName::Eye).xsmall())
                                                    .child(Icon::new(IconName::Image).xsmall())
                                                    .child(Icon::new(IconName::Brain).xsmall())
                                                    .child(Icon::new(IconName::Wrench).xsmall()),
                                            )
                                        },
                                    ),
                            ),
                    )
                    .when(
                        selected && self.item.status != TodoStatus::Done,
                        |this: Div| {
                            this.child(
                                v_flex()
                                    .h_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_1()
                                    .child(
                                        //右侧
                                        h_flex().gap_1().items_center().justify_end().when(
                                            selected,
                                            |div| {
                                                div.child(
                                                    h_flex()
                                                        .gap_1()
                                                        .items_center()
                                                        .justify_end()
                                                        .when(
                                                            self.item.status
                                                                == TodoStatus::InProgress,
                                                            |div| {
                                                                div.child(
                                                                    Indicator::new()
                                                                        .with_size(px(16.))
                                                                        .icon(IconName::RefreshCW)
                                                                        .color(blue_500()),
                                                                )
                                                            },
                                                        )
                                                        .when(
                                                            self.item.status
                                                                != TodoStatus::InProgress,
                                                            |div| {
                                                                div.child(
                                                                    Button::new("button-refresh")
                                                                        .ghost()
                                                                        .icon(IconName::RefreshCW)
                                                                        .small()
                                                                        .on_click(|_, win, app| {
                                                                            win.dispatch_action(
                                                                                Box::new(Redo),
                                                                                app,
                                                                            );
                                                                        }),
                                                                )
                                                            },
                                                        )
                                                        .child(
                                                            Button::new("button-copy")
                                                                .ghost()
                                                                .icon(IconName::Copy)
                                                                .small()
                                                                .on_click(|_, win, app| {
                                                                    win.dispatch_action(
                                                                        Box::new(Clone),
                                                                        app,
                                                                    );
                                                                }),
                                                        )
                                                        .child(
                                                            Button::new("button-star")
                                                                .ghost()
                                                                .when_else(
                                                                    self.item.follow,
                                                                    |this| {
                                                                        this.icon(
                                                                        Icon::new(IconName::Star)
                                                                            .xsmall()
                                                                            .text_color(
                                                                                yellow_500(),
                                                                            ),
                                                                    )
                                                                    },
                                                                    |this| {
                                                                        this.icon(IconName::Star)
                                                                    },
                                                                )
                                                                .small()
                                                                .on_click(|_, win, app| {
                                                                    win.dispatch_action(
                                                                        Box::new(Follow),
                                                                        app,
                                                                    );
                                                                }),
                                                        ),
                                                )
                                            },
                                        ),
                                    )
                                    .child(
                                        // 模型信息 - 为已完成任务降低透明度
                                        h_flex()
                                            .items_center()
                                            .justify_end()
                                            .gap_2()
                                            .when(is_completed, |div| div.opacity(0.5))
                                            .child(Icon::new(IconName::Ear).xsmall())
                                            .child(Icon::new(IconName::Eye).xsmall())
                                            .child(Icon::new(IconName::Image).xsmall())
                                            .child(Icon::new(IconName::Brain).xsmall())
                                            .child(Icon::new(IconName::Wrench).xsmall()),
                                    ),
                            )
                        },
                    ),
            )
    }
}

struct TodoListDelegate {
    matched_todos: Vec<Todo>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
}

impl ListDelegate for TodoListDelegate {
    type Item = TodoItem;

    fn items_count(&self, _: &App) -> usize {
        self.matched_todos.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_todos = TodoManager::list_todos()
            .iter()
            .filter(|todo| todo.id != VPA)
            .filter(|todo| {
                todo.description
                    .to_lowercase()
                    .contains(&query.to_lowercase())
            })
            .cloned()
            .collect();
        Task::ready(())
    }

    // fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
    //     // println!("Confirmed with secondary: {}", secondary);
    //     // window.dispatch_action(Box::new(SelectedCompany), cx);
    // }

    fn on_double_click(
        &mut self,
        ev: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        println!("Double clicked: {:?} {:?}", ev, self.selected_index);
        window.dispatch_action(Box::new(Open), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        // println!("Selected index: {:?}", ix);
        self.selected_index = ix;
        // Remove windows that are no longer active
        cx.notify();
    }

    fn render_item(
        &self,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_todos.get(ix) {
            return Some(TodoItem::new(ix, company.clone(), ix, selected));
        }
        None
    }

    fn context_menu(
        &self,
        row_ix: Option<usize>,
        menu: PopupMenu,
        _window: &Window,
        _cx: &App,
    ) -> PopupMenu {
        //     println!("Context menu for row: {}", row_ix);
        //    self.selected_index = Some(row_ix);
        menu.external_link_icon(true)
            //  .link("About", "https://github.com/longbridge/gpui-component")
            .when_some(row_ix, |menu, _row_idx| {
                menu.menu_with_icon("打开", IconName::NotepadText, Box::new(Open))
                    .menu_with_icon("编辑", IconName::SquarePen, Box::new(Edit))
                    .separator()
                    .menu_with_icon("挂起", IconName::Pause, Box::new(Pause))
                    .menu_with_icon("完成", IconName::Done, Box::new(Completed))
                    .menu_with_icon("关注", IconName::Star, Box::new(Follow))
                    .separator()
                    .menu_with_icon("克隆", IconName::Copy, Box::new(Clone))
                    .menu_with_icon("新建", IconName::FilePlus2, Box::new(New))
                    .separator()
            })
            .when_none(&row_ix, |menu| {
                menu.menu_with_icon("新建", IconName::FilePlus2, Box::new(New))
            })
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn can_load_more(&self, _: &App) -> bool {
        return !self.loading && !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<List<Self>>) {
        // cx.spawn_in(window, async move |view, window| {
        //     // Simulate network request, delay 1s to load data.
        //     Timer::after(Duration::from_secs(1)).await;

        //     _ = view.update_in(window, move |view, window, cx| {
        //         let query = view.delegate().query.clone();
        //         view.delegate_mut()
        //             .companies
        //             .extend((0..200).map(|_| random_todo()));
        //         _ = view.delegate_mut().perform_search(&query, window, cx);
        //         view.delegate_mut().eof = view.delegate().companies.len() >= 6000;
        //     });
        // })
        // .detach();
        // self.companies
        //     .extend((0..200).map(|_| self.manager.list_todos()));
    }
}

impl TodoListDelegate {
    fn selected_todo(&self) -> Option<Todo> {
        let Some(ix) = self.selected_index else {
            return None;
        };
        let todo = self.matched_todos.get(ix).cloned();
        todo
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TodoFilter {
    #[default]
    All,
    Planned,
    Completed,
    Recycle,
}
pub struct TodoList {
    focus_handle: FocusHandle,
    todo_list: Entity<List<TodoListDelegate>>,
    selected_todo: Option<Todo>,
    _subscriptions: Vec<Subscription>,
    todo_filter: TodoFilter,
    active_tab_ix: usize,
    opened_windows: HashMap<String, WindowHandle<Root>>,
    edited_windows: HashMap<String, WindowHandle<Root>>,
}

impl TodoList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        // window.on_window_should_close(cx, |win,app|{
        //     win.dispatch_action(Box::new(Quit), app);
        //     true
        // });
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = TodoListDelegate {
            matched_todos: TodoManager::list_todos()
                .into_iter()
                .filter(|todo| todo.id != VPA)
                .collect(),
            selected_index: None,
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let todo_list = cx.new(|cx| List::new(delegate, window, cx));
        let _subscriptions = vec![cx.subscribe(
            &todo_list,
            |this, _todo_list, ev: &ListEvent, cx| match ev {
                ListEvent::Select(ix) => {
                    println!("List Selected: {:?}", ix);
                    this.selected_todo(cx);
                }
                ListEvent::Confirm(ix) => {
                    println!("List Confirmed: {:?}", ix);
                    this.selected_todo(cx);
                }
                ListEvent::Cancel => {
                    println!("List Cancelled");
                }
            },
        )];

        let mut celf = Self {
            focus_handle: cx.focus_handle(),
            todo_list,
            selected_todo: None,
            _subscriptions,
            todo_filter: TodoFilter::default(),
            active_tab_ix: 0,
            opened_windows: HashMap::new(),
            edited_windows: HashMap::new(),
        };
        celf.set_active_tab(1, window, cx);
        celf
    }

    fn selected_todo(&mut self, cx: &mut Context<Self>) {
        let picker = self.todo_list.read(cx);
        self.selected_todo = picker.delegate().selected_todo();
        self.opened_windows
            .retain(|_, handle| handle.is_active(cx).is_some());
        self.edited_windows
            .retain(|_, handle| handle.is_active(cx).is_some());
    }
    fn follow_todo(&mut self, _: &Follow, window: &mut Window, cx: &mut Context<Self>) {
        println!("Follow action triggered");
        if let Some(mut todo) = self.selected_todo.clone() {
            todo.follow = !todo.follow;
            TodoManager::update_todo(todo).ok();
            self.set_active_tab(self.active_tab_ix, window, cx);
        }
    }
    fn redo_todo(&mut self, _: &Redo, window: &mut Window, cx: &mut Context<Self>) {
        println!("Redo action triggered");
        if let Some(mut todo) = self.selected_todo.clone() {
            todo.status = TodoStatus::Todo;
            TodoManager::update_todo(todo).ok();
            self.set_active_tab(self.active_tab_ix, window, cx);
        }
    }
    fn done_todo(&mut self, _: &Completed, window: &mut Window, cx: &mut Context<Self>) {
        println!("Completed action triggered");
        if let Some(mut todo) = self.selected_todo.clone() {
            todo.status = TodoStatus::Done;
            TodoManager::update_todo(todo).ok();
            self.set_active_tab(self.active_tab_ix, window, cx);
        }
    }
    fn pause_todo(&mut self, _: &Pause, window: &mut Window, cx: &mut Context<Self>) {
        println!("Completed action triggered");
        if let Some(mut todo) = self.selected_todo.clone() {
            if todo.status == TodoStatus::Suspended {
                todo.status = TodoStatus::Todo;
            } else {
                todo.status = TodoStatus::Suspended;
            }

            TodoManager::update_todo(todo).ok();
            self.set_active_tab(self.active_tab_ix, window, cx);
        }
    }
    fn clone_todo(&mut self, _: &Clone, window: &mut Window, cx: &mut Context<Self>) {
        println!("Clone action triggered");
        if let Some(todo) = self.selected_todo.clone() {
            TodoManager::copy_todo(&todo.id).ok();
            self.set_active_tab(self.active_tab_ix, window, cx);
        }
    }

    fn delete_todo(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = self.selected_todo.clone() {
            TodoManager::delete_todo(&todo.id).ok();
            self.set_active_tab(self.active_tab_ix, window, cx);
        }
    }

    fn new_todo(&mut self, _: &New, window: &mut Window, cx: &mut Context<Self>) {
        TodoThreadEdit::add(window, cx);
    }
    fn open_todo(&mut self, _: &Open, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = self.selected_todo.clone() {
            let todo_id = todo.id.clone();

            match self.opened_windows.get(&todo_id) {
                Some(handle) if handle.is_active(cx).is_some() => {
                    // Window exists and is active, just focus it
                    handle
                        .update(cx, |_, window, cx| {
                            window.activate_window();
                            cx.notify();
                        })
                        .ok();
                }
                _ => {
                    // Window doesn't exist or is inactive, create new one
                    let handle = TodoThreadChat::open(todo, cx);
                    self.opened_windows.insert(todo_id, handle);
                }
            }
        }
    }

    fn open_vpa(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = TodoManager::list_todos().iter().find(|todo| todo.id == VPA) {
            self.selected_todo = Some(todo.clone());
        } else {
            // 如果没有找到VPA任务，创建一个新的
            let new_todo = Todo::new_vpa();
            self.selected_todo = Some(new_todo.clone());
            TodoManager::update_todo(new_todo).ok();
        }
        self.open_todo(&Open, window, cx);
    }

    fn edit_todo(&mut self, _: &Edit, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = self.selected_todo.clone() {
            let todo_id = todo.id.clone();

            match self.edited_windows.get(&todo_id) {
                Some(handle) if handle.is_active(cx).is_some() => {
                    // Window exists and is active, just focus it
                    handle
                        .update(cx, |_, window, cx| {
                            window.activate_window();
                            cx.notify();
                        })
                        .ok();
                }
                _ => {
                    // Window doesn't exist or is inactive, create new one
                    let handle = TodoThreadEdit::edit(todo, window, cx);
                    self.edited_windows.insert(todo_id, handle);
                }
            }
        }
    }

    fn todo_updated(&mut self, _: &TodoSaved, window: &mut Window, cx: &mut Context<Self>) {
        println!("Todo updated");
        self.set_active_tab(self.active_tab_ix, window, cx);
    }

    fn set_todo_filter(&mut self, filter: TodoFilter, _: &mut Window, cx: &mut Context<Self>) {
        self.todo_filter = filter;
        self.todo_list.update(cx, |list, _cx| {
            let todos: Vec<Todo> = TodoManager::list_todos()
                .into_iter()
                .filter(|todo| todo.id != VPA)
                .collect();
            list.delegate_mut().matched_todos = match filter {
                TodoFilter::All => todos,
                TodoFilter::Planned => todos
                    .into_iter()
                    .filter(|todo| todo.status == TodoStatus::Todo)
                    .collect(),
                TodoFilter::Completed => todos
                    .into_iter()
                    .filter(|todo| todo.status == TodoStatus::Done)
                    .collect(),
                TodoFilter::Recycle => todos
                    .into_iter()
                    .filter(|todo| todo.status == TodoStatus::Deleted)
                    .collect(),
            };
            list.delegate_mut().selected_index = None;
            list.delegate_mut().confirmed_index = None;
        });
        cx.notify();
    }

    fn set_active_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        println!("Set active tab: {}", ix);
        self.active_tab_ix = ix;
        match ix {
            0 => self.set_todo_filter(TodoFilter::All, window, cx),
            1 => self.set_todo_filter(TodoFilter::Planned, window, cx),
            2 => self.set_todo_filter(TodoFilter::Completed, window, cx),
            3 => self.set_todo_filter(TodoFilter::Recycle, window, cx),
            _ => {}
        }
        cx.notify();
    }
    fn set_scroll(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        println!("Scroll to: {}", ix);
        match ix {
            0 => self.todo_list.update(cx, |list, cx| {
                list.scroll_to_item(0, window, cx);
            }),

            1 => self.todo_list.update(cx, |list, cx| {
                if let Some(selected) = list.selected_index() {
                    list.scroll_to_item(selected, window, cx);
                }
            }),
            2 => self.todo_list.update(cx, |list, cx| {
                list.scroll_to_item(
                    if list.delegate().items_count(cx) == 0 {
                        0
                    } else {
                        list.delegate().items_count(cx) - 1
                    },
                    window,
                    cx,
                );
            }),
            _ => {}
        }
        cx.notify();
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
            .on_action(cx.listener(Self::done_todo))
            .on_action(cx.listener(Self::follow_todo))
            .on_action(cx.listener(Self::redo_todo))
            .on_action(cx.listener(Self::clone_todo))
            .on_action(cx.listener(Self::pause_todo))
            .on_action(cx.listener(Self::new_todo))
            .on_action(cx.listener(Self::open_todo))
            .on_action(cx.listener(Self::edit_todo))
            .on_action(cx.listener(Self::delete_todo))
            .on_action(cx.listener(Self::todo_updated))
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
                            .children(vec!["全部", "计划中", "已完成"])
                            .suffix(
                                h_flex()
                                    .mx_1()
                                    .child(
                                        Button::new("bot-message-square-button")
                                            .ghost()
                                            .small()
                                            .icon(IconName::BotMessageSquare)
                                            .tooltip("助理")
                                            .on_click(cx.listener(|this, _ev, window, cx| {
                                                this.open_vpa(window, cx);
                                            })),
                                    )
                                    // .child(
                                    //     Button::new("bot-off-button")
                                    //         .ghost()
                                    //         .small()
                                    //         .icon(IconName::BotOff)
                                    //         .tooltip("休息"), // .on_click(cx.listener(|_this, _ev, window, cx| {
                                    //                           //     //window.dispatch_action(Box::new(New), cx);
                                    //                           // })),
                                    // )
                                    .child(
                                        Button::new("plus-button")
                                            .ghost()
                                            .small()
                                            .icon(IconName::Plus)
                                            .tooltip("新建待办")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                println!("New Todo clicked");
                                                this.new_todo(&New, window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        ButtonGroup::new("button-group")
                            .small()
                            .child(
                                Button::new("icon-button-top")
                                    .icon(IconName::ArrowUpToLine)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .child(
                                Button::new("icon-button-selected")
                                    .icon(IconName::MousePointerClick)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .child(
                                Button::new("icon-button-bottom")
                                    .icon(IconName::ArrowDownToLine)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .on_click(cx.listener(|this, clicked: &Vec<usize>, window, cx| {
                                if clicked.contains(&0) {
                                    this.set_scroll(0, window, cx);
                                } else if clicked.contains(&1) {
                                    this.set_scroll(1, window, cx);
                                } else if clicked.contains(&2) {
                                    this.set_scroll(2, window, cx);
                                }
                            })),
                    ), // .child(
                       //     Button::new("icon-button-add")
                       //         .icon(IconName::Plus)
                       //         .size(px(24.))
                       //         .compact()
                       //         .ghost()
                       //         .on_click(cx.listener(|_this, _ev, window, cx| {
                       //             window.dispatch_action(Box::new(New), cx);
                       //         })),
                       // ),
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
