use super::ReturnValueGenerator;
use crate::matcher::ArgumentsMatcher;
use nameof::name_of;
use std::fmt::{self, Debug, Display};

pub(crate) struct Closure<'mock, A, R>(
    pub(crate) Box<dyn Fn(<A as ArgumentsMatcher<'_>>::Arguments) -> R + 'mock>,
)
where
    A: for<'args> ArgumentsMatcher<'args>;

impl<'mock, A, R> Debug for Closure<'mock, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple(name_of!(type Closure<'mock, A, R>))
            .finish()
    }
}

impl<'mock, A, R> Display for Closure<'mock, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "?")
    }
}

impl<'mock, A, R> ReturnValueGenerator<A, R> for Closure<'mock, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn generate_return_value(&self, arguments: <A as ArgumentsMatcher<'_>>::Arguments) -> R {
        (self.0)(arguments)
    }
}
