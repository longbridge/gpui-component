use std::{collections::HashMap, sync::Arc};

use futures_util::future::BoxFuture;

use crate::{
    AsyncFunctionType, FuncRegistry, FunctionType,
    func_registry::dyn_wrapper::{DynAsyncFunctionWrapper, DynFunctionType, DynFunctionWrapper},
};

/// A builder for creating a function registry.
#[derive(Default)]
pub struct FuncRegistryBuilder {
    functions: HashMap<String, Box<dyn DynFunctionType>>,
}

impl FuncRegistryBuilder {
    /// Registers a function with the given name.
    pub fn register<F, S, R>(mut self, name: &str, func: F) -> Self
    where
        F: FunctionType<S, R> + Send + Sync + 'static,
        S: Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        self.functions
            .insert(name.to_string(), Box::new(DynFunctionWrapper::new(func)));
        self
    }

    /// Consumes the builder and returns a new [`AsyncFuncRegistryBuilder`].
    pub fn with_spawner<S, R>(self, spawner: S) -> AsyncFuncRegistryBuilder
    where
        S: Fn(BoxFuture<'static, ()>) -> R + Send + Sync + 'static,
    {
        AsyncFuncRegistryBuilder {
            functions: self.functions,
            spawner: Arc::new(move |fut| {
                spawner(fut);
            }),
        }
    }

    /// Builds the [`FuncRegistry`].
    pub fn build(self) -> FuncRegistry {
        FuncRegistry {
            functions: Arc::new(self.functions),
            spawner: None,
        }
    }
}

/// A builder for creating an function registry with async functions.
pub struct AsyncFuncRegistryBuilder {
    functions: HashMap<String, Box<dyn DynFunctionType>>,
    spawner: Arc<dyn Fn(BoxFuture<'static, ()>) + Send + Sync>,
}

impl AsyncFuncRegistryBuilder {
    /// Registers a function with the given name.
    pub fn register<F, S, R>(mut self, name: &str, func: F) -> Self
    where
        F: FunctionType<S, R> + Send + Sync + 'static,
        S: Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        self.functions
            .insert(name.to_string(), Box::new(DynFunctionWrapper::new(func)));
        self
    }

    /// Registers a async function with the given name.
    pub fn register_async<F, S, R>(mut self, name: &str, func: F) -> Self
    where
        F: AsyncFunctionType<S, R> + Send + Sync + 'static,
        S: Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        self.functions.insert(
            name.to_string(),
            Box::new(DynAsyncFunctionWrapper::new(func)),
        );
        self
    }

    /// Builds the [`FuncRegistry`].
    pub fn build(self) -> FuncRegistry {
        FuncRegistry {
            functions: Arc::new(self.functions),
            spawner: Some(self.spawner),
        }
    }
}
