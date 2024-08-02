pub enum ResultWarningsGeneric<T, W, E> {
    Ok(T),
    OkWarn(T, Vec<W>),
    Err(E),
}

impl<T, W, E> ResultWarningsGeneric<T, W, E> {
    pub fn ok(value: T, warnings: Vec<W>) -> Self {
        if warnings.len() == 0 {
            Self::Ok(value)
        } else {
            Self::OkWarn(value, warnings)
        }
    }
}

#[macro_export]
macro_rules! tryres {
    ($value:expr) => {
        match $value {
            ::std::result::Result::Ok(t) => t,
            ::std::result::Result::Err(err) => return crate::error::ResultWarningsGeneric::Err(err),
        }
    };
}

#[macro_export]
macro_rules! tryw {
    ($value:expr) => {
        tryw!($value, crate::error::ResultWarningsGeneric::Err)
    };
    ($value:expr, $err:expr) => {
        match $value {
            crate::error::ResultWarningsGeneric::Ok(t) => (t, Vec::new()),
            crate::error::ResultWarningsGeneric::OkWarn(t, v) => (t, v),
            crate::error::ResultWarningsGeneric::Err(err) => return $err(err),
        }
    };
}

/*
use std::{
    convert::Infallible,
    ops::{ControlFlow, FromResidual, Try},
};

impl<T, W, E> FromResidual<Result<Infallible, E>> for ResultWarningsGeneric<T, W, E> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Ok(_) => unreachable!(),
            Err(err) => Self::Err(err),
        }
    }
}

impl<T, W, E> Try for ResultWarningsGeneric<T, W, E> {
    type Output = (T, Vec<W>);

    type Residual = Result<Infallible, E>;

    fn from_output(output: Self::Output) -> Self {
        Self::ok(output.0, output.1)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            ResultWarningsGeneric::Ok(v) => std::ops::ControlFlow::Continue((v, Vec::new())),
            ResultWarningsGeneric::OkWarn(v, w) => ControlFlow::Continue((v, w)),
            ResultWarningsGeneric::Err(err) => std::ops::ControlFlow::Break(Err(err)),
        }
    }
}*/

impl<T, W, E> From<E> for ResultWarningsGeneric<T, W, E> {
    fn from(value: E) -> Self {
        Self::Err(value)
    }
}

pub type ResultWarnings<T> = ResultWarningsGeneric<T, anyhow::Error, anyhow::Error>;
