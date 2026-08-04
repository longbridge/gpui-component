use gpui::{
    App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement as _, Render,
    RenderOnce, SharedString, Styled as _, Subscription, Window, div, prelude::FluentBuilder,
};
use gpui_component::{
    Icon, IconNamed, icon_named,
    icon_picker::{IconPicker, IconPickerEvent, IconPickerState},
    v_flex,
};
use strum::EnumIter;

use crate::section;

icon_named!(PickableIcon, "../assets/assets/icons", [Copy, EnumIter]);

impl RenderOnce for PickableIcon {
    fn render(self, _: &mut Window, _cx: &mut App) -> impl IntoElement {
        Icon::new(self)
    }
}

pub struct IconPickerStory {
    icon: Entity<IconPickerState<PickableIcon>>,
    selected_icon: Option<PickableIcon>,
    _subscriptions: Vec<Subscription>,
}

impl super::Story for IconPickerStory {
    fn title() -> &'static str {
        "IconPicker"
    }

    fn description() -> &'static str {
        "An icon picker to select an icon."
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl IconPickerStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let icon = cx.new(|cx| {
            IconPickerState::new(window, cx).default_value(PickableIcon::GalleryVerticalEnd)
        });

        let _subscriptions = vec![cx.subscribe(&icon, |this, _, ev, _| match ev {
            IconPickerEvent::Change(icon) => {
                this.selected_icon = *icon;
            }
        })];

        Self {
            icon,
            selected_icon: Some(PickableIcon::GalleryVerticalEnd),
            _subscriptions,
        }
    }
}

impl Focusable for IconPickerStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        self.icon.read(cx).focus_handle(cx)
    }
}

impl Render for IconPickerStory {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3().child(
            section("Normal")
                .max_w_md()
                .child(IconPicker::new(&self.icon))
                .when_some(self.selected_icon, |this, icon| {
                    this.child(div().w_48().child(icon.name()))
                }),
        )
    }
}
