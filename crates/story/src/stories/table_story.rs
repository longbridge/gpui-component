use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, ParentElement,
    Render, Styled, Window, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Selectable as _, Sizable, Size,
    button::{Button, ButtonGroup},
    h_flex,
    table::{
        Table, TableBody, TableCaption, TableCell, TableFooter, TableHead, TableHeader, TableRow,
    },
    tag::Tag,
    v_flex,
};

use crate::section;

pub struct TableStory {
    focus_handle: FocusHandle,
    size: Size,
}

impl TableStory {
    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self { focus_handle: cx.focus_handle(), size: Size::default() }
    }

    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn set_size(&mut self, size: Size, _: &mut Window, cx: &mut Context<Self>) {
        self.size = size;
        cx.notify();
    }
}

impl super::Story for TableStory {
    fn title() -> &'static str {
        "Table"
    }

    fn description() -> &'static str {
        "A basic table component for directly rendering tabular data."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl Focusable for TableStory {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

fn status_tag(status: &str) -> Tag {
    match status {
        "Paid" => Tag::success().outline().child(status.to_string()),
        "Pending" => Tag::warning().outline().child(status.to_string()),
        "Unpaid" => Tag::danger().outline().child(status.to_string()),
        _ => Tag::new().child(status.to_string()),
    }
    .xsmall()
}

impl Render for TableStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let invoices: Vec<(&str, &str, &str, &str, &str)> = vec![
            ("INV001", "Paid", "Credit Card", "$250.00", "2024-01-15"),
            ("INV002", "Pending", "PayPal", "$150.00", "2024-02-01"),
            ("INV003", "Unpaid", "Bank Transfer", "$350.00", "2024-02-15"),
            ("INV004", "Paid", "Credit Card", "$450.00", "2024-03-01"),
            ("INV005", "Paid", "PayPal", "$550.00", "2024-03-15"),
            ("INV006", "Pending", "Bank Transfer", "$200.00", "2024-04-01"),
            ("INV007", "Unpaid", "Credit Card", "$300.00", "2024-04-15"),
        ];

        v_flex()
            .gap_6()
            .size_full()
            .child(
                h_flex().gap_3().child(
                    ButtonGroup::new("toggle-size")
                        .outline()
                        .compact()
                        .child(
                            Button::new("xsmall")
                                .label("XSmall")
                                .selected(self.size == Size::XSmall),
                        )
                        .child(
                            Button::new("small").label("Small").selected(self.size == Size::Small),
                        )
                        .child(
                            Button::new("medium")
                                .label("Medium")
                                .selected(self.size == Size::Medium),
                        )
                        .child(
                            Button::new("large").label("Large").selected(self.size == Size::Large),
                        )
                        .on_click(cx.listener(|this, selecteds: &Vec<usize>, window, cx| {
                            let size = match selecteds[0] {
                                0 => Size::XSmall,
                                1 => Size::Small,
                                2 => Size::Medium,
                                3 => Size::Large,
                                _ => unreachable!(),
                            };
                            this.set_size(size, window, cx);
                        })),
                ),
            )
            .child(
                section("Table").child(
                    Table::new()
                        .with_size(self.size)
                        .child(
                            TableHeader::new().child(
                                TableRow::new()
                                    .child(TableHead::new().w(px(100.)).child("Invoice"))
                                    .child(TableHead::new().col_span(2).child("Status"))
                                    .child(TableHead::new().text_right().child("Amount"))
                                    .child(TableHead::new().text_right().child("Date")),
                            ),
                        )
                        .child(TableBody::new().children(invoices.iter().map(
                            |(invoice, status, method, amount, date)| {
                                TableRow::new()
                                    .child(TableCell::new().w(px(100.)).child(invoice.to_string()))
                                    .child(TableCell::new().child(status_tag(status)))
                                    .child(TableCell::new().child(method.to_string()))
                                    .child(TableCell::new().text_right().child(amount.to_string()))
                                    .child(TableCell::new().text_right().child(date.to_string()))
                            },
                        )))
                        .child(
                            TableFooter::new().child(
                                TableRow::new()
                                    .child(TableCell::new().col_span(3).child("Total"))
                                    .child(
                                        TableCell::new()
                                            .col_span(2)
                                            .text_right()
                                            .child("$2,250.00"),
                                    ),
                            ),
                        )
                        .child(TableCaption::new().child("A list of your recent invoices.")),
                ),
            )
            .child(
                section("With Border").child(
                    Table::new()
                        .with_size(self.size)
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .child(
                            TableHeader::new().child(
                                TableRow::new()
                                    .child(TableHead::new().w(px(100.)).child("Invoice"))
                                    .child(TableHead::new().child("Status"))
                                    .child(TableHead::new().child("Method"))
                                    .child(TableHead::new().text_right().child("Amount"))
                                    .child(TableHead::new().text_right().child("Date")),
                            ),
                        )
                        .child(TableBody::new().children(invoices.iter().enumerate().take(6).map(
                            |(ix, (invoice, status, method, amount, date))| {
                                TableRow::new()
                                    .when(ix % 2 != 0, |this| this.bg(cx.theme().table_even))
                                    .child(TableCell::new().w(px(100.)).child(invoice.to_string()))
                                    .child(TableCell::new().child(status_tag(status)))
                                    .child(TableCell::new().child(method.to_string()))
                                    .child(TableCell::new().text_right().child(amount.to_string()))
                                    .child(TableCell::new().text_right().child(date.to_string()))
                            },
                        ))),
                ),
            )
    }
}
