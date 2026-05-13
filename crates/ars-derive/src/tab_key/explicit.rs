use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, LitStr};

use super::attrs::{self, ExplicitTabKey};

pub(super) fn variant_key_arms(
    collections_path: &TokenStream,
    input: &DeriveInput,
    data: &syn::DataEnum,
) -> syn::Result<Vec<TokenStream>> {
    let mut kind = None;
    let mut int_keys = BTreeSet::new();
    let mut str_keys = BTreeSet::new();
    let mut uuid_keys = BTreeSet::new();
    let mut arms = Vec::new();

    for variant in &data.variants {
        let ident = &variant.ident;

        let Some(key) = attrs::variant_key(variant)? else {
            return Err(syn::Error::new_spanned(
                variant,
                "TabKey requires every variant to have #[tab_key(int = ...)], #[tab_key(str = ...)], or #[tab_key(uuid = ...)] when no enum-level strategy is set",
            ));
        };

        let key_kind = key.kind();

        if let Some(expected) = kind {
            if expected != key_kind {
                return Err(syn::Error::new_spanned(
                    variant,
                    "TabKey per-variant keys must all use the same kind; do not mix int, str, and uuid keys in one enum",
                ));
            }
        } else {
            kind = Some(key_kind);
        }

        match key {
            ExplicitTabKey::Int(literal) => {
                let value = literal.base10_parse::<u64>().map_err(|_| {
                    syn::Error::new_spanned(
                        &literal,
                        "TabKey int keys must be non-negative integer literals that fit in u64",
                    )
                })?;

                if !int_keys.insert(value) {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "TabKey variant keys must be unique",
                    ));
                }

                arms.push(quote!(Self::#ident => #collections_path::Key::int(#value)));
            }

            ExplicitTabKey::Str(literal) => {
                let value = literal.value();

                if !str_keys.insert(value) {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "TabKey variant keys must be unique",
                    ));
                }

                arms.push(quote!(Self::#ident => #collections_path::Key::str(#literal)));
            }

            ExplicitTabKey::Uuid(literal) => {
                let value = literal.value();
                let normalized = validate_uuid_literal(&literal, &value)?;

                if !uuid_keys.insert(normalized) {
                    return Err(syn::Error::new_spanned(
                        literal,
                        "TabKey variant keys must be unique",
                    ));
                }

                arms.push(quote! {
                    Self::#ident => #collections_path::Key::uuid(
                        <#collections_path::uuid::Uuid as ::core::str::FromStr>::from_str(#literal)
                            .expect("TabKey uuid literal was validated by derive")
                    )
                });
            }
        }
    }

    if arms.is_empty() {
        return Err(syn::Error::new_spanned(
            input,
            "TabKey requires at least one enum variant",
        ));
    }

    Ok(arms)
}

fn validate_uuid_literal(literal: &LitStr, value: &str) -> syn::Result<String> {
    let is_uuid_shape = value.len() == 36
        && value.char_indices().all(|(index, ch)| match index {
            8 | 13 | 18 | 23 => ch == '-',
            _ => ch.is_ascii_hexdigit(),
        });

    if !is_uuid_shape {
        return Err(syn::Error::new_spanned(
            literal,
            "TabKey uuid keys must be canonical UUID string literals",
        ));
    }

    Ok(value.to_ascii_lowercase())
}
