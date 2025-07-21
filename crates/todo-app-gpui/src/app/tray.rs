use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use trayicon::*;

pub type Tray = TrayIcon<TrayEvent>;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TrayEvent {
    RightClickTrayIcon,
    LeftClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    Open,
    Hide,
    // Item3,
    // Item4,
    // CheckItem1,
    // SubItem1,
    // SubItem2,
    // SubItem3,
}

const ICON: &[u8] = include_bytes!("../../../../assets/logo2.ico");
// fn load_icon() -> Icon {
//     Icon::from_buffer(ICON, None, None).unwrap()
// }

pub(crate) fn start_tray() -> anyhow::Result<Tray> {
    //   let icon = load_icon();
    let tray_icon = TrayIconBuilder::new()
        .sender(|event: &TrayEvent| {
            CrossRuntimeBridge::global().emit(event.clone());
        })
        .icon_from_buffer(ICON)
        .tooltip("xTo-Do Utility")
        .on_right_click(TrayEvent::RightClickTrayIcon)
        .on_click(TrayEvent::LeftClickTrayIcon)
        .on_double_click(TrayEvent::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("打开", TrayEvent::Open)
                .item("隐藏", TrayEvent::Hide)
                .separator()
                .item("退出", TrayEvent::Exit),
        )
        .build()?;

    Ok(tray_icon)
}
