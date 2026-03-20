mod column;
mod delegate;
pub mod filter_panel;
mod filter_state;
pub(crate) mod loading;
pub mod selection;
mod state;

use gpui::{App, KeyBinding};
use gpui_component::Size;

pub(crate) use column::{ColGroup, DragColumn, DragSelectCell, ResizeColumn};
pub use column::{Column, ColumnFixed, ColumnSort};
pub use delegate::{CellEditor, EditTableDelegate};
pub use filter_panel::FilterValue;
pub use filter_state::FilterState;
pub use selection::{CellCoord, CellRange, TableSelection};
use state::{
    Cancel, Copy, Paste, SelectAll, SelectDown, SelectFirst, SelectLast, SelectPageDown,
    SelectPageUp, SelectUp,
};
pub use state::{EditTableEvent, EditTableState, TableVisibleRange};

const CONTEXT: &str = "EditTable";

gpui::actions!(edit_table, [SelectPrevColumn, SelectNextColumn]);

/// 初始化 EditTable 的键盘绑定
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
        KeyBinding::new("up", SelectUp, Some(CONTEXT)),
        KeyBinding::new("down", SelectDown, Some(CONTEXT)),
        KeyBinding::new("left", SelectPrevColumn, Some(CONTEXT)),
        KeyBinding::new("right", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("home", SelectFirst, Some(CONTEXT)),
        KeyBinding::new("end", SelectLast, Some(CONTEXT)),
        KeyBinding::new("pageup", SelectPageUp, Some(CONTEXT)),
        KeyBinding::new("pagedown", SelectPageDown, Some(CONTEXT)),
        // 复制 (Ctrl+C / Cmd+C)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, Some(CONTEXT)),
        // 粘贴 (Ctrl+V / Cmd+V)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        // 全选 (Ctrl+A / Cmd+A)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, Some(CONTEXT)),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, Some(CONTEXT)),
        // 单元格导航
        KeyBinding::new("tab", SelectNextColumn, Some(CONTEXT)),
        KeyBinding::new("shift-tab", SelectPrevColumn, Some(CONTEXT)),
    ]);
}

#[derive(Clone, Copy, Default)]
pub struct ScrollbarVisible {
    pub right: bool,
    pub bottom: bool,
}

impl ScrollbarVisible {
    pub fn all() -> Self {
        Self {
            right: true,
            bottom: true,
        }
    }

    pub fn none() -> Self {
        Self {
            right: false,
            bottom: false,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TableOptions {
    pub size: Size,
    pub stripe: bool,
    pub scrollbar_visible: ScrollbarVisible,
}

impl Default for TableOptions {
    fn default() -> Self {
        Self {
            size: Size::Medium,
            stripe: true,
            scrollbar_visible: ScrollbarVisible::all(),
        }
    }
}

pub struct EditTable;

impl EditTable {
    pub fn new<D: EditTableDelegate>(
        state: &gpui::Entity<EditTableState<D>>,
    ) -> gpui::Entity<EditTableState<D>> {
        state.clone()
    }
}
