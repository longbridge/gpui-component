use anyhow::{Context as _, Result};
use gpui::*;
use serde::Deserialize;
use std::{sync::Arc, time::Duration};
use story::{
    AccordionStory, AppState, AppTitleBar, Assets, ButtonStory, CalendarStory, DropdownStory,
    FormStory, IconStory, ImageStory, InputStory, ListStory, ModalStory, Open, PopupStory,
    ProgressStory, Quit, ResizableStory, ScrollableStory, SidebarStory, StoryContainer,
    SwitchStory, TableStory, TextStory, TooltipStory,
};
use ui::{
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockAreaState, DockEvent, DockItem, DockPlacement},
    popup_menu::PopupMenuExt,
    IconName, Root, Sizable, Theme, TitleBar,
};

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct AddPanel(DockPlacement);

#[derive(Clone, PartialEq, Eq, Deserialize)]
pub struct TogglePanelVisible(SharedString);

impl_internal_actions!(story, [AddPanel, TogglePanelVisible]);

const MAIN_DOCK_AREA: DockAreaTab = DockAreaTab {
    id: "main-dock",
    version: 5,
};

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

    cx.on_action(|_action: &Open, _cx: &mut App| {});

    ui::init(cx);
    story::init(cx);
}

pub struct StoryWorkspace {
    title_bar: Entity<AppTitleBar>,
    dock_area: Entity<DockArea>,
    last_layout_state: Option<DockAreaState>,
    _save_layout_task: Option<Task<()>>,
}

struct DockAreaTab {
    id: &'static str,
    version: usize,
}

impl StoryWorkspace {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // There will crash on Linux.
        // https://github.com/longbridge/gpui-component/issues/104
        #[cfg(not(target_os = "linux"))]
        window
            .observe_window_appearance(|window, cx| {
                Theme::sync_system_appearance(Some(window), cx);
            })
            .detach();

        let dock_area =
            cx.new(|cx| DockArea::new(MAIN_DOCK_AREA.id, Some(MAIN_DOCK_AREA.version), window, cx));
        let weak_dock_area = dock_area.downgrade();

        match Self::load_layout(dock_area.clone(), window, cx) {
            Ok(_) => {
                println!("load layout success");
            }
            Err(err) => {
                eprintln!("load layout error: {:?}", err);
                Self::reset_default_layout(weak_dock_area, window, cx);
            }
        };

        cx.subscribe_in(
            &dock_area,
            window,
            |this, dock_area, ev: &DockEvent, window, cx| match ev {
                DockEvent::LayoutChanged => this.save_layout(dock_area, window, cx),
            },
        )
        .detach();

        cx.on_app_quit({
            let dock_area = dock_area.clone();
            move |_, cx| {
                let state = dock_area.read(cx).dump(cx);
                cx.background_executor().spawn(async move {
                    // Save layout before quitting
                    Self::save_state(&state).unwrap();
                })
            }
        })
        .detach();

        let title_bar = cx.new(|cx| {
            AppTitleBar::new("Examples", window, cx).child({
                move |_, cx| {
                    Button::new("add-panel")
                        .icon(IconName::LayoutDashboard)
                        .small()
                        .ghost()
                        .popup_menu({
                            let invisible_panels = AppState::global(cx).invisible_panels.clone();

                            move |menu, _, cx| {
                                menu.menu(
                                    "Add Panel to Center",
                                    Box::new(AddPanel(DockPlacement::Center)),
                                )
                                .separator()
                                .menu("Add Panel to Left", Box::new(AddPanel(DockPlacement::Left)))
                                .menu(
                                    "Add Panel to Right",
                                    Box::new(AddPanel(DockPlacement::Right)),
                                )
                                .menu(
                                    "Add Panel to Bottom",
                                    Box::new(AddPanel(DockPlacement::Bottom)),
                                )
                                .separator()
                                .menu_with_check(
                                    "Sidebar",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Sidebar")),
                                    Box::new(TogglePanelVisible(SharedString::from("Sidebar"))),
                                )
                                .menu_with_check(
                                    "Modal",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("SidebModalar")),
                                    Box::new(TogglePanelVisible(SharedString::from("Modal"))),
                                )
                                .menu_with_check(
                                    "Accordion",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("Accordion")),
                                    Box::new(TogglePanelVisible(SharedString::from("Accordion"))),
                                )
                                .menu_with_check(
                                    "List",
                                    !invisible_panels
                                        .read(cx)
                                        .contains(&SharedString::from("List")),
                                    Box::new(TogglePanelVisible(SharedString::from("List"))),
                                )
                            }
                        })
                        .anchor(Corner::TopRight)
                }
            })
        });

        Self {
            dock_area,
            title_bar,
            last_layout_state: None,
            _save_layout_task: None,
        }
    }

    fn save_layout(
        &mut self,
        dock_area: &Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dock_area = dock_area.clone();
        self._save_layout_task = Some(cx.spawn_in(window, |story, mut window| async move {
            Timer::after(Duration::from_secs(10)).await;

            _ = story.update_in(&mut window, move |this, _, cx| {
                let dock_area = dock_area.read(cx);
                let state = dock_area.dump(cx);

                let last_layout_state = this.last_layout_state.clone();
                if Some(&state) == last_layout_state.as_ref() {
                    return;
                }

                Self::save_state(&state).unwrap();
                this.last_layout_state = Some(state);
            });
        }));
    }

    fn save_state(state: &DockAreaState) -> Result<()> {
        println!("Save layout...");
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write("target/layout.json", json)?;
        Ok(())
    }

    fn load_layout(
        dock_area: Entity<DockArea>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let fname = "target/layout.json";
        let json = std::fs::read_to_string(fname)?;
        let state = serde_json::from_str::<DockAreaState>(&json)?;

        // Check if the saved layout version is different from the current version
        // Notify the user and ask if they want to reset the layout to default.
        if state.version != Some(MAIN_DOCK_AREA.version) {
            let answer = window.prompt(
                PromptLevel::Info,
                "The default main layout has been updated.\n\
                Do you want to reset the layout to default?",
                None,
                &["Yes", "No"],
                cx,
            );

            let weak_dock_area = dock_area.downgrade();
            cx.spawn_in(window, |this, mut window| async move {
                if answer.await == Ok(0) {
                    _ = this.update_in(&mut window, |_, window, cx| {
                        Self::reset_default_layout(weak_dock_area, window, cx);
                    });
                }
            })
            .detach();
        }

        dock_area.update(cx, |dock_area, cx| {
            dock_area.load(state, window, cx).context("load layout")?;
            dock_area.set_dock_collapsible(
                Edges {
                    left: true,
                    bottom: true,
                    right: true,
                    ..Default::default()
                },
                window,
                cx,
            );

            Ok::<(), anyhow::Error>(())
        })
    }

    fn reset_default_layout(dock_area: WeakEntity<DockArea>, window: &mut Window, cx: &mut App) {
        let dock_item = Self::init_default_layout(&dock_area, window, cx);

        let left_panels = DockItem::split_with_sizes(
            Axis::Vertical,
            vec![
                DockItem::tab(
                    StoryContainer::panel::<ListStory>(window, cx),
                    &dock_area,
                    window,
                    cx,
                ),
                DockItem::tabs(
                    vec![
                        Arc::new(StoryContainer::panel::<ScrollableStory>(window, cx)),
                        Arc::new(StoryContainer::panel::<AccordionStory>(window, cx)),
                    ],
                    None,
                    &dock_area,
                    window,
                    cx,
                ),
            ],
            vec![None, Some(px(360.))],
            &dock_area,
            window,
            cx,
        );

        let bottom_panels = DockItem::split_with_sizes(
            Axis::Vertical,
            vec![DockItem::tabs(
                vec![
                    Arc::new(StoryContainer::panel::<TooltipStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<IconStory>(window, cx)),
                ],
                None,
                &dock_area,
                window,
                cx,
            )],
            vec![None],
            &dock_area,
            window,
            cx,
        );

        let right_panels = DockItem::split_with_sizes(
            Axis::Vertical,
            vec![
                DockItem::tab(
                    StoryContainer::panel::<ImageStory>(window, cx),
                    &dock_area,
                    window,
                    cx,
                ),
                DockItem::tab(
                    StoryContainer::panel::<IconStory>(window, cx),
                    &dock_area,
                    window,
                    cx,
                ),
            ],
            vec![None],
            &dock_area,
            window,
            cx,
        );

        _ = dock_area.update(cx, |view, cx| {
            view.set_version(MAIN_DOCK_AREA.version, window, cx);
            view.set_center(dock_item, window, cx);
            view.set_left_dock(left_panels, Some(px(350.)), true, window, cx);
            view.set_bottom_dock(bottom_panels, Some(px(200.)), true, window, cx);
            view.set_right_dock(right_panels, Some(px(320.)), true, window, cx);

            Self::save_state(&view.dump(cx)).unwrap();
        });
    }

    fn init_default_layout(
        dock_area: &WeakEntity<DockArea>,
        window: &mut Window,
        cx: &mut App,
    ) -> DockItem {
        DockItem::split_with_sizes(
            Axis::Vertical,
            vec![DockItem::tabs(
                vec![
                    Arc::new(StoryContainer::panel::<ButtonStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<InputStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<DropdownStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<TextStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<ModalStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<PopupStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<SwitchStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<ProgressStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<TableStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<ImageStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<IconStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<TooltipStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<ProgressStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<CalendarStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<ResizableStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<ScrollableStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<AccordionStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<SidebarStory>(window, cx)),
                    Arc::new(StoryContainer::panel::<FormStory>(window, cx)),
                    // Arc::new(StoryContainer::panel::<WebViewStory>(window, cx)),
                ],
                None,
                &dock_area,
                window,
                cx,
            )],
            vec![None],
            &dock_area,
            window,
            cx,
        )
    }

    pub fn new_local(cx: &mut App) -> Task<anyhow::Result<WindowHandle<Root>>> {
        let mut window_size = size(px(1600.0), px(1200.0));
        if let Some(display) = cx.primary_display() {
            let display_size = display.bounds().size;
            window_size.width = window_size.width.min(display_size.width * 0.85);
            window_size.height = window_size.height.min(display_size.height * 0.85);
        }

        let window_bounds = Bounds::centered(None, window_size, cx);

        cx.spawn(|mut cx| async move {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(window_bounds)),
                #[cfg(not(target_os = "linux"))]
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(640.),
                    height: px(480.),
                }),
                #[cfg(target_os = "linux")]
                window_background: gpui::WindowBackgroundAppearance::Transparent,
                #[cfg(target_os = "linux")]
                window_decorations: Some(gpui::WindowDecorations::Client),
                kind: WindowKind::Normal,
                ..Default::default()
            };

            let window = cx.open_window(options, |window, cx| {
                let story_view = cx.new(|cx| Self::new(window, cx));
                cx.new(|cx| Root::new(story_view.into(), window, cx))
            })?;

            window
                .update(&mut cx, |_, window, cx| {
                    window.activate_window();
                    window.set_window_title("GPUI App");
                    cx.on_release(|_, cx| {
                        // exit app
                        cx.quit();
                    })
                    .detach();
                })
                .expect("failed to update window");

            Ok(window)
        })
    }

    fn on_action_add_panel(
        &mut self,
        action: &AddPanel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Random pick up a panel to add
        let panel = match rand::random::<usize>() % 18 {
            0 => Arc::new(StoryContainer::panel::<ButtonStory>(window, cx)),
            1 => Arc::new(StoryContainer::panel::<InputStory>(window, cx)),
            2 => Arc::new(StoryContainer::panel::<DropdownStory>(window, cx)),
            3 => Arc::new(StoryContainer::panel::<TextStory>(window, cx)),
            4 => Arc::new(StoryContainer::panel::<ModalStory>(window, cx)),
            5 => Arc::new(StoryContainer::panel::<PopupStory>(window, cx)),
            6 => Arc::new(StoryContainer::panel::<SwitchStory>(window, cx)),
            7 => Arc::new(StoryContainer::panel::<ProgressStory>(window, cx)),
            8 => Arc::new(StoryContainer::panel::<TableStory>(window, cx)),
            9 => Arc::new(StoryContainer::panel::<ImageStory>(window, cx)),
            10 => Arc::new(StoryContainer::panel::<IconStory>(window, cx)),
            11 => Arc::new(StoryContainer::panel::<TooltipStory>(window, cx)),
            12 => Arc::new(StoryContainer::panel::<ProgressStory>(window, cx)),
            13 => Arc::new(StoryContainer::panel::<CalendarStory>(window, cx)),
            14 => Arc::new(StoryContainer::panel::<ResizableStory>(window, cx)),
            15 => Arc::new(StoryContainer::panel::<ScrollableStory>(window, cx)),
            16 => Arc::new(StoryContainer::panel::<AccordionStory>(window, cx)),
            // 17 => Arc::new(StoryContainer::panel::<WebViewStory>(window, cx)),
            _ => Arc::new(StoryContainer::panel::<ButtonStory>(window, cx)),
        };

        self.dock_area.update(cx, |dock_area, cx| {
            dock_area.add_panel(panel, action.0, window, cx);
        });
    }

    fn on_action_toggle_panel_visible(
        &mut self,
        action: &TogglePanelVisible,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let panel_name = action.0.clone();
        let invisible_panels = AppState::global(cx).invisible_panels.clone();
        invisible_panels.update(cx, |names, cx| {
            if names.contains(&panel_name) {
                names.retain(|id| id != &panel_name);
            } else {
                names.push(panel_name);
            }
            cx.notify();
        });
        cx.notify();
    }
}

pub fn open_new(
    cx: &mut App,
    init: impl FnOnce(&mut Root, &mut Window, &mut Context<Root>) + 'static + Send,
) -> Task<()> {
    let task: Task<std::result::Result<WindowHandle<Root>, anyhow::Error>> =
        StoryWorkspace::new_local(cx);
    cx.spawn(|mut cx| async move {
        if let Some(root) = task.await.ok() {
            root.update(&mut cx, |workspace, window, cx| init(workspace, window, cx))
                .expect("failed to init workspace");
        }
    })
}

impl Render for StoryWorkspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drawer_layer = Root::render_drawer_layer(window, cx);
        let modal_layer = Root::render_modal_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .id("story-workspace")
            .on_action(cx.listener(Self::on_action_add_panel))
            .on_action(cx.listener(Self::on_action_toggle_panel_visible))
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(self.title_bar.clone())
            .child(self.dock_area.clone())
            .children(drawer_layer)
            .children(modal_layer)
            .child(div().absolute().top_8().children(notification_layer))
    }
}

fn main() {
    use ui::input::{Copy, Cut, Paste, Redo, Undo};

    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        init(cx);

        cx.on_action(quit);
        cx.set_menus(vec![
            Menu {
                name: "GPUI App".into(),
                items: vec![MenuItem::action("Quit", Quit)],
            },
            Menu {
                name: "Edit".into(),
                items: vec![
                    MenuItem::os_action("Undo", Undo, gpui::OsAction::Undo),
                    MenuItem::os_action("Redo", Redo, gpui::OsAction::Redo),
                    MenuItem::separator(),
                    MenuItem::os_action("Cut", Cut, gpui::OsAction::Cut),
                    MenuItem::os_action("Copy", Copy, gpui::OsAction::Copy),
                    MenuItem::os_action("Paste", Paste, gpui::OsAction::Paste),
                ],
            },
            Menu {
                name: "Window".into(),
                items: vec![],
            },
        ]);
        cx.activate(true);

        open_new(cx, |_, _, _| {
            // do something
        })
        .detach();
    });
}

fn quit(_: &Quit, cx: &mut App) {
    cx.quit();
}
