use super::bound_lifetimes::rewrite_lifetimes_incrementally;
use super::constant::{
    arguments_matcher_ident, expect_method_calls_in_order_ident, expect_method_ident,
    generic_parameter_ident, mock_lifetime,
};
use super::generics::get_matching_generics_for_method_inputs;
use super::lifetime_rewriter::{LifetimeRewriter, UniformLifetimeGenerator};
use super::GenerateMockParameters;
use crate::parse::method_decl::MethodDecl;
use crate::parse::method_inputs::MethodArg;
use crate::parse::trait_decl::TraitDecl;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Paren;
use syn::visit_mut::visit_type_mut;
use syn::{
    parse_quote, GenericParam, Ident, LitStr, ReturnType, Token, Type, TypeParam, TypeTuple,
    WherePredicate,
};

type ArgumentsWithGenerics<'a> = &'a [(Ident, &'a MethodArg)];

pub(crate) fn generate_mock_struct(
    trait_decl: &TraitDecl,
    parameters: &'_ GenerateMockParameters,
) -> TokenStream {
    let mock_struct_ident = &parameters.mock_struct_ident;
    let mod_ident = &parameters.mod_ident;
    let mut lifetime_rewriter =
        LifetimeRewriter::new(UniformLifetimeGenerator::new(mock_lifetime()));

    let method_fields: TokenStream = trait_decl
        .methods
        .iter()
        .map(|method_decl| {
            generate_method_field(method_decl, trait_decl, mod_ident, &mut lifetime_rewriter)
        })
        .collect();

    let initializer_fields: TokenStream = trait_decl
        .methods
        .iter()
        .map(|method_decl| generate_initializer_field(&method_decl.ident, mock_struct_ident))
        .collect();

    let expect_methods: TokenStream = trait_decl
        .methods
        .iter()
        .map(|method_decl| {
            generate_expect_method(trait_decl, method_decl, mod_ident, &mut lifetime_rewriter)
        })
        .collect();

    let expect_method_call_in_order_methods: TokenStream = trait_decl
        .methods
        .iter()
        .map(|method_decl| generate_expect_method_calls_in_order_method(trait_decl, method_decl))
        .collect();

    let visibility = &trait_decl.visibility;

    const GITHUB_REPOSITORY: &str = "https://github.com/myelin-ai/mockiato";

    let documentation = LitStr::new(
        &format!(
            "Mock for [`{0}`] generated by [mockiato].

[`{0}`]: ./trait.{0}.html
[mockiato]: {1}",
            trait_decl.ident, GITHUB_REPOSITORY
        ),
        Span::call_site(),
    );

    let (impl_generics, ty_generics, where_clause) = parameters.generics.split_for_impl();

    quote! {
        #[derive(Debug, Clone)]
        #[doc = #documentation]
        #visibility struct #mock_struct_ident #ty_generics #where_clause {
            #method_fields
            phantom_data: std::marker::PhantomData<&'mock ()>,
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

        impl #impl_generics Default for #mock_struct_ident #ty_generics #where_clause {
            /// Creates a new mock with no expectations.
            fn default() -> Self {
                Self::new()
            }
        }
    }
}

fn generate_method_field(
    method_decl: &MethodDecl,
    trait_decl: &TraitDecl,
    mod_ident: &Ident,
    lifetime_rewriter: &mut LifetimeRewriter<UniformLifetimeGenerator>,
) -> TokenStream {
    let ident = &method_decl.ident;
    let arguments_matcher_ident = arguments_matcher_ident(ident);
    let mut return_type = return_type(method_decl);
    let mut generics =
        get_matching_generics_for_method_inputs(&method_decl.inputs, &trait_decl.generics);
    generics.params.push(parse_quote!('mock));
    let (_, ty_generics, _) = generics.split_for_impl();

    visit_type_mut(lifetime_rewriter, &mut return_type);

    quote! {
        #ident: mockiato::internal::Method<'mock, self::#mod_ident::#arguments_matcher_ident #ty_generics, #return_type>,
    }
}

fn return_type(method_decl: &MethodDecl) -> Type {
    match &method_decl.output {
        ReturnType::Default => Type::Tuple(TypeTuple {
            paren_token: Paren {
                span: Span::call_site(),
            },
            elems: Punctuated::new(),
        }),
        ReturnType::Type(_, ty) => ty.as_ref().clone(),
    }
}

fn generate_initializer_field(method_ident: &Ident, mock_struct_ident: &Ident) -> TokenStream {
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
    trait_decl: &TraitDecl,
    method_decl: &MethodDecl,
    mod_ident: &Ident,
    lifetime_rewriter: &mut LifetimeRewriter<UniformLifetimeGenerator>,
) -> TokenStream {
    let method_ident = &method_decl.ident;
    let visibility = &trait_decl.visibility;
    let expect_method_ident = expect_method_ident(method_decl);

    let arguments_with_generics: Vec<_> = method_decl
        .inputs
        .args
        .iter()
        .enumerate()
        .map(|(index, argument)| (generic_parameter_ident(index), argument))
        .collect();

    let arguments: TokenStream = arguments_with_generics
        .iter()
        .map(generate_argument)
        .collect();

    let arguments_matcher_ident = arguments_matcher_ident(&method_decl.ident);
    let mut return_type = return_type(method_decl);
    visit_type_mut(lifetime_rewriter, &mut return_type);

    let expected_parameters: TokenStream = arguments_with_generics
        .iter()
        .map(|(_, argument)| &argument.ident)
        .map(|argument_ident| quote! { #argument_ident: Box::new(#argument_ident), })
        .collect();

    let requires_must_use_annotation = !is_empty_return_value(&method_decl.output);

    let must_use_annotation = if requires_must_use_annotation {
        quote! { #[must_use] }
    } else {
        TokenStream::new()
    };

    let documentation = LitStr::new(
        &format!(
            "Expects a call to [`{0}::{1}`],
panicking if the function was not called by the time the object goes out of scope.

[`{0}::{1}`]: ./trait.{0}.html#tymethod.{1}",
            trait_decl.ident, method_decl.ident,
        ),
        Span::call_site(),
    );

    let mut arguments_struct_generics =
        get_matching_generics_for_method_inputs(&method_decl.inputs, &trait_decl.generics);
    arguments_struct_generics
        .params
        .insert(0, parse_quote!('mock));
    let generics = argument_generics(&arguments_with_generics);
    let where_clause = where_clause(&arguments_with_generics);

    let (_, ty_generics, _) = arguments_struct_generics.split_for_impl();

    quote! {
        #must_use_annotation
        #[doc = #documentation]
        #visibility fn #expect_method_ident <#generics> (
            &mut self,
            #arguments
        ) -> mockiato::internal::MethodCallBuilder<
            'mock,
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
    let documentation = LitStr::new(
        &format!(
            "Configures [`{0}::{1}`] to expect calls in the order they were added in.

[`{0}::{1}`]: ./trait.{0}.html#tymethod.{1}",
            trait_decl.ident, method_decl.ident,
        ),
        Span::call_site(),
    );

    let visibility = &trait_decl.visibility;

    let ident = expect_method_calls_in_order_ident(method_decl);
    let method_ident = &method_decl.ident;

    quote! {
        #[doc = #documentation]
        #visibility fn #ident(&mut self) {
            self.#method_ident.expect_method_calls_in_order()
        }
    }
}

fn is_empty_return_value(return_value: &ReturnType) -> bool {
    match return_value {
        ReturnType::Default => true,
        ReturnType::Type(_, ty) => match ty {
            box Type::Tuple(tuple) => tuple.elems.is_empty(),
            _ => false,
        },
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

    parse_quote! {
        #generic_type_ident: #bound_lifetimes mockiato::internal::ArgumentMatcher<#ty> + 'mock
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
