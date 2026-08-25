use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Bounds, ContentMask, Element, ElementId,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, LineLayout, ParentElement,
    Pixels, Point, RenderOnce, SharedString, StyleRefinement, Styled, StyledText, TextAlign,
    Window, WrapBoundary, div, point, prelude::FluentBuilder as _, px, size,
};
use instant::Duration;

use crate::{ActiveTheme as _, Sizable as _, StyledExt as _, h_flex, spinner::Spinner};

/// The visual treatment used by a [`Marker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerVariant {
    /// An inline marker with no additional divider.
    #[default]
    Plain,
    /// A centered marker with semantic divider lines on both sides.
    Separator,
    /// A marker with a semantic bottom border.
    Border,
}

/// The visual treatment used while a [`Marker`] is loading.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerLoadingStyle {
    /// Show a compact rotating spinner beside the marker content.
    #[default]
    Spinner,
    /// Sweep a highlight across marker content without adding an icon.
    Shimmer,
}

enum MarkerChild {
    Icon(MarkerIcon),
    Content(MarkerContent),
    Element(AnyElement),
}

/// A compact, composable row for conversation status and system markers.
///
/// `Marker` intentionally accepts arbitrary children. An icon, text, spinner,
/// or action can be composed directly without introducing fixed icon and
/// content slots. Use [`Styled`] methods on the marker to refine its layout or
/// typography for an application-specific use. Loading effects only affect
/// configured content slots, so icons and separators retain their appearance.
#[derive(IntoElement)]
pub struct Marker {
    style: StyleRefinement,
    separator_style: StyleRefinement,
    variant: MarkerVariant,
    loading: bool,
    loading_style: MarkerLoadingStyle,
    children: Vec<MarkerChild>,
}

impl Marker {
    /// Create a plain marker.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            separator_style: StyleRefinement::default(),
            variant: MarkerVariant::default(),
            loading: false,
            loading_style: MarkerLoadingStyle::default(),
            children: Vec::new(),
        }
    }

    /// Set the visual treatment of the marker.
    pub fn with_variant(mut self, variant: MarkerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set whether the marker should display its configured loading effect.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Set the visual treatment used when [`Self::loading`] is enabled.
    pub fn with_loading_style(mut self, loading_style: MarkerLoadingStyle) -> Self {
        self.loading_style = loading_style;
        self
    }

    /// Refine the decorative lines used by [`MarkerVariant::Separator`].
    pub fn separator_style(mut self, style: StyleRefinement) -> Self {
        self.separator_style = style;
        self
    }

    /// Add a configured icon slot.
    pub fn icon(mut self, icon: MarkerIcon) -> Self {
        self.children.push(MarkerChild::Icon(icon));
        self
    }

    /// Add a configured content slot.
    pub fn content(mut self, content: MarkerContent) -> Self {
        self.children.push(MarkerChild::Content(content));
        self
    }
}

impl Default for Marker {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Marker {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(MarkerChild::Element));
    }
}

impl Styled for Marker {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Marker {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let variant = self.variant;
        let loading = self.loading;
        let loading_style = self.loading_style;
        let has_icon = self
            .children
            .iter()
            .any(|child| matches!(child, MarkerChild::Icon(_)));
        let separator_style = self.separator_style;
        let children = self.children.into_iter().map(move |child| match child {
            MarkerChild::Icon(icon) => icon.into_any_element(),
            MarkerChild::Content(mut content) => {
                content.shimmer = loading && loading_style == MarkerLoadingStyle::Shimmer;
                content.into_any_element()
            }
            MarkerChild::Element(element) => element,
        });

        h_flex()
            .w_full()
            .min_h(tokens.spacing.lg)
            .gap(tokens.spacing.sm)
            .text_size(tokens.typography.sm.size)
            .line_height(tokens.typography.sm.line_height)
            .text_color(tokens.colors.muted_foreground)
            .text_left()
            .when(variant == MarkerVariant::Separator, |this| {
                this.justify_center()
            })
            .when(variant == MarkerVariant::Border, |this| {
                this.border_b_1()
                    .border_color(tokens.colors.border)
                    .pb(tokens.spacing.sm)
            })
            .when(variant == MarkerVariant::Separator, |this| {
                this.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(tokens.spacing.xxs / 2.)
                        .mr(tokens.spacing.xs)
                        .bg(tokens.colors.border)
                        .refine_style(&separator_style),
                )
            })
            .when(
                loading && loading_style == MarkerLoadingStyle::Spinner && !has_icon,
                |this| this.child(MarkerIcon::new().child(Spinner::new().xsmall())),
            )
            .children(children)
            .when(variant == MarkerVariant::Separator, |this| {
                this.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h(tokens.spacing.xxs / 2.)
                        .ml(tokens.spacing.xs)
                        .bg(tokens.colors.border)
                        .refine_style(&separator_style),
                )
            })
            .refine_style(&self.style)
    }
}

/// A compact decorative icon slot inside a [`Marker`].
#[derive(IntoElement)]
pub struct MarkerIcon {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl MarkerIcon {
    /// Create an empty marker icon slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }
}

impl Default for MarkerIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MarkerIcon {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MarkerIcon {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MarkerIcon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        h_flex()
            .size(tokens.spacing.lg)
            .flex_none()
            .items_center()
            .justify_center()
            .refine_style(&self.style)
            .children(self.children)
    }
}

/// The independently styleable text or rich-content slot in a [`Marker`].
#[derive(IntoElement)]
pub struct MarkerContent {
    style: StyleRefinement,
    shimmer: bool,
    children: Vec<MarkerContentChild>,
}

enum MarkerContentChild {
    Text(SharedString),
    Element(AnyElement),
}

impl MarkerContent {
    /// Create an empty marker content slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            shimmer: false,
            children: Vec::new(),
        }
    }

    /// Add text that can receive a continuous loading shimmer.
    ///
    /// Arbitrary children remain supported through [`ParentElement`].
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.children.push(MarkerContentChild::Text(text.into()));
        self
    }
}

impl Default for MarkerContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for MarkerContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(MarkerContentChild::Element));
    }
}

impl Styled for MarkerContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for MarkerContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let animate = self.shimmer && !cx.reduce_motion();
        let has_text = self
            .children
            .iter()
            .any(|child| matches!(child, MarkerContentChild::Text(_)));
        let base_opacity = self.style.opacity.unwrap_or(1.);
        let highlight_color = shimmer_highlight_color(
            cx.theme().semantic_tokens().colors.background,
            cx.theme().is_dark(),
        );
        let children =
            self.children
                .into_iter()
                .enumerate()
                .map(move |(index, child)| match child {
                    MarkerContentChild::Text(text) if animate => {
                        MarkerShimmerText::new(text, highlight_color)
                            .with_animation(
                                ("marker-loading-text", index),
                                loading_animation(),
                                |mut this, phase| {
                                    this.phase = phase;
                                    this
                                },
                            )
                            .into_any_element()
                    }
                    MarkerContentChild::Text(text) => StyledText::new(text).into_any_element(),
                    MarkerContentChild::Element(element) => element,
                });

        let content = div().min_w_0().refine_style(&self.style).children(children);

        if animate && !has_text {
            content
                .with_animation(
                    "marker-loading-content",
                    loading_animation(),
                    move |this, phase| {
                        let highlight = (phase * std::f32::consts::TAU).cos().mul_add(0.5, 0.5);
                        this.opacity(base_opacity * highlight.mul_add(0.4, 0.6))
                    },
                )
                .into_any_element()
        } else {
            content.into_any_element()
        }
    }
}

const SHIMMER_LAYER_COUNT: usize = 12;
const SHIMMER_BAND_HALF_WIDTH: f32 = 0.3;

/// Paint an animated highlight over glyphs already laid out by `StyledText`.
///
/// Keeping `StyledText` as the layout owner preserves wrapping, truncation,
/// inherited typography, and GPUI's glyph cache. Nested content masks produce
/// a soft continuous band without rebuilding text runs on every frame.
struct MarkerShimmerText {
    text: StyledText,
    highlight_color: Hsla,
    phase: f32,
}

impl MarkerShimmerText {
    fn new(text: SharedString, highlight_color: Hsla) -> Self {
        Self {
            text: StyledText::new(text),
            highlight_color,
            phase: 0.,
        }
    }

    fn paint_highlight(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        let masks = std::array::from_fn::<_, SHIMMER_LAYER_COUNT, _>(|layer| {
            shimmer_band_bounds(bounds, self.phase, layer).map(|bounds| ContentMask { bounds })
        });

        if masks.iter().all(Option::is_none) {
            return;
        }

        let layout = self.text.layout();
        let line_height = layout.line_height();
        let text_align = window.text_style().text_align;
        let mut line_origin = bounds.origin;

        window.paint_layer(bounds, |window| {
            for wrapped_line in layout.line_layouts() {
                let line = &wrapped_line.unwrapped_layout;
                let baseline_offset = point(
                    px(0.),
                    (line_height - line.ascent - line.descent) / 2. + line.ascent,
                );
                let mut wraps = wrapped_line.wrap_boundaries.iter().peekable();
                let mut glyph_origin = point(
                    shimmer_aligned_origin_x(
                        line_origin,
                        bounds.size.width,
                        px(0.),
                        text_align,
                        line,
                        wraps.peek().copied(),
                    ),
                    line_origin.y,
                );
                let mut previous_glyph_position = Point::default();

                for (run_index, run) in line.runs.iter().enumerate() {
                    let glyph_size = cx
                        .text_system()
                        .bounding_box(run.font_id, line.font_size)
                        .size;

                    for (glyph_index, glyph) in run.glyphs.iter().enumerate() {
                        glyph_origin.x += glyph.position.x - previous_glyph_position.x;

                        if wraps.peek().is_some_and(|wrap| {
                            wrap.run_ix == run_index && wrap.glyph_ix == glyph_index
                        }) {
                            wraps.next();
                            glyph_origin.x = shimmer_aligned_origin_x(
                                line_origin,
                                bounds.size.width,
                                glyph.position.x,
                                text_align,
                                line,
                                wraps.peek().copied(),
                            );
                            glyph_origin.y += line_height;
                        }

                        previous_glyph_position = glyph.position;

                        if glyph.is_emoji {
                            continue;
                        }

                        let glyph_bounds = Bounds::new(glyph_origin, glyph_size);
                        let paint_origin =
                            glyph_origin + baseline_offset + point(px(0.), glyph.position.y);

                        for mask in masks.iter().flatten() {
                            if !glyph_bounds.intersects(&mask.bounds) {
                                continue;
                            }

                            window.with_content_mask(Some(*mask), |window| {
                                let _ = window.paint_glyph(
                                    paint_origin,
                                    run.font_id,
                                    glyph.id,
                                    line.font_size,
                                    self.highlight_color,
                                );
                            });
                        }
                    }
                }

                line_origin.y += wrapped_line.size(line_height).height;
            }
        });
    }
}

impl IntoElement for MarkerShimmerText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for MarkerShimmerText {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.text
            .request_layout(global_id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.text
            .prepaint(global_id, inspector_id, bounds, layout, window, cx);
    }

    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.text.paint(
            global_id,
            inspector_id,
            bounds,
            layout,
            prepaint,
            window,
            cx,
        );
        self.paint_highlight(bounds, window, cx);
    }
}

fn loading_animation() -> Animation {
    Animation::new(Duration::from_secs(2)).repeat_synced()
}

fn shimmer_highlight_color(background: Hsla, dark: bool) -> Hsla {
    let peak_opacity: f32 = if dark { 0.6 } else { 0.75 };
    let layer_opacity = 1. - (1. - peak_opacity).powf(1. / SHIMMER_LAYER_COUNT as f32);

    background.opacity(layer_opacity)
}

fn shimmer_band_bounds(bounds: Bounds<Pixels>, phase: f32, layer: usize) -> Option<Bounds<Pixels>> {
    let width = bounds.size.width.as_f32();

    if width <= 0. || bounds.size.height <= px(0.) || layer >= SHIMMER_LAYER_COUNT {
        return None;
    }

    let center = phase.mul_add(1.7, -0.35) * width;
    let radius = width * SHIMMER_BAND_HALF_WIDTH * (1. - layer as f32 / SHIMMER_LAYER_COUNT as f32);
    let left = (center - radius).max(0.);
    let right = (center + radius).min(width);

    (right > left).then(|| {
        Bounds::new(
            point(bounds.origin.x + px(left), bounds.origin.y),
            size(px(right - left), bounds.size.height),
        )
    })
}

fn shimmer_aligned_origin_x(
    origin: Point<Pixels>,
    align_width: Pixels,
    previous_glyph_x: Pixels,
    align: TextAlign,
    layout: &LineLayout,
    next_wrap: Option<&WrapBoundary>,
) -> Pixels {
    let line_end = next_wrap
        .map(|wrap| layout.runs[wrap.run_ix].glyphs[wrap.glyph_ix].position.x)
        .unwrap_or(layout.width);
    let line_width = line_end - previous_glyph_x;

    match align {
        TextAlign::Left => origin.x,
        TextAlign::Center => (origin.x * 2. + align_width - line_width) / 2.,
        TextAlign::Right => origin.x + align_width - line_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marker_builder() {
        let marker = Marker::new()
            .with_variant(MarkerVariant::Separator)
            .loading(true)
            .with_loading_style(MarkerLoadingStyle::Shimmer)
            .separator_style(StyleRefinement::default())
            .content(MarkerContent::new().child("Today"));

        assert_eq!(marker.variant, MarkerVariant::Separator);
        assert!(marker.loading);
        assert_eq!(marker.loading_style, MarkerLoadingStyle::Shimmer);
        assert_eq!(marker.children.len(), 1);
        assert_eq!(Marker::default().variant, MarkerVariant::Plain);
        assert!(!Marker::default().loading);
        assert_eq!(Marker::default().loading_style, MarkerLoadingStyle::Spinner);

        let content_first = Marker::new()
            .content(MarkerContent::new().text("Thinking"))
            .with_loading_style(MarkerLoadingStyle::Shimmer)
            .loading(true);
        assert!(content_first.loading);
        assert_eq!(content_first.loading_style, MarkerLoadingStyle::Shimmer);
        assert!(matches!(
            &content_first.children[0],
            MarkerChild::Content(_)
        ));

        let custom_icon = Marker::new()
            .loading(true)
            .icon(MarkerIcon::new().child("custom"))
            .content(MarkerContent::new().text("Loading"));
        assert_eq!(custom_icon.children.len(), 2);
        assert!(matches!(&custom_icon.children[0], MarkerChild::Icon(_)));

        let styled = Marker::new().opacity(0.37).child("Status").child("Details");

        assert_eq!(styled.style.opacity, Some(0.37));
        assert_eq!(styled.children.len(), 2);

        let icon = MarkerIcon::new().child("icon");
        assert_eq!(icon.children.len(), 1);

        let content = MarkerContent::new()
            .text("Thinking")
            .child("…")
            .text("正在思考");
        assert_eq!(content.children.len(), 3);
        assert!(matches!(&content.children[0], MarkerContentChild::Text(_)));
        assert!(matches!(
            &content.children[1],
            MarkerContentChild::Element(_)
        ));
        assert!(matches!(&content.children[2], MarkerContentChild::Text(_)));
    }

    #[test]
    fn test_marker_shimmer_band_moves_smoothly_across_text() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(100.), px(18.)));

        assert!(shimmer_band_bounds(bounds, 0., 0).is_none());
        assert!(shimmer_band_bounds(bounds, 1., 0).is_none());

        let early = shimmer_band_bounds(bounds, 0.35, 0).unwrap();
        let late = shimmer_band_bounds(bounds, 0.65, 0).unwrap();
        assert!(early.origin.x < late.origin.x);

        let outer = shimmer_band_bounds(bounds, 0.5, 0).unwrap();
        let inner = shimmer_band_bounds(bounds, 0.5, SHIMMER_LAYER_COUNT - 1).unwrap();
        assert!(inner.origin.x > outer.origin.x);
        assert!(inner.size.width < outer.size.width);
        assert!(shimmer_band_bounds(bounds, 0.5, SHIMMER_LAYER_COUNT).is_none());
        assert!(
            shimmer_band_bounds(Bounds::new(bounds.origin, size(px(0.), px(18.))), 0.5, 0)
                .is_none()
        );
    }

    #[test]
    fn test_marker_shimmer_uses_theme_background_and_native_frame_rate() {
        let light = shimmer_highlight_color(Hsla::white(), false);
        let dark = shimmer_highlight_color(Hsla::black(), true);

        assert_eq!(light.l, 1.);
        assert_eq!(dark.l, 0.);
        assert!(light.a > dark.a);
        assert!((1. - (1. - light.a).powi(SHIMMER_LAYER_COUNT as i32) - 0.75).abs() < 0.001);
        assert!((1. - (1. - dark.a).powi(SHIMMER_LAYER_COUNT as i32) - 0.6).abs() < 0.001);

        let animation = loading_animation();
        assert!(animation.synced);
        assert_eq!(animation.max_fps, None);
    }
}
