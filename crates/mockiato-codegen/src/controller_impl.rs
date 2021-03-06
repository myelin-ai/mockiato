use crate::code_generator::{self, CodeGenerator};
use crate::diagnostic::DiagnosticBuilder;
use crate::parse::mockable_attr::{MockableAttr, MockableAttrParser, RemoteTraitPath};
use crate::parse::trait_decl::TraitDeclParser;
use crate::result::{Error, Result};
use crate::Controller;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{AttributeArgs, Item, ItemTrait};

#[derive(Debug)]
pub(crate) struct ControllerImpl {
    mockable_attr_parser: Box<dyn MockableAttrParser>,
    trait_decl_parser: Box<dyn TraitDeclParser>,
    code_generator: Box<dyn CodeGenerator>,
}

impl ControllerImpl {
    pub(crate) fn new(
        mockable_attr_parser: Box<dyn MockableAttrParser>,
        trait_decl_parser: Box<dyn TraitDeclParser>,
        code_generator: Box<dyn CodeGenerator>,
    ) -> Self {
        Self {
            mockable_attr_parser,
            trait_decl_parser,
            code_generator,
        }
    }
}

impl Controller for ControllerImpl {
    fn expand_mockable_trait(&self, attr: AttributeArgs, item: Item) -> Result<TokenStream> {
        let mockable_attr = self.mockable_attr_parser.parse(attr)?;
        let item_trait = extract_item_trait(item)?;
        let trait_decl = self
            .trait_decl_parser
            .parse(item_trait.clone())
            .map_err(add_note_to_error)?;

        let emit_item_trait = match mockable_attr.remote_trait_path {
            Some(_) => None,
            None => Some(item_trait),
        };

        let options = generate_options_from_mockable_attr(mockable_attr);
        let generated_mock = self.code_generator.generate(&trait_decl, options);

        Ok(quote! {
            #emit_item_trait
            #generated_mock
        })
    }
}

fn generate_options_from_mockable_attr(
    MockableAttr {
        remote_trait_path,
        name,
        force_static_lifetimes,
    }: MockableAttr,
) -> code_generator::GenerateOptions {
    let custom_trait_path = match remote_trait_path {
        Some(RemoteTraitPath::Path(path)) => Some(path),
        _ => None,
    };
    code_generator::GenerateOptions {
        custom_struct_ident: name,
        force_static_lifetimes,
        custom_trait_path,
    }
}

fn extract_item_trait(item: Item) -> Result<ItemTrait> {
    match item {
        Item::Trait(item_trait) => Ok(item_trait),
        _ => Err(only_traits_can_be_made_mockable_error(&item)),
    }
}

fn add_note_to_error(error: Error) -> Error {
    error
        .diagnostics
        .into_iter()
        .map(|diagnostic| {
            DiagnosticBuilder::from(diagnostic)
                .note_with_span(Span::call_site(), "Required for mockable traits")
                .build()
        })
        .collect()
}

fn only_traits_can_be_made_mockable_error(item: &Item) -> Error {
    DiagnosticBuilder::error(item.span(), "Only traits can be made mockable")
        .note_with_span(Span::call_site(), "Required because of this attribute")
        .build()
        .into()
}
