use super::typed::{Carrier, take};
use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentDescriptor,
    ComponentMaterializer, ComponentPayload, ComponentRegistry, ConstructorDescriptor,
    MaterializeRequest, MethodDescriptor, RegistryError, TypeScriptDescriptor,
    action::ShellAction,
    anyhow,
    gpui::{
        self, IntoElement as _, Menu, MenuItem, OwnedMenu, OwnedMenuItem, ParentElement as _, div,
    },
    gpui_component::{GlobalState, menu::AppMenuBar},
};
use std::sync::Arc;

#[derive(Clone, Debug)]
struct ItemSpec {
    label: String,
    action: String,
    disabled: bool,
    checked: bool,
}
#[derive(Clone, Debug)]
enum Entry {
    Item(ItemSpec),
    Separator,
}
#[derive(Clone, Debug)]
struct MenuSpec {
    label: String,
    disabled: bool,
    entries: Vec<Entry>,
}
#[derive(Clone)]
struct Label(String);
#[derive(Clone, Copy)]
enum BoolOp {
    Disabled(bool),
    Checked(bool),
}

fn reject_style(style: gpui::StyleRefinement, name: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        style == gpui::StyleRefinement::default(),
        "{name} is typed menu data and does not support shell style"
    );
    Ok(())
}
struct ItemMat;
impl ComponentMaterializer for ItemMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut item = r
            .payload()
            .downcast_ref::<ItemSpec>()
            .ok_or_else(|| anyhow::anyhow!("MenuItem incompatible payload"))?
            .clone();
        for op in r
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<BoolOp>())
        {
            match op {
                BoolOp::Disabled(v) => item.disabled = *v,
                BoolOp::Checked(v) => item.checked = *v,
            }
        }
        reject_style(r.take_style(), "MenuItem")?;
        Ok(Carrier::new(Entry::Item(item)).into_any_element())
    }
}
struct SeparatorMat;
impl ComponentMaterializer for SeparatorMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        reject_style(r.take_style(), "MenuSeparator")?;
        Ok(Carrier::new(Entry::Separator).into_any_element())
    }
}
struct MenuMat;
impl ComponentMaterializer for MenuMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let mut spec = MenuSpec {
            label: r
                .payload()
                .downcast_ref::<Label>()
                .ok_or_else(|| anyhow::anyhow!("Menu incompatible payload"))?
                .0
                .clone(),
            disabled: false,
            entries: vec![],
        };
        for op in r
            .methods()
            .filter_map(|m| m.payload().downcast_ref::<BoolOp>())
        {
            if let BoolOp::Disabled(v) = op {
                spec.disabled = *v
            }
        }
        reject_style(r.take_style(), "Menu")?;
        for mut child in r.take_typed_children() {
            anyhow::ensure!(
                matches!(child.component_name(), Some("MenuItem" | "MenuSeparator")),
                "Menu accepts only MenuItem or MenuSeparator children"
            );
            let mut element = r.materialize_child(&mut child)?;
            spec.entries
                .push(take::<Entry>(&mut element, "Menu entry")?);
        }
        Ok(Carrier::new(spec).into_any_element())
    }
}
fn build_menu(spec: &MenuSpec) -> Menu {
    Menu::new(spec.label.clone())
        .disabled(spec.disabled)
        .items(spec.entries.iter().map(|entry| {
            match entry {
                Entry::Separator => MenuItem::separator(),
                Entry::Item(item) => {
                    MenuItem::action(item.label.clone(), ShellAction::new(item.action.clone()))
                        .disabled(item.disabled)
                        .checked(item.checked)
                }
            }
        }))
}
fn restore_menu(menu: &OwnedMenu) -> Menu {
    Menu {
        name: menu.name.clone(),
        disabled: menu.disabled,
        items: menu
            .items
            .iter()
            .map(|item| match item {
                OwnedMenuItem::Separator => MenuItem::Separator,
                OwnedMenuItem::Submenu(menu) => MenuItem::Submenu(restore_menu(menu)),
                OwnedMenuItem::SystemMenu(menu) => MenuItem::SystemMenu(gpui::OsMenu {
                    name: menu.name.clone(),
                    menu_type: menu.menu_type,
                }),
                OwnedMenuItem::Action {
                    name,
                    action,
                    os_action,
                    checked,
                    disabled,
                } => MenuItem::Action {
                    name: name.clone().into(),
                    action: action.boxed_clone(),
                    os_action: *os_action,
                    checked: *checked,
                    disabled: *disabled,
                },
            })
            .collect(),
    }
}
struct BarMat;
impl ComponentMaterializer for BarMat {
    fn materialize(&self, mut r: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = r
            .payload()
            .downcast_ref::<Label>()
            .ok_or_else(|| anyhow::anyhow!("MenuBar incompatible payload"))?
            .0
            .clone();
        let effects = r.app_effects()?;
        let mut specs = Vec::new();
        for mut child in r.take_typed_children() {
            anyhow::ensure!(
                child.component_name() == Some("Menu"),
                "MenuBar accepts only Menu children"
            );
            let mut element = r.materialize_child(&mut child)?;
            specs.push(take::<MenuSpec>(&mut element, "Menu")?);
        }
        let revision = format!("{specs:?}");
        let bar = r.with_window_app(|window, cx| {
            let retained = window.use_keyed_state(format!("shell-menu-bar:{id}"), cx, |_, cx| {
                AppMenuBar::new(cx)
            });
            let bar = retained.read(cx).clone();
            let install_bar = bar.clone();
            effects.replace(format!("menu-bar:{id}"), revision, window, cx, move |cx| {
                let previous = cx.get_menus().unwrap_or_default();
                let owned = specs
                    .iter()
                    .map(build_menu)
                    .map(Menu::owned)
                    .collect::<Vec<_>>();
                cx.set_menus(specs.iter().map(build_menu));
                GlobalState::global_mut(cx).set_app_menus(owned);
                install_bar.update(cx, |bar, cx| bar.reload(cx));
                let cleanup_bar = install_bar.clone();
                Box::new(move |cx| {
                    cx.set_menus(previous.iter().map(restore_menu));
                    GlobalState::global_mut(cx).set_app_menus(previous);
                    cleanup_bar.update(cx, |bar, cx| bar.reload(cx));
                })
            })?;
            Ok(bar)
        })?;
        r.finish(div().child(bar))
    }
}
fn label_constructor(name: &'static str) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        name,
        vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
        move |a| match a {
            [ComponentArgument::String(v)] if !v.trim().is_empty() => {
                Ok(ComponentPayload::new(Label(v.clone())))
            }
            _ => Err(format!("{name} expects a non-empty label")),
        },
    )
}
fn bool_method(name: &'static str, make: fn(bool) -> BoolOp) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |a| match a {
            [ComponentArgument::Boolean(v)] => Ok(ComponentPayload::new(make(*v))),
            _ => Err(format!("{name} expects boolean")),
        },
    )
}
pub(super) fn register(r: &mut ComponentRegistry) -> Result<(), RegistryError> {
    r.register(ComponentDescriptor {
        name: "MenuItem",
        constructors: vec![ConstructorDescriptor::new(
            "MenuItem",
            vec![
                ArgumentDescriptor::new("label", ArgumentSchema::String),
                ArgumentDescriptor::new("action", ArgumentSchema::String),
            ],
            |a| match a {
                [
                    ComponentArgument::String(label),
                    ComponentArgument::String(action),
                ] if !label.trim().is_empty() && !action.trim().is_empty() => {
                    Ok(ComponentPayload::new(ItemSpec {
                        label: label.clone(),
                        action: action.clone(),
                        disabled: false,
                        checked: false,
                    }))
                }
                _ => Err("MenuItem expects non-empty label and action".into()),
            },
        )],
        methods: vec![
            bool_method("disabled", BoolOp::Disabled),
            bool_method("checked", BoolOp::Checked),
        ],
        typescript: TypeScriptDescriptor::new("Typed application-menu action data."),
        materializer: Arc::new(ItemMat),
    })?;
    r.register(ComponentDescriptor {
        name: "MenuSeparator",
        constructors: vec![ConstructorDescriptor::new("MenuSeparator", vec![], |a| {
            if a.is_empty() {
                Ok(ComponentPayload::new(()))
            } else {
                Err("MenuSeparator expects no arguments".into())
            }
        })],
        methods: vec![],
        typescript: TypeScriptDescriptor::new("Typed application-menu separator data."),
        materializer: Arc::new(SeparatorMat),
    })?;
    r.register(ComponentDescriptor {
        name: "Menu",
        constructors: vec![label_constructor("Menu")],
        methods: vec![bool_method("disabled", BoolOp::Disabled)],
        typescript: TypeScriptDescriptor::new("Typed top-level application menu data."),
        materializer: Arc::new(MenuMat),
    })?;
    r.register(ComponentDescriptor {
        name: "MenuBar",
        constructors: vec![label_constructor("MenuBar")],
        methods: vec![],
        typescript: TypeScriptDescriptor::new(
            "A generation-owned native and in-window application menu bar.",
        ),
        materializer: Arc::new(BarMat),
    })?;
    Ok(())
}
