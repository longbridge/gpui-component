use std::{rc::Rc, time::Duration};

use fake::Fake;
use gpui::{
    App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, RenderOnce, ScrollStrategy, SharedString, Styled,
    Subscription, Task, Timer, Window, actions, div, px,
};

use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Selectable, Sizable,
    button::Button,
    checkbox::Checkbox,
    h_flex,
    label::Label,
    list::{List, ListDelegate, ListEvent, ListItem, ListState},
    v_flex,
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
}

#[derive(IntoElement)]
struct CompanyListItem {
    base: ListItem,
    company: Rc<Company>,
    selected: bool,
}

impl CompanyListItem {
    pub fn new(id: impl Into<ElementId>, company: Rc<Company>, selected: bool) -> Self {
        CompanyListItem {
            company,
            base: ListItem::new(id).selected(selected),
            selected,
        }
    }
}

impl Selectable for CompanyListItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
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
            change if change > 0.0 => cx.theme().green,
            change if change < 0.0 => cx.theme().red,
            _ => cx.theme().foreground,
        };

        self.base
            .px_2()
            .py_1()
            .overflow_x_hidden()
            .border_1()
            .rounded(cx.theme().radius)
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .text_color(text_color)
                    .child(
                        h_flex().gap_2().child(
                            v_flex()
                                .gap_1()
                                .max_w(px(500.))
                                .overflow_x_hidden()
                                .flex_nowrap()
                                .child(Label::new(self.company.name.clone()).whitespace_nowrap()),
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
    industries: Vec<SharedString>,
    _companies: Vec<Rc<Company>>,
    matched_companies: Vec<Vec<Rc<Company>>>,
    selected_index: Option<IndexPath>,
    confirmed_index: Option<IndexPath>,
    query: SharedString,
    loading: bool,
    eof: bool,
    lazy_load: bool,
}

impl CompanyListDelegate {
    fn prepare(&mut self, query: impl Into<SharedString>) {
        self.query = query.into();
        let companies: Vec<Rc<Company>> = self
            ._companies
            .iter()
            .filter(|company| {
                company
                    .name
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .cloned()
            .collect();
        for company in companies.into_iter() {
            if let Some(ix) = self.industries.iter().position(|s| s == &company.industry) {
                self.matched_companies[ix].push(company);
            } else {
                self.industries.push(company.industry.clone());
                self.matched_companies.push(vec![company]);
            }
        }
    }

    fn extend_more(&mut self, len: usize) {
        self._companies
            .extend((0..len).map(|_| Rc::new(random_company())));
        self.prepare(self.query.clone());
    }

    fn selected_company(&self) -> Option<Rc<Company>> {
        let Some(ix) = self.selected_index else {
            return None;
        };

        self.matched_companies
            .get(ix.section)
            .and_then(|c| c.get(ix.row))
            .cloned()
    }
}

impl ListDelegate for CompanyListDelegate {
    type Item = CompanyListItem;

    fn sections_count(&self, _: &App) -> usize {
        self.industries.len()
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.matched_companies[section].len()
    }

    fn perform_search(
        &mut self,
        query: &str,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.prepare(query.to_owned());
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        println!("Confirmed with secondary: {}", secondary);
        window.dispatch_action(Box::new(SelectedCompany), cx);
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let Some(industry) = self.industries.get(section) else {
            return None;
        };

        Some(
            h_flex()
                .pb_1()
                .px_2()
                .gap_2()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::Folder))
                .child(industry.clone()),
        )
    }

    fn render_section_footer(
        &mut self,
        section: usize,
        _: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let Some(_) = self.industries.get(section) else {
            return None;
        };

        Some(
            div()
                .pt_1()
                .pb_5()
                .px_2()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!(
                    "Total {} items in section.",
                    self.matched_companies[section].len()
                )),
        )
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let selected = Some(ix) == self.selected_index || Some(ix) == self.confirmed_index;
        if let Some(company) = self.matched_companies[ix.section].get(ix.row) {
            return Some(CompanyListItem::new(ix, company.clone(), selected));
        }

        None
    }

    fn loading(&self, _: &App) -> bool {
        self.loading
    }

    fn has_more(&self, _: &App) -> bool {
        if self.loading {
            return false;
        }

        return !self.eof;
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        if !self.lazy_load {
            return;
        }

        cx.spawn_in(window, async move |view, window| {
            // Simulate network request, delay 1s to load data.
            Timer::after(Duration::from_secs(1)).await;

            _ = view.update_in(window, move |view, window, cx| {
                let query = view.delegate().query.clone();
                view.delegate_mut().extend_more(200);
                _ = view.delegate_mut().perform_search(&query, window, cx);
                view.delegate_mut().eof = view.delegate()._companies.len() >= 6000;
            });
        })
        .detach();
    }
}

pub struct ListStory {
    focus_handle: FocusHandle,
    company_list: Entity<ListState<CompanyListDelegate>>,
    selected_company: Option<Rc<Company>>,
    selectable: bool,
    searchable: bool,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for ListStory {
    fn title() -> &'static str {
        "List"
    }

    fn description() -> &'static str {
        "A list displays a series of items."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ListStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut delegate = CompanyListDelegate {
            industries: vec![],
            matched_companies: vec![vec![]],
            _companies: vec![],
            selected_index: Some(IndexPath::default()),
            confirmed_index: None,
            query: "".into(),
            loading: false,
            eof: false,
            lazy_load: false,
        };
        delegate.extend_more(100);

        let company_list = cx.new(|cx| ListState::new(delegate, window, cx).searchable(true));

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
                        ._companies
                        .iter_mut()
                        .for_each(|company| {
                            let mut new_company = random_company();
                            new_company.name = company.name.clone();
                            new_company.industry = company.industry.clone();
                            *company = Rc::new(new_company);
                        });
                    picker.delegate_mut().prepare("");
                });
                cx.notify();
            })
            .ok();
        })
        .detach();

        Self {
            focus_handle: cx.focus_handle(),
            searchable: true,
            selectable: true,
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

    fn toggle_selectable(&mut self, selectable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.selectable = selectable;
        self.company_list.update(cx, |list, cx| {
            list.set_selectable(self.selectable, cx);
        })
    }

    fn toggle_searchable(&mut self, searchable: bool, _: &mut Window, cx: &mut Context<Self>) {
        self.searchable = searchable;
        self.company_list.update(cx, |list, cx| {
            list.set_searchable(self.searchable, cx);
        })
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
        let lazy_load = self.company_list.read(cx).delegate().lazy_load;

        v_flex()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::selected_company))
            .size_full()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("scroll-top")
                            .outline()
                            .child("Scroll to Top")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        IndexPath::default(),
                                        ScrollStrategy::Top,
                                        window,
                                        cx,
                                    );
                                    cx.notify();
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-selected")
                            .outline()
                            .child("Scroll to selected")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_selected_item(window, cx);
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-to-item")
                            .outline()
                            .child("Scroll to (5, 1)")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    list.scroll_to_item(
                                        IndexPath::new(1).section(5),
                                        ScrollStrategy::Center,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Button::new("scroll-bottom")
                            .outline()
                            .child("Scroll to Bottom")
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.company_list.update(cx, |list, cx| {
                                    let last_section =
                                        list.delegate().sections_count(cx).saturating_sub(1);

                                    list.scroll_to_item(
                                        IndexPath::default().section(last_section).row(
                                            list.delegate()
                                                .items_count(last_section, cx)
                                                .saturating_sub(1),
                                        ),
                                        ScrollStrategy::Top,
                                        window,
                                        cx,
                                    );
                                })
                            })),
                    )
                    .child(
                        Checkbox::new("selectable")
                            .label("Selectable")
                            .checked(self.selectable)
                            .on_click(cx.listener(|this, check: &bool, window, cx| {
                                this.toggle_selectable(*check, window, cx)
                            })),
                    )
                    .child(
                        Checkbox::new("searchable")
                            .label("Searchable")
                            .checked(self.searchable)
                            .on_click(cx.listener(|this, check: &bool, window, cx| {
                                this.toggle_searchable(*check, window, cx)
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
                    )
                    .child(
                        Checkbox::new("lazy_load")
                            .label("Lazy Load")
                            .checked(lazy_load)
                            .on_click(cx.listener(|this, check: &bool, _, cx| {
                                this.company_list.update(cx, |this, cx| {
                                    this.delegate_mut().lazy_load = *check;
                                    cx.notify();
                                })
                            })),
                    ),
            )
            .child(
                List::new(&self.company_list)
                    .p(px(8.))
                    .flex_1()
                    .w_full()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius),
            )
    }
}
