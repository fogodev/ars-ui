use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Expr, Lit, LitInt, LitStr, Meta, Path, Variant};

#[derive(Clone, Copy)]
pub(super) enum TabKeyStrategy {
    Ordinal,
    Discriminant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ExplicitTabKeyKind {
    Int,
    Str,
    Uuid,
}

pub(super) enum ExplicitTabKey {
    Int(LitInt),
    Str(LitStr),
    Uuid(LitStr),
}

pub(super) struct EnumAttrs {
    pub(super) strategy: Option<TabKeyStrategy>,
    pub(super) crate_path: Option<Path>,
}

pub(super) fn find_enum_attrs(input: &DeriveInput) -> syn::Result<EnumAttrs> {
    let mut strategy = None;
    let mut crate_path = None;

    for attr in &input.attrs {
        if !attr.path().is_ident("tab_key") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("ordinal") {
                if strategy.replace(TabKeyStrategy::Ordinal).is_some() {
                    return Err(meta.error("TabKey accepts only one enum-level strategy"));
                }

                Ok(())
            } else if meta.path.is_ident("discriminant") {
                if strategy.replace(TabKeyStrategy::Discriminant).is_some() {
                    return Err(meta.error("TabKey accepts only one enum-level strategy"));
                }

                Ok(())
            } else if meta.path.is_ident("crate") {
                let value = meta.value()?;
                let path = value.parse::<Path>()?;

                if crate_path.replace(path).is_some() {
                    return Err(meta.error("TabKey accepts only one crate path override"));
                }

                Ok(())
            } else {
                Err(meta.error("unknown TabKey option; use ordinal, discriminant, or crate = path"))
            }
        })?;
    }

    Ok(EnumAttrs {
        strategy,
        crate_path,
    })
}

pub(super) fn resolve_collections_path(override_path: Option<&Path>) -> syn::Result<TokenStream> {
    if let Some(path) = override_path {
        return Ok(quote!(::#path));
    }

    if let Some(path) = dependency_path("ars-collections")? {
        return Ok(path);
    }

    let leptos = dependency_path("ars-leptos")?;
    let dioxus = dependency_path("ars-dioxus")?;

    match (leptos, dioxus) {
        (Some(path), None) | (None, Some(path)) => Ok(path),
        (Some(_), Some(_)) => Err(syn::Error::new(
            Span::call_site(),
            "TabKey could not choose between ars-leptos and ars-dioxus; add #[tab_key(crate = ars_leptos)] or #[tab_key(crate = ars_dioxus)]",
        )),
        (None, None) => Err(syn::Error::new(
            Span::call_site(),
            "TabKey could not find ars-collections, ars-leptos, or ars-dioxus in Cargo.toml; add one as a direct dependency or use #[tab_key(crate = path)]",
        )),
    }
}

fn dependency_path(package: &str) -> syn::Result<Option<TokenStream>> {
    match crate_name(package) {
        Ok(FoundCrate::Itself) => {
            let ident = syn::Ident::new(&package.replace('-', "_"), Span::call_site());

            Ok(Some(quote!(::#ident)))
        }

        Ok(FoundCrate::Name(name)) => {
            let ident = syn::Ident::new(&name, Span::call_site());

            Ok(Some(quote!(::#ident)))
        }

        Err(proc_macro_crate::Error::CrateNotFound { .. }) => Ok(None),

        Err(error) => Err(syn::Error::new(
            Span::call_site(),
            format!("TabKey could not inspect Cargo.toml dependency metadata: {error}"),
        )),
    }
}

pub(super) fn reject_variant_key_attrs(data: &syn::DataEnum) -> syn::Result<()> {
    for variant in &data.variants {
        for attr in &variant.attrs {
            if attr.path().is_ident("tab_key") {
                return Err(syn::Error::new_spanned(
                    attr,
                    "TabKey enum-level strategies cannot be mixed with per-variant #[tab_key(... = ...)] attributes",
                ));
            }
        }
    }

    Ok(())
}

pub(super) fn variant_key(variant: &Variant) -> syn::Result<Option<ExplicitTabKey>> {
    let mut found = None;

    for attr in &variant.attrs {
        if !attr.path().is_ident("tab_key") {
            continue;
        }

        if found.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "TabKey accepts only one #[tab_key(...)] key attribute per variant",
            ));
        }

        let meta = attr.parse_args::<Meta>().map_err(|_| {
            syn::Error::new_spanned(
                attr,
                "TabKey variant key must be #[tab_key(int = 42)], #[tab_key(str = \"profile\")], or #[tab_key(uuid = \"018f9b58-8f3d-7c8b-9d71-000000000000\")]",
            )
        })?;

        let Meta::NameValue(name_value) = meta else {
            return Err(syn::Error::new_spanned(
                attr,
                "TabKey variant key must be #[tab_key(int = 42)], #[tab_key(str = \"profile\")], or #[tab_key(uuid = \"018f9b58-8f3d-7c8b-9d71-000000000000\")]",
            ));
        };

        let Expr::Lit(expr_lit) = name_value.value else {
            return Err(syn::Error::new_spanned(
                name_value,
                "TabKey variant key values must be literals",
            ));
        };

        let key = if name_value.path.is_ident("int") {
            let Lit::Int(literal) = expr_lit.lit else {
                return Err(syn::Error::new_spanned(
                    expr_lit,
                    "TabKey int keys must be non-negative integer literals",
                ));
            };

            ExplicitTabKey::Int(literal)
        } else if name_value.path.is_ident("str") {
            let Lit::Str(literal) = expr_lit.lit else {
                return Err(syn::Error::new_spanned(
                    expr_lit,
                    "TabKey str keys must be string literals",
                ));
            };

            ExplicitTabKey::Str(literal)
        } else if name_value.path.is_ident("uuid") {
            let Lit::Str(literal) = expr_lit.lit else {
                return Err(syn::Error::new_spanned(
                    expr_lit,
                    "TabKey uuid keys must be string literals",
                ));
            };

            ExplicitTabKey::Uuid(literal)
        } else {
            return Err(syn::Error::new_spanned(
                name_value.path,
                "unknown TabKey variant key kind; use int, str, or uuid",
            ));
        };

        found = Some(key);
    }

    Ok(found)
}

impl ExplicitTabKey {
    pub(super) const fn kind(&self) -> ExplicitTabKeyKind {
        match self {
            Self::Int(_) => ExplicitTabKeyKind::Int,
            Self::Str(_) => ExplicitTabKeyKind::Str,
            Self::Uuid(_) => ExplicitTabKeyKind::Uuid,
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, Path, Variant, parse_quote};

    use super::*;

    #[test]
    fn enum_attrs_reject_duplicate_strategy_and_crate_overrides() {
        let duplicate_strategy: DeriveInput = parse_quote! {
            #[tab_key(ordinal, discriminant)]
            enum Tabs {
                Profile,
            }
        };

        let Err(error) = find_enum_attrs(&duplicate_strategy) else {
            panic!("strategy should be rejected");
        };

        assert!(error.to_string().contains("only one enum-level strategy"));

        let duplicate_crate: DeriveInput = parse_quote! {
            #[tab_key(crate = ars_collections, crate = ars_collections)]
            enum Tabs {
                Profile,
            }
        };

        let Err(error) = find_enum_attrs(&duplicate_crate) else {
            panic!("crate should be rejected");
        };

        assert!(error.to_string().contains("only one crate path override"));
    }

    #[test]
    fn enum_attrs_reject_unknown_options() {
        let input: DeriveInput = parse_quote! {
            #[tab_key(domain = widgets)]
            enum Tabs {
                Profile,
            }
        };

        let Err(error) = find_enum_attrs(&input) else {
            panic!("unknown option should be rejected");
        };

        assert!(error.to_string().contains("unknown TabKey option"));
    }

    #[test]
    fn dependency_path_covers_itself_and_missing_dependency() {
        let itself = dependency_path("ars-derive").expect("crate metadata should be readable");

        assert!(itself.is_some());

        let missing =
            dependency_path("ars-ui-not-a-real-crate").expect("missing crates are not hard errors");

        assert!(missing.is_none());
    }

    #[test]
    fn collections_path_uses_explicit_override() {
        let path: Path = parse_quote!(my_crate::collections);

        let resolved = resolve_collections_path(Some(&path)).expect("override should resolve");

        assert_eq!(resolved.to_string(), ":: my_crate :: collections");
    }

    #[test]
    fn variant_keys_cover_valid_literal_kinds() {
        let int_variant: Variant = parse_quote! {
            #[tab_key(int = 42)]
            Profile
        };

        let str_variant: Variant = parse_quote! {
            #[tab_key(str = "profile")]
            Profile
        };

        let uuid_variant: Variant = parse_quote! {
            #[tab_key(uuid = "018f9b58-8f3d-7c8b-9d71-000000000000")]
            Profile
        };

        assert!(matches!(
            variant_key(&int_variant)
                .expect("valid key")
                .expect("key")
                .kind(),
            ExplicitTabKeyKind::Int
        ));
        assert!(matches!(
            variant_key(&str_variant)
                .expect("valid key")
                .expect("key")
                .kind(),
            ExplicitTabKeyKind::Str
        ));
        assert!(matches!(
            variant_key(&uuid_variant)
                .expect("valid key")
                .expect("key")
                .kind(),
            ExplicitTabKeyKind::Uuid
        ));
    }

    #[test]
    fn variant_keys_reject_malformed_and_duplicate_attributes() {
        let malformed: Variant = parse_quote! {
            #[tab_key(int)]
            Profile
        };

        let duplicate: Variant = parse_quote! {
            #[tab_key(str = "profile")]
            #[tab_key(str = "again")]
            Profile
        };

        assert!(variant_key(&malformed).is_err());
        assert!(variant_key(&duplicate).is_err());
    }

    #[test]
    fn strategy_mode_rejects_variant_key_attributes() {
        let input: DeriveInput = parse_quote! {
            enum Tabs {
                #[tab_key(str = "profile")]
                Profile,
            }
        };

        let syn::Data::Enum(data) = input.data else {
            unreachable!("test input is an enum");
        };

        let error = reject_variant_key_attrs(&data).expect_err("variant key should be rejected");

        assert!(error.to_string().contains("cannot be mixed"));
    }
}
