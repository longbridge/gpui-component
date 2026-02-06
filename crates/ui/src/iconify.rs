use crate::{ActiveTheme, Sizable};
use directories::BaseDirs;
use gpui::{
    AnyElement, App, Asset, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Negate, Pixels, Point, Radians, Refineable, SharedString, Size, Style,
    StyleRefinement, Styled, TransformationMatrix, Window,
    http_client::{Uri, Url},
    point, px, radians, size,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smol::{future::Future, io::AsyncReadExt};
use std::{env, fs, panic::Location, path::PathBuf};

/// The settings for Iconify.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IconifySettings {
    /// The URL of the Iconify API, default is [https://api.iconify.design](https://iconify.design/docs/api).
    pub api_url: Option<SharedString>,
    /// The cache directory for Iconify, keep same with [iconify-rs](https://github.com/wrapperup/iconify-rs).
    pub cache_dir: Option<PathBuf>,
}

impl Default for IconifySettings {
    fn default() -> Self {
        Self {
            api_url: Some(iconify_url().into()),
            cache_dir: Some(iconify_cache()),
        }
    }
}

/// Copied from [iconify-rs](https://github.com/wrapperup/iconify-rs/blob/314561148b29fb1498cee3437ef70c77add036aa/src/svg.rs#L233).
fn iconify_url() -> String {
    env::var("ICONIFY_URL").unwrap_or("https://api.iconify.design".into())
}
/// Copied from [iconify-rs](https://github.com/wrapperup/iconify-rs/blob/314561148b29fb1498cee3437ef70c77add036aa/src/svg.rs#L238).
fn iconify_cache() -> PathBuf {
    if let Ok(dir) = env::var("ICONIFY_CACHE_DIR") {
        return PathBuf::from(dir);
    }

    let dir = if cfg!(target_family = "unix") {
        // originally we used cache_dir for all non-Windows platforms but that returns
        // a path that's not writable in cross-rs Docker. /tmp should always work
        PathBuf::from("/tmp")
    } else if cfg!(target_os = "windows") {
        // I didn't like the idea of having a cache dir in the root of %LOCALAPPDATA%.
        PathBuf::from(BaseDirs::new().unwrap().cache_dir()).join("cache")
    } else {
        PathBuf::from(BaseDirs::new().unwrap().cache_dir())
    };

    dir.join("iconify-rs")
}

/// A convenience function to create an Iconify element.
///
/// # Example
/// ```
/// use gpui_component::iconify;
///
/// let icon = iconify().path("lucide:smile");
/// let icon = iconify().path("lucide/smile");
/// let icon = iconify().path("lucide/smile.svg");
/// ```
/// # Note
/// The additional query parameters such as `?size=24`, `?color=red` etc. will be ignored.
#[track_caller]
pub fn iconify() -> Iconify {
    Iconify::new()
}

/// An element to render an icon from Iconify API or a cached path if exists.
///
/// # Example
/// ```
/// use gpui_component::Iconify;
///
/// let icon = Iconify::new().path("lucide:smile");
/// let icon = Iconify::new().path("lucide/smile");
/// let icon = Iconify::new().path("lucide/smile.svg");
/// ```
/// # Note
/// The additional query parameters such as `?size=24`, `?color=red` etc. will be ignored.
pub struct Iconify {
    path: Option<SharedString>,
    data: Option<SharedString>,
    scale: Size<f32>,
    translate: Point<Pixels>,
    rotate: Radians,
    style: StyleRefinement,
}

impl Iconify {
    pub fn new() -> Self {
        Self {
            path: None,
            data: None,
            scale: size(1.0, 1.0),
            translate: point(px(0.0), px(0.0)),
            rotate: radians(0.0),
            style: StyleRefinement::default().flex_none().size_4(),
        }
    }

    pub fn path(mut self, path: impl Into<SharedString>) -> Self {
        self.path = Some(Self::convert_path(path));
        self
    }

    /// Set the svg data for the icon.
    /// # Example
    /// ```
    /// use gpui_component::Iconify;
    ///
    /// let icon = Iconify::new().data(r#"<svg xmlns="http://www.w3.org/2000/svg" width="1em" height="1em" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M8 14s1.5 2 4 2s4-2 4-2M9 9h.01M15 9h.01"/></g></svg>"#);
    /// ```
    pub fn data(mut self, data: impl Into<SharedString>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Set the scale for the svg, same as [`gpui::Transformation`] for [`gpui::Svg`].
    pub fn scale(mut self, scale: Size<f32>) -> Self {
        self.scale = scale;
        self
    }

    /// Set the translate for the svg, same as [`gpui::Transformation`] for [`gpui::Svg`].
    pub fn translate(mut self, translate: Point<Pixels>) -> Self {
        self.translate = translate;
        self
    }

    /// Set the rotate for the svg, same as [`gpui::Transformation`] for [`gpui::Svg`].
    pub fn rotate(mut self, rotate: impl Into<Radians>) -> Self {
        self.rotate = rotate.into();
        self
    }

    /// Convert to a valid path for Iconify API, ignored the query params and add .svg if not exists.
    fn convert_path(path: impl Into<SharedString>) -> SharedString {
        let mut path = path.into().replace(":", "/");
        path.insert(0, '/');

        if let Ok(url) = path.parse::<Uri>() {
            path = url.path().to_string();
        }
        if !path.ends_with(".svg") {
            path.push_str(".svg");
        }
        path = path.trim_start_matches('/').into();

        path.into()
    }

    fn into_matrix(
        center: Point<Pixels>,
        factor: f32,
        scale: Size<f32>,
        translate: Point<Pixels>,
        rotate: Radians,
    ) -> TransformationMatrix {
        TransformationMatrix::unit()
            .translate(center.scale(factor) + translate.scale(factor))
            .rotate(rotate)
            .scale(scale)
            .translate(center.scale(factor).negate())
    }
}

impl IntoElement for Iconify {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Styled for Iconify {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl From<Iconify> for AnyElement {
    fn from(val: Iconify) -> Self {
        val.into_any_element()
    }
}

impl Sizable for Iconify {
    fn with_size(mut self, size: impl Into<crate::styled::Size>) -> Self {
        let style = StyleRefinement::default();
        let style = match size.into() {
            crate::styled::Size::Size(px) => style.size(px),
            crate::styled::Size::XSmall => style.size_3(),
            crate::styled::Size::Small => style.size_3p5(),
            crate::styled::Size::Medium => style.size_4(),
            crate::styled::Size::Large => style.size_6(),
        };
        self.style.refine(&style);
        self
    }
}

impl Element for Iconify {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.refine(&self.style);
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) where
        Self: Sized,
    {
        let color = self.style.text.color.unwrap_or_default();

        let data = if let Some(data) = self.data.as_ref() {
            Some(data.as_bytes().to_vec())
        } else if let Some(path) = &self.path {
            window
                .use_asset::<IconifyAsset>(path, cx)
                .and_then(|asset| asset)
        } else {
            None
        };

        let path = data
            .as_ref()
            .map(|d| blake3::hash(&d).to_string().into())
            .unwrap_or_else(|| self.path.clone().unwrap_or_default());

        let transformation = Self::into_matrix(
            bounds.center(),
            window.scale_factor(),
            self.scale,
            self.translate,
            self.rotate,
        );

        window
            .paint_svg(bounds, path, data.as_deref(), transformation, color, cx)
            .ok();
    }
}

enum IconifyAsset {}

impl Asset for IconifyAsset {
    type Source = SharedString;
    type Output = Option<Vec<u8>>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        // Without setting `cx.http_client`, this will fail but no panic.
        let client = cx.http_client();

        let path = cx
            .theme()
            .iconify
            .cache_dir
            .as_ref()
            .map(|p| p.join(source.as_ref()));

        let url = cx
            .theme()
            .iconify
            .api_url
            .as_ref()
            .and_then(|u| Url::parse(u).and_then(|u| u.join(source.as_ref())).ok());

        async move {
            // If cached, just return it.
            if let Some(path) = &path
                && path.exists()
                && let Ok(bytes) = fs::read(path)
            {
                return Some(bytes);
            }

            let mut bytes = Vec::new();

            // If not cached, download from iconify api.
            if let Some(url) = url
                && let Ok(mut resp) = client.get(url.as_ref(), ().into(), true).await
                && resp.status().is_success()
                && resp.body_mut().read_to_end(&mut bytes).await.is_ok()
                && !bytes.is_empty()
            {
                // Try to cache the svg.
                if let Some(path) = path {
                    if let Some(parent) = path.parent()
                        && !parent.exists()
                    {
                        fs::create_dir_all(parent).ok();
                    }
                    fs::write(&path, &bytes).ok();
                }

                return Some(bytes);
            }

            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_convert_path() {
        let paths = vec![
            "lucide:smile",
            "lucide:angry?width=24",
            "mdi/smiley-outline",
            "mdi/emoticon-angry-outline?height=32",
            "icon-park-outline/slightly-smiling-face.svg",
            "icon-park-outline/angry-face.svg?rotate=180deg",
        ];
        let expected = vec![
            "lucide/smile.svg",
            "lucide/angry.svg",
            "mdi/smiley-outline.svg",
            "mdi/emoticon-angry-outline.svg",
            "icon-park-outline/slightly-smiling-face.svg",
            "icon-park-outline/angry-face.svg",
        ];
        for (i, path) in paths.into_iter().enumerate() {
            assert_eq!(super::Iconify::convert_path(path), expected[i]);
        }
    }
}
