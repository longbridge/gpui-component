use gpui::{App, IntoElement, ParentElement, Role};

use crate::Dialog;

/// Alert-dialog specialization of the Base modal host.
pub struct AlertDialog(Dialog);

impl AlertDialog {
    pub fn new(cx: &mut App) -> Self {
        Self(
            Dialog::new(cx)
                .role(Role::AlertDialog)
                .overlay_closable(false),
        )
    }
    pub fn dialog(mut self, build: impl FnOnce(Dialog) -> Dialog) -> Self {
        self.0 = build(self.0);
        self
    }
    pub fn into_dialog(self) -> Dialog {
        self.0
    }
}

impl ParentElement for AlertDialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = gpui::AnyElement>) {
        self.0.extend(elements);
    }
}
impl IntoElement for AlertDialog {
    type Element = <Dialog as IntoElement>::Element;
    fn into_element(self) -> Self::Element {
        self.0.into_element()
    }
}
