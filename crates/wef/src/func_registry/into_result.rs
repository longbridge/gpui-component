use std::error::Error;

use serde::Serialize;
use serde_json::Value;

use crate::func_registry::CallFunctionError;

pub(crate) trait IntoFunctionResult<T> {
    fn into_function_result(self) -> Result<Value, CallFunctionError>;
}

impl<T, E> IntoFunctionResult<T> for Result<T, E>
where
    T: Serialize,
    E: Error,
{
    fn into_function_result(self) -> Result<Value, CallFunctionError> {
        self.map(|value| serde_json::to_value(value).unwrap())
            .map_err(|err| CallFunctionError::Other(err.to_string()))
    }
}

impl<T> IntoFunctionResult<T> for T
where
    T: Serialize,
{
    fn into_function_result(self) -> Result<Value, CallFunctionError> {
        Ok(serde_json::to_value(self).unwrap())
    }
}
