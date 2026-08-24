use gpui::{
    AnyElement, App, Axis, ImageSource, IntoElement, ObjectFit, ParentElement, RenderOnce,
    SharedString, StyleRefinement, Styled, StyledImage as _, Window, div, img,
    prelude::FluentBuilder as _,
};

use crate::{ActiveTheme as _, Sizable, Size, StyledExt as _, v_flex};

/// The lifecycle status of an attachment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AttachmentStatus {
    /// The attachment has been selected and is waiting to be uploaded.
    Pending,
    /// The attachment is currently being uploaded.
    Uploading,
    /// The upload has completed and the attachment is being processed.
    Processing,
    /// The attachment failed to upload or process.
    Failed,
    /// The attachment is ready.
    #[default]
    Complete,
}

impl AttachmentStatus {
    /// Returns whether the attachment is waiting to start.
    pub fn is_pending(self) -> bool {
        matches!(self, Self::Pending)
    }

    /// Returns whether the attachment is being uploaded.
    pub fn is_uploading(self) -> bool {
        matches!(self, Self::Uploading)
    }

    /// Returns whether the attachment is being processed.
    pub fn is_processing(self) -> bool {
        matches!(self, Self::Processing)
    }

    /// Returns whether the attachment has failed.
    pub fn is_failed(self) -> bool {
        matches!(self, Self::Failed)
    }

    /// Returns whether the attachment is in an in-progress state.
    pub fn is_in_progress(self) -> bool {
        matches!(self, Self::Uploading | Self::Processing)
    }
}

/// A file or image attachment composed from media, content, and actions slots.
#[derive(IntoElement)]
pub struct Attachment {
    style: StyleRefinement,
    status: AttachmentStatus,
    size: Size,
    axis: Axis,
    media: Option<AttachmentMedia>,
    content: Option<AttachmentContent>,
    actions: Option<AttachmentActions>,
}

impl Attachment {
    /// Create an attachment in the [`AttachmentStatus::Complete`] state.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            status: AttachmentStatus::Complete,
            size: Size::Medium,
            axis: Axis::Horizontal,
            media: None,
            content: None,
            actions: None,
        }
    }

    /// Set the attachment lifecycle status.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = status;
        self
    }

    /// Set the attachment layout axis.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Set the media slot.
    pub fn media(mut self, media: AttachmentMedia) -> Self {
        self.media = Some(media);
        self
    }

    /// Set the metadata content slot.
    pub fn content(mut self, content: AttachmentContent) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the actions slot.
    pub fn actions(mut self, actions: AttachmentActions) -> Self {
        self.actions = Some(actions);
        self
    }
}

impl Default for Attachment {
    fn default() -> Self {
        Self::new()
    }
}

impl Sizable for Attachment {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Attachment {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Attachment {
    fn layout_slots(&mut self) {
        let size = self.size;
        let axis = self.axis;
        let status = self.status;

        self.media = self.media.take().map(|media| media.layout(size, status));
        self.content = self
            .content
            .take()
            .map(|content| content.layout_for_axis(axis));
        self.actions = self
            .actions
            .take()
            .map(|actions| actions.layout_for_axis(axis));
    }
}

impl RenderOnce for Attachment {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let size = self.size;
        let axis = self.axis;
        let status = self.status;

        self.layout_slots();

        let (gap, padding_x, padding_y) = attachment_spacing(size, &tokens);

        let mut element = div()
            .relative()
            .flex()
            .flex_none()
            .max_w_full()
            .min_w_0()
            .gap(gap)
            .rounded(tokens.radius.lg)
            .border_1()
            .border_color(if status.is_failed() {
                tokens.colors.destructive.opacity(0.3)
            } else {
                tokens.colors.border
            })
            .when(status.is_pending(), |this| this.border_dashed())
            .bg(tokens.colors.surface)
            .text_color(tokens.colors.surface_foreground)
            .px(padding_x)
            .py(padding_y)
            .text_size(attachment_text_size(size, &tokens))
            .line_height(tokens.typography.sm.line_height);

        element = match axis {
            Axis::Horizontal => element.items_center(),
            Axis::Vertical => element.flex_col().items_start(),
        };

        element
            .when_some(self.media, |this, media| this.child(media))
            .when_some(self.content, |this, content| this.child(content))
            .when_some(self.actions, |this, actions| this.child(actions))
            .refine_style(&self.style)
    }
}

/// The media slot for an attachment.
///
/// Add an icon or another element as a child for an icon-style preview. Use
/// [`Self::src`] when the attachment has an image preview.
#[derive(IntoElement)]
pub struct AttachmentMedia {
    style: StyleRefinement,
    size: Option<Size>,
    status: AttachmentStatus,
    source: Option<ImageSource>,
    children: Vec<AnyElement>,
}

impl AttachmentMedia {
    /// Create an empty media slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: None,
            status: AttachmentStatus::Complete,
            source: None,
            children: Vec::new(),
        }
    }

    /// Set an image preview source.
    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.source = Some(source.into());
        self
    }

    fn layout(mut self, size: Size, status: AttachmentStatus) -> Self {
        if self.size.is_none() {
            self.size = Some(size);
        }
        self.status = status;
        self
    }
}

impl Default for AttachmentMedia {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AttachmentMedia {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for AttachmentMedia {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl Styled for AttachmentMedia {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentMedia {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let resolved_size = self.size.unwrap_or_default();
        let size = media_size(resolved_size);
        let radius = media_radius(resolved_size, &tokens);
        let source = self.source;
        let has_source = source.is_some();
        let failed_media = self.status.is_failed() && !has_source;
        let children = self.children;

        let element = div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .w(size)
            .h(size)
            .rounded(radius)
            .bg(if failed_media {
                tokens.colors.destructive.opacity(0.1)
            } else {
                tokens.colors.muted
            })
            .text_color(if failed_media {
                tokens.colors.destructive
            } else {
                tokens.colors.foreground
            })
            .when_some(source, |this, source| {
                this.child(img(source).size_full().object_fit(ObjectFit::Cover))
            })
            .when(!has_source, |this| this.children(children))
            .refine_style(&self.style);

        element
    }
}

/// The metadata slot for an attachment.
#[derive(IntoElement)]
pub struct AttachmentContent {
    style: StyleRefinement,
    vertical_layout: bool,
    children: Vec<AnyElement>,
}

impl AttachmentContent {
    /// Create an empty metadata slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            vertical_layout: false,
            children: Vec::new(),
        }
    }

    fn layout_for_axis(mut self, axis: Axis) -> Self {
        self.vertical_layout = axis == Axis::Vertical;
        self
    }
}

impl Default for AttachmentContent {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AttachmentContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for AttachmentContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentContent {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        v_flex()
            .max_w_full()
            .min_w_0()
            .flex_1()
            .gap(tokens.spacing.xxs)
            .line_height(tokens.typography.sm.line_height)
            .when(self.vertical_layout, |this| {
                this.w_full().px(tokens.spacing.xs)
            })
            .children(self.children)
            .refine_style(&self.style)
    }
}

/// A single-line attachment title.
#[derive(IntoElement)]
pub struct AttachmentTitle {
    style: StyleRefinement,
    text: SharedString,
}

impl AttachmentTitle {
    /// Create an attachment title.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
        }
    }
}

impl Styled for AttachmentTitle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentTitle {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();

        div()
            .max_w_full()
            .min_w_0()
            .truncate()
            .font_medium()
            .line_height(tokens.typography.sm.line_height)
            .child(self.text)
            .refine_style(&self.style)
    }
}

/// A single-line attachment description or status message.
#[derive(IntoElement)]
pub struct AttachmentDescription {
    style: StyleRefinement,
    text: SharedString,
    status: Option<AttachmentStatus>,
}

impl AttachmentDescription {
    /// Create an attachment description.
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            style: StyleRefinement::default(),
            text: text.into(),
            status: None,
        }
    }

    /// Set the status used for the semantic description color.
    pub fn status(mut self, status: AttachmentStatus) -> Self {
        self.status = Some(status);
        self
    }
}

impl Styled for AttachmentDescription {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentDescription {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        let color = self
            .status
            .is_some_and(AttachmentStatus::is_failed)
            .then(|| tokens.colors.destructive.opacity(0.8))
            .unwrap_or(tokens.colors.muted_foreground);

        div()
            .max_w_full()
            .min_w_0()
            .truncate()
            .text_size(tokens.typography.xs.size)
            .line_height(tokens.typography.xs.line_height)
            .text_color(color)
            .child(self.text)
            .refine_style(&self.style)
    }
}

/// A composition slot for attachment actions.
///
/// Add existing [`crate::button::Button`] or other controls as children. A
/// separate attachment-specific action wrapper is intentionally unnecessary.
#[derive(IntoElement)]
pub struct AttachmentActions {
    style: StyleRefinement,
    vertical_layout: bool,
    children: Vec<AnyElement>,
}

impl AttachmentActions {
    /// Create an empty actions slot.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            vertical_layout: false,
            children: Vec::new(),
        }
    }

    fn layout_for_axis(mut self, axis: Axis) -> Self {
        self.vertical_layout = axis == Axis::Vertical;
        self
    }
}

impl Default for AttachmentActions {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for AttachmentActions {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for AttachmentActions {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AttachmentActions {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let tokens = cx.theme().semantic_tokens();
        div()
            .relative()
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(tokens.spacing.xs)
            .when(self.vertical_layout, |this| {
                this.absolute()
                    .top(tokens.spacing.md)
                    .right(tokens.spacing.md)
            })
            .children(self.children)
            .refine_style(&self.style)
    }
}

fn attachment_spacing(
    size: Size,
    tokens: &gpui_base::SemanticThemeTokens,
) -> (gpui::Pixels, gpui::Pixels, gpui::Pixels) {
    match size {
        Size::XSmall => (tokens.spacing.xs, tokens.spacing.xs, tokens.spacing.xs),
        Size::Small => (tokens.spacing.sm, tokens.spacing.sm, tokens.spacing.xs),
        Size::Medium => (tokens.spacing.sm, tokens.spacing.md, tokens.spacing.sm),
        Size::Large => (tokens.spacing.md, tokens.spacing.lg, tokens.spacing.md),
        Size::Size(value) => {
            let padding = value * 0.25;
            (tokens.spacing.xs, padding, padding)
        }
    }
}

fn attachment_text_size(size: Size, tokens: &gpui_base::SemanticThemeTokens) -> gpui::Pixels {
    match size {
        Size::XSmall => tokens.typography.xs.size,
        Size::Small | Size::Medium => tokens.typography.sm.size,
        Size::Large => tokens.typography.md.size,
        Size::Size(value) => value * 0.875,
    }
}

fn media_size(size: Size) -> gpui::Pixels {
    match size {
        Size::XSmall => gpui::px(28.),
        Size::Small => gpui::px(32.),
        Size::Medium => gpui::px(40.),
        Size::Large => gpui::px(48.),
        Size::Size(value) => value,
    }
}

fn media_radius(size: Size, tokens: &gpui_base::SemanticThemeTokens) -> gpui::Pixels {
    match size {
        Size::XSmall => tokens.radius.md,
        Size::Small => tokens.radius.md,
        Size::Medium | Size::Large | Size::Size(_) => tokens.radius.lg,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_builder() {
        let mut attachment = Attachment::new()
            .status(AttachmentStatus::Uploading)
            .axis(Axis::Vertical)
            .with_size(Size::Small)
            .media(AttachmentMedia::new().src("preview.png"))
            .content(
                AttachmentContent::new()
                    .child(AttachmentTitle::new("report.pdf"))
                    .child(AttachmentDescription::new("Uploading")),
            )
            .actions(AttachmentActions::new().child("Cancel"));

        assert_eq!(attachment.status, AttachmentStatus::Uploading);
        assert_eq!(attachment.axis, Axis::Vertical);
        assert_eq!(attachment.size, Size::Small);
        assert!(attachment.media.is_some());
        assert!(attachment.content.is_some());
        assert!(attachment.actions.is_some());

        attachment.layout_slots();
        assert_eq!(attachment.media.as_ref().unwrap().size, Some(Size::Small));
        assert_eq!(
            attachment.media.as_ref().unwrap().status,
            AttachmentStatus::Uploading
        );
        assert!(attachment.content.as_ref().unwrap().vertical_layout);
        assert!(attachment.actions.as_ref().unwrap().vertical_layout);
    }

    #[test]
    fn test_attachment_defaults_and_status_helpers() {
        assert_eq!(Attachment::new().status, AttachmentStatus::Complete);
        assert_eq!(AttachmentStatus::default(), AttachmentStatus::Complete);
        assert!(AttachmentStatus::Pending.is_pending());
        assert!(AttachmentStatus::Uploading.is_in_progress());
        assert!(AttachmentStatus::Processing.is_processing());
        assert!(AttachmentStatus::Failed.is_failed());
        assert!(!AttachmentStatus::Complete.is_in_progress());
    }

    #[test]
    fn test_attachment_slots_are_composable() {
        let media = AttachmentMedia::new().child("icon");
        assert_eq!(media.children.len(), 1);

        let content = AttachmentContent::new().child(AttachmentTitle::new("name"));
        assert_eq!(content.children.len(), 1);

        let actions = AttachmentActions::new().child("remove");
        assert_eq!(actions.children.len(), 1);
    }

    #[test]
    fn test_attachment_media_size_inherits_root_unless_explicit() {
        let inherited = AttachmentMedia::new().layout(Size::Small, AttachmentStatus::Complete);
        assert_eq!(inherited.size, Some(Size::Small));

        let explicit = AttachmentMedia::new()
            .with_size(Size::XSmall)
            .layout(Size::Large, AttachmentStatus::Failed);
        assert_eq!(explicit.size, Some(Size::XSmall));
        assert_eq!(explicit.status, AttachmentStatus::Failed);
    }
}
