use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

mod attrs;
mod explicit;
mod strategy;

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "TabKey can only be derived for enums",
        ));
    };

    let enum_attrs = attrs::find_enum_attrs(input)?;

    let collections_path = attrs::resolve_collections_path(enum_attrs.crate_path.as_ref())?;

    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                variant,
                "TabKey can only be derived for fieldless enums with unit variants",
            ));
        }
    }

    let name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let arms = if let Some(strategy) = enum_attrs.strategy {
        attrs::reject_variant_key_attrs(data)?;

        match strategy {
            attrs::TabKeyStrategy::Ordinal => strategy::ordinal_arms(&collections_path, data),

            attrs::TabKeyStrategy::Discriminant => {
                strategy::discriminant_arms(&collections_path, data)?
            }
        }
    } else {
        explicit::variant_key_arms(&collections_path, input, data)?
    };

    Ok(quote! {
        impl #impl_generics #collections_path::TabKey for #name #ty_generics #where_clause {
            fn into_key(self) -> #collections_path::Key {
                match self {
                    #(#arms),*
                }
            }
        }

        impl #impl_generics ::core::convert::From<#name #ty_generics> for #collections_path::Key #where_clause {
            fn from(value: #name #ty_generics) -> Self {
                #collections_path::TabKey::into_key(value)
            }
        }
    })
}
