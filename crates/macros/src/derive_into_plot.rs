use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics gpui::IntoElement for #type_name #type_generics #where_clause {
            type Element = Self;

            fn into_element(self) -> Self::Element {
                self
            }
        }

        impl #impl_generics gpui::Element for #type_name #type_generics #where_clause {
            type RequestLayoutState = ();
            // Carries the prepainted tooltip overlay (if any) from `prepaint` to `paint`.
            type PrepaintState = Option<gpui::AnyElement>;

            fn id(&self) -> Option<gpui::ElementId> {
                // `Some` opts the plot in to interactive tooltips; `None` (the default)
                // keeps the element a pure, non-interactive plot identical to before.
                <Self as Plot>::id(self)
            }

            fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
                None
            }

            fn request_layout(
                &mut self,
                _: Option<&gpui::GlobalElementId>,
                _: Option<&gpui::InspectorElementId>,
                window: &mut gpui::Window,
                cx: &mut gpui::App,
            ) -> (gpui::LayoutId, Self::RequestLayoutState) {
                let style = gpui::Style {
                    size: gpui::Size::full(),
                    ..Default::default()
                };

                (window.request_layout(style, None, cx), ())
            }

            fn prepaint(
                &mut self,
                global_id: Option<&gpui::GlobalElementId>,
                _: Option<&gpui::InspectorElementId>,
                bounds: gpui::Bounds<gpui::Pixels>,
                _: &mut Self::RequestLayoutState,
                window: &mut gpui::Window,
                cx: &mut gpui::App,
            ) -> Self::PrepaintState {
                // No id => tooltips disabled => behave exactly like a non-interactive plot.
                let Some(global_id) = global_id else {
                    return None;
                };

                // Read the cursor position recorded by the previous frame's mouse handler.
                let position = window.with_element_state::<
                    std::rc::Rc<std::cell::Cell<Option<gpui::Point<gpui::Pixels>>>>,
                    _,
                >(global_id, |prev, _| {
                    let cell = prev.unwrap_or_default();
                    (cell.get(), cell)
                });

                // Per-plot slide-animation state: (last hovered index, that point's x, a
                // monotonic epoch, and the slide's start offset for the current epoch).
                let anim = window.with_element_state::<
                    std::rc::Rc<std::cell::Cell<Option<(usize, gpui::Pixels, u64, gpui::Pixels)>>>,
                    _,
                >(global_id, |prev, _| {
                    let cell = prev.unwrap_or_default();
                    (cell.clone(), cell)
                });

                let Some(position) = position else {
                    anim.set(None);
                    return None;
                };
                let Some(state) = <Self as Plot>::tooltip_state(self, position, bounds, cx) else {
                    anim.set(None);
                    return None;
                };

                // Start a fresh eased slide (from the previous data point's x to the new one,
                // under a new epoch id) only when the hovered data point changes; otherwise keep
                // the current epoch so an in-flight slide runs to completion. The vertical axis
                // follows the cursor directly, so only the horizontal jump is animated.
                // Animate along whichever axis the plot's data points snap along.
                let vertical = state.slides_vertically();
                let coord = if vertical {
                    state.cross_line.y
                } else {
                    state.cross_line.x
                };

                let (epoch, slide_from) = match anim.get() {
                    Some((last_index, _, epoch, slide_from)) if last_index == state.index => {
                        (epoch, slide_from)
                    }
                    Some((_, prev_coord, epoch, _)) => {
                        let next = (epoch.wrapping_add(1), prev_coord - coord);
                        anim.set(Some((state.index, coord, next.0, next.1)));
                        next
                    }
                    None => {
                        anim.set(Some((state.index, coord, 0, gpui::px(0.))));
                        (0, gpui::px(0.))
                    }
                };

                let Some(overlay) = <Self as Plot>::tooltip(self, &state, bounds, window, cx) else {
                    return None;
                };

                let mut overlay = if slide_from.abs() > gpui::px(0.5) {
                    // Ease the whole overlay from the previous data point to the new one along
                    // the snap axis (self-driving via the animation's `request_animation_frame`).
                    use gpui::{
                        AnimationExt as _, IntoElement as _, ParentElement as _, Styled as _,
                    };
                    let animated = gpui::div()
                        .absolute()
                        .size_full()
                        .child(overlay)
                        .with_animation(
                            gpui::ElementId::NamedInteger("plot-tooltip-slide".into(), epoch),
                            gpui::Animation::new(std::time::Duration::from_millis(300))
                                .with_easing(gpui::ease_in_out),
                            move |el, delta| {
                                let offset = slide_from * (1.0 - delta);
                                if vertical {
                                    el.top(offset)
                                } else {
                                    el.left(offset)
                                }
                            },
                        );
                    // Wrap in a root container so the animated child's `left` offset is honored:
                    // a root element's own position insets are dropped by `prepaint_as_root`.
                    gpui::div()
                        .absolute()
                        .size_full()
                        .child(animated)
                        .into_any_element()
                } else {
                    overlay
                };

                overlay.prepaint_as_root(bounds.origin, bounds.size.into(), window, cx);
                Some(overlay)
            }

            fn paint(
                &mut self,
                global_id: Option<&gpui::GlobalElementId>,
                _: Option<&gpui::InspectorElementId>,
                bounds: gpui::Bounds<gpui::Pixels>,
                _: &mut Self::RequestLayoutState,
                overlay: &mut Self::PrepaintState,
                window: &mut gpui::Window,
                cx: &mut gpui::App,
            ) {
                <Self as Plot>::paint(self, bounds, window, cx);

                if let Some(global_id) = global_id {
                    // Record the cursor position into element-local state on every move so the
                    // next frame can hit-test it. The handler never touches `self`, satisfying
                    // the `'static` bound; it only captures the (Copy) bounds and the state cell.
                    let cell = window.with_element_state::<
                        std::rc::Rc<std::cell::Cell<Option<gpui::Point<gpui::Pixels>>>>,
                        _,
                    >(global_id, |prev, _| {
                        let cell = prev.unwrap_or_default();
                        (cell.clone(), cell)
                    });

                    window.on_mouse_event(
                        move |e: &gpui::MouseMoveEvent, _, window: &mut gpui::Window, _| {
                            let next = if bounds.contains(&e.position) {
                                Some(e.position - bounds.origin)
                            } else {
                                None
                            };

                            if cell.get() != next {
                                cell.set(next);
                                window.refresh();
                            }
                        },
                    );
                }

                // Paint the tooltip overlay (crosshair, dots, box) above the plot graphics.
                if let Some(overlay) = overlay.as_mut() {
                    overlay.paint(window, cx);
                }
            }
        }
    };

    TokenStream::from(expanded)
}
