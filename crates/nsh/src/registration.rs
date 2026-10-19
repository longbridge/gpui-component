//! Shared NSH registration plus local Component builder helpers.
pub(crate) use nsh_platform_gpui::NativeRegistry as BindingRegistry;

// Stateless leaves: one field declaration feeds NSH Props and native builders.
macro_rules! native_leaf {
    ($(#[$meta:meta])* $name:ident($props:ident,$context:ident,$cx:ident) => $base:expr; {$($fields:tt)*}) => {
        $crate::registration::native_leaf!($(#[$meta])* $name($props,$context,$cx,p) => $base; {$($fields)*});
    };
    ($(#[$meta:meta])* $name:ident($props:ident,$context:ident,$cx:ident,$p:ident) => $base:expr; {$($field:ident : $ty:ty => $op:ident $(($map:expr))?;)*}) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq, nsh_core::ui::Props)]
        pub(crate) struct $props { $(#[prop(default)] $field: $ty,)* }
        pub(crate) struct $name { context:nsh_platform_gpui::TypedComponentContext<nsh_core::ui::NoEvent> }
        impl $name { pub(crate) fn new(context:nsh_platform_gpui::TypedComponentContext<nsh_core::ui::NoEvent>)->Self { Self{context} } }
        impl nsh_platform_gpui::TypedNativeComponent for $name {
            type Props=$props; type Event=nsh_core::ui::NoEvent;
            fn render(&mut self,$p:&$props,_:&mut gpui_kit::Window,$cx:&mut gpui_kit::App)->Option<gpui_kit::AnyElement> {
                let $context=&self.context;
                let _=(&$context,&$cx,&$p);
                Some($crate::registration::native_leaf!(@chain $base; $p; $($field => $op $(($map))?;)*).into_any_element())
            }
        }
    };
    (@chain $base:expr; $p:ident;) => { $base };
    (@chain $base:expr; $p:ident; $field:ident => optional; $($rest:tt)*) => {
        $crate::registration::native_leaf!(@chain $base.when_some($p.$field.clone(),|element,value|element.$field(value)); $p; $($rest)*)
    };
    (@chain $base:expr; $p:ident; $field:ident => mapped($map:expr); $($rest:tt)*) => {
        $crate::registration::native_leaf!(@chain $base.when_some($p.$field.clone(),|element,value|element.$field(($map)(value))); $p; $($rest)*)
    };
    (@chain $base:expr; $p:ident; $field:ident => flag; $($rest:tt)*) => {
        $crate::registration::native_leaf!(@chain $base.when($p.$field,|element|element.$field()); $p; $($rest)*)
    };
    (@chain $base:expr; $p:ident; $field:ident => set; $($rest:tt)*) => {
        $crate::registration::native_leaf!(@chain $base.$field($p.$field.clone()); $p; $($rest)*)
    };
    (@chain $base:expr; $p:ident; $field:ident => size($map:expr); $($rest:tt)*) => {
        $crate::registration::native_leaf!(@chain $base.with_size(($map)(&$p.$field)); $p; $($rest)*)
    };
    (@chain $base:expr; $p:ident; $field:ident => skip; $($rest:tt)*) => {
        $crate::registration::native_leaf!(@chain $base; $p; $($rest)*)
    };
}
pub(crate) use native_leaf;

/// Keeps a named JavaScript type while delegating its value codec to an existing
/// NSH type. Unlike a Rust `type` alias, this declares the name in generated TS.
macro_rules! named_value {
    ($(#[$meta:meta])* $visibility:vis struct $name:ident($inner:ty);) => {
        $(#[$meta])*
        #[derive(Clone, PartialEq)]
        $visibility struct $name($inner);

        #[allow(dead_code)]
        impl $name {
            pub(crate) fn new(value: $inner) -> Self { Self(value) }
            pub(crate) fn value(&self) -> &$inner { &self.0 }
            pub(crate) fn into_value(self) -> $inner { self.0 }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self { Self::new(value) }
        }

        impl nsh_core::ui::JsValue for $name {
            fn ts_type(types: &mut nsh_core::ui::TsTypes) -> String {
                types.declare(stringify!($name), |types| {
                    format!("export type {} = {};", stringify!($name), <$inner as nsh_core::ui::JsValue>::ts_type(types))
                })
            }

            fn from_js(value: nsh_core::ui::JsIn<'_>) -> Result<Self, nsh_core::ui::PropError> {
                <$inner as nsh_core::ui::JsValue>::from_js(value).map(Self)
            }

            fn to_js<'a>(&self, host: &'a dyn nsh_core::ui::JsHost) -> Result<nsh_core::ui::JsOwned<'a>, nsh_core::ui::PropError> {
                <$inner as nsh_core::ui::JsValue>::to_js(&self.0, host)
            }
        }
    };
}
pub(crate) use named_value;
