use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let type_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics IntoElement for #type_name #type_generics #where_clause {
            type Element = Self;

            fn into_element(self) -> Self::Element {
                self
            }
        }

        impl #impl_generics gpui::Element for #type_name #type_generics #where_clause {
            type RequestLayoutState = ();
            type PrepaintState = ();

            fn id(&self) -> Option<gpui::ElementId> {
                None
            }

            fn request_layout(
                &mut self,
                _: Option<&gpui::GlobalElementId>,
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
                _: Option<&gpui::GlobalElementId>,
                _: gpui::Bounds<gpui::Pixels>,
                _: &mut Self::RequestLayoutState,
                _: &mut gpui::Window,
                _: &mut gpui::App,
            ) -> Self::PrepaintState {
            }

            fn paint(
                &mut self,
                _: Option<&gpui::GlobalElementId>,
                bounds: gpui::Bounds<gpui::Pixels>,
                _: &mut Self::RequestLayoutState,
                _: &mut Self::PrepaintState,
                window: &mut gpui::Window,
                cx: &mut gpui::App,
            ) {
                <Self as Plot>::paint(self, bounds, window, cx)
            }
        }
    };

    TokenStream::from(expanded)
}
