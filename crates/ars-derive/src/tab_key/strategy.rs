use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Lit, LitInt};

pub(super) fn ordinal_arms(
    collections_path: &TokenStream,
    data: &syn::DataEnum,
) -> Vec<TokenStream> {
    data.variants
        .iter()
        .enumerate()
        .map(|(index, variant)| {
            let ident = &variant.ident;
            let key = index as u64;

            quote!(Self::#ident => #collections_path::Key::int(#key))
        })
        .collect()
}

pub(super) fn discriminant_arms(
    collections_path: &TokenStream,
    data: &syn::DataEnum,
) -> syn::Result<Vec<TokenStream>> {
    let mut seen = BTreeSet::new();

    let mut arms = Vec::new();

    for variant in &data.variants {
        let ident = &variant.ident;

        let Some((_eq, expr)) = &variant.discriminant else {
            return Err(syn::Error::new_spanned(
                variant,
                "TabKey #[tab_key(discriminant)] requires every variant to have an explicit non-negative integer literal discriminant",
            ));
        };

        let literal = integer_literal_discriminant(expr)?;

        let key = literal.base10_parse::<u64>().map_err(|_| {
            syn::Error::new_spanned(
                literal,
                "TabKey discriminants must be non-negative integer literals that fit in u64",
            )
        })?;

        if !seen.insert(key) {
            return Err(syn::Error::new_spanned(
                literal,
                "TabKey discriminants must be unique",
            ));
        }

        arms.push(quote!(Self::#ident => #collections_path::Key::int(#key)));
    }

    Ok(arms)
}

fn integer_literal_discriminant(expr: &Expr) -> syn::Result<&LitInt> {
    let Expr::Lit(expr_lit) = expr else {
        return Err(syn::Error::new_spanned(
            expr,
            "TabKey discriminants must be non-negative integer literals",
        ));
    };

    let Lit::Int(literal) = &expr_lit.lit else {
        return Err(syn::Error::new_spanned(
            expr,
            "TabKey discriminants must be non-negative integer literals",
        ));
    };

    Ok(literal)
}
