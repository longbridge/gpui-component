use crate::{
  ActiveTheme as _, Collapsible, Icon, IconName, Sizable as _, StyledExt,
  button::{Button, ButtonVariants as _},
  h_flex,
  menu::{ContextMenuExt, PopupMenu},
  sidebar::SidebarItem,
  v_flex,
};
use gpui::{
  AnyElement, App, ClickEvent, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
  SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, div, percentage,
  prelude::FluentBuilder,
};
use std::rc::Rc;

/// Menu for the [`super::Sidebar`]
#[derive(Clone)]
pub struct SidebarMenu {
  style: StyleRefinement,
  collapsed: bool,
  items: Vec<SidebarMenuItem>,
}

impl SidebarMenu {
  /// Create a new SidebarMenu
  pub fn new() -> Self {
    Self {
      style: StyleRefinement::default(),
      items: Vec::new(),
      collapsed: false,
    }
  }

  /// Add a [`SidebarMenuItem`] child menu item to the sidebar menu.
  ///
  /// See also [`SidebarMenu::children`].
  pub fn child(mut self, child: impl Into<SidebarMenuItem>) -> Self {
    self.items.push(child.into());
    self
  }

  /// Add multiple [`SidebarMenuItem`] child menu items to the sidebar menu.
  pub fn children(
    mut self,
    children: impl IntoIterator<Item = impl Into<SidebarMenuItem>>,
  ) -> Self {
    self.items = children.into_iter().map(Into::into).collect();
    self
  }
}

impl Collapsible for SidebarMenu {
  fn is_collapsed(&self) -> bool {
    self.collapsed
  }

  fn collapsed(mut self, collapsed: bool) -> Self {
    self.collapsed = collapsed;
    self
  }
}

impl SidebarItem for SidebarMenu {
  fn render(self, id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let id = id.into();

    v_flex()
      .gap_2()
      .refine_style(&self.style)
      .children(self.items.into_iter().enumerate().map(|(ix, item)| {
        let id = SharedString::from(format!("{}-{}", id, ix));
        item
          .collapsed(self.collapsed)
          .render(id, window, cx)
          .into_any_element()
      }))
  }
}

impl Styled for SidebarMenu {
  fn style(&mut self) -> &mut StyleRefinement {
    &mut self.style
  }
}

/// Menu item for the [`SidebarMenu`]
#[derive(Clone)]
pub struct SidebarMenuItem {
  icon: Option<Icon>,
  label: SharedString,
  handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
  active: bool,
  default_open: bool,
  click_to_open: bool,
  collapsed: bool,
  children: Vec<Self>,
  suffix: Option<Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>>,
  disabled: bool,
  context_menu: Option<Rc<dyn Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static>>,
}

impl SidebarMenuItem {
  /// Create a new [`SidebarMenuItem`] with a label.
  pub fn new(label: impl Into<SharedString>) -> Self {
    Self {
      icon: None,
      label: label.into(),
      handler: Rc::new(|_, _, _| {}),
      active: false,
      collapsed: false,
      default_open: false,
      click_to_open: false,
      children: Vec::new(),
      suffix: None,
      disabled: false,
      context_menu: None,
    }
  }

  /// Set the icon for the menu item
  pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
    self.icon = Some(icon.into());
    self
  }

  /// Set the active state of the menu item
  pub fn active(mut self, active: bool) -> Self {
    self.active = active;
    self
  }

  /// Add a click handler to the menu item
  pub fn on_click(
    mut self,
    handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
  ) -> Self {
    self.handler = Rc::new(handler);
    self
  }

  /// Set the collapsed state of the menu item
  pub fn collapsed(mut self, collapsed: bool) -> Self {
    self.collapsed = collapsed;
    self
  }

  /// Set the default open state of the Submenu, default is `false`.
  ///
  /// This only used on initial render, the internal state will be used afterwards.
  pub fn default_open(mut self, open: bool) -> Self {
    self.default_open = open;
    self
  }

  /// Set whether clicking the menu item open the submenu.
  ///
  /// Default is `false`.
  ///
  /// If `false` we only handle open/close via the caret button.
  pub fn click_to_open(mut self, click_to_open: bool) -> Self {
    self.click_to_open = click_to_open;
    self
  }

  pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Self>>) -> Self {
    self.children = children.into_iter().map(Into::into).collect();
    self
  }

  /// Set the suffix for the menu item.
  pub fn suffix<F, E>(mut self, builder: F) -> Self
  where
    F: Fn(&mut Window, &mut App) -> E + 'static,
    E: IntoElement,
  {
    self.suffix = Some(Rc::new(move |window, cx| {
      builder(window, cx).into_any_element()
    }));
    self
  }

  /// Set disabled flat for menu item.
  pub fn disable(mut self, disable: bool) -> Self {
    self.disabled = disable;
    self
  }

  fn is_submenu(&self) -> bool {
    self.children.len() > 0
  }

  /// Set the context menu for the menu item.
  pub fn context_menu(
    mut self,
    f: impl Fn(PopupMenu, &mut Window, &mut App) -> PopupMenu + 'static,
  ) -> Self {
    self.context_menu = Some(Rc::new(f));
    self
  }
}

impl FluentBuilder for SidebarMenuItem {}

impl Collapsible for SidebarMenuItem {
  fn is_collapsed(&self) -> bool {
    self.collapsed
  }

  fn collapsed(mut self, collapsed: bool) -> Self {
    self.collapsed = collapsed;
    self
  }
}

impl SidebarItem for SidebarMenuItem {
  fn render(self, id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> impl IntoElement {
    let click_to_open = self.click_to_open;
    let default_open = self.default_open;
    let id = id.into();
    let open_state = window.use_keyed_state(id.clone(), cx, |_, _| default_open);
    let handler = self.handler.clone();
    let is_collapsed = self.collapsed;
    let is_active = self.active;
    let is_hoverable = !is_active && !self.disabled;
    let is_disabled = self.disabled;
    let is_submenu = self.is_submenu();
    let is_open = is_submenu && !is_collapsed && *open_state.read(cx);

    div()
      .id(id.clone())
      .w_full()
      .child(
        h_flex()
          .size_full()
          .id("item")
          .overflow_x_hidden()
          .flex_shrink_0()
          .p_2()
          .gap_x_2()
          .rounded(cx.theme().radius)
          .text_sm()
          .when(is_hoverable, |this| {
            this.hover(|this| {
              this
                .bg(cx.theme().sidebar_accent.opacity(0.8))
                .text_color(cx.theme().sidebar_accent_foreground)
            })
          })
          .when(is_active, |this| {
            this
              .font_medium()
              .bg(cx.theme().sidebar_accent)
              .text_color(cx.theme().sidebar_accent_foreground)
          })
          .when_some(self.icon.clone(), |this, icon| this.child(icon))
          .when(is_collapsed, |this| {
            this.justify_center().when(is_active, |this| {
              this
                .bg(cx.theme().sidebar_accent)
                .text_color(cx.theme().sidebar_accent_foreground)
            })
          })
          .when(!is_collapsed, |this| {
            this
              .h_7()
              .child(
                h_flex()
                  .flex_1()
                  .gap_x_2()
                  .justify_between()
                  .overflow_x_hidden()
                  .child(
                    h_flex()
                      .flex_1()
                      .overflow_x_hidden()
                      .child(self.label.clone()),
                  )
                  .when_some(self.suffix.clone(), |this, suffix| {
                    this.child(suffix(window, cx).into_any_element())
                  }),
              )
              .when(is_submenu, |this| {
                this.child(
                  Button::new("caret")
                    .xsmall()
                    .ghost()
                    .icon(
                      Icon::new(IconName::ChevronRight)
                        .size_4()
                        .when(is_open, |this| this.rotate(percentage(90. / 360.))),
                    )
                    .on_click({
                      let open_state = open_state.clone();
                      move |_, _, cx| {
                        // Avoid trigger item click, just expand/collapse submenu
                        cx.stop_propagation();
                        open_state.update(cx, |is_open, cx| {
                          *is_open = !*is_open;
                          cx.notify();
                        })
                      }
                    }),
                )
              })
          })
          .when(is_disabled, |this| {
            this.text_color(cx.theme().muted_foreground)
          })
          .when(!is_disabled, |this| {
            this.on_click({
              let open_state = open_state.clone();
              move |ev, window, cx| {
                if click_to_open {
                  open_state.update(cx, |is_open, cx| {
                    *is_open = true;
                    cx.notify();
                  });
                }

                handler(ev, window, cx)
              }
            })
          })
          .map(|this| {
            if let Some(context_menu) = self.context_menu {
              this
                .context_menu(move |menu, window, cx| context_menu(menu, window, cx))
                .into_any_element()
            } else {
              this.into_any_element()
            }
          }),
      )
      .when(is_open, |this| {
        this.child(
          v_flex()
            .id("submenu")
            .border_l_1()
            .border_color(cx.theme().sidebar_border)
            .gap_1()
            .ml_3p5()
            .pl_2p5()
            .py_0p5()
            .children(self.children.into_iter().enumerate().map(|(ix, item)| {
              item
                .render(SharedString::from(format!("{:?}-{}", id, ix)), window, cx)
                .into_any_element()
            })),
        )
      })
  }
}
