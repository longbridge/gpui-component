use gpui::{
    Axis, Div, InteractiveElement as _, Length, ParentElement as _, Pixels, Stateful,
    StatefulInteractiveElement as _, Styled as _, blue, green, prelude::FluentBuilder as _, px,
    red,
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

#[test]
fn base_controls_expose_typed_semantic_style_contexts() {
    let _ = gpui_component_base::Button::new("button")
        .styles(|styles| styles.disabled(|style| style.opacity(0.5)));
    let _ = gpui_component_base::Checkbox::new("checkbox").styles(|styles| {
        styles
            .checked(|style| style.bg(green()))
            .indeterminate(|style| style.bg(blue()))
            .disabled(|style| style.when(true, |style| style.opacity(0.5)))
    });
    let _ = gpui_component_base::Radio::new("radio")
        .styles(|styles| styles.checked(|style| style.bg(green())));
    let _ = gpui_component_base::Switch::new("switch")
        .styles(|styles| styles.checked(|style| style.bg(green())));
    let _ = gpui_component_base::Toggle::new("toggle")
        .styles(|styles| styles.pressed(|style| style.bg(green())));
    let _ = gpui_component_base::Link::new("link")
        .styles(|styles| styles.disabled(|style| style.opacity(0.5)));
}

#[test]
fn transition_ids_accept_strings_and_named_channels() {
    let _: gpui_component_base::TransitionId = "opacity".into();
    let _: gpui_component_base::TransitionId = ("checkbox", "fill").into();

    fn requires_interpolation<T: gpui_component_base::Interpolate>() {}
    requires_interpolation::<f32>();
}

#[test]
fn legacy_styled_and_sizing_exports_remain_available() {
    use gpui_component::Sizable as _;

    let _: gpui_component::Size = gpui_component::Size::Medium;
    let _ = gpui_component::StyledExt::font_medium(gpui::div());
    let _ = gpui_component::h_flex();
    let _ = gpui_component::v_flex();
    let _ = gpui_component::box_shadow(0., 0., 0., 0., gpui::hsla(0., 0., 0., 0.));

    struct SizedValue;
    impl gpui_component::Sizable for SizedValue {
        fn with_size(self, _: impl Into<gpui_component::Size>) -> Self {
            self
        }
    }
    let _ = SizedValue.small();
}

#[test]
fn element_ext_is_available_from_base_and_the_legacy_root() {
    use gpui_component::ElementExt as _;

    fn requires_base<T: gpui_component_base::ElementExt>() {}
    fn requires_legacy<T: gpui_component::ElementExt>() {}
    requires_base::<gpui::Div>();
    requires_legacy::<gpui::Div>();

    let _ = gpui::div().on_prepaint(|_, _, _| {});
}

#[test]
fn legacy_history_path_reexports_the_base_type() {
    #[derive(Clone, PartialEq)]
    struct Item {
        version: usize,
    }

    impl gpui_component_base::HistoryItem for Item {
        fn version(&self) -> usize {
            self.version
        }

        fn set_version(&mut self, version: usize) {
            self.version = version;
        }
    }

    fn through_legacy_path(
        history: gpui_component_base::History<Item>,
    ) -> gpui_component::history::History<Item> {
        history
    }

    let _ = through_legacy_path(gpui_component_base::History::new());
}

#[test]
fn legacy_auto_scroll_path_reexports_the_base_type() {
    fn through_legacy_path(
        scroll: gpui_component_base::AutoScroll,
    ) -> gpui_component::scroll::AutoScroll {
        scroll
    }

    let _ = through_legacy_path(gpui_component_base::AutoScroll::default());
}
