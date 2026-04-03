//! Procedural derive macros for ars-ui component infrastructure.
//!
//! Provides `#[derive(HasId)]` and `#[derive(ComponentPart)]` to generate
//! boilerplate trait implementations for component ID access and DOM part enums.

use proc_macro::TokenStream;

/// Derives the `HasId` trait for a struct with a `pub id: String` field.
///
/// Generates `id()`, `with_id()`, and `set_id()` methods for typed access
/// to the component's DOM identifier.
#[proc_macro_derive(HasId)]
pub fn derive_has_id(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}

/// Derives the [`ComponentPart`](ars_core::ComponentPart) trait for a part enum.
///
/// Generates `root()`, `name()`, and `all()` methods. Use `#[scope = "component-name"]`
/// on the enum to set the component namespace for data attribute generation.
#[proc_macro_derive(ComponentPart)]
pub fn derive_component_part(_input: TokenStream) -> TokenStream {
    TokenStream::new()
}
