use tray_icon::menu::MenuEvent;
use tray_icon::{
    menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::backoffice::cross_runtime::CrossRuntimeBridge;

#[derive(Debug, Clone)]
pub enum TrayEvent {
    Menu(MenuEvent),
    Tray(TrayIconEvent),
}
const ICON: &[u8] = include_bytes!("../../../../assets/logo0.png");
fn load_icon() -> tray_icon::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(ICON)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    tray_icon::Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
}

pub(crate) fn start_tray() -> anyhow::Result<TrayIcon> {
    let icon = load_icon();
    let tray_menu = Menu::new();
    let shown = MenuItem::with_id("SHOW_MAIN", "打开", true, None);
    let exit = MenuItem::with_id("EXIT_MAIN", "退出", true, None);
    tray_menu
        .append_items(&[
            &shown,
            &PredefinedMenuItem::about(
                Some("关于"),
                Some(AboutMetadata {
                    name: Some("xTo-Do".to_string()),
                    copyright: Some("© xTo-Do 2025".to_string()),
                    ..Default::default()
                }),
            ),
            &PredefinedMenuItem::separator(),
            &exit,
        ])
        .ok();
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        // .with_title("")
        .with_tooltip("xTo-Do 您的工作助理")
        .with_icon(icon)
        // .with_menu_on_left_click(false)
        .build()?;

    MenuEvent::set_event_handler(Some(move |event| {
        CrossRuntimeBridge::global().emit(TrayEvent::Menu(event));
    }));
    TrayIconEvent::set_event_handler(Some(move |event| {
        CrossRuntimeBridge::global().emit(TrayEvent::Tray(event));
    }));
    Ok(tray_icon)
}
