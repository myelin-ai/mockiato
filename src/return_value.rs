pub(crate) use self::cloned::*;
pub(crate) use self::closure::*;
pub(crate) use self::once::*;
pub(crate) use self::panic::*;

use crate::matcher::ArgumentsMatcher;
use std::fmt::{Debug, Display};

mod cloned;
mod closure;
mod once;
mod panic;

pub(crate) trait ReturnValueGenerator<A, R>: Display + Debug
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn generate_return_value(&self, input: <A as ArgumentsMatcher<'_>>::Arguments) -> R;

    fn can_return_more_than_once(&self) -> bool {
        true
    }
}
