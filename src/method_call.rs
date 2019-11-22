use crate::default_return_value::DefaultReturnValue;
use crate::expected_calls::ExpectedCalls;
use crate::fmt::{DisplayOption, DisplayTimes};
use crate::matcher::ArgumentsMatcher;
use crate::return_value::{self, ReturnValueGenerator};
use nameof::name_of;
use std::cell::RefCell;
use std::fmt::{self, Debug, Display};
use std::rc::Rc;

/// Configures an expected method call.
/// This builder is returned from the `expect_*` methods on a generated mock.
pub struct MethodCallBuilder<'mock, 'a, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    call: &'a mut MethodCall<'mock, A, R>,
}

impl<'mock, 'a, A, R> Debug for MethodCallBuilder<'mock, 'a, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name_of!(type MethodCallBuilder<'mock, 'a, A, R>))
            .field(name_of!(call in Self), &self.call)
            .finish()
    }
}

impl<'mock, 'a, A, R> MethodCallBuilder<'mock, 'a, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    /// Defines the return value for this method.
    /// The value must be [`Clone`]able.
    ///
    /// This method does not need to be called on nightly for the unit type `()`.
    pub fn returns(&mut self, return_value: R) -> &mut Self
    where
        R: Clone + 'mock,
    {
        self.call.return_value = Some(Rc::new(return_value::Cloned(return_value)));
        self
    }

    /// Defines a return value for this method that will be returned once.
    /// The mocked method will panic on subsequent calls.
    ///
    /// This method does not need to be called on nightly for the unit type `()`.
    pub fn returns_once(&mut self, return_value: R) -> &mut Self
    where
        R: 'mock,
    {
        self.call.return_value = Some(Rc::new(return_value::Once::new(return_value)));
        self.assert_times_and_return_value_are_compatible();
        self
    }

    /// Defines a return value for this method by passing an [`Fn`] that will be
    /// called once for each call of the mocked method.
    pub fn returns_with<F>(&mut self, return_value_fn: F) -> &mut Self
    where
        F: (Fn(<A as ArgumentsMatcher<'_>>::Arguments) -> R) + 'mock,
        A: 'mock,
        R: 'mock,
    {
        self.call.return_value = Some(Rc::new(return_value::Closure(Box::new(return_value_fn))));
        self
    }

    /// Defines that this method panics.
    pub fn panics(&mut self) -> &mut Self {
        self.call.return_value = Some(Rc::new(return_value::Panic(None)));
        self
    }

    /// Defines that this method panics with a message.
    pub fn panics_with_message(&mut self, message: &'static str) -> &mut Self {
        self.call.return_value = Some(Rc::new(return_value::Panic(Some(message))));
        self
    }

    /// Defines how often this method should be called.
    ///
    /// # Accepted values
    /// | Description           | Type                 | Example |
    /// | --------------------- | -------------------- | ------- |
    /// | Exact amount of times | [`u64`]              | `3`     |
    /// | Any amount of times   | [`RangeFull`]        | `..`    |
    /// | At least              | [`RangeFrom`]        | `3..`   |
    /// | At most (exclusive)   | [`RangeTo`]          | `..3`   |
    /// | At most (inclusive)   | [`RangeToInclusive`] | `..=3`  |
    /// | Between (exclusive)   | [`Range`]            | `3..4`  |
    /// | Between (inclusive)   | [`RangeInclusive`]   | `3..=4` |
    ///
    /// [`u64`]: u64
    /// [`RangeFull`]: std::ops::RangeFull
    /// [`RangeFrom`]: std::ops::RangeFrom
    /// [`RangeTo`]: std::ops::RangeTo
    /// [`RangeToInclusive`]: std::ops::RangeToInclusive
    /// [`Range`]: std::ops::Range
    /// [`RangeInclusive`]: std::ops::RangeInclusive
    pub fn times<E>(&mut self, expected_calls: E) -> &mut Self
    where
        E: Into<ExpectedCalls>,
    {
        self.call.expected_calls = expected_calls.into();
        self.assert_times_and_return_value_are_compatible();
        self
    }

    pub(crate) fn new(call: &'a mut MethodCall<'mock, A, R>) -> Self {
        Self { call }
    }

    fn assert_times_and_return_value_are_compatible(&self) {
        let one_expected_call = ExpectedCalls::from(1);
        let returns_only_once = self
            .call
            .return_value
            .as_ref()
            .map(|r| !r.can_return_more_than_once())
            .unwrap_or_default();
        let expected_calls = &self.call.expected_calls;
        if returns_only_once && expected_calls != &one_expected_call {
            panic!(
                "Return value can only be returned once but call was expected {}.",
                expected_calls
            );
        }
    }
}

pub(crate) struct MethodCall<'mock, A, R> {
    expected_calls: ExpectedCalls,
    actual_number_of_calls: RefCell<u64>,
    matcher: Rc<A>,
    return_value: Option<Rc<dyn ReturnValueGenerator<A, R> + 'mock>>,
}

impl<'mock, A, R> Debug for MethodCall<'mock, A, R>
where
    A: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(name_of!(type MethodCall<'mock, A, R>))
            .field(name_of!(expected_calls in Self), &self.expected_calls)
            .field(
                name_of!(actual_number_of_calls in Self),
                &self.actual_number_of_calls,
            )
            .field(name_of!(matcher in Self), &self.matcher)
            .field(name_of!(return_value in Self), &self.return_value)
            .finish()
    }
}

impl<'mock, A, R> Clone for MethodCall<'mock, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn clone(&self) -> Self {
        Self {
            expected_calls: self.expected_calls.clone(),
            actual_number_of_calls: self.actual_number_of_calls.clone(),
            matcher: self.matcher.clone(),
            return_value: self.return_value.clone(),
        }
    }
}

impl<'mock, A, R> MethodCall<'mock, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    pub(crate) fn new(matcher: A) -> Self {
        Self {
            expected_calls: ExpectedCalls::default(),
            actual_number_of_calls: RefCell::default(),
            matcher: Rc::new(matcher),
            return_value: R::default_return_value(),
        }
    }

    pub(crate) fn call(&self, arguments: <A as ArgumentsMatcher<'_>>::Arguments) -> R {
        *self.actual_number_of_calls.borrow_mut() += 1;

        match self.return_value {
            Some(ref return_value) => return_value.generate_return_value(arguments),
            None => panic!("No return value was specified"),
        }
    }

    pub(crate) fn was_called_expected_number_of_times(&self) -> bool {
        self.expected_calls
            .contains(*self.actual_number_of_calls.borrow())
    }

    pub(crate) fn accepts_more_calls(&self) -> bool {
        let number_of_calls = *self.actual_number_of_calls.borrow();
        match self.expected_calls.max_value() {
            Some(max_value) => number_of_calls < max_value,
            None => true,
        }
    }

    pub(crate) fn matches_expected_arguments<'a>(
        &self,
        arguments: &<A as ArgumentsMatcher<'a>>::Arguments,
    ) -> bool {
        self.matcher.matches_arguments(arguments)
    }
}

impl<'mock, A, R> Display for MethodCall<'mock, A, R>
where
    A: for<'args> ArgumentsMatcher<'args>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} -> {} {}, was called {}",
            self.matcher,
            DisplayOption(self.return_value.as_ref()),
            self.expected_calls,
            DisplayTimes(*self.actual_number_of_calls.borrow())
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::arguments::ArgumentsMock;
    use crate::matcher::ArgumentsMatcherMock;
    use std::cell::RefCell;
    use std::fmt::Debug;
    use std::thread::panicking;

    #[derive(Debug)]
    struct ReturnValueGeneratorMock<R>
    where
        R: Clone + Debug,
    {
        generate_return_value_was_called: RefCell<bool>,
        return_value: Option<R>,
    }

    impl<R> ReturnValueGeneratorMock<R>
    where
        R: Clone + Debug,
    {
        fn new(return_value: Option<R>) -> Self {
            Self {
                return_value,
                generate_return_value_was_called: RefCell::default(),
            }
        }
    }

    impl<R> Display for ReturnValueGeneratorMock<R>
    where
        R: Clone + Debug,
    {
        fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
            panic!();
        }
    }

    impl<A, R> ReturnValueGenerator<A, R> for ReturnValueGeneratorMock<R>
    where
        A: for<'args> ArgumentsMatcher<'args>,
        R: Clone + Debug,
    {
        fn generate_return_value(&self, _input: <A as ArgumentsMatcher<'_>>::Arguments) -> R {
            *self.generate_return_value_was_called.borrow_mut() = true;

            self.return_value
                .as_ref()
                .expect("Return value was not specified for mock")
                .clone()
        }
    }

    impl<R> Drop for ReturnValueGeneratorMock<R>
    where
        R: Clone + Debug,
    {
        fn drop(&mut self) {
            if !panicking() && self.return_value.is_some() {
                assert!(
                    *self.generate_return_value_was_called.borrow(),
                    "generate_return_value_was_called() was never called"
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "No return value was specified")]
    fn call_panics_if_no_return_value_is_specified() {
        let call: MethodCall<'_, _, String> = MethodCall::new(ArgumentsMatcherMock::new(None));

        call.call(ArgumentsMock);
    }

    #[test]
    fn call_uses_return_value() {
        let mut call: MethodCall<'_, _, String> = MethodCall::new(ArgumentsMatcherMock::new(None));

        call.return_value = Some(Rc::new(ReturnValueGeneratorMock::new(Some(String::from(
            "foo",
        )))));

        let return_value = call.call(ArgumentsMock);

        assert_eq!(String::from("foo"), return_value);
    }

    #[test]
    fn was_called_expected_number_of_times_returns_true() {
        let mut call: MethodCall<'_, _, ()> = MethodCall::new(ArgumentsMatcherMock::new(None));
        call.return_value = Some(Rc::new(ReturnValueGeneratorMock::new(Some(()))));
        call.expected_calls = 4.into();

        call.call(ArgumentsMock);
        call.call(ArgumentsMock);
        call.call(ArgumentsMock);
        call.call(ArgumentsMock);

        assert!(call.was_called_expected_number_of_times());
    }

    #[test]
    fn was_called_expected_number_of_times_returns_false() {
        let call: MethodCall<'_, _, ()> = {
            let mut call = MethodCall::new(ArgumentsMatcherMock::new(None));
            call.return_value = Some(Rc::new(ReturnValueGeneratorMock::new(Some(()))));
            call.expected_calls = (2..).into();
            call
        };

        call.call(ArgumentsMock);

        assert!(!call.was_called_expected_number_of_times());
    }

    #[test]
    fn matches_expected_arguments_returns_true() {
        let call: MethodCall<'_, _, ()> = {
            let mut call = MethodCall::new(ArgumentsMatcherMock::new(Some(true)));
            call.return_value = Some(Rc::new(ReturnValueGeneratorMock::new(None)));
            call
        };

        assert!(call.matches_expected_arguments(&ArgumentsMock));
    }

    #[test]
    fn matches_expected_arguments_returns_false() {
        let call: MethodCall<'_, _, ()> = {
            let mut call = MethodCall::new(ArgumentsMatcherMock::new(Some(false)));
            call.return_value = Some(Rc::new(ReturnValueGeneratorMock::new(None)));
            call
        };

        assert!(!call.matches_expected_arguments(&ArgumentsMock));
    }
}
