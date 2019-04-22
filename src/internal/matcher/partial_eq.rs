use super::ArgumentMatcher;
use crate::internal::fmt::{MaybeDebug, MaybeDebugWrapper};
use nameof::name_of;
use std::fmt::{self, Debug, Display};

/// Creates a new `ArgumentMatcher` that matches against values using [`PartialEq`]
pub fn partial_eq<T>(value: T) -> PartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    PartialEqArgumentMatcher { value }
}

/// Creates a new `ArgumentMatcher` that matches against values using [`PartialEq`].
/// Supports comparing a reference against an owned value.
pub fn partial_eq_owned<T>(value: T) -> OwnedPartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    OwnedPartialEqArgumentMatcher { value }
}

pub struct PartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    value: T,
}

impl<T> Display for PartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        MaybeDebug::fmt(&self.value, f)
    }
}

impl<T> Debug for PartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name_of!(type PartialEqArgumentMatcher<T>))
            .field(name_of!(value in Self), &MaybeDebugWrapper(&self.value))
            .finish()
    }
}

impl<T, U> ArgumentMatcher<U> for PartialEqArgumentMatcher<T>
where
    T: PartialEq<U> + MaybeDebug,
{
    fn matches_argument(&self, input: &U) -> bool {
        &self.value == input
    }
}

pub struct OwnedPartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    value: T,
}

impl<T> Display for OwnedPartialEqArgumentMatcher<T>
where
    T: MaybeDebug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        MaybeDebug::fmt(&self.value, f)
    }
}

impl<'args, T, U> ArgumentMatcher<&'args U> for OwnedPartialEqArgumentMatcher<T>
where
    T: PartialEq<U> + MaybeDebug,
{
    fn matches_argument(&self, input: &&U) -> bool {
        &self.value == *input
    }
}

impl<T> Debug for OwnedPartialEqArgumentMatcher<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name_of!(type OwnedPartialEqArgumentMatcher<T>))
            .field(name_of!(value in Self), &MaybeDebugWrapper(&self.value))
            .finish()
    }
}
