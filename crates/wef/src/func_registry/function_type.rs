use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{
    Frame,
    func_registry::{CallFunctionError, into_result::IntoFunctionResult},
};

/// Represents a function type that can be called from JavaScript.
pub trait FunctionType<S, R> {
    /// Number of arguments.
    const NUM_ARGUMENTS: usize;

    /// Calls the function with the given arguments.
    fn call(&self, frame: Frame, args: Vec<Value>) -> Result<Value, CallFunctionError>;
}

macro_rules! impl_function_types {
    ($($name:ident),*) => {
        impl<F, $($name,)* R, Ret> FunctionType<($($name,)*), R> for F
        where
            F: Fn($($name),*) -> Ret,
            Ret: IntoFunctionResult<R>,
            $($name: DeserializeOwned,)*
        {
            const NUM_ARGUMENTS: usize = tuple_len::tuple_len!(($($name,)*));

            fn call(&self, _frame: Frame, args: Vec<Value>) -> Result<Value, CallFunctionError> {
                let expected_args = tuple_len::tuple_len!(($($name,)*));

                if args.len() != expected_args {
                    return Err(CallFunctionError::InvalidNumberOfArguments {
                        expected: expected_args,
                        actual: args.len(),
                    });
                }

                let mut args = args;
                args.reverse();

                $(
                    #[allow(non_snake_case)]
                    let $name: $name = serde_json::from_value(args.pop().unwrap()).map_err(|e| {
                        CallFunctionError::InvalidArgument {
                            arg_name: "A1".to_string(),
                            error: e.to_string(),
                        }
                    })?;
                )*


                let result = IntoFunctionResult::into_function_result(self($($name),*));
                match result {
                    Ok(value) => Ok(serde_json::to_value(value).unwrap()),
                    Err(e) => Err(CallFunctionError::Other(e.to_string())),
                }
            }
        }

        impl<F, $($name,)* R, Ret> FunctionType<(Frame, $($name,)*), R> for F
        where
            F: Fn(Frame, $($name),*) -> Ret,
            Ret: IntoFunctionResult<R>,
            $($name: DeserializeOwned,)*
        {
            const NUM_ARGUMENTS: usize = tuple_len::tuple_len!(($($name,)*));

            fn call(&self, frame: Frame, args: Vec<Value>) -> Result<Value, CallFunctionError> {
                let expected_args = tuple_len::tuple_len!(($($name,)*));

                if args.len() != expected_args {
                    return Err(CallFunctionError::InvalidNumberOfArguments {
                        expected: expected_args,
                        actual: args.len(),
                    });
                }

                let mut args = args;
                args.reverse();

                $(
                    #[allow(non_snake_case)]
                    let $name: $name = serde_json::from_value(args.pop().unwrap()).map_err(|e| {
                        CallFunctionError::InvalidArgument {
                            arg_name: "A1".to_string(),
                            error: e.to_string(),
                        }
                    })?;
                )*


                let result = IntoFunctionResult::into_function_result(self(frame, $($name),*));
                match result {
                    Ok(value) => Ok(serde_json::to_value(value).unwrap()),
                    Err(e) => Err(CallFunctionError::Other(e.to_string())),
                }
            }
        }
    };
}

impl_function_types!();
impl_function_types!(A1);
impl_function_types!(A1, A2);
impl_function_types!(A1, A2, A3);
impl_function_types!(A1, A2, A3, A4);
impl_function_types!(A1, A2, A3, A4, A5);
impl_function_types!(A1, A2, A3, A4, A5, A6);
impl_function_types!(A1, A2, A3, A4, A5, A6, A7);
impl_function_types!(A1, A2, A3, A4, A5, A6, A7, A8);
