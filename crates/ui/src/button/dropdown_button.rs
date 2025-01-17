use gpui::{
    prelude::FluentBuilder, Corner, Corners, Edges, ElementId, IntoElement, ParentElement,
    RenderOnce, ViewContext, WindowContext,
};

use crate::{
    h_flex,
    popup_menu::{PopupMenu, PopupMenuExt},
    IconName,
};

use super::Button;

#[derive(IntoElement)]
pub struct DropdownButton {
    id: ElementId,
    button: Option<Button>,
    popup_menu: Option<Box<dyn Fn(PopupMenu, &mut ViewContext<PopupMenu>) -> PopupMenu + 'static>>,
}

impl DropdownButton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            button: None,
            popup_menu: None,
        }
    }

    pub fn button(mut self, button: Button) -> Self {
        self.button = Some(
            button
                .border_corners(Corners {
                    top_left: true,
                    top_right: false,
                    bottom_left: true,
                    bottom_right: false,
                })
                .border_edges(Edges {
                    left: true,
                    top: true,
                    right: true,
                    bottom: true,
                }),
        );
        self
    }

    pub fn popup_menu(
        mut self,
        popup_menu: impl Fn(PopupMenu, &mut ViewContext<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.popup_menu = Some(Box::new(popup_menu));
        self
    }
}

impl RenderOnce for DropdownButton {
    fn render(self, _: &mut WindowContext) -> impl IntoElement {
        h_flex().when_some(self.button, |this, button| {
            this.child(button)
                .when_some(self.popup_menu, |this, popup_menu| {
                    this.child(
                        Button::new(self.id)
                            .icon(IconName::ChevronDown)
                            .border_edges(Edges {
                                left: false,
                                top: true,
                                right: true,
                                bottom: true,
                            })
                            .border_corners(Corners {
                                top_left: false,
                                top_right: true,
                                bottom_left: false,
                                bottom_right: true,
                            })
                            .popup_menu_with_anchor(Corner::TopRight, move |this, cx| {
                                popup_menu(this, cx)
                            }),
                    )
                })
        })
    }
}
