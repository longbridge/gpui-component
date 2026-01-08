use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, div, px,
};

use gpui_component::{
    ActiveTheme as _,
    button::Button,
    dock::{DockArea, DockItem, DockPlacement},
    h_flex, v_flex,
};

use crate::{Story, section};

// Simple panel for demonstration
struct SimpleDockPanel {
    title: SharedString,
    content: Vec<SharedString>,
    focus_handle: FocusHandle,
}

impl gpui::EventEmitter<gpui_component::dock::PanelEvent> for SimpleDockPanel {}

impl gpui::Focusable for SimpleDockPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl gpui_component::dock::Panel for SimpleDockPanel {
    fn panel_name(&self) -> &'static str {
        "SimpleDockPanel"
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.title.clone()
    }
}

impl Render for SimpleDockPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .p_4()
            .gap_2()
            .children(
                self.content.iter().map(|line| {
                    div()
                        .text_sm()
                        .text_color(cx.theme().foreground)
                        .child(line.clone())
                })
            )
    }
}


pub struct DockStory {
    focus_handle: FocusHandle,
    dock_area: Entity<DockArea>,
}

impl Story for DockStory {
    fn title() -> &'static str {
        "Dock"
    }

    fn description() -> &'static str {
        "Resizable dock panels at window edges"
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render> {
        Self::view(window, cx)
    }
}

impl DockStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dock_area = cx.new(|cx| {
            let mut area = DockArea::new("dock-story", None, window, cx);
            
            let weak_area = cx.entity().downgrade();
            
            // Create a simple panel for left dock
            let left_panel = cx.new(|cx| SimpleDockPanel {
                title: "Explorer".into(),
                content: vec![
                    "📁 src/".into(),
                    "  📄 main.rs".into(),
                    "  📄 lib.rs".into(),
                    "📁 tests/".into(),
                    "  📄 integration.rs".into(),
                ],
                focus_handle: cx.focus_handle(),
            });
            
            area.set_left_dock(
                DockItem::tab(left_panel, &weak_area, window, cx),
                Some(px(250.)),
                true,
                window,
                cx,
            );
            
            // Create a simple panel for right dock
            let right_panel = cx.new(|cx| SimpleDockPanel {
                title: "Properties".into(),
                content: vec![
                    "Width: 300px".into(),
                    "Height: 200px".into(),
                    "Position: Absolute".into(),
                    "".into(),
                    "Try resizing this panel!".into(),
                ],
                focus_handle: cx.focus_handle(),
            });
            
            area.set_right_dock(
                DockItem::tab(right_panel, &weak_area, window, cx),
                Some(px(300.)),
                true,
                window,
                cx,
            );
            
            // Create a simple panel for bottom dock
            let bottom_panel = cx.new(|cx| SimpleDockPanel {
                title: "Console".into(),
                content: vec![
                    "> Dock Story loaded successfully".into(),
                    "> Resize handles are active".into(),
                    "> Drag the edges to resize panels".into(),
                ],
                focus_handle: cx.focus_handle(),
            });
            
            area.set_bottom_dock(
                DockItem::tab(bottom_panel, &weak_area, window, cx),
                Some(px(200.)),
                true,
                window,
                cx,
            );
            
            area
        });

        Self {
            focus_handle: cx.focus_handle(),
            dock_area,
        }
    }

    fn toggle_dock(&mut self, placement: DockPlacement, window: &mut Window, cx: &mut Context<Self>) {
        self.dock_area.update(cx, |area, cx| {
            area.toggle_dock(placement, window, cx);
        });
    }
}

impl Focusable for DockStory {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DockStory {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let left_open = self.dock_area.read(cx).is_dock_open(DockPlacement::Left, cx);
        let right_open = self.dock_area.read(cx).is_dock_open(DockPlacement::Right, cx);
        let bottom_open = self.dock_area.read(cx).is_dock_open(DockPlacement::Bottom, cx);

        div()
            .id("dock-story")
            .track_focus(&self.focus_handle)
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .gap_6()
                    .child(
                        section("Dock Controls")
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child(
                                        Button::new("toggle-left")
                                            .outline()
                                            .label(if left_open { "Hide Left Dock" } else { "Show Left Dock" })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_dock(DockPlacement::Left, window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("toggle-right")
                                            .outline()
                                            .label(if right_open { "Hide Right Dock" } else { "Show Right Dock" })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_dock(DockPlacement::Right, window, cx);
                                            })),
                                    )
                                    .child(
                                        Button::new("toggle-bottom")
                                            .outline()
                                            .label(if bottom_open { "Hide Bottom Dock" } else { "Show Bottom Dock" })
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.toggle_dock(DockPlacement::Bottom, window, cx);
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        section("Dock Display")
                            .child(
                                div()
                                    .flex_1()
                                    .min_h(px(400.))
                                    .w_full()
                                    .overflow_hidden()
                                    .child(self.dock_area.clone()),
                            ),
                    ),
            )
    }
}
