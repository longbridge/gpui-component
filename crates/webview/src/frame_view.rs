use std::sync::Arc;

use gpui::{RenderImage, Window};

#[derive(Debug, Default)]
pub(crate) struct FrameView {
    id: usize,
    rendered_frame: Option<(usize, Arc<RenderImage>)>,
    frame: Option<(usize, Arc<RenderImage>)>,
}

impl FrameView {
    pub(crate) fn render(&mut self, window: &mut Window) -> Option<Arc<RenderImage>> {
        let (current_frame_id, image) = self.frame.clone()?;

        if let Some((rendered_frame_id, rendered_image)) = self.rendered_frame.take() {
            if rendered_frame_id != current_frame_id {
                _ = window.drop_image(rendered_image);
            }
            self.rendered_frame = None;
        }

        self.rendered_frame = self.frame.clone();
        Some(image)
    }

    pub(crate) fn update(&mut self, image: Arc<RenderImage>) {
        self.frame = Some((self.id, image));
        self.id += 1;
    }

    pub(crate) fn clear(&mut self, window: &mut Window) {
        if let Some((_, image)) = self.rendered_frame.take() {
            _ = window.drop_image(image);
        }
    }
}
