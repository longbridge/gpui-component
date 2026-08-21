use std::rc::Rc;

use gpui::{
    AbsoluteLength, App, DefiniteLength, Entity, FontWeight, IntoElement, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, prelude::FluentBuilder as _, relative,
};

use super::{EditorState, Input, InputFont};
use crate::native_menu::NativeMenu;
use crate::{ActiveTheme as _, RoleOverride, StyledExt as _};

/// A code editor takes its rows from the font, so that a smaller or larger
/// font keeps its leading in proportion.
const EDITOR_LINE_HEIGHT: f32 = 1.5;

/// A styled source-code editor.
#[derive(IntoElement)]
pub struct Editor {
    state: Entity<EditorState>,
    style: StyleRefinement,
    height: Option<DefiniteLength>,
    appearance: bool,
    bordered: bool,
    disabled: bool,
    readonly: bool,
    tab_index: isize,
    role: RoleOverride,
    aria_label: Option<SharedString>,
    font: InputFont,

    /// An optional context menu builder to allow a custom context menu.
    ///
    /// If set, this overrides the built-in context menu.
    context_menu_builder: Option<Rc<dyn Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu>>,
}

impl Editor {
    pub fn new(state: &Entity<EditorState>) -> Self {
        Self {
            state: state.clone(),
            style: StyleRefinement::default(),
            height: None,
            appearance: true,
            bordered: true,
            disabled: false,
            readonly: false,
            tab_index: 0,
            role: RoleOverride::default(),
            aria_label: None,
            font: InputFont::default(),
            context_menu_builder: None,
        }
    }

    /// Set the font of the code.
    ///
    /// The family and size default to [`crate::Theme::mono_font_family`] and
    /// [`crate::Theme::mono_font_size`], and the rows follow the size, so the
    /// editor keeps its leading in proportion at any size. The four settings
    /// below fill this in one at a time.
    pub fn font(mut self, font: InputFont) -> Self {
        self.font = font;
        self
    }

    /// Set the font family of the code, default is [`crate::Theme::mono_font_family`].
    pub fn font_family(mut self, font_family: impl Into<SharedString>) -> Self {
        self.font = self.font.with_family(font_family);
        self
    }

    /// Set the font size of the code, default is [`crate::Theme::mono_font_size`].
    pub fn font_size(mut self, font_size: impl Into<AbsoluteLength>) -> Self {
        self.font = self.font.with_size(font_size);
        self
    }

    /// Set the font weight of the code.
    pub fn font_weight(mut self, font_weight: FontWeight) -> Self {
        self.font = self.font.with_weight(font_weight);
        self
    }

    /// Set the height of one row, a fraction of the font size or an absolute
    /// length, default is 1.5 times the font size.
    pub fn line_height(mut self, line_height: impl Into<DefiniteLength>) -> Self {
        self.font = self.font.with_line_height(line_height);
        self
    }

    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        self.height = Some(height.into());
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the editor to read-only, default is `false`.
    ///
    /// Unlike [`Self::disabled`], a read-only editor keeps the normal appearance
    /// and still can be focused, selected and copied, it only rejects the changes
    /// made by the user.
    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn tab_index(mut self, index: isize) -> Self {
        self.tab_index = index;
        self
    }

    pub fn role(mut self, role: impl Into<RoleOverride>) -> Self {
        self.role = role.into();
        self
    }

    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    /// Replace the built-in context menu shown on right-click.
    ///
    /// The closure receives an empty menu and returns the one to show, so it
    /// decides entirely what appears — the default items are not added.
    pub fn context_menu(
        mut self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.context_menu_builder = Some(Rc::new(f));
        self
    }
}

impl Styled for Editor {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Editor {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        // The editor paints its own text, so the font has to reach it as one
        // resolved value rather than as an ambient style. Each setting comes
        // from this element's own option, else from a style refined onto it —
        // `.text_sm()` and friends — else from the theme.
        let text = &self.style.text;
        let font = InputFont::new()
            .with_family(
                self.font
                    .family()
                    .map(SharedString::from)
                    .or_else(|| text.font_family.clone())
                    .unwrap_or_else(|| cx.theme().mono_font_family.clone()),
            )
            .with_size(
                self.font
                    .size()
                    .or(text.font_size)
                    .unwrap_or_else(|| cx.theme().mono_font_size.into()),
            )
            .with_line_height(
                self.font
                    .line_height()
                    .or(text.line_height)
                    .unwrap_or_else(|| relative(EDITOR_LINE_HEIGHT)),
            )
            .when_some(self.font.weight().or(text.font_weight), |font, weight| {
                font.with_weight(weight)
            });
        self.state.update(cx, |state, cx| state.set_font(font, cx));

        Input::from_state(self.state.clone())
            .appearance(self.appearance)
            .bordered(self.bordered)
            .focus_bordered(false)
            .disabled(self.disabled)
            .readonly(self.readonly)
            .tab_index(self.tab_index)
            .role(self.role)
            .when_some(self.height, |this, height| this.h(height))
            .when_some(self.aria_label, |this, label| this.aria_label(label))
            .when_some(self.context_menu_builder, |this, build| {
                this.context_menu(move |menu, window, cx| build(menu, window, cx))
            })
            .refine_style(&self.style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::EditorState;
    use gpui::{
        AppContext as _, Context, ParentElement as _, Pixels, Render, TestAppContext,
        VisualTestContext, div, px,
    };

    struct Harness {
        state: Entity<EditorState>,
        /// The `font_size` option, when the test sets one.
        font_size: Option<Pixels>,
        /// A text size refined onto the element, as `.text_sm()` would.
        style_size: Option<Pixels>,
    }

    impl Render for Harness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                Editor::new(&self.state)
                    .when_some(self.style_size, |this, size| this.text_size(size))
                    .when_some(self.font_size, |this, size| this.font_size(size)),
            )
        }
    }

    /// The row height the editor laid out with, which follows its font size.
    fn line_height(
        cx: &mut TestAppContext,
        font_size: Option<Pixels>,
        style_size: Option<Pixels>,
    ) -> Pixels {
        cx.update(crate::init);
        let mut state = None;
        let (_, cx) = cx.add_window_view(|window, cx| {
            let editor = cx.new(|cx| EditorState::new(window, cx).default_value("fn main() {}"));
            state = Some(editor.clone());
            Harness {
                state: editor,
                font_size,
                style_size,
            }
        });
        let state = state.unwrap();
        VisualTestContext::update(cx, |window, cx| window.draw(cx).clear(cx));

        cx.read(|cx| {
            state
                .read(cx)
                .line_height()
                .expect("the editor must lay out")
        })
    }

    #[gpui::test]
    fn the_font_size_option_wins_over_a_refined_text_size(cx: &mut TestAppContext) {
        let default = line_height(cx, None, None);
        let refined = line_height(cx, None, Some(px(24.)));
        let option = line_height(cx, Some(px(40.)), Some(px(24.)));

        // The default is the theme's monospace size, not the ambient one.
        assert_eq!(default, px(20.));
        assert_eq!(refined, px(36.));
        assert_eq!(option, px(60.));
    }
}
