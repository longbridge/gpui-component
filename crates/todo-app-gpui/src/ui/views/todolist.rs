use super::todo_thread_edit::TodoThreadEdit;
use crate::models::todo_item::*;
use crate::ui::views::todo_thread::TodoThreadChat;
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
        SelectedCompany,
        Open,
        Edit,
        Completed,
        Pause,
        Clone,
        Star,
        Delete
    ]
);

#[derive(IntoElement)]
pub struct TodoItem {
    base: ListItem,
    ix: usize,
    item: Todo,
    selected: bool,
    star: bool,
}

impl TodoItem {
    pub fn new(id: impl Into<ElementId>, item: Todo, ix: usize, selected: bool) -> Self {
        TodoItem {
            item,
            ix,
            base: ListItem::new(id),
            selected,
            star: false,
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

        // 为已完成任务设置不同的文本颜色和透明度
        let title_color = if is_completed {
            text_color.opacity(0.6)
        } else {
            text_color
        };

        let description_color = if is_completed {
            text_color.opacity(0.3)
        } else {
            text_color.opacity(0.5)
        };

        self.base
            .px_3()
            .py_1()
            .overflow_x_hidden()
            .bg(bg_color)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        v_flex()
                            .gap_1()
                            .max_w(px(500.))
                            .overflow_x_hidden()
                            .text_xs()
                            .child(
                                // 标题 - 为已完成任务添加删除线
                                div().child(
                                    Label::new(self.item.title.clone())
                                        .whitespace_nowrap()
                                        .text_color(title_color)
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
                                // 描述 - 为已完成任务添加删除线
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
                            ),
                    )
                    .child(
                        v_flex()
                            .h_full()
                            .gap_1()
                            .items_end()
                            .justify_end()
                            .when(selected, |div| {
                                div.child(
                                    h_flex()
                                        .gap_1()
                                        .items_center()
                                        .justify_end()
                                        .when(self.item.status == TodoStatus::InProgress, |div| {
                                            div.child(
                                                Indicator::new()
                                                    .with_size(px(16.))
                                                    .icon(IconName::RefreshCW)
                                                    .color(blue_500()),
                                            )
                                        })
                                        .when(self.item.status != TodoStatus::InProgress, |div| {
                                            div.child(
                                                Button::new("button-refresh")
                                                    .ghost()
                                                    .icon(IconName::RefreshCW)
                                                    .small()
                                                    .on_click(|event, win, app| {}),
                                            )
                                        })
                                        .child(
                                            Button::new("button-copy")
                                                .ghost()
                                                .icon(IconName::Copy)
                                                .small()
                                                .on_click(|event, win, app| {}),
                                        )
                                        .child(
                                            Button::new("button-star")
                                                .ghost()
                                                .icon(IconName::Star)
                                                .small()
                                                .on_click(|event, win, app| {}),
                                        ),
                                )
                            })
                            .child(
                                h_flex().child(
                                    Label::new("10/01 17:36")
                                        .whitespace_nowrap()
                                        .text_xs()
                                        .text_color(text_color.opacity(0.5)),
                                ),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
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
                            }),
                    )
                    .child(
                        // 模型信息 - 为已完成任务降低透明度
                        h_flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .when(is_completed, |div| div.opacity(0.5))
                            .child(Icon::new(IconName::Mic).xsmall())
                            .child(Icon::new(IconName::Image).xsmall())
                            .child(Icon::new(IconName::Brain).xsmall())
                            .child(Icon::new(IconName::Wrench).xsmall()),
                    ),
            )
    }
}

struct TodoListDelegate {
    todos: Vec<Todo>,
    matched_todos: Vec<Todo>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
    manager: TodoManager,
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
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_todos = self
            .todos
            .iter()
            .filter(|todo| todo.title.to_lowercase().contains(&query.to_lowercase()))
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
        window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        println!("Selected index: {:?}", ix);
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
        if let Some(company) = self.matched_todos.get(ix) {
            return Some(TodoItem::new(ix, company.clone(), ix, selected));
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
        println!("Context menu for row: {}", row_ix);
        // self.selected_index = Some(row_ix);
        menu.external_link_icon(true)
            //  .link("About", "https://github.com/longbridge/gpui-component")
            .menu("打开", Box::new(Open))
            .menu("编辑", Box::new(Edit))
            .separator()
            .menu_with_icon("克隆", IconName::Copy, Box::new(Clone))
            .menu_with_icon("暂停", IconName::Pause, Box::new(Pause))
            .menu_with_icon("完成", IconName::Done, Box::new(Completed))
            .menu_with_icon("关注", IconName::Star, Box::new(Completed))
            // .separator()
            // .menu_with_check("删除", true, Box::new(ToggleCheck))
            .separator()
            .menu_with_icon("删除", IconName::Trash, Box::new(Delete))
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

        self.todos.get(ix).cloned()
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TodoFilter {
    #[default]
    All,
    Planned,
    Completed,
}
pub struct TodoList {
    focus_handle: FocusHandle,
    todo_list: Entity<List<TodoListDelegate>>,
    selected_todo: Option<Todo>,
    _subscriptions: Vec<Subscription>,
    todo_filter: TodoFilter,
    active_tab_ix: usize,
}

impl TodoList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let manager = TodoManager::create_fake_data();

        let todos = manager.list_todos();

        let delegate = TodoListDelegate {
            matched_todos: todos.clone(),
            todos,
            selected_index: None,
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
            manager,
        };

        let todo_list = cx.new(|cx| List::new(delegate, window, cx));
        let _subscriptions = vec![cx.subscribe(
            &todo_list,
            |this, _todo_list, ev: &ListEvent, cx| match ev {
                ListEvent::Select(ix) => {
                    println!("List Selected: {:?}", ix);
                }
                ListEvent::Confirm(ix) => {
                    this.selected_todo(cx);
                    println!("List Confirmed: {:?}", ix);
                }
                ListEvent::Cancel => {
                    println!("List Cancelled");
                }
            },
        )];

        // Spawn a background to random refresh the list
        // cx.spawn(async move |this, cx| {
        //     this.update(cx, |this, cx| {
        //         this.company_list.update(cx, |picker, _| {
        //             picker
        //                 .delegate_mut()
        //                 .companies
        //                 .iter_mut()
        //                 .for_each(|company| {
        //                     company.random_update();
        //                 });
        //         });
        //         cx.notify();
        //     })
        //     .ok();
        // })
        // .detach();

        let mut celf = Self {
            focus_handle: cx.focus_handle(),
            todo_list,
            selected_todo: None,
            _subscriptions,
            todo_filter: TodoFilter::default(),
            active_tab_ix: 0,
        };
        celf.set_active_tab(1, window, cx);
        celf
    }

    fn selected_todo(&mut self, cx: &mut Context<Self>) {
        println!("Selected todo action triggered");
        let picker = self.todo_list.read(cx);
        self.selected_todo = picker.delegate().selected_todo();
    }

    fn clone(&mut self, _: &Clone, _: &mut Window, cx: &mut Context<Self>) {
        println!("Clone action triggered");
    }

    fn open_todo(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        // self.company_list.update(cx, update)
        println!("Open action triggered");
        if let Some(todo) = self.selected_todo.clone() {
            println!("Opening todo: {}", todo.title);
            TodoThreadChat::open(todo, cx);
        }
    }

    fn edit_todo(&mut self, _: &Edit, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(todo) = self.selected_todo.clone() {
            TodoThreadEdit::edit(todo, cx);
        }
    }

    fn set_todo_filter(&mut self, filter: TodoFilter, _: &mut Window, cx: &mut Context<Self>) {
        self.todo_filter = filter;
        self.todo_list.update(cx, |list, _cx| {
            let todos = list.delegate_mut().todos.clone();
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
            };
            list.delegate_mut().selected_index = None;
            list.delegate_mut().confirmed_index = None;
        });
        cx.notify();
    }

    fn set_active_tab(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        match ix {
            0 => self.set_todo_filter(TodoFilter::All, window, cx),
            1 => self.set_todo_filter(TodoFilter::Planned, window, cx),
            2 => self.set_todo_filter(TodoFilter::Completed, window, cx),
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
                list.scroll_to_item(list.delegate().items_count(cx) - 1, window, cx);
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
            //  .on_action(cx.listener(Self::selected_company))
            .on_action(cx.listener(Self::clone))
            .on_action(cx.listener(Self::open_todo))
            .on_action(cx.listener(Self::edit_todo))
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
                            .children(vec!["全部", "计划中", "已完成"]),
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
                    )
                    .child(
                        Button::new("icon-button-add")
                            .icon(IconName::Plus)
                            .size(px(24.))
                            .compact()
                            .ghost()
                            .on_click(cx.listener(|_this, _ev, _widnow, cx| {
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
