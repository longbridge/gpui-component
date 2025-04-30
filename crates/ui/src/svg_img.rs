use std::{
    hash::Hash,
    ops::Deref,
    sync::{Arc, LazyLock},
};

use gpui::{
    hash, px, App, Asset, AssetLogger, Bounds, Element, ElementId, GlobalElementId, Hitbox,
    ImageCacheError, InteractiveElement, Interactivity, IntoElement, Pixels, RenderImage,
    SharedString, StyleRefinement, Styled, Task, Window,
};
use image::Frame;
use smallvec::SmallVec;

use futures_util::{future::Shared, FutureExt};
use image::ImageBuffer;

const SCALE: f32 = 2.;

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
        }
    }
}

enum SvgImageLoader {}

#[derive(Debug, Clone)]
pub struct ImageSource {
    source: SvgSource,
}

impl Hash for ImageSource {
    /// Hash to to control the Asset cache
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.source.hash(state);
    }
}

impl Asset for SvgImageLoader {
    type Source = ImageSource;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        cx: &mut App,
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        let asset_source = cx.asset_source().clone();

        async move {
            let bytes = match source.source.clone() {
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

            // Get svg size
            let svg_size = tree.size();
            let mut pixmap = resvg::tiny_skia::Pixmap::new(
                (svg_size.width() * SCALE) as u32,
                (svg_size.height() * SCALE) as u32,
            )
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

            let image = Arc::new(RenderImage::new(SmallVec::from_elem(Frame::new(buffer), 1)));
            Ok(image)
        }
    }
}

pub struct SvgImg {
    id: ElementId,
    interactivity: Interactivity,
    source: ImageSource,
}

impl SvgImg {
    /// Create a new svg image element.
    ///
    /// The `source` can be a string of SVG XML data or a Asset Path.
    pub fn new(id: impl Into<ElementId>, source: impl Into<SvgSource>) -> Self {
        Self {
            id: id.into(),
            interactivity: Interactivity::default(),
            source: ImageSource {
                source: source.into(),
            },
        }
    }

    /// Get the source of the svg image.
    pub fn source(&self) -> &ImageSource {
        &self.source
    }
}

impl IntoElement for SvgImg {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn load_svg(
    source: &ImageSource,
    window: &mut Window,
    cx: &mut App,
) -> Shared<Task<Result<Arc<RenderImage>, ImageCacheError>>> {
    let fut = AssetLogger::<SvgImageLoader>::load(source.clone(), cx);
    let task = cx.background_executor().spawn(fut).shared();

    let entity = window.current_view();
    window
        .spawn(cx, {
            let task = task.clone();
            async move |cx| {
                _ = task.await;
                cx.on_next_frame(move |_, cx| {
                    cx.notify(entity);
                });
            }
        })
        .detach();
    task
}

struct SvgImgState {
    hash: u64,
    image: Option<Arc<RenderImage>>,
    task: Shared<Task<Result<Arc<RenderImage>, ImageCacheError>>>,
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
        let global_id = global_id.unwrap();

        let layout_id =
            self.interactivity
                .request_layout(Some(global_id), window, cx, |style, window, cx| {
                    window.request_layout(style, None, cx)
                });

        let source = &self.source;
        let source_hash = hash(source);

        window.with_element_state::<Option<SvgImgState>, _>(global_id, |state, window| {
            match state {
                Some(state) => {
                    // Try to keep the previous image if it's still loading.
                    let mut prev_image = None;
                    if let Some(mut state) = state {
                        prev_image = state.image.clone();
                        if source_hash == state.hash {
                            state.image = state
                                .task
                                .clone()
                                .now_or_never()
                                .transpose()
                                .ok()
                                .flatten()
                                .or(state.image);

                            return ((layout_id, state.image.clone()), Some(state));
                        }
                    }

                    let task = load_svg(source, window, cx);
                    let mut image = task.clone().now_or_never().transpose().ok().flatten();
                    if let Some(new_image) = image.as_ref() {
                        _ = window.drop_image(new_image.clone());
                    } else {
                        image = prev_image;
                    }

                    (
                        (layout_id, image.clone()),
                        Some(SvgImgState {
                            hash: source_hash,
                            image,
                            task,
                        }),
                    )
                }
                None => {
                    let task = load_svg(source, window, cx);
                    (
                        (layout_id, None),
                        Some(SvgImgState {
                            hash: source_hash,
                            image: None,
                            task,
                        }),
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
        let hitbox = state.0.as_ref();
        let Some(image) = state.1.take() else {
            return;
        };
        let size = image.size(0).map(|x| x.0 as f32);

        self.interactivity
            .paint(global_id, bounds, hitbox, window, cx, |_, window, _| {
                // To calculate the ratio of the original image size to the container bounds size.
                // Scale by shortest side (width or height) to get a fit image.
                // And center the image in the container bounds.
                let ratio = if bounds.size.width < bounds.size.height {
                    bounds.size.width / size.width
                } else {
                    bounds.size.height / size.height
                };

                let ratio = ratio.0.min(1.0);
                let new_size = size.map(|dim| px(dim) * ratio);

                let new_origin = gpui::Point {
                    x: bounds.origin.x + px(((bounds.size.width - new_size.width) / 2.).into()),
                    y: bounds.origin.y + px(((bounds.size.height - new_size.height) / 2.).into()),
                };

                let img_bounds = Bounds {
                    origin: new_origin.map(|origin| origin.floor()),
                    size: new_size.map(|size| size.ceil()),
                };

                if let Err(err) = window.paint_image(img_bounds, px(0.).into(), image, 0, false) {
                    eprintln!("failed to paint svg image: {:?}", err);
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
