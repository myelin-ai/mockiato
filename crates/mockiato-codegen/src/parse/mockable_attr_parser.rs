pub(crate) use self::mockable_attr_parser_impl::*;
use crate::result::Result;
use syn::{AttributeArgs, Ident};

mod mockable_attr_parser_impl;

/// The `#[mockable]` attribute, which is placed on a trait.
#[cfg_attr(feature = "debug-impls", derive(Debug))]
pub(crate) struct MockableAttr {
    /// Customizes the name of the generated mock struct.
    /// Example usage: `#[name = "FooMock"]`
    pub(crate) name: Option<Ident>,
    /// Enforces that only static lifetimes are used within the mock.
    /// Example usage: `#[mockable(static_references)]`.
    pub(crate) force_static_lifetimes: bool,
}

pub(crate) trait MockableAttrParser {
    fn parse(&self, args: AttributeArgs) -> Result<MockableAttr>;
}
