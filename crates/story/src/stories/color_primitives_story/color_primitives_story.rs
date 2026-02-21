use gpui::{
    AnyElement, App, AppContext, Context, Entity, Focusable, IntoElement, ParentElement, Render,
    Styled, Subscription, Window,
};
use gpui_component::{
    tab::{Tab, TabBar},
    v_flex,
};

use crate::stories::color_primitives_story::{
    story_color_arc_tab::StoryColorArcTab, story_color_field_tab::StoryColorFieldTab,
    story_color_ring_tab::StoryColorRingTab, story_color_slider_tab::StoryColorSliderTab,
    story_compositions_tab::StoryCompositionsTab,
};

pub struct ColorPrimitivesStory {
    focus_handle: gpui::FocusHandle,
    active_tab_ix: usize,
    color_slider_tab: Entity<StoryColorSliderTab>,
    color_arc_tab: Entity<StoryColorArcTab>,
    color_ring_tab: Entity<StoryColorRingTab>,
    compositions_tab: Entity<StoryCompositionsTab>,
    color_field_tab: Entity<StoryColorFieldTab>,

    _subscriptions: Vec<Subscription>,
}

impl super::super::Story for ColorPrimitivesStory {
    fn title() -> &'static str {
        "ColorPrimitives"
    }

    fn description() -> &'static str {
        "Color components for composing color focused tools"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl ColorPrimitivesStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _initial_color = gpui::red();

        let color_slider_tab = StoryColorSliderTab::view(window, cx);
        let color_arc_tab = StoryColorArcTab::view(window, cx);
        let color_ring_tab = StoryColorRingTab::view(window, cx);
        let compositions_tab = StoryCompositionsTab::view(window, cx);
        let color_field_tab = StoryColorFieldTab::view(window, cx);

        let _subscriptions = vec![];

        Self {
            focus_handle: cx.focus_handle(),
            active_tab_ix: 0,
            color_slider_tab,
            color_arc_tab,
            color_ring_tab,
            color_field_tab,

            compositions_tab,

            _subscriptions,
        }
    }

    fn set_active_tab(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.active_tab_ix = ix;
        cx.notify();
    }
}

impl Focusable for ColorPrimitivesStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ColorPrimitivesStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs: [(&str, AnyElement); 5] = [
            (
                "Color Slider",
                self.color_slider_tab.clone().into_any_element(),
            ),
            ("Color Arc", self.color_arc_tab.clone().into_any_element()),
            ("Color Ring", self.color_ring_tab.clone().into_any_element()),
            (
                "Color Field",
                self.color_field_tab.clone().into_any_element(),
            ),
            (
                "Compositions",
                self.compositions_tab.clone().into_any_element(),
            ),
        ];

        let active_ix = self.active_tab_ix.min(tabs.len().saturating_sub(1));

        let tab_bar = tabs.iter().fold(
            TabBar::new("color-primitives-tabs")
                .w_full()
                .underline()
                .selected_index(active_ix)
                .on_click(cx.listener(|this, ix: &usize, _window, cx| {
                    this.set_active_tab(*ix, cx);
                })),
            |bar, (label, _)| bar.child(Tab::new().label(*label)),
        );

        let active_panel = tabs
            .into_iter()
            .nth(active_ix)
            .map(|(_, panel)| panel)
            .unwrap_or_else(|| self.color_slider_tab.clone().into_any_element());

        v_flex()
            .w_full()
            .items_center()
            .gap_y_6()
            .pb_8()
            .child(tab_bar)
            .child(active_panel)
    }
}
