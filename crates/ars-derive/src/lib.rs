//! Procedural derive macros for ars-ui component infrastructure.
//!
//! Provides `#[derive(HasId)]` and `#[derive(ComponentPart)]` to generate
//! boilerplate trait implementations for component ID access and DOM part enums.

use heck::ToKebabCase as _;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Fields, Generics, Lit, LitStr, Meta, Type, Variant, Visibility,
    parse_macro_input, parse_quote,
};

/// Derives the `HasId` trait for a struct with a `pub id: String` field.
///
/// Generates `id()`, `with_id()`, and `set_id()` methods for typed access
/// to the component's DOM identifier. Generated code uses hidden
/// `::ars_core::__private` re-exports so downstream crates do not need to
/// import `alloc` just to use the derive.
#[proc_macro_derive(HasId)]
pub fn derive_has_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_has_id(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

/// Derives the [`ComponentPart`](ars_core::ComponentPart) trait for a part enum.
///
/// Generates `ROOT`, `scope()`, `name()`, and `all()` methods. Use
/// `#[scope = "component-name"]` on the enum to set the component namespace
/// for data attribute generation. Generated code uses hidden
/// `::ars_core::__private` re-exports so downstream crates do not need to
/// import `alloc` just to use the derive.
#[proc_macro_derive(ComponentPart, attributes(scope))]
pub fn derive_component_part(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match expand_component_part(&input) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand_has_id(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "HasId can only be derived for structs",
        ));
    };

    let id_field = find_id_field(data).ok_or_else(|| {
        syn::Error::new_spanned(input, "HasId requires a field named `id` of type String")
    })?;

    if !is_string_type(&id_field.ty) {
        return Err(syn::Error::new_spanned(
            &id_field.ty,
            "HasId: `id` field must be of type String",
        ));
    }

    if !matches!(id_field.vis, Visibility::Public(_)) {
        return Err(syn::Error::new_spanned(
            &id_field.vis,
            "HasId: `id` field must be public",
        ));
    }

    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::ars_core::HasId for #name #ty_generics #where_clause {
            fn id(&self) -> &str {
                &self.id
            }

            fn with_id(self, id: ::ars_core::__private::String) -> Self {
                Self { id, ..self }
            }

            fn set_id(&mut self, id: ::ars_core::__private::String) {
                self.id = id;
            }
        }
    })
}

fn expand_component_part(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "ComponentPart can only be derived for enums",
        ));
    };

    let scope = find_scope_attr(&input.attrs)?;
    validate_root_variant(data)?;

    let name = &input.ident;
    let field_types = collect_field_types(data);

    let component_part_generics = with_field_bounds(
        &input.generics,
        &field_types,
        &[
            quote!(::core::clone::Clone),
            quote!(::core::fmt::Debug),
            quote!(::core::cmp::PartialEq),
            quote!(::core::cmp::Eq),
            quote!(::core::hash::Hash),
            quote!(::core::default::Default),
        ],
    );
    let clone_generics = with_field_bounds(
        &input.generics,
        &field_types,
        &[quote!(::core::clone::Clone)],
    );
    let debug_generics =
        with_field_bounds(&input.generics, &field_types, &[quote!(::core::fmt::Debug)]);
    let partial_eq_generics = with_field_bounds(
        &input.generics,
        &field_types,
        &[quote!(::core::cmp::PartialEq)],
    );
    let eq_generics = with_field_bounds(&input.generics, &field_types, &[quote!(::core::cmp::Eq)]);
    let hash_generics =
        with_field_bounds(&input.generics, &field_types, &[quote!(::core::hash::Hash)]);

    let (component_part_impl_generics, component_part_ty_generics, component_part_where_clause) =
        component_part_generics.split_for_impl();
    let (clone_impl_generics, clone_ty_generics, clone_where_clause) =
        clone_generics.split_for_impl();
    let (debug_impl_generics, debug_ty_generics, debug_where_clause) =
        debug_generics.split_for_impl();
    let (partial_eq_impl_generics, partial_eq_ty_generics, partial_eq_where_clause) =
        partial_eq_generics.split_for_impl();
    let (eq_impl_generics, eq_ty_generics, eq_where_clause) = eq_generics.split_for_impl();
    let (hash_impl_generics, hash_ty_generics, hash_where_clause) = hash_generics.split_for_impl();

    let name_arms = data.variants.iter().map(name_arm);
    let all_values = data.variants.iter().map(all_value);
    let clone_arms = data.variants.iter().map(clone_arm);
    let debug_arms = data.variants.iter().map(debug_arm);
    let partial_eq_arms = data
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| partial_eq_arm(index, variant));
    let hash_arms = data
        .variants
        .iter()
        .enumerate()
        .map(|(index, variant)| hash_arm(index, variant));

    Ok(quote! {
        impl #component_part_impl_generics ::ars_core::ComponentPart for #name #component_part_ty_generics #component_part_where_clause {
            const ROOT: Self = Self::Root;

            fn scope() -> &'static str {
                #scope
            }

            fn name(&self) -> &'static str {
                match self {
                    #(#name_arms),*
                }
            }

            fn all() -> ::ars_core::__private::Vec<Self> {
                ::ars_core::__private::Vec::from([#(#all_values),*])
            }
        }

        impl #clone_impl_generics ::core::clone::Clone for #name #clone_ty_generics #clone_where_clause {
            fn clone(&self) -> Self {
                match self {
                    #(#clone_arms),*
                }
            }
        }

        impl #debug_impl_generics ::core::fmt::Debug for #name #debug_ty_generics #debug_where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(#debug_arms),*
                }
            }
        }

        impl #partial_eq_impl_generics ::core::cmp::PartialEq for #name #partial_eq_ty_generics #partial_eq_where_clause {
            fn eq(&self, other: &Self) -> bool {
                match (self, other) {
                    #(#partial_eq_arms),*,
                    _ => false,
                }
            }
        }

        impl #eq_impl_generics ::core::cmp::Eq for #name #eq_ty_generics #eq_where_clause {}

        impl #hash_impl_generics ::core::hash::Hash for #name #hash_ty_generics #hash_where_clause {
            fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
                match self {
                    #(#hash_arms),*
                }
            }
        }
    })
}

fn find_id_field(data: &syn::DataStruct) -> Option<&syn::Field> {
    let Fields::Named(fields) = &data.fields else {
        return None;
    };

    fields
        .named
        .iter()
        .find(|field| field.ident.as_ref().is_some_and(|ident| ident == "id"))
}

fn find_scope_attr(attrs: &[syn::Attribute]) -> syn::Result<LitStr> {
    for attr in attrs {
        if !attr.path().is_ident("scope") {
            continue;
        }

        let Meta::NameValue(name_value) = &attr.meta else {
            break;
        };
        let Expr::Lit(expr_lit) = &name_value.value else {
            break;
        };
        let Lit::Str(scope) = &expr_lit.lit else {
            break;
        };

        return Ok(scope.clone());
    }

    Err(syn::Error::new_spanned(
        attrs.first().map_or_else(
            proc_macro2::TokenStream::new,
            quote::ToTokens::to_token_stream,
        ),
        "ComponentPart requires a #[scope = \"kebab-case-name\"] attribute",
    ))
}

fn validate_root_variant(data: &syn::DataEnum) -> syn::Result<()> {
    let Some(first_variant) = data.variants.first() else {
        return Err(syn::Error::new_spanned(
            data.enum_token,
            "ComponentPart requires the first variant to be `Root`",
        ));
    };

    if first_variant.ident != "Root" {
        return Err(syn::Error::new_spanned(
            &first_variant.ident,
            "ComponentPart requires the first variant to be `Root`",
        ));
    }

    if !matches!(first_variant.fields, Fields::Unit) {
        return Err(syn::Error::new_spanned(
            first_variant,
            "ComponentPart requires `Root` to be a unit variant",
        ));
    }

    Ok(())
}

fn is_string_type(ty: &Type) -> bool {
    let Type::Path(type_path) = ty else {
        return false;
    };

    type_path.qself.is_none()
        && type_path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "String" && segment.arguments.is_empty())
}

fn collect_field_types(data: &syn::DataEnum) -> Vec<Type> {
    data.variants
        .iter()
        .flat_map(|variant| match &variant.fields {
            Fields::Unit => Vec::new(),
            Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .map(|field| field.ty.clone())
                .collect(),
            Fields::Named(fields) => fields.named.iter().map(|field| field.ty.clone()).collect(),
        })
        .collect()
}

fn with_field_bounds(
    generics: &Generics,
    field_types: &[Type],
    bounds: &[TokenStream2],
) -> Generics {
    let mut generics = generics.clone();
    let where_clause = generics.make_where_clause();

    for field_ty in field_types {
        for bound in bounds {
            where_clause
                .predicates
                .push(parse_quote!(#field_ty: #bound));
        }
    }

    generics
}

fn name_arm(variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;
    let name = LitStr::new(&ident.to_string().to_kebab_case(), ident.span());

    match &variant.fields {
        Fields::Unit => quote!(Self::#ident => #name),
        Fields::Unnamed(_) => quote!(Self::#ident(..) => #name),
        Fields::Named(_) => quote!(Self::#ident { .. } => #name),
    }
}

fn all_value(variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;

    match &variant.fields {
        Fields::Unit => quote!(Self::#ident),
        Fields::Unnamed(fields) => {
            let defaults = fields
                .unnamed
                .iter()
                .map(|_| quote!(::core::default::Default::default()));
            quote!(Self::#ident(#(#defaults),*))
        }
        Fields::Named(fields) => {
            let defaults = fields.named.iter().map(|field| {
                let ident = field.ident.as_ref().expect("named field");
                quote!(#ident: ::core::default::Default::default())
            });
            quote!(Self::#ident { #(#defaults),* })
        }
    }
}

fn clone_arm(variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;

    match &variant.fields {
        Fields::Unit => quote!(Self::#ident => Self::#ident),
        Fields::Unnamed(fields) => {
            let bindings = (0..fields.unnamed.len())
                .map(|index| format_ident!("field_{index}"))
                .collect::<Vec<_>>();
            let clones = bindings
                .iter()
                .map(|binding| quote!(::core::clone::Clone::clone(#binding)));

            quote!(Self::#ident(#(#bindings),*) => Self::#ident(#(#clones),*))
        }
        Fields::Named(fields) => {
            let bindings = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named field").clone())
                .collect::<Vec<_>>();
            let clones = bindings
                .iter()
                .map(|binding| quote!(#binding: ::core::clone::Clone::clone(#binding)));

            quote!(Self::#ident { #(#bindings),* } => Self::#ident { #(#clones),* })
        }
    }
}

fn debug_arm(variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;
    let debug_name = LitStr::new(&ident.to_string(), ident.span());

    match &variant.fields {
        Fields::Unit => quote!(Self::#ident => f.write_str(#debug_name)),
        Fields::Unnamed(fields) => {
            let bindings = (0..fields.unnamed.len())
                .map(|index| format_ident!("field_{index}"))
                .collect::<Vec<_>>();
            let field_calls = bindings
                .iter()
                .map(|binding| quote!(debug.field(#binding);));

            quote! {
                Self::#ident(#(#bindings),*) => {
                    let mut debug = f.debug_tuple(#debug_name);
                    #(#field_calls)*
                    debug.finish()
                }
            }
        }
        Fields::Named(fields) => {
            let bindings = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named field").clone())
                .collect::<Vec<_>>();
            let field_calls = bindings.iter().map(|binding| {
                let name = LitStr::new(&binding.to_string(), binding.span());
                quote!(debug.field(#name, #binding);)
            });

            quote! {
                Self::#ident { #(#bindings),* } => {
                    let mut debug = f.debug_struct(#debug_name);
                    #(#field_calls)*
                    debug.finish()
                }
            }
        }
    }
}

fn partial_eq_arm(_index: usize, variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;

    match &variant.fields {
        Fields::Unit => quote!((Self::#ident, Self::#ident) => true),
        Fields::Unnamed(fields) => {
            let lefts = (0..fields.unnamed.len())
                .map(|index| format_ident!("left_{index}"))
                .collect::<Vec<_>>();
            let rights = (0..fields.unnamed.len())
                .map(|index| format_ident!("right_{index}"))
                .collect::<Vec<_>>();
            let comparisons = lefts
                .iter()
                .zip(rights.iter())
                .map(|(left, right)| quote!(#left == #right));

            quote!((Self::#ident(#(#lefts),*), Self::#ident(#(#rights),*)) => true #(&& #comparisons)*)
        }
        Fields::Named(fields) => {
            let lefts = fields
                .named
                .iter()
                .map(|field| format_ident!("left_{}", field.ident.as_ref().expect("named field")))
                .collect::<Vec<_>>();
            let rights = fields
                .named
                .iter()
                .map(|field| format_ident!("right_{}", field.ident.as_ref().expect("named field")))
                .collect::<Vec<_>>();
            let left_pattern = fields.named.iter().zip(lefts.iter()).map(|(field, left)| {
                let ident = field.ident.as_ref().expect("named field");
                quote!(#ident: #left)
            });
            let right_pattern = fields
                .named
                .iter()
                .zip(rights.iter())
                .map(|(field, right)| {
                    let ident = field.ident.as_ref().expect("named field");
                    quote!(#ident: #right)
                });
            let comparisons = lefts
                .iter()
                .zip(rights.iter())
                .map(|(left, right)| quote!(#left == #right));

            quote! {
                (
                    Self::#ident { #(#left_pattern),* },
                    Self::#ident { #(#right_pattern),* }
                ) => true #(&& #comparisons)*
            }
        }
    }
}

fn hash_arm(index: usize, variant: &Variant) -> TokenStream2 {
    let ident = &variant.ident;

    match &variant.fields {
        Fields::Unit => quote! {
            Self::#ident => {
                ::core::hash::Hash::hash(&#index, state);
            }
        },
        Fields::Unnamed(fields) => {
            let bindings = (0..fields.unnamed.len())
                .map(|field_index| format_ident!("field_{field_index}"))
                .collect::<Vec<_>>();
            let hashes = bindings
                .iter()
                .map(|binding| quote!(::core::hash::Hash::hash(#binding, state);));

            quote! {
                Self::#ident(#(#bindings),*) => {
                    ::core::hash::Hash::hash(&#index, state);
                    #(#hashes)*
                }
            }
        }
        Fields::Named(fields) => {
            let bindings = fields
                .named
                .iter()
                .map(|field| field.ident.as_ref().expect("named field").clone())
                .collect::<Vec<_>>();
            let hashes = bindings
                .iter()
                .map(|binding| quote!(::core::hash::Hash::hash(#binding, state);));

            quote! {
                Self::#ident { #(#bindings),* } => {
                    ::core::hash::Hash::hash(&#index, state);
                    #(#hashes)*
                }
            }
        }
    }
}
