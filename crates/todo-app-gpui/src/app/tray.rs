use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use anyhow::Ok;
use tray_item::{IconSource, TrayItem};
pub type Tray = TrayItem;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TrayEvent {
    RightClickTrayIcon,
    LeftClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    Open,
    Hide,
}

pub(crate) fn start_tray() -> anyhow::Result<Tray> {
    #[cfg(target_os = "windows")]
    const ICON_NAME: &str = "tray-default";
    #[cfg(not(target_os = "windows"))]
    const ICON_NAME: &str = "";
    let mut tray = TrayItem::new("xTo-Do Utility", IconSource::Resource(ICON_NAME))?;

    tray.add_label("xTo-Do 实用工具")?;

    tray.add_menu_item("Open", || {
        CrossRuntimeBridge::global().emit(TrayEvent::Open);
    })?;
    tray.add_menu_item("Hide", || {
        CrossRuntimeBridge::global().emit(TrayEvent::Hide);
    })?;
    tray.inner_mut().add_separator()?;
    tray.add_menu_item("Quit", move || {
        CrossRuntimeBridge::global().emit(TrayEvent::Exit);
    })?;
    #[cfg(target_os = "macos")]
    tray.inner_mut().display();
    Ok(tray)
}
