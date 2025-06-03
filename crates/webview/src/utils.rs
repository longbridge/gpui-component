use gpui::{CursorStyle, Modifiers, MouseButton};

pub(crate) fn to_wef_mouse_button(button: MouseButton) -> Option<wef::MouseButton> {
    Some(match button {
        MouseButton::Left => wef::MouseButton::Left,
        MouseButton::Middle => wef::MouseButton::Middle,
        MouseButton::Right => wef::MouseButton::Right,
        _ => return None,
    })
}

pub(crate) fn to_wef_key_modifiers(modifiers: &Modifiers) -> wef::KeyModifier {
    let mut wef_modifiers = wef::KeyModifier::empty();
    if modifiers.shift {
        wef_modifiers |= wef::KeyModifier::SHIFT;
    }
    if modifiers.control {
        wef_modifiers |= wef::KeyModifier::CONTROL;
    }
    if modifiers.alt {
        wef_modifiers |= wef::KeyModifier::ALT;
    }
    wef_modifiers
}

pub(crate) fn to_wef_key_code(key_code: &str) -> Option<wef::KeyCode> {
    Some(match key_code {
        "backspace" => wef::KeyCode::Backspace,
        "delete" => wef::KeyCode::Delete,
        "tab" => wef::KeyCode::Tab,
        "enter" => wef::KeyCode::Enter,
        "pageup" => wef::KeyCode::PageUp,
        "pagedown" => wef::KeyCode::PageDown,
        "end" => wef::KeyCode::End,
        "home" => wef::KeyCode::Home,
        "left" => wef::KeyCode::ArrowLeft,
        "up" => wef::KeyCode::ArrowUp,
        "right" => wef::KeyCode::ArrowRight,
        "down" => wef::KeyCode::ArrowDown,
        _ => return None,
    })
}

pub(crate) fn from_wef_cursor_type(cursor: wef::CursorType) -> CursorStyle {
    use wef::CursorType::*;

    match cursor {
        Cross => CursorStyle::Crosshair,
        Hand => CursorStyle::PointingHand,
        IBeam => CursorStyle::IBeam,
        EastResize => CursorStyle::ResizeRight,
        NorthResize => CursorStyle::ResizeUp,
        NorthEastResize => CursorStyle::ResizeUpRightDownLeft,
        NorthWestResize => CursorStyle::ResizeUpLeftDownRight,
        SouthResize => CursorStyle::ResizeDown,
        SouthEastResize => CursorStyle::ResizeUpRightDownLeft,
        SouthWestResize => CursorStyle::ResizeUpLeftDownRight,
        WestResize => CursorStyle::ResizeLeft,
        NorthSouthResize => CursorStyle::ResizeUpDown,
        EastWestResize => CursorStyle::ResizeLeftRight,
        NorthEastSouthWestResize => CursorStyle::ResizeUpRightDownLeft,
        NorthWestSouthEastResize => CursorStyle::ResizeUpLeftDownRight,
        ColumnResize => CursorStyle::ResizeColumn,
        RowResize => CursorStyle::ResizeRow,
        ContextMenu => CursorStyle::ContextualMenu,
        NoDrop => CursorStyle::OperationNotAllowed,
        Copy => CursorStyle::DragCopy,
        None => CursorStyle::None,
        NotAllowed => CursorStyle::OperationNotAllowed,
        Grab => CursorStyle::OpenHand,
        Grabbing => CursorStyle::ClosedHand,
        DndNone => CursorStyle::None,
        DndCopy => CursorStyle::DragCopy,
        DndLink => CursorStyle::DragLink,
        _ => CursorStyle::Arrow,
    }
}
