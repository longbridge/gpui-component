use std::marker::PhantomData;

use futures_util::future::BoxFuture;
use serde_json::Value;

use crate::{AsyncFunctionType, Frame, FunctionType, query::QueryCallback};

pub(crate) trait DynFunctionType: Send + Sync {
    fn num_arguments(&self) -> usize;

    fn call(
        &self,
        spawner: Option<&(dyn Fn(BoxFuture<'static, ()>) + Send + Sync)>,
        frame: Frame,
        args: Vec<Value>,
        callback: QueryCallback,
    );
}

pub(crate) struct DynFunctionWrapper<F, S, R> {
    func: F,
    _mark: PhantomData<(S, R)>,
}

impl<F, S, R> DynFunctionWrapper<F, S, R> {
    #[inline]
    pub(crate) fn new(func: F) -> Self {
        Self {
            func,
            _mark: PhantomData,
        }
    }
}

impl<F, S, R> DynFunctionType for DynFunctionWrapper<F, S, R>
where
    F: FunctionType<S, R> + Send + Sync,
    S: Send + Sync,
    R: Send + Sync,
{
    #[inline]
    fn num_arguments(&self) -> usize {
        F::NUM_ARGUMENTS
    }

    #[inline]
    fn call(
        &self,
        _spawner: Option<&(dyn Fn(BoxFuture<'static, ()>) + Send + Sync)>,
        frame: Frame,
        args: Vec<Value>,
        callback: QueryCallback,
    ) {
        callback.result(self.func.call(frame, args))
    }
}

pub(crate) struct DynAsyncFunctionWrapper<F, S, R> {
    func: F,
    _mark: PhantomData<(S, R)>,
}

impl<F, S, R> DynAsyncFunctionWrapper<F, S, R> {
    #[inline]
    pub(crate) fn new(func: F) -> Self {
        Self {
            func,
            _mark: PhantomData,
        }
    }
}

impl<F, S, R> DynFunctionType for DynAsyncFunctionWrapper<F, S, R>
where
    F: AsyncFunctionType<S, R> + Send + Sync,
    S: Send + Sync,
    R: Send + Sync,
{
    #[inline]
    fn num_arguments(&self) -> usize {
        F::NUM_ARGUMENTS
    }

    #[inline]
    fn call(
        &self,
        spawner: Option<&(dyn Fn(BoxFuture<'static, ()>) + Send + Sync)>,
        frame: Frame,
        args: Vec<Value>,
        callback: QueryCallback,
    ) {
        let spawner = spawner.expect("BUG: spawner is None");
        let fut = self.func.call(frame, args);
        spawner(Box::pin(async move { callback.result(fut.await) }));
    }
}
