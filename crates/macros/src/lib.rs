use proc_macro::TokenStream;
use quote::quote;

mod derive_into_plot;

#[proc_macro_derive(IntoPlot)]
pub fn derive_into_plot(input: TokenStream) -> TokenStream {
    derive_into_plot::derive_into_plot(input)
}

/// Convert an SVG filename to a PascalCase identifier.
///
/// Convention: lowercase the filename, strip `.svg`, split on `-`,
/// capitalize the first letter of each segment, join.
fn filename_to_pascal(filename: &str) -> String {
    let name = filename.strip_suffix(".svg").unwrap_or(filename);
    let lowered = name.to_lowercase();
    lowered
        .split('-')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
            }
        })
        .collect()
}

/// Generate an `IconName` enum and its `IconNamed` impl by scanning a directory of SVG files.
///
/// Accepts a path relative to the calling crate's `CARGO_MANIFEST_DIR`.
/// Each `.svg` file becomes an enum variant using PascalCase conversion.
///
/// # Example
///
/// ```ignore
/// generate_icon_enum!("../assets/assets/icons");
/// ```
#[proc_macro]
pub fn generate_icon_enum(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::LitStr);
    let relative_path = input.value();

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let icons_dir = std::path::Path::new(&manifest_dir).join(&relative_path);

    let mut entries: Vec<(String, String)> = Vec::new();

    let dir = std::fs::read_dir(&icons_dir).unwrap_or_else(|e| {
        panic!(
            "generate_icon_enum: failed to read '{}': {}",
            icons_dir.display(),
            e
        )
    });

    for entry in dir {
        let entry = entry.expect("failed to read directory entry");
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.ends_with(".svg") {
            let variant_name = filename_to_pascal(&filename);
            let path = format!("icons/{}", filename);
            entries.push((variant_name, path));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let variants: Vec<proc_macro2::Ident> = entries
        .iter()
        .map(|(name, _)| proc_macro2::Ident::new(name, proc_macro2::Span::call_site()))
        .collect();
    let paths: Vec<&str> = entries.iter().map(|(_, p)| p.as_str()).collect();

    let expanded = quote! {
        #[derive(IntoElement, Clone)]
        pub enum IconName {
            #(#variants,)*
        }

        impl IconNamed for IconName {
            fn path(self) -> SharedString {
                match self {
                    #(Self::#variants => #paths,)*
                }
                .into()
            }
        }
    };

    TokenStream::from(expanded)
}
