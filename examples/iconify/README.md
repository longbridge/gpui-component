## Iconify in GPUI Component

Well, I just feel it's a little boring to manually download and collect icon resources. Is there a way to automatically download icon?

[Iconify](https://iconify.design/) is a awesome project that provides a convenient way to use a rich set of icon resources. So after some searching and inspired from [iconify-rs](https://github.com/wrapperup/iconify-rs), I think it's feasible.

With `Iconify` component we can download icon resources from the Iconify server through [Iconify API](https://iconify.design/docs/api) automatically in develop mode, while in release mode, embeds them into the binary.

Now, we can fetch the icon and render it with `Iconify` component easily when we know the `collection:icon` name, and don't forget to set `cx.http_client` or we will get nothing.

## How to use

First, define `IconAssets` struct, implement `gpui::AssetSource` and `rust_embed::RustEmbed` for it.

```rust
#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "assets"]
#[include = "**/*.svg"]
pub struct IconAssets;

#[cfg(not(debug_assertions))]
impl gpui::AssetSource for IconAssets {
    fn load(&self, path: &str) -> anyhow::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow::anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<gpui::SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
```

Then, set `cx.http_client` in develop mode, and use `IconAssets` to register icon assets in release mode.

```rust
use gpui::Application;

#[cfg(debug_assertions)]
let app = Application::new().with_http_client(std::sync::Arc::new(
    reqwest_client::ReqwestClient::user_agent("gpui-component-example-iconify").unwrap(),
));

#[cfg(not(debug_assertions))]
let app = Application::new().with_assets(IconAssets);
```

Set the different value of `IconifySetting` separately in develop mode and release mode.

```rust
use gpui_component::theme;

theme::init(cx);

let theme = theme::Theme::global_mut(cx);
if cfg!(debug_assertions) {
    // Where to cache the downloaded icons in develop mode and embeded in release mode.
    theme.iconify.cache_dir = Some("assets".into());
} else {
    // Fetch from embeded icon from binary.
    theme.iconify.api_url = None;
    theme.iconify.cache_dir = None;
}
```

Then just use `Iconify` component to render a icon from `collection:icon` name.
Note it does not support query parameters for Iconify API, and will be ignored if set.

```rust
use gpui_component::{Iconify, iconify};

Iconify::new().path("lucide:smile");
iconify().path("lucide:smile");
```


And it can also render a icon from standard `<svg />` string.

```rust
use gpui_component::Iconify;

Iconify::new().data(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 48 48"><g fill="none" stroke="currentColor" stroke-linejoin="round" stroke-width="4"><path d="M24 44c11.046 0 20-8.954 20-20S35.046 4 24 4S4 12.954 4 24s8.954 20 20 20Z"/><path stroke-linecap="round" d="M31 18v1m-14-1v1m14 12s-2 4-7 4s-7-4-7-4"/></g></svg>"#)
```