use num_enum::TryFromPrimitive;

use crate::{LogicalUnit, Point};

bitflags::bitflags! {
    /// The type of node that is selected.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ContextMenuTypeFlags: i32 {
        /// No node is selected.
        const NONE = 0;
        /// The top page is selected.
        const PAGE = 1 << 0;
        /// A subframe page is selected.
        const FRAME = 1 << 1;
        /// A link is selected.
        const LINK = 1 << 2;
        /// A media node is selected.
        const MEDIA = 1 << 3;
        /// There is a textual or mixed selection that is selected.
        const SELECTION = 1 << 4;
        /// An editable element is selected.
        const EDITABLE = 1 << 5;
    }
}

///  Supported context menu media types.
#[derive(Copy, Clone, Debug, PartialEq, Eq, TryFromPrimitive, Default)]
#[repr(i32)]
pub enum ContextMenuMediaType {
    /// No special node is in context.
    #[default]
    None = 0,
    /// An image node is selected.
    Image = 1,
    /// A video node is selected.
    Video = 2,
    /// An audio node is selected.
    Audio = 3,
    /// A canvas node is selected.
    Canvas = 4,
    /// A file node is selected.
    File = 5,
    /// A plugin node is selected.
    Plugin = 6,
}

bitflags::bitflags! {
    /// Supported context menu media state bit flags.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ContextMenuMediaStateFlags: i32 {
        /// No special state is in context.
        const NONE = 0;
        /// The media is in error.
        const IN_ERROR = 1 << 0;
        /// The media is paused.
        const PAUSED = 1 << 1;
        /// The media is muted.
        const MUTED = 1 << 2;
        /// The media is set to loop.
        const LOOP = 1 << 3;
        /// The media can be saved.
        const CAN_SAVE = 1 << 4;
        /// The media has audio.
        const HAS_AUDIO = 1 << 5;
        /// The media can toggle controls.
        const CAN_TOGGLE_CONTROLS = 1 << 6;
        /// The media has controls enabled.
        const CONTROLS = 1 << 7;
        /// The media can be printed.
        const CAN_PRINT = 1 << 8;
        /// The media can be rotated.
        const CAN_ROTATE = 1 << 9;
        /// The media can be displayed in picture-in-picture mode.
        const CAN_PICTURE_IN_PICTURE = 1 << 10;
        /// The media is in picture-in-picture mode.
        const PICTURE_IN_PICTURE = 1 << 11;
        /// The media can be looped.
        const CAN_LOOP = 1 << 12;
    }
}

bitflags::bitflags! {
    /// Supported context menu type bit flags.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct ContextMenuEditStateFlags: i32 {
        /// No special state is in context.
        const NONE = 0;
        /// The edit control can be undone.
        const CAN_UNDO = 1 << 0;
        /// The edit control can be redone.
        const CAN_REDO = 1 << 1;
        /// The edit control can be cut.
        const CAN_CUT = 1 << 2;
        /// The edit control can be copied.
        const CAN_COPY = 1 << 3;
        /// The edit control can be pasted.
        const CAN_PASTE = 1 << 4;
        /// The edit control can be deleted.
        const CAN_DELETE = 1 << 5;
        /// The edit control can select all text.
        const CAN_SELECT_ALL = 1 << 6;
        /// The edit control can be translated.
        const CAN_TRANSLATE = 1 << 7;
        /// The edit control can be edited richly.
        const CAN_EDIT_RICHLY = 1 << 8;
    }
}

///  Provides information about the context menu state.
#[derive(Debug)]
pub struct ContextMenuParams<'a> {
    /// The X coordinate of the mouse where the context menu was invoked.
    ///
    /// Coords are relative to the associated RenderView's origin.
    pub crood: Point<LogicalUnit<i32>>,
    /// The flags representing the type of node that the context menu was
    /// invoked on.
    pub type_: ContextMenuTypeFlags,
    /// The URL of the link
    pub link_url: Option<&'a str>,
    /// The link URL to be used ONLY for "copy link address".
    pub unfiltered_link_url: Option<&'a str>,
    /// The source URL for the element that the context menu was invoked on.
    ///
    /// Example of elements with source URLs are img, audio, and video.
    pub source_url: Option<&'a str>,
    /// Whether the element that the context menu was invoked on has image
    /// contents.
    pub has_image_contents: bool,
    /// The title text or the alt text if the context menu was invoked on
    /// an image.
    pub title_text: Option<&'a str>,
    /// The URL of the top level page that the context menu was invoked on.
    pub page_url: &'a str,
    /// The URL of the subframe that the context menu was invoked on.
    pub frame_url: &'a str,
    /// The type of context node that the context menu was invoked on.
    pub media_type: ContextMenuMediaType,
    /// The flags representing the actions supported by the media element.
    pub media_state_flags: ContextMenuMediaStateFlags,
    /// The text of the selection, if any, that the context menu was invoked on.
    pub selection_text: Option<&'a str>,
    /// Whether the context menu was invoked on an editable element.
    pub is_editable: bool,
    /// The flags representing the actions supported by the editable node.
    pub edit_state_flags: ContextMenuEditStateFlags,
}
