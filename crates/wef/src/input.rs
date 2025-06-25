/// Mouse button
#[derive(Debug, Clone, Copy)]
#[repr(i32)]
pub enum MouseButton {
    /// Left
    Left = 0,
    /// Middle
    Middle = 1,
    /// Right
    Right = 2,
}

/// Key codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum KeyCode {
    /// Backspace
    Backspace = 0x08,
    /// Delete
    Delete = 0x2E,
    /// Tab
    Tab = 0x09,
    /// Enter
    Enter = 0x0D,
    /// PageUp
    PageUp = 0x21,
    /// PageDown
    PageDown = 0x22,
    /// End
    End = 0x23,
    /// Home
    Home = 0x24,
    /// Arrow left
    ArrowLeft = 0x25,
    /// Arrow up
    ArrowUp = 0x26,
    /// Arrow right
    ArrowRight = 0x27,
    /// Arrow down
    ArrowDown = 0x28,
}

impl KeyCode {
    pub(crate) fn as_char(&self) -> Option<u16> {
        use KeyCode::*;

        match self {
            Enter => Some(0x0D),
            _ => None,
        }
    }
}

bitflags::bitflags! {
    /// Key modifiers for keyboard events.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct KeyModifier: i32 {
        /// Shift key
        const SHIFT = 0x1;
        /// Control key
        const CONTROL = 0x2;
        /// Alt key
        const ALT = 0x4;
    }
}
