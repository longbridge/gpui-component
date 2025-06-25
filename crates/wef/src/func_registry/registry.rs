use std::{collections::HashMap, sync::Arc};

use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::{
    Frame, FuncRegistryBuilder,
    func_registry::{CallFunctionError, dyn_wrapper::DynFunctionType},
    query::QueryCallback,
};

/// A registry for functions that can be called from JavaScript.
///
/// To create a new `FuncRegistry`, use the [`FuncRegistry::builder`] method to
/// create a `FuncRegistryBuilder`, register your functions, and then call
/// [`FuncRegistryBuilder::build`] to create the `FuncRegistry`.
///
/// ```rust, no_run
/// use wef::{Browser, FuncRegistry};
///
/// let registry = FuncRegistry::builder()
///     .register("add", |a: i32, b: i32| a + b)
///     .register("sub", |a: i32, b: i32| a - b)
///     .build();
///
/// let browser = Browser::builder()
///     .func_registry(registry) // Register the functions with the browser
///     .build();
/// ```
///
/// The functions can be synchronous or asynchronous, and they can accept any
/// number of arguments. The arguments must implement the `serde::Serialize` and
/// `serde::Deserialize` traits.
///
/// Call the functions from JavaScript:
///
/// ```javascript
/// jsBridge.add(1, 2); // Returns 3
/// jsBridge.sub(5, 3); // Returns 2
/// ```
///
/// # Asynchronous Functions
///
/// You can also register asynchronous functions. Call
/// `FuncRegistryBuilder::with_spawner` to create an `AsyncFuncRegistryBuilder`
/// that allows you to register async functions.
///
/// ```rust, ignore
/// use wef::{Browser, FuncRegistry};
///
/// let registry = FuncRegistry::builder()
///     .with_spawner(tokio::spawn)
///     .register_async("sleep", |millis: u64| async move {
///         tokio::time::sleep(std::time::Duration::from_millis(millis)).await;
///         "done"
///     })
///     .build();
///
/// let browser = Browser::builder()
///     .func_registry(registry) // Register the functions with the browser
///     .build();
/// ```
///
/// _You can clone the `FuncRegistry` and use it in multiple browsers._
#[derive(Default, Clone)]
pub struct FuncRegistry {
    pub(crate) functions: Arc<HashMap<String, Box<dyn DynFunctionType>>>,
    pub(crate) spawner: Option<Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>>,
}

impl FuncRegistry {
    /// Creates a new `FuncRegistryBuilder`.
    #[inline]
    pub fn builder() -> FuncRegistryBuilder {
        FuncRegistryBuilder::default()
    }

    pub(crate) fn call(&self, frame: Frame, name: &str, args: Vec<Value>, callback: QueryCallback) {
        let Some(func) = self.functions.get(name) else {
            callback.result(Err(CallFunctionError::NotFound(name.to_string())));
            return;
        };
        func.call(self.spawner.as_deref(), frame, args, callback)
    }

    pub(crate) fn javascript(&self) -> String {
        let mut code = include_str!("inject.js").to_string();

        for (name, func) in &*self.functions {
            let args = (0..func.num_arguments())
                .map(|i| format!("arg{}", i))
                .collect::<Vec<_>>()
                .join(",");
            code += &format!(
                r#"window.jsBridge.{name} = function({args}) {{
                return window.jsBridge.__internal.call("{name}", [{args}]);
            }};"#,
                name = name,
                args = args
            );
        }

        code
    }
}
