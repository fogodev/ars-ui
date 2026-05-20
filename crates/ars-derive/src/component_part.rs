use heck::ToKebabCase as _;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    Data, DeriveInput, Expr, Field, Fields, Generics, Lit, LitStr, Meta, Type, Variant, parse_quote,
};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "ComponentPart can only be derived for enums",
        ));
    };

    let scope = find_scope_attr(&input.attrs)?;

    let root_variant = validate_root_variant(data)?;

    let root_ident = &root_variant.ident;

    let name = &input.ident;

    let field_types = collect_field_types(data);

    let defaultless_field_types = collect_defaultless_field_types(data)?;

    let component_part_generics = with_field_bounds(
        &input.generics,
        &defaultless_field_types,
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

    let all_values = data
        .variants
        .iter()
        .map(all_value)
        .collect::<syn::Result<Vec<_>>>()?;

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
            const ROOT: Self = Self::#root_ident;

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
        attrs
            .first()
            .map_or_else(TokenStream::new, quote::ToTokens::to_token_stream),
        "ComponentPart requires a #[scope = \"kebab-case-name\"] attribute",
    ))
}

fn validate_root_variant(data: &syn::DataEnum) -> syn::Result<&Variant> {
    let Some(first_variant) = data.variants.first() else {
        return Err(syn::Error::new_spanned(
            data.enum_token,
            "ComponentPart requires a first unit variant to use as ROOT",
        ));
    };

    if !matches!(first_variant.fields, Fields::Unit) {
        return Err(syn::Error::new_spanned(
            first_variant,
            "ComponentPart requires the first variant to be a unit variant",
        ));
    }

    Ok(first_variant)
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

fn collect_defaultless_field_types(data: &syn::DataEnum) -> syn::Result<Vec<Type>> {
    let mut types = Vec::new();

    for variant in &data.variants {
        match &variant.fields {
            Fields::Unit => {}

            Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    if field_default_expr(field)?.is_none() {
                        types.push(field.ty.clone());
                    }
                }
            }

            Fields::Named(fields) => {
                for field in &fields.named {
                    if field_default_expr(field)?.is_none() {
                        types.push(field.ty.clone());
                    }
                }
            }
        }
    }

    Ok(types)
}

fn with_field_bounds(
    generics: &Generics,
    field_types: &[Type],
    bounds: &[TokenStream],
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

fn name_arm(variant: &Variant) -> TokenStream {
    let ident = &variant.ident;
    let name = LitStr::new(&ident.to_string().to_kebab_case(), ident.span());

    match &variant.fields {
        Fields::Unit => quote!(Self::#ident => #name),
        Fields::Unnamed(_) => quote!(Self::#ident(..) => #name),
        Fields::Named(_) => quote!(Self::#ident { .. } => #name),
    }
}

fn all_value(variant: &Variant) -> syn::Result<TokenStream> {
    let ident = &variant.ident;

    Ok(match &variant.fields {
        Fields::Unit => quote!(Self::#ident),

        Fields::Unnamed(fields) => {
            let defaults = fields
                .unnamed
                .iter()
                .map(field_all_expr)
                .collect::<syn::Result<Vec<_>>>()?;

            quote!(Self::#ident(#(#defaults),*))
        }

        Fields::Named(fields) => {
            let defaults = fields
                .named
                .iter()
                .map(|field| -> syn::Result<TokenStream> {
                    let ident = field.ident.as_ref().expect("named field");
                    let expr = field_all_expr(field)?;

                    Ok(quote!(#ident: #expr))
                })
                .collect::<syn::Result<Vec<_>>>()?;

            quote!(Self::#ident { #(#defaults),* })
        }
    })
}

fn field_all_expr(field: &Field) -> syn::Result<TokenStream> {
    Ok(if let Some(expr) = field_default_expr(field)? {
        quote!(#expr)
    } else {
        quote!(::core::default::Default::default())
    })
}

fn field_default_expr(field: &Field) -> syn::Result<Option<Expr>> {
    let mut default = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("part") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                let value = meta.value()?;
                let expr = value.parse::<Expr>()?;

                if default.replace(expr).is_some() {
                    return Err(meta.error("ComponentPart accepts only one field default"));
                }

                Ok(())
            } else {
                Err(meta.error("unknown ComponentPart field option; use default = expr"))
            }
        })?;
    }

    Ok(default)
}

fn clone_arm(variant: &Variant) -> TokenStream {
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

fn debug_arm(variant: &Variant) -> TokenStream {
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

fn partial_eq_arm(_index: usize, variant: &Variant) -> TokenStream {
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

fn hash_arm(index: usize, variant: &Variant) -> TokenStream {
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

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, parse_quote};

    use super::*;

    #[test]
    fn scope_attribute_rejects_malformed_shapes() {
        let list_scope: DeriveInput = parse_quote! {
            #[scope(component)]
            enum Part {
                Root,
            }
        };

        let non_literal_scope: DeriveInput = parse_quote! {
            #[scope = component]
            enum Part {
                Root,
            }
        };

        let non_string_scope: DeriveInput = parse_quote! {
            #[scope = 7]
            enum Part {
                Root,
            }
        };

        assert!(find_scope_attr(&list_scope.attrs).is_err());
        assert!(find_scope_attr(&non_literal_scope.attrs).is_err());
        assert!(find_scope_attr(&non_string_scope.attrs).is_err());
    }

    #[test]
    fn root_variant_validation_rejects_empty_and_payload_roots() {
        let empty: DeriveInput = parse_quote! {
            enum Part {}
        };

        let payload_root: DeriveInput = parse_quote! {
            enum Part {
                Root(String),
            }
        };

        let Data::Enum(empty_data) = empty.data else {
            unreachable!("test input is an enum");
        };

        let Data::Enum(payload_data) = payload_root.data else {
            unreachable!("test input is an enum");
        };

        assert!(validate_root_variant(&empty_data).is_err());
        assert!(validate_root_variant(&payload_data).is_err());
    }

    #[test]
    fn expand_covers_unit_tuple_and_named_variant_generation() {
        let input: DeriveInput = parse_quote! {
            #[scope = "tabs"]
            enum Part<T> {
                Root,
                Trigger(T),
                Panel { index: usize },
            }
        };

        let tokens = expand(&input)
            .expect("component part should expand")
            .to_string();

        assert!(tokens.contains("impl < T > :: ars_core :: ComponentPart for Part < T >"));
        assert!(tokens.contains("const ROOT : Self = Self :: Root"));
        assert!(tokens.contains("\"tabs\""));
        assert!(tokens.contains("\"trigger\""));
        assert!(tokens.contains("Self :: Trigger"));
        assert!(tokens.contains("Self :: Panel"));
        assert!(tokens.contains("debug_tuple"));
        assert!(tokens.contains("debug_struct"));
    }

    #[test]
    fn expand_uses_field_defaults_for_all_values() {
        let input: DeriveInput = parse_quote! {
            #[scope = "pagination"]
            enum Part {
                Root,
                PageTrigger {
                    #[part(default = 1)]
                    page_number: u32,
                },
                Link(
                    #[part(default = SafeUrl::from_static("/"))]
                    SafeUrl,
                ),
            }
        };

        let tokens = expand(&input)
            .expect("component part should expand")
            .to_string();

        assert!(tokens.contains("page_number : 1"));
        assert!(tokens.contains("SafeUrl :: from_static (\"/\")"));
    }

    #[test]
    fn field_defaults_reject_malformed_options() {
        let unknown_option: DeriveInput = parse_quote! {
            #[scope = "bad"]
            enum Part {
                Root,
                Item {
                    #[part(value = 1)]
                    index: usize,
                },
            }
        };

        let duplicate_default: DeriveInput = parse_quote! {
            #[scope = "bad"]
            enum Part {
                Root,
                Item {
                    #[part(default = 1, default = 2)]
                    index: usize,
                },
            }
        };

        assert!(expand(&unknown_option).is_err());
        assert!(expand(&duplicate_default).is_err());
    }

    #[test]
    fn expand_rejects_non_enum_inputs() {
        let input: DeriveInput = parse_quote! {
            #[scope = "tabs"]
            struct Part;
        };

        let error = expand(&input).expect_err("structs should be rejected");

        assert!(error.to_string().contains("only be derived for enums"));
    }
}
