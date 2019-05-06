use super::bound_lifetimes::rewrite_lifetimes_incrementally;
use super::constant::{
    arguments_matcher_ident, expect_method_calls_in_order_ident, expect_method_ident,
    generic_parameter_ident, mock_lifetime, mock_lifetime_as_generic_param,
};
use super::debug_impl::{generate_debug_impl, DebugImplField};
use super::generics::get_matching_generics_for_method_inputs;
use super::lifetime_rewriter::{LifetimeRewriter, UniformLifetimeGenerator};
use super::GenerateMockParameters;
use super::MethodDeclMetadata;
use crate::generate::util::doc_attribute;
use crate::parse::method_decl::MethodDecl;
use crate::parse::method_inputs::MethodArg;
use crate::parse::trait_decl::TraitDecl;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::visit_mut::visit_type_mut;
use syn::{parse_quote, GenericParam, Ident, LitStr, Token, Type, TypeParam, WherePredicate};

type ArgumentsWithGenerics<'a> = &'a [(Ident, &'a MethodArg)];

pub(crate) fn generate_mock_struct(
    trait_decl: &TraitDecl,
    parameters: &'_ GenerateMockParameters,
) -> TokenStream {
    let mock_struct_ident = &parameters.mock_struct_ident;
    let mod_ident = &parameters.mod_ident;

    let method_fields: TokenStream = parameters
        .methods
        .iter()
        .map(|method| generate_method_field(method, mod_ident))
        .collect();

    let initializer_fields: TokenStream = parameters
        .methods
        .iter()
        .map(|method| generate_initializer_field(method, mock_struct_ident))
        .collect();

    let expect_methods: TokenStream = parameters
        .methods
        .iter()
        .map(|method| generate_expect_method(method, trait_decl, mod_ident))
        .collect();

    let expect_method_call_in_order_methods: TokenStream = trait_decl
        .methods
        .iter()
        .map(|method_decl| generate_expect_method_calls_in_order_method(trait_decl, method_decl))
        .collect();

    let debug_impl_fields = parameters
        .methods
        .iter()
        .map(|method| debug_impl_field(&method.method_decl));

    let debug_impl =
        generate_debug_impl(debug_impl_fields, mock_struct_ident, &parameters.generics);

    let visibility = &trait_decl.visibility;

    const GITHUB_REPOSITORY: &str = "https://github.com/myelin-ai/mockiato";

    let documentation = doc_attribute(format!(
        "Mock for [`{0}`] generated by [mockiato].

[`{0}`]: ./trait.{0}.html
[mockiato]: {1}",
        trait_decl.ident, GITHUB_REPOSITORY
    ));

    let (impl_generics, ty_generics, where_clause) = parameters.generics.split_for_impl();
    let mock_lifetime = mock_lifetime();

    quote! {
        #[derive(Clone)]
        #documentation
        #visibility struct #mock_struct_ident #ty_generics #where_clause {
            #method_fields
            phantom_data: std::marker::PhantomData<&#mock_lifetime ()>,
        }

        impl #impl_generics #mock_struct_ident #ty_generics #where_clause {
            /// Creates a new mock with no expectations.
            #visibility fn new() -> Self {
                Self {
                    #initializer_fields
                    phantom_data: std::marker::PhantomData,
                }
            }

            #expect_methods

            #expect_method_call_in_order_methods
        }

        #debug_impl

        impl #impl_generics Default for #mock_struct_ident #ty_generics #where_clause {
            /// Creates a new mock with no expectations.
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

fn generate_method_field(
    MethodDeclMetadata {
        method_decl: MethodDecl { ident, .. },
        arguments_matcher_struct_ident,
        generics,
        return_type,
        ..
    }: &MethodDeclMetadata,
    mod_ident: &Ident,
) -> TokenStream {
    let return_type = rewrite_lifetimes_to_mock_lifetime(return_type);

    let mut generics = generics.clone();
    generics.params.push(mock_lifetime_as_generic_param());
    let (_, ty_generics, _) = generics.split_for_impl();

    let mock_lifetime = mock_lifetime();

    quote! {
        #ident: mockiato::internal::Method<#mock_lifetime, self::#mod_ident::#arguments_matcher_struct_ident #ty_generics, #return_type>,
    }
}

fn generate_initializer_field(
    method: &MethodDeclMetadata,
    mock_struct_ident: &Ident,
) -> TokenStream {
    let method_ident = &method.method_decl.ident;
    let name = LitStr::new(
        &format!(
            "{}::{}",
            mock_struct_ident.to_string(),
            method_ident.to_string()
        ),
        Span::call_site(),
    );

    quote! {
        #method_ident: mockiato::internal::Method::new(#name),
    }
}

fn generate_expect_method(
    MethodDeclMetadata {
        return_type,
        method_decl:
            MethodDecl {
                ident: method_ident,
                inputs,
                ..
            },
        ..
    }: &MethodDeclMetadata,
    TraitDecl {
        visibility,
        generics,
        ident: trait_ident,
        ..
    }: &TraitDecl,
    mod_ident: &Ident,
) -> TokenStream {
    let expect_method_ident = expect_method_ident(method_ident);

    let arguments_with_generics: Vec<_> = inputs
        .args
        .iter()
        .enumerate()
        .map(|(index, argument)| (generic_parameter_ident(index), argument))
        .collect();

    let arguments: TokenStream = arguments_with_generics
        .iter()
        .map(generate_argument)
        .collect();

    let arguments_matcher_ident = arguments_matcher_ident(method_ident);
    let return_type = rewrite_lifetimes_to_mock_lifetime(return_type);

    let expected_parameters: TokenStream = arguments_with_generics
        .iter()
        .map(|(_, argument)| &argument.ident)
        .map(|argument_ident| quote! { #argument_ident: Box::new(#argument_ident), })
        .collect();

    let requires_must_use_annotation = !is_empty_return_value(&return_type);

    let must_use_annotation = if requires_must_use_annotation {
        quote! { #[must_use] }
    } else {
        TokenStream::new()
    };

    let documentation = doc_attribute(format!(
        "Expects a call to [`{0}::{1}`],
panicking if the function was not called by the time the object goes out of scope.

[`{0}::{1}`]: ./trait.{0}.html#tymethod.{1}",
        trait_ident, method_ident,
    ));

    let mut arguments_struct_generics = get_matching_generics_for_method_inputs(inputs, generics);
    arguments_struct_generics
        .params
        .push(mock_lifetime_as_generic_param());
    let generics = argument_generics(&arguments_with_generics);
    let where_clause = where_clause(&arguments_with_generics);

    let (_, ty_generics, _) = arguments_struct_generics.split_for_impl();
    let mock_lifetime = mock_lifetime();

    quote! {
        #must_use_annotation
        #documentation
        #visibility fn #expect_method_ident <#generics> (
            &mut self,
            #arguments
        ) -> mockiato::internal::MethodCallBuilder<
            #mock_lifetime,
            '_,
            self::#mod_ident::#arguments_matcher_ident #ty_generics,
            #return_type
        > where #where_clause
        {
            self.#method_ident.add_expected_call(
                self::#mod_ident::#arguments_matcher_ident {
                    #expected_parameters
                    phantom_data: std::marker::PhantomData,
                }
            )
        }
    }
}

fn generate_expect_method_calls_in_order_method(
    trait_decl: &TraitDecl,
    method_decl: &MethodDecl,
) -> TokenStream {
    let documentation = doc_attribute(format!(
        "Configures [`{0}::{1}`] to expect calls in the order they were added in.

[`{0}::{1}`]: ./trait.{0}.html#tymethod.{1}",
        trait_decl.ident, method_decl.ident,
    ));

    let visibility = &trait_decl.visibility;

    let ident = expect_method_calls_in_order_ident(method_decl);
    let method_ident = &method_decl.ident;

    quote! {
        #documentation
        #visibility fn #ident(&mut self) {
            self.#method_ident.expect_method_calls_in_order()
        }
    }
}

fn debug_impl_field(method_decl: &MethodDecl) -> DebugImplField<'_> {
    let ident = &method_decl.ident;
    DebugImplField {
        ident,
        expression: quote! { self.#ident },
    }
}

fn is_empty_return_value(return_type: &Type) -> bool {
    match return_type {
        Type::Tuple(tuple) => tuple.elems.is_empty(),
        _ => false,
    }
}

fn where_clause(arguments: ArgumentsWithGenerics<'_>) -> Punctuated<WherePredicate, Token![,]> {
    arguments
        .iter()
        .map(|(generic_type_ident, method_argument)| {
            where_clause_predicate(generic_type_ident, method_argument)
        })
        .collect()
}

fn where_clause_predicate(
    generic_type_ident: &Ident,
    method_argument: &MethodArg,
) -> WherePredicate {
    let mut ty = method_argument.ty.clone();
    let bound_lifetimes = rewrite_lifetimes_incrementally(&mut ty);
    let mock_lifetime = mock_lifetime();

    parse_quote! {
        #generic_type_ident: #bound_lifetimes mockiato::internal::ArgumentMatcher<#ty> + #mock_lifetime
    }
}

fn argument_generics(arguments: ArgumentsWithGenerics<'_>) -> Punctuated<GenericParam, Token![,]> {
    arguments
        .iter()
        .map(|(generic_type_ident, _)| {
            GenericParam::Type(TypeParam::from(generic_type_ident.clone()))
        })
        .collect()
}

fn generate_argument((generic_type_ident, method_argument): &(Ident, &MethodArg)) -> TokenStream {
    let argument_ident = &method_argument.ident;

    quote! {
        #argument_ident: #generic_type_ident,
    }
}

fn rewrite_lifetimes_to_mock_lifetime(ty: &Type) -> Type {
    let mut ty = ty.clone();
    let mut lifetime_rewriter =
        LifetimeRewriter::new(UniformLifetimeGenerator::new(mock_lifetime()));
    visit_type_mut(&mut lifetime_rewriter, &mut ty);
    ty
}
