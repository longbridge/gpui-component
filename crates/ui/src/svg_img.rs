use std::{
    hash::Hash,
    ops::Deref,
    sync::{Arc, LazyLock},
};

use gpui::{
    hash, px, size, App, Asset, Bounds, Element, ElementId, GlobalElementId, Hitbox,
    ImageCacheError, InteractiveElement, Interactivity, IntoElement, IsZero, Pixels, RenderImage,
    SharedString, Size, StyleRefinement, Styled, Window,
};
use image::Frame;
use smallvec::SmallVec;

use image::ImageBuffer;

const SCALE: f32 = 2.;

struct SvgImgState(Option<(u64, Arc<RenderImage>)>);

static OPTIONS: LazyLock<usvg::Options> = LazyLock::new(|| {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    options
});

#[derive(Debug, Clone, Hash)]
pub enum SvgSource {
    /// A svg bytes
    Data(Arc<[u8]>),
    /// An asset path
    Path(SharedString),
}

impl From<&[u8]> for SvgSource {
    fn from(data: &[u8]) -> Self {
        Self::Data(data.into())
    }
}

impl From<Arc<[u8]>> for SvgSource {
    fn from(data: Arc<[u8]>) -> Self {
        Self::Data(data)
    }
}

impl From<SharedString> for SvgSource {
    fn from(path: SharedString) -> Self {
        Self::Path(path)
    }
}

impl From<&'static str> for SvgSource {
    fn from(path: &'static str) -> Self {
        Self::Path(path.into())
    }
}

impl Clone for SvgImg {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            interactivity: Interactivity::default(),
            source: self.source.clone(),
            size: self.size,
        }
    }
}

pub enum Image {}

#[derive(Debug, Clone)]
pub struct ImageSource {
    source: SvgSource,
    size: Size<Pixels>,
}

impl Hash for ImageSource {
    /// Hash to to control the Asset cache
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.hash(state);
    }
}

impl Asset for Image {
    type Source = ImageSource;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        let asset_source = cx.asset_source().clone();

        async move {
            let size = source.size;
            if size.width.is_zero() || size.height.is_zero() {
                return Err(usvg::Error::InvalidSize.into());
            }
            let size = Size {
                width: (size.width * SCALE).ceil(),
                height: (size.height * SCALE).ceil(),
            };

            let bytes = match source.source {
                SvgSource::Data(data) => data,
                SvgSource::Path(path) => {
                    if let Ok(Some(data)) = asset_source.load(&path) {
                        data.deref().to_vec().into()
                    } else {
                        Err(std::io::Error::other(format!(
                            "failed to load svg image from path: {}",
                            path
                        )))
                        .map_err(|e| ImageCacheError::Io(Arc::new(e)))?
                    }
                }
            };

            let tree = usvg::Tree::from_data(&bytes, &OPTIONS)?;

            let mut pixmap =
                resvg::tiny_skia::Pixmap::new(size.width.0 as u32, size.height.0 as u32)
                    .ok_or(usvg::Error::InvalidSize)?;

            let transform = resvg::tiny_skia::Transform::from_scale(SCALE, SCALE);

            resvg::render(&tree, transform, &mut pixmap.as_mut());

            let mut buffer = ImageBuffer::from_raw(pixmap.width(), pixmap.height(), pixmap.take())
                .expect("invalid svg image buffer");

            // Convert from RGBA with premultiplied alpha to BGRA with straight alpha.
            for pixel in buffer.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                if pixel[3] > 0 {
                    let a = pixel[3] as f32 / 255.;
                    pixel[0] = (pixel[0] as f32 / a) as u8;
                    pixel[1] = (pixel[1] as f32 / a) as u8;
                    pixel[2] = (pixel[2] as f32 / a) as u8;
                }
            }

            Ok(Arc::new(RenderImage::new(SmallVec::from_elem(
                Frame::new(buffer),
                1,
            ))))
        }
    }
}

pub struct SvgImg {
    id: ElementId,
    interactivity: Interactivity,
    source: Option<ImageSource>,
    size: Size<Pixels>,
}

impl SvgImg {
    /// Create a new svg image element.
    ///
    /// The `src_width` and `src_height` are the original width and height of the svg image.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            interactivity: Interactivity::default(),
            source: None,
            size: Size::default(),
        }
    }

    /// Set the path of the svg image from the asset.
    ///
    /// The `size` argument is the size of the original svg image.
    #[must_use]
    pub fn source(
        mut self,
        source: impl Into<SvgSource>,
        width: impl Into<Pixels>,
        height: impl Into<Pixels>,
    ) -> Self {
        let size = size(width.into(), height.into());
        self.size = size;
        self.source = Some(ImageSource {
            source: source.into(),
            size,
        });
        self
    }

    pub fn get_source(&self) -> Option<&ImageSource> {
        self.source.as_ref()
    }
}

impl IntoElement for SvgImg {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SvgImg {
    type RequestLayoutState = Option<Arc<RenderImage>>;
    type PrepaintState = (Option<Hitbox>, Option<Arc<RenderImage>>);

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, Self::RequestLayoutState) {
        let layout_id =
            self.interactivity
                .request_layout(global_id, window, cx, |style, window, cx| {
                    window.request_layout(style, None, cx)
                });
        let global_id = global_id.unwrap();

        window.with_element_state::<SvgImgState, _>(global_id, |state, window| {
            match (state, &self.source) {
                (_, None) => ((layout_id, None), SvgImgState(None)),
                (Some(SvgImgState(Some((prev_hash, image)))), Some(source))
                    if hash(source) == prev_hash =>
                {
                    (
                        (layout_id, Some(image.clone())),
                        SvgImgState(Some((prev_hash, image))),
                    )
                }
                (state, Some(source)) => {
                    if let Some(SvgImgState(Some((_, prev_image)))) = state {
                        // Drop the previous image from the cache
                        _ = window.drop_image(prev_image);
                    }

                    let image = window
                        .use_asset::<Image>(&source, cx)
                        .transpose()
                        .ok()
                        .flatten();
                    (
                        (layout_id, image.clone()),
                        SvgImgState(image.map(|image| (hash(source), image))),
                    )
                }
            }
        })
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        state: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = self.interactivity.prepaint(
            global_id,
            bounds,
            bounds.size,
            window,
            cx,
            |_, _, hitbox, _, _| hitbox,
        );

        (hitbox, state.clone())
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let size = self.size;
        let hitbox = state.0.as_ref();
        let data = state.1.clone();

        self.interactivity
            .paint(global_id, bounds, hitbox, window, cx, |_, window, _| {
                if let Some(data) = data {
                    // To calculate the ratio of the original image size to the container bounds size.
                    // Scale by shortest side (width or height) to get a fit image.
                    // And center the image in the container bounds.
                    let ratio = if bounds.size.width < bounds.size.height {
                        bounds.size.width / size.width
                    } else {
                        bounds.size.height / size.height
                    };

                    let ratio = ratio.min(1.0);

                    let new_size = gpui::Size {
                        width: size.width * ratio,
                        height: size.height * ratio,
                    };
                    let new_origin = gpui::Point {
                        x: bounds.origin.x + px(((bounds.size.width - new_size.width) / 2.).into()),
                        y: bounds.origin.y
                            + px(((bounds.size.height - new_size.height) / 2.).into()),
                    };

                    let img_bounds = Bounds {
                        origin: new_origin.map(|origin| origin.floor()),
                        size: new_size.map(|size| size.ceil()),
                    };

                    match window.paint_image(img_bounds, px(0.).into(), data, 0, false) {
                        Ok(_) => {}
                        Err(err) => eprintln!("failed to paint svg image: {:?}", err),
                    }
                }
            })
    }
}

impl Styled for SvgImg {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.interactivity.base_style
    }
}

impl InteractiveElement for SvgImg {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}
