use crate::backoffice::cross_runtime::CrossRuntimeBridge;
use trayicon::*;

pub type Tray  = TrayIcon<TrayEvent>;
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TrayEvent {
    RightClickTrayIcon,
    LeftClickTrayIcon,
    DoubleClickTrayIcon,
    Exit,
    Item1,
    Item2,
    Item3,
    Item4,
    CheckItem1,
    SubItem1,
    SubItem2,
    SubItem3,
}

const ICON: &[u8] = include_bytes!("../../../../assets/logo2.ico");
fn load_icon() -> Icon {
   Icon::from_buffer(ICON, None, None).unwrap()
}

pub(crate) fn start_tray() -> anyhow::Result<Tray> {
 //   let icon = load_icon();
    let  tray_icon = TrayIconBuilder::new()
        .sender(move |event:&TrayEvent| {
            CrossRuntimeBridge::global().emit(event.clone());
        })
        .icon_from_buffer(ICON)
        .tooltip("xTo-Do Utility")
        .on_right_click(TrayEvent::RightClickTrayIcon)
        .on_click(TrayEvent::LeftClickTrayIcon)
        .on_double_click(TrayEvent::DoubleClickTrayIcon)
        .menu(
            MenuBuilder::new()
                .item("Item 3 Replace Menu 👍", TrayEvent::Item3)
                .item("Item 2 Change Icon Green", TrayEvent::Item2)
                .item("Item 1 Change Icon Red", TrayEvent::Item1)
                .separator()
                .checkable("This is checkable", true, TrayEvent::CheckItem1)
                .submenu(
                    "Sub Menu",
                    MenuBuilder::new()
                        .item("Sub item 1", TrayEvent::SubItem1)
                        .item("Sub Item 2", TrayEvent::SubItem2)
                        .item("Sub Item 3", TrayEvent::SubItem3),
                )
                .with(MenuItem::Item {
                    name: "Item Disabled".into(),
                    disabled: true, // Disabled entry example
                    id: TrayEvent::Item4,
                    icon: None,
                })
                .separator()
                .item("E&xit", TrayEvent::Exit),
        )
        .build()?;
    Ok(tray_icon)
}
