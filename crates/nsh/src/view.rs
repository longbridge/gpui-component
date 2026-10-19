use gpui_kit::{
    App, AppContext as _, Context, Entity, FocusHandle, Focusable, IntoElement, Render,
    Subscription, Window,
    component::{ActiveTheme as _, Theme},
};
use nsh_platform_gpui::{AppOptions, AppView, Components};

use crate::theme;

/// An NSH app embedded in a GPUI Kit window.
///
/// It runs one app at a time on an engine of its own, fills the space its
/// parent gives it, and follows GPUI Kit's theme and light or dark mode. Apps
/// it runs can use GPUI Base's controls ([`crate::base`]); register more
/// controls by passing a [`crate::Catalog`] to [`NshView::with_catalog`].
///
/// Dropping the view stops its engine.
pub struct NshView {
    app: Entity<AppView>,
    /// Shared with the surface: controls registered here reach its apps.
    components: Components,
    _theme: Subscription,
}

impl NshView {
    /// A view showing nothing until [`NshView::load`].
    pub fn new(options: AppOptions, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let components = options.components.clone();
        crate::base::catalog().register(&components);
        let app = cx.new(|cx| AppView::new(options, window, cx));
        Self::apply_theme(&app, cx);
        let theme = cx.observe_global::<Theme>(|this: &mut Self, cx| {
            Self::apply_theme(&this.app, cx);
        });
        Self {
            app,
            components,
            _theme: theme,
        }
    }

    /// Registers `catalog`'s controls for the apps loaded from now on.
    pub fn with_catalog(self, catalog: &crate::Catalog) -> Self {
        catalog.register(&self.components);
        self
    }

    /// Runs `app` (a bundle or a Rust root), replacing the one running.
    pub fn load(&self, app: impl Into<nsh_platform_gpui::App>, cx: &App) {
        self.app.read(cx).load(app.into());
    }

    /// The NSH surface, for what this view does not wrap: limits, asset
    /// locations, error handlers, the dev server.
    pub fn app_view(&self) -> &Entity<AppView> {
        &self.app
    }

    fn apply_theme(app: &Entity<AppView>, cx: &mut App) {
        let theme = cx.theme();
        let (nsh_theme, appearance) = (theme::nsh_theme(theme), theme::appearance(theme.mode));
        app.update(cx, |app, cx| {
            app.set_theme(Some(nsh_theme), cx);
            app.set_appearance(appearance, cx);
        });
    }
}

impl Focusable for NshView {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.app.focus_handle(cx)
    }
}

impl Render for NshView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.app.clone()
    }
}
