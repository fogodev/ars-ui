use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type, Visibility};

pub(crate) fn expand(input: &DeriveInput) -> syn::Result<TokenStream> {
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

fn find_id_field(data: &syn::DataStruct) -> Option<&syn::Field> {
    let Fields::Named(fields) = &data.fields else {
        return None;
    };

    fields
        .named
        .iter()
        .find(|field| field.ident.as_ref().is_some_and(|ident| ident == "id"))
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

#[cfg(test)]
mod tests {
    use syn::{DeriveInput, Type, parse_quote};

    use super::*;

    #[test]
    fn id_field_lookup_rejects_tuple_structs_and_non_path_types() {
        let input: DeriveInput = parse_quote! {
            struct Props(String);
        };

        let reference: Type = parse_quote!(&'static str);

        let Data::Struct(tuple) = input.data else {
            unreachable!("test input is a struct");
        };

        assert!(find_id_field(&tuple).is_none());
        assert!(!is_string_type(&reference));
    }
}
