mod components;

use gpui::{
    App, AppContext as _, Application, Context, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, ScrollHandle, StatefulInteractiveElement as _, Styled as _, Window,
    WindowOptions, div, prelude::FluentBuilder as _, px, rgb, size,
};
use gpui_base::input::InputEditorStyle;
use gpui_base::input::InputState;
use gpui_base::slider::SliderState;
use gpui_base::{
    Accordion, AccordionHeader, AccordionItem, AccordionPanel, AccordionTrigger, AlertDialog,
    AlertDialogAction, AlertDialogBackdrop, AlertDialogCancel, AlertDialogDescription,
    AlertDialogPopup, AlertDialogTitle, Avatar, AvatarFallback, Button, Calendar, CalendarItemKind,
    CalendarState, Checkbox, CheckboxIndicator, CheckboxState, Collapsible, Combobox, DatePicker,
    Dialog, DialogBackdrop, DialogDescription, DialogPopup, DialogTitle, HoverCard, Input,
    OtpState, Popup, Scrollbar, ScrollbarMode, Select, Sheet, Slider, SliderIndicator, SliderThumb,
    SliderTrack, Switch, SwitchThumb, SwitchTrack, Tab, Table, TableBody, TableCell, TableHead,
    TableHeader, TableRow, Tabs, Toast, ToastTransitionStatus, Toggle, ToggleGroup, Tooltip, Tree,
    TreeItem, TreeState, v_virtual_list,
};
#[cfg(target_family = "wasm")]
use std::borrow::Cow;
use std::rc::Rc;

pub const COMPONENTS: &[&str] = &[
    "accordion",
    "alert-dialog",
    "avatar",
    "button",
    "calendar",
    "checkbox",
    "collapsible",
    "color-picker",
    "combobox",
    "date-picker",
    "dialog",
    "hover-card",
    "input",
    "link",
    "number-input",
    "otp-input",
    "pagination",
    "popover",
    "popup",
    "progress",
    "radio",
    "radio-group",
    "resizable",
    "scrollbar",
    "select",
    "sheet",
    "slider",
    "switch",
    "table",
    "tabs",
    "toast",
    "toggle",
    "toggle-group",
    "tooltip",
    "tree",
    "virtual-list",
];

pub struct BaseShowcase {
    component: String,
    checkbox_checked: bool,
    radio_selected: usize,
    switch_checked: bool,
    toggle_pressed: bool,
    toggle_group_selection: u8,
    selected_tab: usize,
    select_open: bool,
    select_index: usize,
    sheet_open: bool,
    toast_visible: bool,
    tooltip_visible: bool,
    accordion_items: [bool; 3],
    alert_dialog_open: bool,
    collapsible_open: bool,
    combobox_open: bool,
    combobox_query: gpui::Entity<InputState>,
    combobox_selection: String,
    color_index: usize,
    date_open: bool,
    dialog_open: bool,
    popup_open: bool,
    page: usize,
    slider: gpui::Entity<SliderState>,
    input: gpui::Entity<InputState>,
    multiline_input: gpui::Entity<InputState>,
    otp: gpui::Entity<OtpState>,
    calendar: gpui::Entity<CalendarState>,
    tree: gpui::Entity<TreeState>,
    date_focus: gpui::FocusHandle,
    scroll: ScrollHandle,
    example_scroll: ScrollHandle,
}

impl BaseShowcase {
    pub fn new(component: impl Into<String>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let component = component.into();
        let input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .placeholder("Type something…")
                .default_value(if component == "number-input" {
                    "12"
                } else {
                    "Hello GPUI"
                });
            state.set_editor_style(InputEditorStyle {
                foreground: rgb(0x171717).into(),
                muted_foreground: rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: rgb(0x171717).into(),
                ..InputEditorStyle::default()
            });
            state
        });
        let otp = cx.new(|cx| OtpState::new(6, window, cx).default_value("12"));
        let multiline_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .multi_line(true)
                .default_value("Build focused interfaces.\nKeep behavior composable.");
            state.set_editor_style(InputEditorStyle {
                foreground: rgb(0x171717).into(),
                muted_foreground: rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: rgb(0x171717).into(),
                ..InputEditorStyle::default()
            });
            state
        });
        let combobox_query = cx.new(|cx| {
            let mut state = InputState::new(window, cx).placeholder("Search frameworks…");
            state.set_editor_style(InputEditorStyle {
                foreground: rgb(0x171717).into(),
                muted_foreground: rgb(0x737373).into(),
                selection: gpui::hsla(0.6, 0.8, 0.7, 0.45),
                caret: rgb(0x171717).into(),
                ..InputEditorStyle::default()
            });
            state
        });
        cx.subscribe(
            &combobox_query,
            |_, _, _: &gpui_base::input::InputEvent, cx| cx.notify(),
        )
        .detach();
        if matches!(component.as_str(), "input" | "number-input") {
            input.update(cx, |state, cx| state.focus(window, cx));
        } else if component == "otp-input" {
            otp.update(cx, |state, cx| state.focus(window, cx));
        }

        let slider = cx.new(|_| SliderState::new().min(0.).max(100.).default_value(64.));
        cx.observe(&slider, |_, _, cx| cx.notify()).detach();

        Self {
            component,
            checkbox_checked: true,
            radio_selected: 0,
            switch_checked: true,
            toggle_pressed: true,
            toggle_group_selection: 0,
            selected_tab: 0,
            select_open: false,
            select_index: 0,
            sheet_open: false,
            toast_visible: false,
            tooltip_visible: false,
            accordion_items: [true, false, false],
            alert_dialog_open: false,
            collapsible_open: false,
            combobox_open: false,
            combobox_query,
            combobox_selection: "Select framework".into(),
            color_index: 3,
            date_open: false,
            dialog_open: false,
            popup_open: false,
            page: 3,
            slider,
            input,
            multiline_input,
            otp,
            calendar: cx.new(|cx| CalendarState::new(window, cx)),
            tree: cx.new(|cx| {
                TreeState::new(cx).items(vec![
                    TreeItem::new("src", "src").expanded(true).children(vec![
                        TreeItem::new("components", "components")
                            .expanded(true)
                            .children(vec![
                                TreeItem::new("button", "button.rs"),
                                TreeItem::new("tree-file", "tree.rs"),
                            ]),
                        TreeItem::new("lib", "lib.rs"),
                    ]),
                    TreeItem::new("examples", "examples")
                        .children(vec![TreeItem::new("showcase", "showcase.rs")]),
                    TreeItem::new("cargo", "Cargo.toml"),
                ])
            }),
            date_focus: cx.focus_handle(),
            scroll: ScrollHandle::new(),
            example_scroll: ScrollHandle::new(),
        }
    }
}

impl Render for BaseShowcase {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.component.as_str() {
            "accordion" => self.accordion(cx).into_any_element(),
            "alert-dialog" => self.alert_dialog(cx).into_any_element(),
            "avatar" => self.avatar().into_any_element(),
            "button" => self.button().into_any_element(),
            "calendar" => self.calendar().into_any_element(),
            "checkbox" => self.checkbox(cx).into_any_element(),
            "collapsible" => self.collapsible(cx).into_any_element(),
            "color-picker" => self.color_picker(cx).into_any_element(),
            "combobox" => self.combobox(window, cx).into_any_element(),
            "date-picker" => self.date_picker(cx).into_any_element(),
            "dialog" => self.dialog(cx).into_any_element(),
            "hover-card" => self.hover_card().into_any_element(),
            "input" => self.input().into_any_element(),
            "link" => self.link().into_any_element(),
            "number-input" => self.number_input(cx).into_any_element(),
            "otp-input" => self.otp_input(cx).into_any_element(),
            "pagination" => self.pagination(cx).into_any_element(),
            "popover" => self.popover().into_any_element(),
            "popup" => self.popup(cx).into_any_element(),
            "progress" => self.progress().into_any_element(),
            "radio" => self.radio(cx).into_any_element(),
            "radio-group" => self.radio_group(cx).into_any_element(),
            "resizable" => self.resizable().into_any_element(),
            "scrollbar" => self.scrollbar().into_any_element(),
            "slider" => self.slider(cx).into_any_element(),
            "select" => self.select(false, cx).into_any_element(),
            "sheet" => self.sheet(cx).into_any_element(),
            "switch" => self.switch(cx).into_any_element(),
            "table" => self.table().into_any_element(),
            "tabs" => self.tabs(cx).into_any_element(),
            "toast" => self.toast(cx).into_any_element(),
            "toggle" => self.toggle(cx).into_any_element(),
            "toggle-group" => self.toggle_group(cx).into_any_element(),
            "tooltip" => self.tooltip(cx).into_any_element(),
            "tree" => self.tree().into_any_element(),
            "virtual-list" => self.virtual_list(cx).into_any_element(),
            _ => div()
                .flex()
                .flex_wrap()
                .gap_3()
                .children(COMPONENTS.iter().map(|name| {
                    div()
                        .px_3()
                        .py_2()
                        .border_1()
                        .border_color(rgb(0xd4d4d4))
                        .child(*name)
                }))
                .into_any_element(),
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0xffffff))
            .text_color(rgb(0x171717))
            .text_xs()
            .font_family("Inter Variable")
            .child(
                div()
                    .id("showcase-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll)
                    .child(
                        div()
                            .min_h_full()
                            .w_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .p_4()
                            .child(div().flex_none().child(content)),
                    ),
            )
    }
}

pub fn run(app: Application, component: impl Into<String>) {
    let component = component.into();
    app.run(move |cx: &mut App| {
        gpui_base::init(cx);
        #[cfg(target_family = "wasm")]
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../story-web/fonts/Inter-Regular.ttf").as_slice(),
            )])
            .expect("failed to load gpui-base example font");
        cx.open_window(WindowOptions::default(), move |window, cx| {
            cx.new(|cx| BaseShowcase::new(component, window, cx))
        })
        .expect("failed to open gpui-base example window");
        cx.activate(true);
    });
}

#[cfg(target_family = "wasm")]
pub fn run_embedded(app: Application, component: impl Into<String>) -> gpui::ApplicationHandle {
    let component = component.into();
    app.run_embedded(move |cx: &mut App| {
        gpui_base::init(cx);
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(
                include_bytes!("../../../story-web/fonts/Inter-Regular.ttf").as_slice(),
            )])
            .expect("failed to load gpui-base example font");
        cx.open_window(WindowOptions::default(), move |window, cx| {
            cx.new(|cx| BaseShowcase::new(component, window, cx))
        })
        .expect("failed to open gpui-base example window");
        cx.activate(true);
    })
}

#[cfg(not(target_family = "wasm"))]
pub fn run_native(component: &str) {
    run(gpui_platform::application(), component.to_owned());
}
