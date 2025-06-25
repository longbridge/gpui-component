#![doc = include_str!("../README.md")]

mod app_handler;
mod browser;
mod browser_handler;
mod builder;
mod context_menu;
mod cursor;
mod dirty_rects;
mod dpi;
mod error;
mod ffi;
mod file_dialog;
mod frame;
#[cfg(target_os = "macos")]
mod framework_loader;
mod func_registry;
mod geom;
mod input;
mod js_dialog;
mod query;
#[cfg(target_os = "macos")]
mod sandbox_context;
mod settings;
mod wef;

pub use app_handler::ApplicationHandler;
pub use browser::Browser;
pub use browser_handler::{BrowserHandler, ImageBuffer, LogSeverity, PaintElementType};
pub use builder::BrowserBuilder;
pub use context_menu::{
    ContextMenuEditStateFlags, ContextMenuMediaStateFlags, ContextMenuMediaType, ContextMenuParams,
    ContextMenuTypeFlags,
};
pub use cursor::{CursorInfo, CursorType};
pub use dirty_rects::{DirtyRects, DirtyRectsIter};
pub use dpi::{LogicalUnit, PhysicalUnit};
pub use error::Error;
pub use file_dialog::{Accept, FileDialogCallback, FileDialogMode};
pub use frame::Frame;
#[cfg(target_os = "macos")]
pub use framework_loader::FrameworkLoader;
pub use func_registry::{
    AsyncFuncRegistryBuilder, AsyncFunctionType, CallFunctionError, FuncRegistry,
    FuncRegistryBuilder, FunctionType,
};
pub use geom::{Point, Rect, Size};
pub use input::{KeyCode, KeyModifier, MouseButton};
pub use js_dialog::{JsDialogCallback, JsDialogType};
#[cfg(target_os = "macos")]
pub use sandbox_context::SandboxContext;
pub use serde_json::Value;
pub use settings::Settings;
pub use wef::*;
