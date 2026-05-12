use std::collections::{BTreeMap, BTreeSet};

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{Data, DeriveInput, Expr, Fields, Ident, Lit, LitStr, Path, Variant};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "Translate can only be derived for enums",
        ));
    };

    let enum_attrs = find_enum_attrs(input)?;

    let fallback = enum_attrs.fallback.ok_or_else(|| {
        syn::Error::new_spanned(
            input,
            "Translate derive requires #[translate(fallback = \"en\")] on the enum",
        )
    })?;

    let i18n_path = resolve_i18n_path(enum_attrs.crate_path.as_ref())?;

    let name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let arms = data
        .variants
        .iter()
        .map(|variant| variant_arm(&i18n_path, &fallback, variant))
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        impl #impl_generics #i18n_path::Translate for #name #ty_generics #where_clause {
            fn translate(
                &self,
                locale: &#i18n_path::Locale,
                _intl: &dyn #i18n_path::IntlBackend,
            ) -> #i18n_path::__private::String {
                let __ars_translate_exact = locale.to_bcp47();
                let __ars_translate_language = locale.language();

                match self {
                    #(#arms),*
                }
            }
        }
    })
}

struct EnumAttrs {
    fallback: Option<String>,
    crate_path: Option<Path>,
}

fn find_enum_attrs(input: &DeriveInput) -> syn::Result<EnumAttrs> {
    let mut fallback = None;
    let mut crate_path = None;

    for attr in &input.attrs {
        if !attr.path().is_ident("translate") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("fallback") {
                let value = meta.value()?;

                let literal = value.parse::<LitStr>()?;

                let locale = normalize_locale(&literal.value());

                if fallback.replace(locale).is_some() {
                    return Err(meta.error("Translate accepts only one fallback locale"));
                }

                Ok(())
            } else if meta.path.is_ident("crate") {
                let value = meta.value()?;
                let path = value.parse::<Path>()?;

                if crate_path.replace(path).is_some() {
                    return Err(meta.error("Translate accepts only one crate path override"));
                }

                Ok(())
            } else {
                Err(meta
                    .error("unknown Translate enum option; use fallback = \"en\" or crate = path"))
            }
        })?;
    }

    Ok(EnumAttrs {
        fallback,
        crate_path,
    })
}

fn resolve_i18n_path(override_path: Option<&Path>) -> syn::Result<TokenStream> {
    if let Some(path) = override_path {
        return Ok(quote!(::#path));
    }

    if let Some(path) = dependency_path("ars-i18n")? {
        return Ok(path);
    }

    let leptos = dependency_path("ars-leptos")?;
    let dioxus = dependency_path("ars-dioxus")?;

    match (leptos, dioxus) {
        (Some(path), None) | (None, Some(path)) => Ok(path),

        (Some(_), Some(_)) => Err(syn::Error::new(
            Span::call_site(),
            "Translate could not choose between ars-leptos and ars-dioxus; add #[translate(crate = ars_leptos)] or #[translate(crate = ars_dioxus)]",
        )),

        (None, None) => Err(syn::Error::new(
            Span::call_site(),
            "Translate could not find ars-i18n, ars-leptos, or ars-dioxus in Cargo.toml; add one as a direct dependency or use #[translate(crate = path)]",
        )),
    }
}

fn dependency_path(package: &str) -> syn::Result<Option<TokenStream>> {
    match crate_name(package) {
        Ok(FoundCrate::Itself) => {
            let ident = Ident::new(&package.replace('-', "_"), Span::call_site());

            Ok(Some(quote!(::#ident)))
        }

        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());

            Ok(Some(quote!(::#ident)))
        }

        Err(proc_macro_crate::Error::CrateNotFound { .. }) => Ok(None),

        Err(error) => Err(syn::Error::new(
            Span::call_site(),
            format!("Translate could not inspect Cargo.toml dependency metadata: {error}"),
        )),
    }
}

fn variant_arm(
    i18n_path: &TokenStream,
    fallback: &str,
    variant: &Variant,
) -> syn::Result<TokenStream> {
    let translations = variant_translations(variant)?;

    if !translations.contains_key(fallback) {
        return Err(syn::Error::new_spanned(
            variant,
            format!(
                "Translate variant `{}` must define fallback locale `{fallback}`",
                variant.ident
            ),
        ));
    }

    let fields = match &variant.fields {
        Fields::Unit => Vec::new(),

        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|field| {
                field.ident.clone().ok_or_else(|| {
                    syn::Error::new_spanned(field, "Translate named field is missing an identifier")
                })
            })
            .collect::<syn::Result<Vec<_>>>()?,

        Fields::Unnamed(_) => {
            return Err(syn::Error::new_spanned(
                variant,
                "Translate derive supports only unit variants and variants with named fields",
            ));
        }
    };

    for (locale, message) in &translations {
        validate_placeholders(variant, locale, message, &fields)?;
    }

    let variant_ident = &variant.ident;

    let pattern = if fields.is_empty() {
        quote!(Self::#variant_ident)
    } else {
        quote!(Self::#variant_ident { #(#fields),* })
    };

    let exact_arms = translations.iter().map(|(locale, message)| {
        let expr = message_expr(i18n_path, message, &fields);

        quote!(#locale => #expr)
    });

    let language_arms = language_fallbacks(&translations)
        .into_iter()
        .map(|(language, message)| {
            let expr = message_expr(i18n_path, &message, &fields);

            quote!(#language => #expr)
        });

    let fallback_message = translations
        .get(fallback)
        .expect("fallback presence checked above");
    let fallback_expr = message_expr(i18n_path, fallback_message, &fields);

    Ok(quote! {
        #pattern => match __ars_translate_exact.as_str() {
            #(#exact_arms,)*
            _ => match __ars_translate_language {
                #(#language_arms,)*
                _ => #fallback_expr,
            },
        }
    })
}

fn variant_translations(variant: &Variant) -> syn::Result<BTreeMap<String, LitStr>> {
    let mut translations = BTreeMap::new();

    for attr in &variant.attrs {
        if !attr.path().is_ident("translate") {
            continue;
        }

        let mut explicit_locale = None;
        let mut explicit_text = None;

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("locale") {
                let value = meta.value()?;
                let literal = value.parse::<LitStr>()?;

                if explicit_locale.replace(literal).is_some() {
                    return Err(meta.error("Translate accepts only one locale = ... per attribute"));
                }

                Ok(())
            } else if meta.path.is_ident("text") {
                let value = meta.value()?;
                let literal = value.parse::<LitStr>()?;

                if explicit_text.replace(literal).is_some() {
                    return Err(meta.error("Translate accepts only one text = ... per attribute"));
                }

                Ok(())
            } else {
                let locale = locale_from_path(&meta.path)?;
                let value = meta.value()?;

                let Expr::Lit(expr_lit) = value.parse::<Expr>()? else {
                    return Err(meta.error("Translate messages must be string literals"));
                };

                let Lit::Str(message) = expr_lit.lit else {
                    return Err(meta.error("Translate messages must be string literals"));
                };

                insert_translation(&mut translations, &meta.path, &locale, message)
            }
        })?;

        match (explicit_locale, explicit_text) {
            (Some(locale), Some(text)) => {
                let normalized = normalize_locale(&locale.value());

                insert_translation(&mut translations, &locale, &normalized, text)?;
            }

            (None, None) => {}

            (Some(locale), None) => {
                return Err(syn::Error::new_spanned(
                    locale,
                    "Translate explicit locale attributes must include text = \"...\"",
                ));
            }
            (None, Some(text)) => {
                return Err(syn::Error::new_spanned(
                    text,
                    "Translate explicit text attributes must include locale = \"...\"",
                ));
            }
        }
    }

    if translations.is_empty() {
        return Err(syn::Error::new_spanned(
            variant,
            "Translate variants must have at least one #[translate(...)] message",
        ));
    }

    Ok(translations)
}

fn locale_from_path(path: &Path) -> syn::Result<String> {
    let Some(ident) = path.get_ident() else {
        return Err(syn::Error::new_spanned(
            path,
            "Translate locale keys must be identifiers like en or pt_BR",
        ));
    };

    Ok(normalize_locale(&ident.to_string()))
}

fn insert_translation(
    translations: &mut BTreeMap<String, LitStr>,
    span: &impl ToTokens,
    locale: &str,
    message: LitStr,
) -> syn::Result<()> {
    if translations.insert(locale.to_string(), message).is_some() {
        return Err(syn::Error::new_spanned(
            span,
            format!("Translate duplicate locale `{locale}`"),
        ));
    }

    Ok(())
}

fn validate_placeholders(
    variant: &Variant,
    locale: &str,
    message: &LitStr,
    fields: &[Ident],
) -> syn::Result<()> {
    let available = fields.iter().map(Ident::to_string).collect::<BTreeSet<_>>();

    for placeholder in placeholders(message)? {
        if !available.contains(&placeholder) {
            return Err(syn::Error::new_spanned(
                message,
                format!(
                    "Translate message for variant `{}` locale `{locale}` references unknown field `{placeholder}`",
                    variant.ident
                ),
            ));
        }
    }

    Ok(())
}

fn placeholders(message: &LitStr) -> syn::Result<Vec<String>> {
    let value = message.value();
    let mut placeholders = Vec::new();

    let mut chars = value.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        match ch {
            '{' => {
                if chars.next_if(|(_, inner)| *inner == '{').is_some() {
                    continue;
                }

                let mut name = String::new();
                let mut closed = false;

                for (_, inner) in chars.by_ref() {
                    if inner == '}' {
                        closed = true;
                        break;
                    }

                    if !(inner == '_' || inner.is_ascii_alphanumeric()) {
                        return Err(syn::Error::new_spanned(
                            message,
                            "Translate placeholders must use Rust field identifiers like {count}",
                        ));
                    }

                    name.push(inner);
                }

                if !closed || name.is_empty() {
                    return Err(syn::Error::new_spanned(
                        message,
                        format!(
                            "Translate message has malformed placeholder starting at byte {start}"
                        ),
                    ));
                }

                placeholders.push(name);
            }
            '}' => {
                if chars.next_if(|(_, inner)| *inner == '}').is_some() {
                    continue;
                }

                return Err(syn::Error::new_spanned(
                    message,
                    "Translate message has unmatched `}`",
                ));
            }
            _ => {}
        }
    }

    Ok(placeholders)
}

fn message_expr(i18n_path: &TokenStream, message: &LitStr, fields: &[Ident]) -> TokenStream {
    let used_fields = placeholders(message).expect("message placeholders validated");

    let mut seen_fields = BTreeSet::new();
    let used_fields = used_fields
        .iter()
        .filter(|field| seen_fields.insert((*field).clone()))
        .map(|field| Ident::new(field, message.span()))
        .collect::<Vec<_>>();

    if used_fields.is_empty() {
        quote!(#i18n_path::__private::String::from(#message))
    } else {
        let _ = fields;

        quote!(#i18n_path::__private::format!(#message, #(#used_fields = #used_fields),*))
    }
}

fn language_fallbacks(translations: &BTreeMap<String, LitStr>) -> Vec<(String, LitStr)> {
    let mut seen = BTreeSet::new();
    let mut fallbacks = Vec::new();

    for (locale, message) in translations {
        if locale.contains('-') {
            continue;
        }

        if seen.insert(locale.clone()) {
            fallbacks.push((locale.clone(), message.clone()));
        }
    }

    fallbacks
}

fn normalize_locale(locale: &str) -> String {
    locale
        .split(['-', '_'])
        .enumerate()
        .map(|(index, part)| normalize_locale_part(index, part))
        .collect::<Vec<_>>()
        .join("-")
}

fn normalize_locale_part(index: usize, part: &str) -> String {
    if index == 0 {
        return part.to_ascii_lowercase();
    }

    if part.len() == 4 && part.chars().all(|ch| ch.is_ascii_alphabetic()) {
        let mut chars = part.chars();

        let first = chars
            .next()
            .expect("length checked above")
            .to_ascii_uppercase();

        let rest = chars.as_str().to_ascii_lowercase();

        return format!("{first}{rest}");
    }

    if (part.len() == 2 && part.chars().all(|ch| ch.is_ascii_alphabetic()))
        || (part.len() == 3 && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return part.to_ascii_uppercase();
    }

    part.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, LitStr, Path, Variant, parse_quote};

    use super::*;

    #[test]
    fn expand_rejects_non_enum_and_missing_fallback() {
        let input: DeriveInput = parse_quote! {
            struct Text;
        };

        let error = expand(&input).expect_err("structs should be rejected");

        assert!(error.to_string().contains("only be derived for enums"));

        let input: DeriveInput = parse_quote! {
            enum Text {
                #[translate(en = "Hello")]
                Hello,
            }
        };

        let error = expand(&input).expect_err("missing fallback should be rejected");

        assert!(error.to_string().contains("requires #[translate"));
    }

    #[test]
    fn enum_attrs_cover_duplicate_and_unknown_options() {
        let duplicate_fallback: DeriveInput = parse_quote! {
            #[translate(fallback = "en", fallback = "pt")]
            enum Text {
                #[translate(en = "Hello")]
                Hello,
            }
        };

        let Err(error) = find_enum_attrs(&duplicate_fallback) else {
            panic!("duplicate fallback should fail");
        };

        assert!(error.to_string().contains("only one fallback locale"));

        let duplicate_crate: DeriveInput = parse_quote! {
            #[translate(fallback = "en", crate = ars_i18n, crate = ars_i18n)]
            enum Text {
                #[translate(en = "Hello")]
                Hello,
            }
        };

        let Err(error) = find_enum_attrs(&duplicate_crate) else {
            panic!("duplicate crate should fail");
        };

        assert!(error.to_string().contains("only one crate path override"));

        let unknown: DeriveInput = parse_quote! {
            #[translate(fallback = "en", domain = "widgets")]
            enum Text {
                #[translate(en = "Hello")]
                Hello,
            }
        };

        let Err(error) = find_enum_attrs(&unknown) else {
            panic!("unknown option should fail");
        };

        assert!(error.to_string().contains("unknown Translate enum option"));
    }

    #[test]
    fn i18n_path_resolution_covers_override_itself_and_missing_dependency() {
        let path: Path = parse_quote!(my_crate::i18n);

        let resolved = resolve_i18n_path(Some(&path)).expect("override should resolve");

        assert_eq!(resolved.to_string(), ":: my_crate :: i18n");

        let itself = dependency_path("ars-derive").expect("crate metadata should be readable");

        assert!(itself.is_some());

        let missing =
            dependency_path("ars-ui-not-a-real-crate").expect("missing crates are not hard errors");

        assert!(missing.is_none());
    }

    #[test]
    fn variant_translations_cover_explicit_and_identifier_locale_forms() {
        let variant: Variant = parse_quote! {
            #[translate(en_US = "Color")]
            #[translate(locale = "pt_BR", text = "Cor")]
            Label
        };

        let translations = variant_translations(&variant).expect("translations should parse");

        assert_eq!(
            translations
                .get("en-US")
                .expect("normalized locale")
                .value(),
            "Color"
        );
        assert_eq!(
            translations.get("pt-BR").expect("explicit locale").value(),
            "Cor"
        );
    }

    #[test]
    fn variant_translations_reject_bad_attribute_shapes() {
        let duplicate_locale: Variant = parse_quote! {
            #[translate(locale = "en", locale = "pt", text = "Hello")]
            Label
        };

        let duplicate_text: Variant = parse_quote! {
            #[translate(locale = "en", text = "Hello", text = "Again")]
            Label
        };

        let only_locale: Variant = parse_quote! {
            #[translate(locale = "en")]
            Label
        };

        let only_text: Variant = parse_quote! {
            #[translate(text = "Hello")]
            Label
        };

        let non_string_message: Variant = parse_quote! {
            #[translate(en = 7)]
            Label
        };

        assert!(variant_translations(&duplicate_locale).is_err());
        assert!(variant_translations(&duplicate_text).is_err());
        assert!(variant_translations(&only_locale).is_err());
        assert!(variant_translations(&only_text).is_err());
        assert!(variant_translations(&non_string_message).is_err());
    }

    #[test]
    fn locale_and_placeholder_helpers_cover_error_and_normalization_paths() {
        let path: Path = parse_quote!(foo::bar);

        let non_ident = locale_from_path(&path).expect_err("path locales should be rejected");

        assert!(non_ident.to_string().contains("must be identifiers"));

        let message = LitStr::new("{count} items with {{literal}} braces", Span::call_site());

        assert_eq!(
            placeholders(&message).expect("valid placeholder"),
            ["count"]
        );

        let unclosed = LitStr::new("{count items", Span::call_site());
        let empty = LitStr::new("{} items", Span::call_site());
        let bad_char = LitStr::new("{item-count}", Span::call_site());
        let unmatched = LitStr::new("items}", Span::call_site());

        assert!(placeholders(&unclosed).is_err());
        assert!(placeholders(&empty).is_err());
        assert!(placeholders(&bad_char).is_err());
        assert!(placeholders(&unmatched).is_err());

        assert_eq!(normalize_locale("ZH_hant_tw"), "zh-Hant-TW");
        assert_eq!(normalize_locale("en_001"), "en-001");
        assert_eq!(normalize_locale("sl_rozaj"), "sl-rozaj");
    }

    #[test]
    fn language_fallbacks_only_use_explicit_base_language_messages() {
        let mut translations = BTreeMap::new();

        translations.insert("en".to_string(), LitStr::new("English", Span::call_site()));
        translations.insert(
            "en-US".to_string(),
            LitStr::new("American English", Span::call_site()),
        );
        translations.insert(
            "pt-BR".to_string(),
            LitStr::new("Portuguese", Span::call_site()),
        );

        let fallbacks = language_fallbacks(&translations);

        assert_eq!(fallbacks.len(), 1);
        assert_eq!(fallbacks[0].0, "en");
        assert_eq!(fallbacks[0].1.value(), "English");
    }

    #[test]
    fn variant_arm_covers_placeholder_validation_and_message_generation() {
        let i18n_path = quote!(::ars_i18n);

        let variant: Variant = parse_quote! {
            #[translate(en = "{count} items")]
            #[translate(pt_BR = "{count} itens")]
            ItemCount { count: usize }
        };

        let tokens = variant_arm(&i18n_path, "en", &variant)
            .expect("named fields with matching placeholders should expand")
            .to_string();

        assert!(tokens.contains("Self :: ItemCount"));
        assert!(tokens.contains("__ars_translate_exact"));
        assert!(tokens.contains("__ars_translate_language"));
        assert!(tokens.contains(":: ars_i18n :: __private :: format !"));
        assert!(tokens.contains("count = count"));

        let unknown_field: Variant = parse_quote! {
            #[translate(en = "{total} items")]
            ItemCount { count: usize }
        };

        let error = variant_arm(&i18n_path, "en", &unknown_field)
            .expect_err("unknown placeholders should be rejected");

        assert!(
            error
                .to_string()
                .contains("references unknown field `total`")
        );

        let missing_fallback: Variant = parse_quote! {
            #[translate(pt = "Olá")]
            Greeting
        };

        let error = variant_arm(&i18n_path, "en", &missing_fallback)
            .expect_err("fallback locale must be present");

        assert!(
            error
                .to_string()
                .contains("must define fallback locale `en`")
        );

        let tuple_variant: Variant = parse_quote! {
            #[translate(en = "Hello")]
            Greeting(String)
        };

        let error = variant_arm(&i18n_path, "en", &tuple_variant).expect_err("tuple variants fail");

        assert!(error.to_string().contains("supports only unit variants"));
    }
}
