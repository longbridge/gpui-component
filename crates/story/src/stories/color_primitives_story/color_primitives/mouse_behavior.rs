use gpui::{CursorStyle, Hitbox, Window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseCursorDecision {
    Set(CursorStyle),
    Passthrough,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedMousePreset {
    Default,
    Crosshair,
    Passthrough,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharedMousePresetContext {
    pub hovered: bool,
    pub contains_pointer: bool,
    pub dragging: bool,
    pub external_drag_active: bool,
}

pub fn resolve_shared_mouse_preset(
    preset: SharedMousePreset,
    ctx: SharedMousePresetContext,
    active_cursor_style: CursorStyle,
) -> MouseCursorDecision {
    if ctx.external_drag_active {
        return MouseCursorDecision::Passthrough;
    }

    match preset {
        SharedMousePreset::Passthrough => MouseCursorDecision::Passthrough,
        SharedMousePreset::Default | SharedMousePreset::Crosshair => {
            if ctx.dragging || (ctx.hovered && ctx.contains_pointer) {
                MouseCursorDecision::Set(active_cursor_style)
            } else if ctx.hovered {
                MouseCursorDecision::Set(CursorStyle::Arrow)
            } else {
                MouseCursorDecision::Passthrough
            }
        }
    }
}

pub fn apply_hover_cursor(window: &mut Window, hitbox: &Hitbox, decision: MouseCursorDecision) {
    if let MouseCursorDecision::Set(style) = decision {
        window.set_cursor_style(style, hitbox);
    }
}

pub fn apply_window_cursor(window: &mut Window, decision: MouseCursorDecision, claimed: &mut bool) {
    if let MouseCursorDecision::Set(style) = decision {
        window.set_window_cursor_style(style);
        *claimed = true;
    }
}

pub fn reset_window_cursor_if_claimed(window: &mut Window, claimed: &mut bool) {
    if *claimed {
        window.set_window_cursor_style(CursorStyle::Arrow);
        *claimed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(hovered: bool, contains_pointer: bool, dragging: bool) -> SharedMousePresetContext {
        SharedMousePresetContext {
            hovered,
            contains_pointer,
            dragging,
            external_drag_active: false,
        }
    }

    #[test]
    fn shared_resolver_external_drag_is_passthrough() {
        let decision = resolve_shared_mouse_preset(
            SharedMousePreset::Default,
            SharedMousePresetContext {
                hovered: true,
                contains_pointer: true,
                dragging: true,
                external_drag_active: true,
            },
            CursorStyle::PointingHand,
        );

        assert_eq!(decision, MouseCursorDecision::Passthrough);
    }

    #[test]
    fn shared_resolver_default_preset_active_hover_idle() {
        assert_eq!(
            resolve_shared_mouse_preset(
                SharedMousePreset::Default,
                context(true, true, false),
                CursorStyle::PointingHand,
            ),
            MouseCursorDecision::Set(CursorStyle::PointingHand)
        );
        assert_eq!(
            resolve_shared_mouse_preset(
                SharedMousePreset::Default,
                context(true, false, false),
                CursorStyle::PointingHand,
            ),
            MouseCursorDecision::Set(CursorStyle::Arrow)
        );
        assert_eq!(
            resolve_shared_mouse_preset(
                SharedMousePreset::Default,
                context(false, false, false),
                CursorStyle::PointingHand,
            ),
            MouseCursorDecision::Passthrough
        );
    }

    #[test]
    fn shared_resolver_crosshair_preset_active_hover_idle() {
        assert_eq!(
            resolve_shared_mouse_preset(
                SharedMousePreset::Crosshair,
                context(true, true, false),
                CursorStyle::Crosshair,
            ),
            MouseCursorDecision::Set(CursorStyle::Crosshair)
        );
        assert_eq!(
            resolve_shared_mouse_preset(
                SharedMousePreset::Crosshair,
                context(true, false, false),
                CursorStyle::Crosshair,
            ),
            MouseCursorDecision::Set(CursorStyle::Arrow)
        );
        assert_eq!(
            resolve_shared_mouse_preset(
                SharedMousePreset::Crosshair,
                context(false, false, false),
                CursorStyle::Crosshair,
            ),
            MouseCursorDecision::Passthrough
        );
    }
}
