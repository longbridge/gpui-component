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

        impl #impl_generics #type_name #type_generics #where_clause {
            /// Element-local cell holding the last cursor position (plot-relative), shared by
            /// the generated `prepaint`/`paint` so the cell type lives in a single place.
            #[doc(hidden)]
            fn __plot_tooltip_cursor(
                global_id: &gpui::GlobalElementId,
                window: &mut gpui::Window,
            ) -> std::rc::Rc<std::cell::Cell<Option<gpui::Point<gpui::Pixels>>>> {
                window.with_element_state(global_id, |prev, _| {
                    let cell: std::rc::Rc<
                        std::cell::Cell<Option<gpui::Point<gpui::Pixels>>>,
                    > = prev.unwrap_or_default();
                    (cell.clone(), cell)
                })
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
                let global_id = global_id?;

                // Read the cursor position recorded by the previous frame's mouse handler.
                let position = Self::__plot_tooltip_cursor(global_id, window).get()?;
                let state = <Self as Plot>::tooltip_state(self, position, bounds, cx)?;

                // Pass the live cursor so the tooltip box can follow it; the crosshair and
                // dots in `state` stay snapped to the data point by `tooltip_state`.
                let overlay = <Self as Plot>::tooltip(self, &state, position, bounds, window, cx)?;

                // Defer the overlay so it paints above sibling content drawn after the plot
                // (e.g. a chart card's footer text). The tooltip box can extend past the plot
                // bounds; without deferral those later siblings would cover the overflow.
                let mut overlay = gpui::IntoElement::into_any_element(
                    gpui::deferred(overlay),
                );
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
                    let cell = Self::__plot_tooltip_cursor(global_id, window);

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
