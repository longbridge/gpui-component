use chrono::Days;
use fake::Fake;
use gpui::prelude::*;
use gpui::*;
use std::time::Duration;

use gpui_component::{
    button::{Button, ButtonGroup, ButtonVariants},
    date_picker::{DatePicker, DatePickerEvent, DatePickerState, DateRangePreset},
    h_flex, hsl,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    tab::TabBar,
    v_flex, ActiveTheme, IconName, Selectable, Sizable, Size, *,
};
use story::Story;

use super::todo_view::TodoView;

actions!(list_story, [SelectedCompany]);

#[derive(Clone, Default)]
struct Todo {
    name: SharedString,
    industry: SharedString,
    last_done: f64,
    prev_close: f64,

    change_percent: f64,
    change_percent_str: SharedString,
    last_done_str: SharedString,
    prev_close_str: SharedString,
    // description: String,
}

impl Todo {
    fn prepare(mut self) -> Self {
        self.change_percent = (self.last_done - self.prev_close) / self.prev_close;
        self.change_percent_str = format!("{:.2}%", self.change_percent).into();
        self.last_done_str = format!("{:.2}", self.last_done).into();
        self.prev_close_str = format!("{:.2}", self.prev_close).into();
        self
    }

    fn random_update(&mut self) {
        self.last_done = self.prev_close * (1.0 + (-0.2..0.2).fake::<f64>());
    }
}

#[derive(IntoElement)]
struct TodoItem {
    base: ListItem,
    ix: usize,
    company: Todo,
    selected: bool,
    date_picker: Entity<DatePickerState>,
    hovered: bool,
    star: bool,
    alert: bool,
    completed: bool,
}

impl TodoItem {
    pub fn new(
        id: impl Into<ElementId>,
        company: Todo,
        ix: usize,
        selected: bool,

        date_picker: Entity<DatePickerState>,
    ) -> Self {
        TodoItem {
            company,
            ix,
            base: ListItem::new(id),
            selected,
            date_picker,
            hovered: false,
            star: false,
            alert: false,
            completed: false,
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

        let trend_color = match self.company.change_percent {
            change if change > 0.0 => hsl(0.0, 79.0, 53.0),
            change if change < 0.0 => hsl(100.0, 79.0, 53.0),
            _ => cx.theme().foreground,
        };

        let bg_color = if self.selected {
            cx.theme().list_active
        } else if self.ix % 2 == 0 {
            cx.theme().list
        } else {
            cx.theme().list_even
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
                            .flex_nowrap()
                            .child(Label::new(self.company.name.clone()).whitespace_nowrap())
                            .child(
                                div().text_sm().overflow_x_hidden().child(
                                    Label::new(self.company.industry.clone())
                                        .whitespace_nowrap()
                                        .text_color(text_color.opacity(0.5)),
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
                                        .when(self.alert, |div| {
                                            div.child(
                                                Button::new("button-alert")
                                                    .ghost()
                                                    .icon(IconName::TriangleAlert)
                                                    .xsmall()
                                                    .on_click(|event, win, app| {}),
                                            )
                                        })
                                        .child(
                                            Button::new("button-redo")
                                                .ghost()
                                                .icon(IconName::Redo)
                                                .xsmall()
                                                .on_click(|event, win, app| {}),
                                        )
                                        .child(
                                            Button::new("button-copy")
                                                .ghost()
                                                .icon(IconName::Copy)
                                                .xsmall()
                                                .on_click(|event, win, app| {}),
                                        )
                                        .child(
                                            Button::new("button-completed")
                                                .ghost()
                                                .icon(IconName::Timer)
                                                .xsmall()
                                                .when(self.completed, |this| {
                                                    this.icon(IconName::Done)
                                                })
                                                .on_click(|event, win, app| {
                                                    // Toggle completed state
                                                    // Note: This would need to be handled at a higher level
                                                    // since TodoItem doesn't have mutable access to itself in the click handler
                                                }),
                                        )
                                        .child(
                                            Button::new("button-star")
                                                .ghost()
                                                .icon(IconName::Star)
                                                .xsmall()
                                                .on_click(|event, win, app| {}),
                                        )
                                        .child(
                                            Button::new("button-trash")
                                                .ghost()
                                                .icon(IconName::Trash)
                                                .xsmall()
                                                .on_click(|event, win, app| {}),
                                        ),
                                )
                            })
                            .child(
                                h_flex()
                                    // .child(IconName::Calendar)
                                    .child(
                                        Label::new("10/01 17:36")
                                            .whitespace_nowrap()
                                            .text_xs()
                                            .text_color(text_color.opacity(0.5)),
                                    )
                                    .on_mouse_up(MouseButton::Left, |a, b, c| {
                                        println!("Mouse up on date picker: {:?}", a,);
                                    }),
                            ),
                    ),
            )
    }
}

struct TodoListDelegate {
    companies: Vec<Todo>,
    matched_companies: Vec<Todo>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
}

impl ListDelegate for TodoListDelegate {
    type Item = TodoItem;

    fn items_count(&self, _: &App) -> usize {
        self.matched_companies.len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Task<()> {
        self.query = query.to_string();
        self.matched_companies = self
            .companies
            .iter()
            .filter(|company| company.name.to_lowercase().contains(&query.to_lowercase()))
            .cloned()
            .collect();
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<List<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedCompany), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<usize>,
        _: &mut Window,
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
        if let Some(company) = self.matched_companies.get(ix) {
            let now = chrono::Local::now().naive_local().date();

            let date_picker = cx.new(|cx| {
                let mut picker = DatePickerState::new(window, cx);
                picker.set_disabled(
                    calendar::Matcher::interval(Some(now), now.checked_add_days(Days::new(5))),
                    window,
                    cx,
                );
                picker.set_date(now, window, cx);
                picker
            });
            return Some(TodoItem::new(
                ix,
                company.clone(),
                ix,
                selected,
                date_picker,
            ));
        }

        None
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
        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut()
                    .companies
                    .extend((0..200).map(|_| random_company()));
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate().companies.len() >= 6000;
            });
        })
        .detach();
    }
}

impl TodoListDelegate {
    fn selected_company(&self) -> Option<Todo> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.companies.get(ix).cloned()
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
    company_list: Entity<List<TodoListDelegate>>,
    selected_company: Option<Todo>,
    _subscriptions: Vec<Subscription>,
    todo_filter: TodoFilter,
    active_tab_ix: usize,
}

impl Story for TodoList {
    fn title() -> &'static str {
        "Todo List"
    }

    fn description() -> &'static str {
        "The list displays a series of to-do items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl TodoList {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let companies = (0..1_000).map(|_| random_company()).collect::<Vec<Todo>>();

        let delegate = TodoListDelegate {
            matched_companies: companies.clone(),
            companies,
            selected_index: Some(0),
            confirmed_index: None,
            query: "".to_string(),
            loading: false,
            eof: false,
        };

        let company_list = cx.new(|cx| List::new(delegate, window, cx));
        // company_list.update(cx, |list, cx| {
        //     list.set_selected_index(Some(3), cx);
        // });
        let _subscriptions =
            vec![
                cx.subscribe(&company_list, |_, _, ev: &ListEvent, _| match ev {
                    ListEvent::Select(ix) => {
                        println!("List Selected: {:?}", ix);
                    }
                    ListEvent::Confirm(ix) => {
                        println!("List Confirmed: {:?}", ix);
                    }
                    ListEvent::Cancel => {
                        println!("List Cancelled");
                    }
                }),
            ];

        // Spawn a background to random refresh the list
        cx.spawn(async move |this, cx| {
            this.update(cx, |this, cx| {
                this.company_list.update(cx, |picker, _| {
                    picker
                        .delegate_mut()
                        .companies
                        .iter_mut()
                        .for_each(|company| {
                            company.random_update();
                        });
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            company_list,
            selected_company: None,
            _subscriptions,
            todo_filter: TodoFilter::default(),
            active_tab_ix: 0,
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_company() {
            self.selected_company = Some(company);
        }
    }

    fn set_todo_filter(&mut self, filter: TodoFilter, _: &mut Window, cx: &mut Context<Self>) {
        self.todo_filter = filter;
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
            0 => self.company_list.update(cx, |list, cx| {
                list.scroll_to_item(0, window, cx);
            }),

            1 => self.company_list.update(cx, |list, cx| {
                if let Some(selected) = list.selected_index() {
                    list.scroll_to_item(selected, window, cx);
                }
            }),
            2 => self.company_list.update(cx, |list, cx| {
                list.scroll_to_item(list.delegate().items_count(cx) - 1, window, cx);
            }),
            _ => {}
        }
        cx.notify();
    }
}

fn random_company() -> Todo {
    let last_done = (0.0..999.0).fake::<f64>();
    let prev_close = last_done * (-0.1..0.1).fake::<f64>();

    Todo {
        name: fake::faker::company::en::CompanyName()
            .fake::<String>()
            .into(),
        industry: fake::faker::company::en::Industry().fake::<String>().into(),
        last_done,
        prev_close,
        ..Default::default()
    }
    .prepare()
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
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(
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
                            .children(vec!["All", "Planned", "Completed"]),
                    )
                    .child(
                        ButtonGroup::new("button-group")
                            .small()
                            .child(
                                Button::new("icon-button-top")
                                    .icon(IconName::ChevronUp)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .child(
                                Button::new("icon-button-top-bottom")
                                    .icon(IconName::ChevronsUpDown)
                                    .size(px(24.))
                                    .ghost(),
                            )
                            .child(
                                Button::new("icon-button-bottom")
                                    .icon(IconName::ChevronDown)
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
                            .on_click(cx.listener(|this, ev, widnow, cx| {
                                // let _ = cx.open_window(WindowOptions::default(), TodoView::view);
                                cx.activate(true);
                                let window_size = size(px(600.0), px(800.0));
                                let window_bounds = Bounds::centered(None, window_size, cx);
                                let options = WindowOptions {
                                    app_id: Some("x-todo-app".to_string()),
                                    window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                                    titlebar: None,
                                    window_min_size: Some(gpui::Size {
                                        width: px(600.),
                                        height: px(800.),
                                    }),

                                    kind: WindowKind::PopUp,
                                    #[cfg(target_os = "linux")]
                                    window_background:
                                        gpui::WindowBackgroundAppearance::Transparent,
                                    #[cfg(target_os = "linux")]
                                    window_decorations: Some(gpui::WindowDecorations::Client),
                                    ..Default::default()
                                };
                                story::create_new_window_options(
                                    "xTodo",
                                    options,
                                    move |window, cx| TodoView::view(window, cx),
                                    cx,
                                );
                            })),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(self.company_list.clone()),
            )
    }
}
