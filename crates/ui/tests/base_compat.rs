use gpui::{
    Axis, Div, InteractiveElement as _, Length, ParentElement as _, Pixels, Stateful,
    StatefulInteractiveElement as _, Styled as _, blue, green, px, red,
};
use gpui_component::{
    AxisExt as _, FocusTrapElement, InteractiveElementExt, LengthExt as _, Placement, Side,
    animation,
};

#[test]
fn legacy_foundation_exports_remain_available() {
    assert!(Axis::Horizontal.is_horizontal());
    assert!(Axis::Vertical.is_vertical());
    assert_eq!(Placement::Top.axis(), Axis::Vertical);
    assert!(Side::Left.is_left());

    let length = Length::Definite(px(12.).into());
    let pixels: Option<Pixels> = length.to_pixels(px(16.).into(), px(16.));
    assert_eq!(pixels, Some(px(12.)));

    assert_eq!(animation::ease_in_cubic(0.5), 0.125);

    fn requires_interaction_extensions<T: FocusTrapElement + InteractiveElementExt>() {}
    requires_interaction_extensions::<Stateful<Div>>();
}

#[test]
fn base_crate_exports_the_same_foundation_types() {
    let legacy = gpui_component::Edges::all(1_u8);
    let base: gpui_component_base::Edges<u8> = legacy;

    assert_eq!(base.top, 1);
    assert_eq!(base.right, 1);
    assert_eq!(base.bottom, 1);
    assert_eq!(base.left, 1);
}

#[test]
fn base_button_accepts_application_owned_state_styles() {
    let _button = gpui_component_base::Button::new("save")
        .accessibility_label("Save")
        .disabled(false)
        .on_click(|_, _, _| {})
        .child("Save")
        .bg(red())
        .hover(|style| style.bg(green()))
        .active(|style| style.bg(blue()))
        .focus_visible(|style| style.border_1());
}
