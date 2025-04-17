use std::time::Duration;

use fake::Fake;
use gpui::{
    actions, div, px, App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, RenderOnce, SharedString, Styled,
    Subscription, Task, Timer, Window,
};

use gpui_component::{
    button::Button,
    checkbox::Checkbox,
    h_flex, hsl,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem},
    v_flex, ActiveTheme, Sizable,
};

actions!(list_story, [SelectedCompany]);

#[derive(Clone, Default)]
struct Company {
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

impl Company {
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
struct CompanyListItem {
    base: ListItem,
    ix: usize,
    company: Company,
    selected: bool,
}

impl CompanyListItem {
    pub fn new(id: impl Into<ElementId>, company: Company, ix: usize, selected: bool) -> Self {
        CompanyListItem {
            company,
            ix,
            base: ListItem::new(id),
            selected,
        }
    }
}

impl RenderOnce for CompanyListItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
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
                        h_flex()
                            .gap_2()
                            .items_center()
                            .justify_end()
                            .child(
                                div()
                                    .w(px(65.))
                                    .text_color(text_color)
                                    .child(self.company.last_done_str.clone()),
                            )
                            .child(
                                h_flex().w(px(65.)).justify_end().child(
                                    div()
                                        .rounded(cx.theme().radius)
                                        .whitespace_nowrap()
                                        .text_size(px(12.))
                                        .px_1()
                                        .text_color(trend_color)
                                        .child(self.company.change_percent_str.clone()),
                                ),
                            ),
                    ),
            )
    }
}

struct CompanyListDelegate {
    companies: Vec<Company>,
    matched_companies: Vec<Company>,
    selected_index: Option<usize>,
    confirmed_index: Option<usize>,
    query: String,
    loading: bool,
    eof: bool,
}

impl ListDelegate for CompanyListDelegate {
    type Item = CompanyListItem;

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
        _: &mut Window,
        _: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_companies.get(ix) {
            return Some(CompanyListItem::new(ix, company.clone(), ix, selected));
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

impl CompanyListDelegate {
    fn selected_company(&self) -> Option<Company> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.companies.get(ix).cloned()
    }
}

pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<List<CompanyListDelegate>>,
    selected_company: Option<Company>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for ListStory {
    fn title() -> &'static str {
        "List"
    }

    fn description() -> &'static str {
        "A list displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl ListStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let companies = (0..1_000)
            .map(|_| random_company())
            .collect::<Vec<Company>>();

        let delegate = CompanyListDelegate {
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
        }
    }

    fn selected_company(&mut self, _: &SelectedCompany, _: &mut Window, cx: &mut Context<Self>) {
        let picker = self.company_list.read(cx);
        if let Some(company) = picker.delegate().selected_company() {
            self.selected_company = Some(company);
        }
    }
}

fn random_company() -> Company {
    let last_done = (0.0..999.0).fake::<f64>();
    let prev_close = last_done * (-0.1..0.1).fake::<f64>();

    Company {
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

impl Focusable for ListStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ListStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("scroll-top")
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(0, window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        list.delegate().items_count(cx) - 1,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-selected")
                            .child("Scroll to Selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    if let Some(selected) = list.selected_index() {
                                        list.scroll_to_item(selected, window, cx);
                                    }
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("loading")
                            .label("Loading")
                            .checked(self.company_list.read(cx).delegate().loading)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.company_list.update(cx, |this, cx| {
                                    this.delegate_mut().loading = *check;
                                    cx.notify();
                                })
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
